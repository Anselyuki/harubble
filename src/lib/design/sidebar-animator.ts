/**
 * 侧栏动画编排器
 *
 * 负责侧栏展开/折叠的全流程命令式动画编排，基于 GSAP + Flip 插件实现。
 * 动画期间整个侧栏屏蔽 hover 与点击交互，直到所有阶段完成后恢复。
 *
 * ## 动画设计
 *
 * - Logo 字符旋转：折叠态字符逆时针旋转 -90°，展开态恢复 0°，带 stagger 依次触发
 * - Logo FLIP 布局切换：字符从竖向堆栈飞向横向双行（或反向），按堆栈底部优先顺序依次飞出
 * - 文字标签跟随：展开时标签不等侧栏完全展开，当可用空间达到标签宽度 50% 时即开始同速展开
 * - 侧栏宽度：展开 300ms / 折叠 200ms，使用 ios-spring 缓动
 * - 所有缓动曲线统一使用 iOS 风格 CustomEase（ios / ios-in / ios-out / ios-spring）
 *
 * ## 展开动画时间线
 *
 *  Phase 1 — 宽度展开 + 旋转（并行，300ms）
 *  ┌─────────────────────────────────────────────────────────────┐
 *  │ [0ms ─────────────────────────────────── 300ms]             │
 *  │  ├─ 侧栏宽度 56px → 248px (ios-spring)                     │
 *  │  ├─ 字符旋转 -90° → 0° (ios-spring, stagger 50ms)          │
 *  │  └─ 文字标签：可用空间 ≥ 标签宽度×50% 时开始同速展开        │
 *  └─────────────────────────────────────────────────────────────┘
 *
 *  Phase 2 — 底座向右展开（240ms）
 *  ┌─────────────────────────────────────────────────────────────┐
 *  │  字符旋转完成后，底座先延展到展开态右边界                  │
 *  └─────────────────────────────────────────────────────────────┘
 *
 *  Phase 3 — FLIP 堆栈弹出（240ms/字符，stagger 50ms，底部优先）
 *  ┌─────────────────────────────────────────────────────────────┐
 *  │ 字符按折叠态 Y 坐标从底到顶排序，依次飞向展开态目标位置      │
 *  │  ├─ 每个字符飞行 240ms (ios-spring)                         │
 *  │  └─ 容器高度同步过渡至目标高度 (240ms + totalStagger)        │
 *  └─────────────────────────────────────────────────────────────┘
 *
 *  完成 → 恢复交互、清理瞬态样式
 */
import { tick } from 'svelte';
import {
  gsap,
  Flip,
  reducedMotionQuery,
  awaitPaintReady,
} from '$lib/design/gsap';
import { runExpand, runExpandLogoOnly } from './sidebar-animator-expand';
import { runCollapse } from './sidebar-animator-collapse';

interface SidebarAnimatorConfig {
  shellEl: HTMLElement;
  sidebarEl: HTMLElement;
  logoCharEls: HTMLSpanElement[];
  logoContainerEl: HTMLDivElement;
  logoSlabEl: HTMLElement;
  navRegionEl: HTMLElement;
  collectionsRegionEl: HTMLElement;
  collectionsCollapsedEl: HTMLElement;
  bottomLabelEl: HTMLSpanElement;
  initialCollapsed: boolean;
  onContentInteractive: (interactive: boolean) => void;
  onContentSwitch: (collapsed: boolean) => void;
  onLayoutSwitch: (collapsed: boolean) => void;
  onComplete?: (collapsed: boolean) => void;
}

export interface SidebarAnimator {
  collapse(): void;
  expand(): void;
  /**
   * 仅展开 logo（拖曳松手专用）。
   *
   * 用于「拖曳从折叠态拉出」的松手吸附：侧栏宽度与内容布局已在拖曳期间随宽度
   * 实时切到展开态，这里只补 logo 的折叠竖排 → 展开横排 FLIP，**不**重置内容
   * 布局（避免内容从展开态闪回折叠态）。
   */
  expandLogoOnly(): void;
  /**
   * 拖曳期间实时切换侧栏内容布局（仅内容，不动 logo / slab）。
   *
   * 用于「拖曳实时展开」：随宽度跨越阈值时即时把导航、收藏区在折叠（图标）与
   * 展开（带标签）两种布局间切换，并正确处理折叠收藏浮层的内联 opacity/visibility
   * （否则上一轮动画或清理留下的内联样式会盖过 `.hidden` 类，导致浮层无法隐藏）。
   * logo 字形旋转、layout 折叠态与 slab 均保持不变，由松手后的 {@link expandLogoOnly}
   * 单独补完 logo FLIP。
   */
  previewContentCollapsed(collapsed: boolean): void;
  /**
   * 拖曳调整侧栏宽度时，实时把展开态 slab 右边界跟随侧栏宽度变化。
   *
   * 仅在「展开稳定态」下生效（committed 折叠态保持折叠宽度不跟随）：按
   * {@link measureExpandedSlabWidth} 的跟随插值重算 `--slab-width`，使 slab 右边界
   * 落在 logo 右缘与侧栏右缘之间，而 logo 本身不动。动画进行中（展开 / 折叠过渡）
   * 不介入，避免与正在运行的 slab 宽度 tween 抢写。
   *
   * @param sidebarWidth 当前侧栏宽度（px）
   */
  updateSlabFollow(sidebarWidth: number): void;
  interrupt(): void;
  dispose(): void;
  /** 外部直接同步折叠状态（跳过动画），用于拖曳松手后的吸附场景 */
  syncCollapsedState(collapsed: boolean): void;
}

