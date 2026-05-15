/**
 * 侧栏收缩动画
 *
 * Phase 1 — 字符原地旋转 + 变淡（~200ms）
 *  ├─ 字符旋转 0° → -90° (ios-spring, stagger 50ms)
 *  └─ 字符透明度 1 → 0.35 (ios-in)
 *
 * Phase 2+3 — logo 容器推高 + 字符飞向竖向目标位置（~240ms + stagger）
 *  ├─ 用隐藏克隆元素测量折叠态 logo 高度与各字符目标位置
 *  ├─ logo 容器高度动画至目标高度（推下方内容）
 *  ├─ 字符按左→右、上→下顺序飞向竖向目标位置
 *  ├─ 飞出瞬间开始实体化（0.35 → 0.6）
 *  └─ 到位后 100ms 内 0.6 → 1
 *
 * Phase 4 — 宽度收缩（~200ms）
 *  ├─ 侧栏宽度 248px → 56px (ios-spring)
 *  ├─ 文字标签 maxWidth → 0, opacity → 0（同速率）
 *  └─ 导航/收藏区域淡出
 *
 * 完成 — 切换 CSS 布局、清理 inline 样式、恢复交互
 */
import { gsap } from '$lib/design/gsap';
import {
  collectSidebarAnimatorLabelEls,
  chainTimelineComplete,
} from './sidebar-animator';
import type { AnimatorContext } from './sidebar-animator';

/**
 * 用隐藏克隆元素测量折叠态 logo 的目标高度和每个字符的目标位置。
 *
 * 克隆当前 logo 容器，将其切换到 collapsed 布局（通过添加 .collapsed class），
 * 在不影响当前 DOM 的情况下测量折叠态各字符位置和容器高度。
 */
function measureCollapsedTargets(
  logoContainerEl: HTMLDivElement,
  charEls: HTMLSpanElement[],
  collapsedWidth: string
): { targetHeight: number; charTargets: { x: number; y: number }[] } | null {
  const clone = logoContainerEl.cloneNode(true) as HTMLDivElement;
  clone.style.position = 'absolute';
  clone.style.visibility = 'hidden';
  clone.style.height = 'auto';
  clone.style.width = collapsedWidth;
  clone.style.overflow = '';
  clone.style.pointerEvents = 'none';
  clone.classList.add('collapsed');

  // 清除克隆内 glyph 的 inline transform（旋转），以获得准确的布局位置
  clone.querySelectorAll<HTMLElement>('[data-logo-glyph]').forEach((el) => {
    el.style.transform = '';
  });

  logoContainerEl.parentElement!.appendChild(clone);

  const cloneChars = clone.querySelectorAll<HTMLSpanElement>('.brand-char');
  if (cloneChars.length !== charEls.length) {
    clone.remove();
    return null;
  }

  const containerRect = logoContainerEl.getBoundingClientRect();
  const targetHeight = clone.offsetHeight;

  const charTargets = Array.from(cloneChars).map((cloneChar, i) => {
    const cloneRect = cloneChar.getBoundingClientRect();
    const originalRect = charEls[i].getBoundingClientRect();
    return {
      x: cloneRect.left - originalRect.left,
      y:
        cloneRect.top -
        containerRect.top -
        (originalRect.top - containerRect.top),
    };
  });

  clone.remove();
  return { targetHeight, charTargets };
}

/**
 * 根据字符在展开态中的位置生成从左到右、从上到下的排序索引。
 */
function getLeftToRightTopToBottomOrder(charEls: HTMLSpanElement[]): number[] {
  const indexed = charEls.map((el, i) => {
    const rect = el.getBoundingClientRect();
    return { index: i, x: rect.left, y: rect.top };
  });

  indexed.sort((a, b) => {
    const rowDiff = a.y - b.y;
    if (Math.abs(rowDiff) > 2) return rowDiff;
    return a.x - b.x;
  });

  return indexed.map((item) => item.index);
}

