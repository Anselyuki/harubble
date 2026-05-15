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
 * - 透明度叙事：旋转阶段字符渐变为半透明（0.35），飞行中恢复至 0.6，到位后 100ms 内实体化至 1
 * - 文字标签跟随：展开时标签不等侧栏完全展开，当可用空间达到标签宽度 50% 时即开始同速展开
 * - 侧栏宽度：展开 300ms / 折叠 200ms，使用 ios-spring 缓动
 * - 所有缓动曲线统一使用 iOS 风格 CustomEase（ios / ios-in / ios-out / ios-spring）
 *
 * ## 展开动画时间线
 *
 *  Phase 1 — 宽度展开 + 旋转 + 变淡（并行，300ms）
 *  ┌─────────────────────────────────────────────────────────────┐
 *  │ [0ms ─────────────────────────────────── 300ms]             │
 *  │  ├─ 侧栏宽度 56px → 248px (ios-spring)                     │
 *  │  ├─ 字符旋转 -90° → 0° (ios-spring, stagger 50ms)          │
 *  │  ├─ 字符透明度 1 → 0.35 (ios-in)                           │
 *  │  └─ 文字标签：可用空间 ≥ 标签宽度×50% 时开始同速展开        │
 *  └─────────────────────────────────────────────────────────────┘
 *
 *  Phase 2 — FLIP 堆栈弹出（240ms/字符，stagger 50ms，底部优先）
 *  ┌─────────────────────────────────────────────────────────────┐
 *  │ 字符按折叠态 Y 坐标从底到顶排序，依次飞向展开态目标位置      │
 *  │  ├─ 每个字符飞行 240ms (ios-spring)                         │
 *  │  ├─ 飞行中透明度 0.35 → 0.6 (ios-out)                      │
 *  │  ├─ 到位后 100ms 内 0.6 → 1 (ios-out)                      │
 *  │  └─ 容器高度同步过渡至目标高度 (240ms + totalStagger)        │
 *  └─────────────────────────────────────────────────────────────┘
 *
 *  完成 → 恢复交互、清理瞬态样式
 */
import { tick } from 'svelte';
import { gsap, Flip, reducedMotionQuery } from '$lib/design/gsap';
import { runExpand } from './sidebar-animator-expand';
import { runCollapse } from './sidebar-animator-collapse';

export interface SidebarAnimatorConfig {
  shellEl: HTMLElement;
  sidebarEl: HTMLElement;
  logoCharEls: HTMLSpanElement[];
  logoContainerEl: HTMLDivElement;
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
  interrupt(): void;
  dispose(): void;
}

export type AnimationParams = ReturnType<typeof getAnimationParams>;

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

export function getLogoCenterPinX(
  collapsedWidth: number,
  currentWidth: number
): number {
  const offset = -(currentWidth - collapsedWidth) / 2;
  return Object.is(offset, -0) ? 0 : offset;
}

export function getLogoCenterPinFrame(
  collapsedWidth: number,
  expandedWidth: number,
  progress: number
): { width: number; x: number } {
  const clampedProgress = Math.min(Math.max(progress, 0), 1);
  const width =
    collapsedWidth + (expandedWidth - collapsedWidth) * clampedProgress;

  return {
    width,
    x: getLogoCenterPinX(collapsedWidth, width),
  };
}