type AnimationParams = ReturnType<typeof getAnimationParams>;

export interface AnimatorContext {
  config: SidebarAnimatorConfig;
  logoGlyphEls: HTMLElement[];
  params: AnimationParams;
  constants: {
    COLLAPSED_WIDTH: string;
    EXPANDED_WIDTH: string;
    COLLAPSED_WIDTH_VALUE: number;
    EXPANDED_WIDTH_VALUE: number;
  };
  isStale: (id: number) => boolean;
  setTimeline: (tl: gsap.core.Timeline | null) => void;
  applyExpandedWidthFrame: (
    measuredLogoWidth: number,
    currentWidth: number
  ) => void;
  flipPhase: (
    id: number,
    toCollapsed: boolean
  ) => Promise<gsap.core.Timeline | null>;
  commitState: (collapsed: boolean) => void;
  /** 等待首次 paint + 字体就绪，确保 getBoundingClientRect 度量准确 */
  awaitReady: () => Promise<void>;
}

const TIMING = {
  WIDTH_DUR: 300,
  ROTATE_DUR: 200,
  MOVE_DUR: 240,
  STAGGER: 0.05,
  FLIP_STAGGER: 0.025,
  CONTENT_FADE: 200,
  LABEL_DUR: 150,
  PHASE_GAP: 50,
  FLIP_DELAY: 0,
} as const;

const EXPANDED_WIDTH = '248px';
const COLLAPSED_WIDTH = '56px';
const EXPANDED_WIDTH_VALUE = Number.parseFloat(EXPANDED_WIDTH);
const COLLAPSED_WIDTH_VALUE = Number.parseFloat(COLLAPSED_WIDTH);
/**
 * 展开（横向）态 slab 右边界在「logo 字母内容块右边缘」与「侧栏右边界」之间的插值比例。
 *
 * slab 右边界不再固定贴着 logo 右缘，而是随侧栏宽度变化在 logo 右缘与侧栏右缘
 * 之间按此比例插值：`slab 宽度 = logoRight + (sidebarWidth - logoRight) * 比例`。
 * 这样拖曳调整侧栏宽度时 slab 右边界跟随侧栏移动，而 logo 本身保持不动。
 * 仅作用于展开态，折叠态宽度仍由 `COLLAPSED_WIDTH_VALUE - 10` 决定，不受影响。
 */
const SLAB_FOLLOW_RATIO = 0.75;
const COLLAPSED_COLLECTIONS_OVERLAY_PROPS =
  'opacity,visibility,position,top,left,right,zIndex,height,overflow,padding';
const LOGO_SLAB_INSETS = {
  expanded: {
    marginLeft: '0px',
    marginRight: '8px',
  },
  collapsed: {
    marginLeft: '0px',
    marginRight: '10px',
  },
} as const;

type LogoSlabInsetTarget = keyof typeof LOGO_SLAB_INSETS;

function getPinnedLogoWidthFrame(
  collapsedWidth: number,
  expandedWidth: number,
  progress: number,
  measuredLogoWidth = collapsedWidth
): { sidebarWidth: number; logoWidth: number; alignSelf: 'flex-start' } {
  const clampedProgress = Math.min(Math.max(progress, 0), 1);
  const sidebarWidth =
    collapsedWidth + (expandedWidth - collapsedWidth) * clampedProgress;

  return {
    sidebarWidth,
    logoWidth: measuredLogoWidth,
    alignSelf: 'flex-start',
  };
}

type RectLike = Pick<DOMRect, 'left' | 'top' | 'width' | 'height'>;