export async function runCollapse(id: number, ctx: AnimatorContext) {
  const { config, logoGlyphEls, params, isStale, setTimeline } = ctx;

  config.onContentInteractive(false);

  // Phase 1: 字符原地旋转 + 变淡
  const phase1 = gsap.timeline();
  setTimeline(phase1);

  phase1.to(
    logoGlyphEls,
    {
      rotation: -90,
      duration: params.rotateDur,
      stagger: params.stagger,
      ease: 'ios-spring',
    },
    0
  );

  phase1.to(
    config.logoCharEls,
    {
      opacity: 0.35,
      duration: params.rotateDur,
      ease: 'ios-in',
    },
    0
  );

  await chainTimelineComplete(phase1);
  if (isStale(id)) return;

  // Phase 2+3: 测量折叠态目标位置 → 推高容器 + 字符飞行
  const measured = measureCollapsedTargets(
    config.logoContainerEl,
    config.logoCharEls,
    ctx.constants.COLLAPSED_WIDTH
  );
  if (!measured || isStale(id)) return;

  const { targetHeight, charTargets } = measured;

  // 锁定当前高度后动画至目标高度（推下方内容）
  const currentHeight = config.logoContainerEl.offsetHeight;
  config.logoContainerEl.style.height = `${currentHeight}px`;
  config.logoContainerEl.style.overflow = 'hidden';

  const sortOrder = getLeftToRightTopToBottomOrder(config.logoCharEls);
  const totalStagger = params.flipStagger * (sortOrder.length - 1);

  const flyTl = gsap.timeline();
  setTimeline(flyTl);

  flyTl.to(config.logoContainerEl, {
    height: targetHeight,
    duration: params.moveDur + totalStagger,
    ease: 'ios-spring',
  });

  sortOrder.forEach((charIndex, staggerIndex) => {
    const target = charTargets[charIndex];
    const charEl = config.logoCharEls[charIndex];

    flyTl.to(
      charEl,
      {
        x: target.x,
        y: target.y,
        duration: params.moveDur,
        ease: 'ios-spring',
      },
      staggerIndex * params.flipStagger
    );

    flyTl.to(
      charEl,
      {
        opacity: 0.6,
        duration: params.moveDur,
        ease: 'ios-out',
        onComplete: () => {
          gsap.to(charEl, { opacity: 1, duration: 0.1, ease: 'ios-out' });
        },
      },
      staggerIndex * params.flipStagger
    );
  });

  await chainTimelineComplete(flyTl);
  if (isStale(id)) return;

  // Phase 4: 宽度收缩 + 标签收缩 + 内容淡出
  // 不切换 CSS 布局——字符保持 inline x/y 在视觉上的正确位置
  const labelEls = collectSidebarAnimatorLabelEls(config);
  const phase4 = gsap.timeline();
  setTimeline(phase4);

  phase4.to(
    config.shellEl,
    {
      '--sidebar-width': ctx.constants.COLLAPSED_WIDTH,
      duration: params.rotateDur,
      ease: 'ios-spring',
    },
    0
  );

  phase4.to(
    labelEls,
    {
      maxWidth: 0,
      opacity: 0,
      duration: params.rotateDur,
      ease: 'ios-in',
    },
    0
  );

  phase4.to(
    [config.collectionsRegionEl, config.navRegionEl],
    {
      opacity: 0,
      duration: params.rotateDur,
      ease: 'ios-in',
    },
    0
  );

  await chainTimelineComplete(phase4);
  if (isStale(id)) return;

  // 全部动画完成，一次性切换 CSS 布局 + 清理所有 inline 样式
  config.onLayoutSwitch(true);
  config.logoCharEls.forEach((el) => {
    gsap.set(el, { clearProps: 'x,y,opacity,transform' });
  });
  gsap.set(logoGlyphEls, { rotation: -90 });
  gsap.set(config.logoContainerEl, { clearProps: 'height,overflow,x' });

  config.onContentSwitch(true);
  config.onContentInteractive(true);
  ctx.commitState(true);
}