export function getPinnedLogoWidthFrame(
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

export function resolveLogoGlyphEl(charEl: HTMLSpanElement): HTMLElement {
  return charEl.querySelector<HTMLElement>('[data-logo-glyph]') ?? charEl;
}

export function getLogoFlipTargets(
  charEls: HTMLSpanElement[],
  _glyphEls: HTMLElement[]
): HTMLElement[] {
  return charEls;
}

export function getLogoSlotClearProps(): string {
  return 'all';
}

export function getLogoGlyphClearProps(
  target: 'collapsed' | 'expanded'
): string {
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

export function collectSidebarItemLabelEls(
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

function syncToState(config: SidebarAnimatorConfig, collapsed: boolean) {
  config.shellEl.style.setProperty(
    '--sidebar-width',
    collapsed ? COLLAPSED_WIDTH : EXPANDED_WIDTH
  );

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
}

function cleanupTransientStyles(
  config: SidebarAnimatorConfig,
  target: 'collapsed' | 'expanded'
) {
  const labelEls = collectSidebarAnimatorLabelEls(config);

  gsap.set(config.logoContainerEl, {
    clearProps: 'height,overflow,width,visibility,transform,alignSelf',
  });
  gsap.set(labelEls, { clearProps: 'maxWidth,opacity' });
  gsap.set(config.navRegionEl, { clearProps: 'opacity' });
  gsap.set(config.collectionsRegionEl, { clearProps: 'opacity' });
  gsap.set(config.collectionsCollapsedEl, { clearProps: 'opacity' });

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
}

type AwaitableTimeline = Omit<gsap.core.Timeline, 'then'> | TimelineLike;

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
  });
}

export function createSidebarAnimator(
  config: SidebarAnimatorConfig
): SidebarAnimator {
  let currentAnimationId = 0;
  let currentTimeline: gsap.core.Timeline | null = null;
  let heightTween: gsap.core.Tween | null = null;
  let lastCommittedCollapsed = config.initialCollapsed;

  const logoGlyphEls = config.logoCharEls.map(resolveLogoGlyphEl);

  syncToState(config, config.initialCollapsed);

  function startNewAnimation(): number {
    currentAnimationId++;
    currentTimeline?.kill();
    currentTimeline = null;
    heightTween?.kill();
    heightTween = null;
    normalizeToCommittedState();
    return currentAnimationId;
  }

  function normalizeToCommittedState() {
    syncToState(config, lastCommittedCollapsed);
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
    const { sidebarWidth, logoWidth, alignSelf } = getPinnedLogoWidthFrame(
      COLLAPSED_WIDTH_VALUE,
      EXPANDED_WIDTH_VALUE,
      (currentWidth - COLLAPSED_WIDTH_VALUE) /
        (EXPANDED_WIDTH_VALUE - COLLAPSED_WIDTH_VALUE),
      measuredLogoWidth
    );

    config.shellEl.style.setProperty('--sidebar-width', `${sidebarWidth}px`);
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
      config.logoContainerEl.style.width = '';
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

    sortedEls.forEach((el, i) => {
      gsap.to(el, {
        opacity: 0.6,
        duration: params.moveDur,
        delay: params.flipStagger * i,
        ease: 'ios-out',
        onComplete: () => {
          gsap.to(el, { opacity: 1, duration: 0.1, ease: 'ios-out' });
        },
      });
    });

    const flipTl = Flip.from(state, {
      targets: sortedEls,
      duration: params.moveDur,
      stagger: params.flipStagger,
      ease: 'ios-spring',
      absolute: true,
    });
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
        config.onComplete?.(collapsed);
        currentTimeline = null;
      },
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

  function interrupt() {
    currentAnimationId++;
    currentTimeline?.kill();
    currentTimeline = null;
    heightTween?.kill();
    heightTween = null;
    normalizeToCommittedState();
  }

  function dispose() {
    currentAnimationId++;
    currentTimeline?.kill();
    currentTimeline = null;
    heightTween?.kill();
    heightTween = null;
    const allEls = [
      config.shellEl,
      config.sidebarEl,
      config.logoContainerEl,
      config.navRegionEl,
      config.collectionsRegionEl,
      config.collectionsCollapsedEl,
      config.bottomLabelEl,
      ...collectSidebarAnimatorLabelEls(config),
      ...config.logoCharEls,
      ...logoGlyphEls,
    ];
    allEls.forEach((el) => gsap.set(el, { clearProps: 'all' }));
  }

  return { collapse, expand, interrupt, dispose };
}