export function getCenterLockTransform(
  initialRect: RectLike,
  currentRect: RectLike,
  currentTransform: { x: number; y: number }
): { x: number; y: number } {
  const initialCenterX = initialRect.left + initialRect.width / 2;
  const initialCenterY = initialRect.top + initialRect.height / 2;
  const currentCenterX = currentRect.left + currentRect.width / 2;
  const currentCenterY = currentRect.top + currentRect.height / 2;

  return {
    x: currentTransform.x + initialCenterX - currentCenterX,
    y: currentTransform.y + initialCenterY - currentCenterY,
  };
}

/**
 * 计算保持元素左上角不变的锁定 transform。
 *
 * 与 {@link getCenterLockTransform} 不同，本函数锁定的是元素的左上角（left/top），
 * 而非几何中心。用于折叠收尾阶段：字符飞行目标与最终 `clearProps` 落点都以
 * 左上角为基准（取自折叠态克隆测量的 `cloneRect.left/top`），但锁定期间字符仍处于
 * 展开态布局，其行高（`line-height: 0.88`）比折叠态（`calc(0.88em - 2px)`）高约 2px。
 * 若按中心锁定，盒高差异会在布局切换瞬间造成约 1px 的垂直跳变；按左上角锁定可消除该跳变。
 *
 * @param initialRect 锁定起始时元素的边界矩形（飞行结束时捕获）
 * @param currentRect 当前帧元素的边界矩形
 * @param currentTransform 当前帧已应用的 x/y transform
 * @returns 使元素左上角对齐 `initialRect` 左上角所需的新 x/y transform
 */
export function getTopLeftLockTransform(
  initialRect: RectLike,
  currentRect: RectLike,
  currentTransform: { x: number; y: number }
): { x: number; y: number } {
  return {
    x: currentTransform.x + initialRect.left - currentRect.left,
    y: currentTransform.y + initialRect.top - currentRect.top,
  };
}

function resolveLogoGlyphEl(charEl: HTMLSpanElement): HTMLElement {
  return charEl.querySelector<HTMLElement>('[data-logo-glyph]') ?? charEl;
}

/**
 * 测量 logo 字母内容相对容器左边缘的右边界宽度。
 *
 * logo 容器是 grid-area:1/1 会被拉伸到整个 sidebar 宽，内层
 * .brand-logo-mark 作为 flex 子项默认 align-items:stretch 也会被撑满，
 * 两者的 offsetWidth/right 都不能用作 slab 宽度。
 * slab 应对齐最右侧字母的右边缘，因此取所有字母 right 的最大值
 * 减去容器 left，再加上 mark 的右内边距，得到字母内容块的真实右边界。
 */
export function measureLogoMarkWidth(logoContainerEl: HTMLElement): number {
  const charEls = logoContainerEl.querySelectorAll<HTMLElement>('.brand-char');
  if (charEls.length === 0) return logoContainerEl.offsetWidth;
  const containerLeft = logoContainerEl.getBoundingClientRect().left;
  let maxRight = 0;
  charEls.forEach((el) => {
    maxRight = Math.max(maxRight, el.getBoundingClientRect().right);
  });

  // 加上 .brand-logo-mark 的右内边距，保持与折叠态视觉留白一致
  const markEl = logoContainerEl.querySelector<HTMLElement>('.brand-logo-mark');
  const markPaddingRight = markEl
    ? Number.parseFloat(getComputedStyle(markEl).paddingRight) || 0
    : 0;

  return Math.max(0, maxRight - containerLeft + markPaddingRight);
}

/**
 * 计算展开（横向）态 slab 的目标宽度。
 *
 * slab 右边界在「logo 字母内容块右边缘」与「侧栏右边界」之间按
 * {@link SLAB_FOLLOW_RATIO} 插值：
 * `slabWidth = logoRight + (sidebarWidth - logoRight) * SLAB_FOLLOW_RATIO`。
 * 这样侧栏宽度变化（拖曳 / 吸附）时 slab 右边界跟随侧栏移动，而 logo 不动。
 * 当 `sidebarWidth` 小于 logo 右缘时退化为贴紧 logo 右缘（不会出现负向收缩）。
 *
 * 所有展开方向写入 `--slab-width` 的位置都应走此函数，保持静态同步、动画终点与
 * 中断恢复三处取值一致。
 *
 * @param logoContainerEl logo 容器元素，用于测量字母内容块右边缘
 * @param sidebarWidth 当前侧栏宽度（px），缺省时回退到展开态宽度 {@link EXPANDED_WIDTH_VALUE}
 */
export function measureExpandedSlabWidth(
  logoContainerEl: HTMLElement,
  sidebarWidth: number = EXPANDED_WIDTH_VALUE
): number {
  const logoRight = measureLogoMarkWidth(logoContainerEl);
  const available = Math.max(0, sidebarWidth - logoRight);
  return logoRight + available * SLAB_FOLLOW_RATIO;
}

/**
 * 读取壳层当前的侧栏宽度（`--sidebar-width` CSS 变量）。
 *
 * 拖曳 / 吸附期间侧栏宽度由外部实时写入 `--sidebar-width`，slab 跟随计算需要
 * 取这个实时值而非展开态常量。读取失败（变量缺失或非法）时回退到展开态宽度，
 * 保证 {@link measureExpandedSlabWidth} 始终拿到合理的基准。
 */
function readSidebarWidth(shellEl: HTMLElement): number {
  const raw = shellEl.style.getPropertyValue('--sidebar-width');
  const parsed = Number.parseFloat(raw);
  return Number.isFinite(parsed) ? parsed : EXPANDED_WIDTH_VALUE;
}

export function setLogoSlabInsets(
  slabEl: HTMLElement,
  target: LogoSlabInsetTarget
): void {
  const inset = LOGO_SLAB_INSETS[target];
  slabEl.style.marginLeft = inset.marginLeft;
  slabEl.style.marginRight = inset.marginRight;
}

export function animateLogoSlabInsets(
  tl: gsap.core.Timeline,
  slabEl: HTMLElement,
  target: LogoSlabInsetTarget,
  duration: number,
  ease: string,
  position: number | string = 0
): void {
  const inset = LOGO_SLAB_INSETS[target];
  tl.to(
    slabEl,
    {
      marginLeft: inset.marginLeft,
      marginRight: inset.marginRight,
      duration,
      ease,
    },
    position
  );
}

/**
 * 计算展开态 slab 的目标 marginRight。
 *
 * slab 右边界对齐 logo 容器的右边缘，
 * 即 marginRight = sidebar 宽度 - logo 容器宽度。
 */
function _getExpandedSlabRightForLogoWidth(logoWidth: number): string {
  const marginRight = Math.max(0, EXPANDED_WIDTH_VALUE - logoWidth);
  return `${marginRight}px`;
}

function getLogoFlipTargets(
  charEls: HTMLSpanElement[],
  _glyphEls: HTMLElement[]
): HTMLElement[] {
  return charEls;
}

function getLogoSlotClearProps(): string {
  return 'all';
}

function getLogoGlyphClearProps(target: 'collapsed' | 'expanded'): string {
  const props = [
    'gridArea',
    'height',
    'left',
    'maxHeight',
    'maxWidth',
    'minHeight',
    'minWidth',
    'padding',
    'position',
    'top',
    'transition',
    'width',
  ];

  if (target === 'expanded') {
    props.push('transform', 'translate', 'rotate', 'scale');
  }

  return props.join(',');
}

function collectSidebarItemLabelEls(
  regions: HTMLElement[],
  extraLabelEls: HTMLSpanElement[] = []
): HTMLSpanElement[] {
  const labels = [
    ...regions.flatMap((region) =>
      Array.from(
        region.querySelectorAll<HTMLSpanElement>('[data-sidebar-item-label]')
      )
    ),
    ...extraLabelEls,
  ];

  return Array.from(new Set(labels));
}

export function collectSidebarAnimatorLabelEls(
  config: Pick<
    SidebarAnimatorConfig,
    'navRegionEl' | 'collectionsRegionEl' | 'bottomLabelEl'
  >
): HTMLSpanElement[] {
  return collectSidebarItemLabelEls(
    [config.navRegionEl, config.collectionsRegionEl],
    [config.bottomLabelEl]
  );
}

function getAnimationParams() {
  const reduced = reducedMotionQuery?.matches ?? false;
  return {
    widthDur: reduced ? 0 : TIMING.WIDTH_DUR / 1000,
    rotateDur: reduced ? 0 : TIMING.ROTATE_DUR / 1000,
    moveDur: reduced ? 0 : TIMING.MOVE_DUR / 1000,
    stagger: reduced ? 0 : TIMING.STAGGER,
    flipStagger: reduced ? 0 : TIMING.FLIP_STAGGER,
    contentFade: reduced ? 0 : TIMING.CONTENT_FADE / 1000,
    labelDur: reduced ? 0 : TIMING.LABEL_DUR / 1000,
    phaseGap: reduced ? 0 : TIMING.PHASE_GAP / 1000,
    flipDelay: reduced ? 0 : TIMING.FLIP_DELAY / 1000,
    reduced,
  };
}

/**
 * 计算 slab 动画的目标 height。
 *
 * slab 通过 CSS Grid align-self:stretch 与 logo 容器等高，
 * 但自身有 margin-top / margin-bottom 排除了 logo 的外层 padding。
 * GSAP 动画设置的 height 是 content-box 高度，不含 margin——
 * 所以需要从 logo 容器完整高度中减去 slab 的垂直 margin。
 */
export function getSlabTargetHeight(
  slabEl: HTMLElement,
  logoTargetHeight: number
): number {
  const style = getComputedStyle(slabEl);
  const verticalMargin =
    Number.parseFloat(style.marginTop) + Number.parseFloat(style.marginBottom);
  return Math.max(0, logoTargetHeight - verticalMargin);
}

function syncToState(
  config: SidebarAnimatorConfig,
  collapsed: boolean,
  cachedHeight?: number | null
) {
  // slab 宽度通过 CSS 变量驱动，始终对齐字母内容块右边缘
  if (collapsed) {
    const slabWidth = COLLAPSED_WIDTH_VALUE - 10; // marginRight=10 equivalent
    config.logoSlabEl.style.setProperty('--slab-width', `${slabWidth}px`);
  } else {
    config.logoSlabEl.style.setProperty(
      '--slab-width',
      `${measureExpandedSlabWidth(
        config.logoContainerEl,
        readSidebarWidth(config.shellEl)
      )}px`
    );
  }

  config.logoCharEls.forEach((el) => {
    const glyphEl = resolveLogoGlyphEl(el);
    if (collapsed) {
      gsap.set(glyphEl, { rotation: -90 });
    } else {
      gsap.set(glyphEl, { clearProps: 'transform' });
    }
  });

  config.onLayoutSwitch(collapsed);
  config.onContentSwitch(collapsed);
  config.onContentInteractive(true);

  // 静态同步后，用 GSAP 锁定 slab 高度与 logo 内容区域一致
  // 优先使用缓存高度，避免在中断恢复时读到动画中间值
  const logoHeight = cachedHeight ?? config.logoContainerEl.offsetHeight;
  gsap.set(config.logoSlabEl, {
    height: getSlabTargetHeight(config.logoSlabEl, logoHeight),
  });
}

function cleanupTransientStyles(
  config: SidebarAnimatorConfig,
  target: 'collapsed' | 'expanded'
) {
  const labelEls = collectSidebarAnimatorLabelEls(config);

  gsap.set(config.logoContainerEl, {
    clearProps: 'height,overflow,width,visibility,transform,alignSelf',
  });

  // slab 清理：清除动画瞬态属性，CSS 变量 --slab-width 由 syncToState 管理
  gsap.set(config.logoSlabEl, { clearProps: 'height' });
  // 设置终态的 --slab-width
  if (target === 'expanded') {
    config.logoSlabEl.style.setProperty(
      '--slab-width',
      `${measureExpandedSlabWidth(
        config.logoContainerEl,
        readSidebarWidth(config.shellEl)
      )}px`
    );
  } else {
    const slabWidth = COLLAPSED_WIDTH_VALUE - 10;
    config.logoSlabEl.style.setProperty('--slab-width', `${slabWidth}px`);
  }
  gsap.set(labelEls, { clearProps: 'maxWidth,opacity' });
  gsap.set(config.navRegionEl, { clearProps: 'opacity' });
  gsap.set(config.collectionsRegionEl, { clearProps: 'opacity' });
  if (target === 'expanded') {
    gsap.set(config.collectionsCollapsedEl, {
      clearProps: COLLAPSED_COLLECTIONS_OVERLAY_PROPS,
    });
  } else {
    gsap.set(config.collectionsCollapsedEl, {
      opacity: 1,
      visibility: 'visible',
    });
  }

  config.logoCharEls.forEach((el) => {
    gsap.set(el, { clearProps: `visibility,${getLogoSlotClearProps()}` });
  });

  config.logoCharEls.forEach((el) =>
    gsap.set(resolveLogoGlyphEl(el), {
      clearProps: getLogoGlyphClearProps(target),
    })
  );
}

interface TimelineLike {
  totalDuration(): number;
  progress(): number;
  eventCallback(name: 'onComplete'): (() => void) | undefined;
  eventCallback(name: 'onComplete', callback: () => void): unknown;
  eventCallback(name: 'onInterrupt'): (() => void) | undefined;
  eventCallback(name: 'onInterrupt', callback: () => void): unknown;
}

type AwaitableTimeline = Omit<gsap.core.Timeline, 'then'> | TimelineLike;

/**
 * 将 timeline 的完成（或中断）事件包装为 Promise。
 *
 * timeline 自然完成或被 kill 时均会 resolve，避免 Promise 悬挂导致的内存泄漏。
 * 调用侧应配合 `isStale(id)` 判断动画是否仍有效。
 */
export function chainTimelineComplete(tl: AwaitableTimeline): Promise<void> {
  return new Promise<void>((resolve) => {
    if (tl.totalDuration() === 0 || tl.progress() >= 1) {
      resolve();
      return;
    }
    const existingOnComplete = tl.eventCallback('onComplete');
    tl.eventCallback('onComplete', () => {
      existingOnComplete?.();
      resolve();
    });
    // timeline 被 kill() 时触发 onInterrupt，防止 Promise 永不 resolve
    const existingOnInterrupt = tl.eventCallback('onInterrupt');
    tl.eventCallback('onInterrupt', () => {
      existingOnInterrupt?.();
      resolve();
    });
  });
}

export function createSidebarAnimator(
  config: SidebarAnimatorConfig
): SidebarAnimator {
  let currentAnimationId = 0;
  let currentTimeline: gsap.core.Timeline | null = null;
  let heightTween: gsap.core.Tween | null = null;
  /** flipPhase 中与 currentTimeline 独立的辅助 tween，需在中断时统一清理 */
  let auxiliaryTweens: gsap.core.Tween[] = [];
  let lastCommittedCollapsed = config.initialCollapsed;

  const logoGlyphEls = config.logoCharEls.map(resolveLogoGlyphEl);

  // --- 首帧测量保护：双帧 rAF + 字体就绪 ---
  let paintReadyResolved = false;
  const paintReadyPromise = awaitPaintReady().then(() => {
    paintReadyResolved = true;
  });

  async function awaitReady(): Promise<void> {
    if (paintReadyResolved) return;
    await paintReadyPromise;
  }

  // --- 终态高度缓存：中断恢复时避免读取动画中间值 ---
  const cachedLogoHeight: {
    collapsed: number | null;
    expanded: number | null;
  } = { collapsed: null, expanded: null };

  syncToState(config, config.initialCollapsed);
  // 初始化后立即缓存当前稳定态高度
  const initialKey = config.initialCollapsed ? 'collapsed' : 'expanded';
  cachedLogoHeight[initialKey] = config.logoContainerEl.offsetHeight;

  function startNewAnimation(): number {
    currentAnimationId++;
    currentTimeline?.kill();
    currentTimeline = null;
    heightTween?.kill();
    heightTween = null;
    auxiliaryTweens.forEach((t) => t.kill());
    auxiliaryTweens = [];
    normalizeToCommittedState();
    return currentAnimationId;
  }

  function normalizeToCommittedState() {
    const key = lastCommittedCollapsed ? 'collapsed' : 'expanded';
    syncToState(config, lastCommittedCollapsed, cachedLogoHeight[key]);
    cleanupTransientStyles(
      config,
      lastCommittedCollapsed ? 'collapsed' : 'expanded'
    );
  }

  function isStale(id: number): boolean {
    return id !== currentAnimationId;
  }

  function applyExpandedWidthFrame(
    currentWidth: number,
    measuredLogoWidth = COLLAPSED_WIDTH_VALUE
  ) {
    const { logoWidth, alignSelf } = getPinnedLogoWidthFrame(
      COLLAPSED_WIDTH_VALUE,
      EXPANDED_WIDTH_VALUE,
      (currentWidth - COLLAPSED_WIDTH_VALUE) /
        (EXPANDED_WIDTH_VALUE - COLLAPSED_WIDTH_VALUE),
      measuredLogoWidth
    );

    gsap.set(config.logoContainerEl, {
      width: logoWidth,
      alignSelf,
    });
  }

  async function flipPhase(
    id: number,
    toCollapsed: boolean,
    params: ReturnType<typeof getAnimationParams>
  ): Promise<gsap.core.Timeline | null> {
    const els = getLogoFlipTargets(config.logoCharEls, logoGlyphEls);

    config.logoContainerEl.style.height = `${config.logoContainerEl.offsetHeight}px`;
    if (!toCollapsed) {
      config.logoContainerEl.style.width = `${config.logoContainerEl.offsetWidth}px`;
    } else {
      config.logoContainerEl.style.overflow = 'hidden';
    }

    const state = Flip.getState(els);

    els.forEach((el) => (el.style.visibility = 'hidden'));

    if (!toCollapsed) {
      gsap.set(config.logoContainerEl, { clearProps: 'x' });
      gsap.set(logoGlyphEls, { clearProps: 'transform' });
    }

    config.onLayoutSwitch(toCollapsed);
    await tick();
    if (isStale(id)) return null;

    els.forEach((el) => (el.style.visibility = ''));

    if (!toCollapsed) {
      gsap.set(config.logoContainerEl, {
        width: '',
        overflow: 'hidden',
      });
    }

    const clone = config.logoContainerEl.cloneNode(true) as HTMLDivElement;
    clone.style.position = 'absolute';
    clone.style.visibility = 'hidden';
    clone.style.height = 'auto';
    clone.style.width = '';
    clone.style.overflow = '';
    clone.style.pointerEvents = 'none';
    config.logoContainerEl.parentElement!.appendChild(clone);
    const targetHeight = clone.offsetHeight;
    // 测量目标布局下字母内容块的真实右边界宽度（展开方向用于 slab 宽度终点），
    // 展开方向按当前侧栏宽度走跟随插值，与静态同步、中断恢复保持一致。
    const targetLogoWidth = toCollapsed
      ? measureLogoMarkWidth(clone)
      : measureExpandedSlabWidth(clone, readSidebarWidth(config.shellEl));
    clone.remove();

    const totalStagger = params.flipStagger * (els.length - 1);

    heightTween = gsap.to(config.logoContainerEl, {
      height: targetHeight,
      duration: params.moveDur + totalStagger,
      ease: 'ios-spring',
      onComplete: () => {
        heightTween = null;
      },
    });

    // slab 高度与 logo 容器同步动画（显式 GSAP 驱动）
    const slabHeightTween = gsap.to(config.logoSlabEl, {
      height: getSlabTargetHeight(config.logoSlabEl, targetHeight),
      duration: params.moveDur + totalStagger,
      ease: 'ios-spring',
    });
    auxiliaryTweens.push(slabHeightTween);

    // 展开方向：slab 宽度随字母散开同步展开至两行布局的真实宽度，
    // 与 FLIP 时间线同时结束，避免 FLIP 结束时骤然展开。
    if (!toCollapsed) {
      const slabWidthFrame = {
        width:
          Number.parseFloat(
            config.logoSlabEl.style.getPropertyValue('--slab-width')
          ) || config.logoSlabEl.offsetWidth,
      };
      const slabWidthTween = gsap.to(slabWidthFrame, {
        width: targetLogoWidth,
        duration: params.moveDur + totalStagger,
        ease: 'ios-spring',
        onUpdate: () => {
          config.logoSlabEl.style.setProperty(
            '--slab-width',
            `${slabWidthFrame.width}px`
          );
        },
      });
      auxiliaryTweens.push(slabWidthTween);
    }

    const sortedEls = [...els].sort((a, b) => {
      const rectA = state.elementStates.find(
        (s: { element: Element }) => s.element === a
      )!;
      const rectB = state.elementStates.find(
        (s: { element: Element }) => s.element === b
      )!;
      const yA = rectA.bounds.top;
      const yB = rectB.bounds.top;
      return yB - yA;
    });

    const flipTl = Flip.from(state, {
      targets: sortedEls,
      duration: params.moveDur,
      stagger: params.flipStagger,
      ease: 'ios-spring',
      absolute: true,
    });
    if (toCollapsed) {
      animateLogoSlabInsets(
        flipTl as gsap.core.Timeline,
        config.logoSlabEl,
        'collapsed',
        params.moveDur + totalStagger,
        'ios-spring',
        0
      );
    }
    currentTimeline = flipTl as gsap.core.Timeline;

    return flipTl as gsap.core.Timeline;
  }

  function buildContext(): AnimatorContext {
    const params = getAnimationParams();
    return {
      config,
      logoGlyphEls,
      params,
      constants: {
        COLLAPSED_WIDTH,
        EXPANDED_WIDTH,
        COLLAPSED_WIDTH_VALUE,
        EXPANDED_WIDTH_VALUE,
      },
      isStale,
      setTimeline: (tl) => {
        currentTimeline = tl;
      },
      applyExpandedWidthFrame: (measuredLogoWidth, currentWidth) => {
        applyExpandedWidthFrame(currentWidth, measuredLogoWidth);
      },
      flipPhase: (id, toCollapsed) => flipPhase(id, toCollapsed, params),
      commitState: (collapsed) => {
        lastCommittedCollapsed = collapsed;
        cleanupTransientStyles(config, collapsed ? 'collapsed' : 'expanded');
        // 刷新终态高度缓存，保证下次中断恢复使用准确值
        const key = collapsed ? 'collapsed' : 'expanded';
        cachedLogoHeight[key] = config.logoContainerEl.offsetHeight;
        config.onComplete?.(collapsed);
        currentTimeline = null;
      },
      awaitReady,
    };
  }

  function collapse() {
    const id = startNewAnimation();
    if (lastCommittedCollapsed) return;
    runCollapse(id, buildContext()).catch(() => {
      if (!isStale(id)) {
        lastCommittedCollapsed = true;
        syncToState(config, true);
        cleanupTransientStyles(config, 'collapsed');
      }
    });
  }

  function expand() {
    const id = startNewAnimation();
    if (!lastCommittedCollapsed) return;
    runExpand(id, buildContext()).catch(() => {
      if (!isStale(id)) {
        lastCommittedCollapsed = false;
        syncToState(config, false);
        cleanupTransientStyles(config, 'expanded');
      }
    });
  }

  /**
   * 仅展开 logo（拖曳松手专用）——见 {@link SidebarAnimator.expandLogoOnly}。
   *
   * 与 {@link expand} 不同，这里不调用 `startNewAnimation()`（它会
   * `normalizeToCommittedState()` 把内容布局重置回折叠态，造成拖曳期间已展开的内容
   * 闪回）。改为手动作废上一动画并清理动画瞬态 tween，但保留当前的内容布局状态。
   */
  function expandLogoOnly() {
    currentAnimationId++;
    const id = currentAnimationId;
    currentTimeline?.kill();
    currentTimeline = null;
    heightTween?.kill();
    heightTween = null;
    auxiliaryTweens.forEach((t) => t.kill());
    auxiliaryTweens = [];
    if (!lastCommittedCollapsed) return;
    runExpandLogoOnly(id, buildContext()).catch(() => {
      if (!isStale(id)) {
        lastCommittedCollapsed = false;
        syncToState(config, false);
        cleanupTransientStyles(config, 'expanded');
      }
    });
  }

  function previewContentCollapsed(collapsed: boolean) {
    if (collapsed) {
      // 切回折叠（图标）布局：恢复折叠收藏浮层可见，隐藏展开收藏区由 CSS .hidden 接管
      gsap.set(config.collectionsCollapsedEl, {
        clearProps: COLLAPSED_COLLECTIONS_OVERLAY_PROPS,
      });
      gsap.set(config.collectionsCollapsedEl, {
        opacity: 1,
        visibility: 'visible',
      });
    } else {
      // 切到展开（带标签）布局：隐藏折叠收藏浮层，清掉标签的折叠态内联 maxWidth/opacity
      gsap.set(config.collectionsCollapsedEl, {
        opacity: 0,
        visibility: 'hidden',
      });
      const labelEls = collectSidebarAnimatorLabelEls(config);
      gsap.set(labelEls, { clearProps: 'maxWidth,opacity' });
    }
    config.onContentSwitch(collapsed);
  }

  function updateSlabFollow(sidebarWidth: number) {
    // 仅展开稳定态跟随：折叠态保持折叠宽度，动画进行中交给运行中的 tween 处理
    if (lastCommittedCollapsed || currentTimeline) return;
    config.logoSlabEl.style.setProperty(
      '--slab-width',
      `${measureExpandedSlabWidth(config.logoContainerEl, sidebarWidth)}px`
    );
  }

  function interrupt() {
    currentAnimationId++;
    currentTimeline?.kill();
    currentTimeline = null;
    heightTween?.kill();
    heightTween = null;
    auxiliaryTweens.forEach((t) => t.kill());
    auxiliaryTweens = [];
    normalizeToCommittedState();
  }

  function dispose() {
    currentAnimationId++;
    currentTimeline?.kill();
    currentTimeline = null;
    heightTween?.kill();
    heightTween = null;
    auxiliaryTweens.forEach((t) => t.kill());
    auxiliaryTweens = [];
    normalizeToCommittedState();
  }

  function syncCollapsedState(collapsed: boolean) {
    currentAnimationId++;
    currentTimeline?.kill();
    currentTimeline = null;
    heightTween?.kill();
    heightTween = null;
    auxiliaryTweens.forEach((t) => t.kill());
    auxiliaryTweens = [];
    lastCommittedCollapsed = collapsed;
    syncToState(config, collapsed);
    cleanupTransientStyles(config, collapsed ? 'collapsed' : 'expanded');
    const key = collapsed ? 'collapsed' : 'expanded';
    cachedLogoHeight[key] = config.logoContainerEl.offsetHeight;
  }

  return {
    collapse,
    expand,
    expandLogoOnly,
    previewContentCollapsed,
    updateSlabFollow,
    interrupt,
    dispose,
    syncCollapsedState,
  };
}
