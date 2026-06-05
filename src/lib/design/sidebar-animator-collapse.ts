/**
 * 侧栏收缩动画
 *
 * Phase 1 — 字符原地旋转（~200ms）
 *  └─ 字符旋转 0° → -90° (ios-spring, stagger 50ms)
 *
 * Phase 2+3 — logo 容器推高 + 字符飞向竖向目标位置（~240ms + stagger）
 *  ├─ 用隐藏克隆元素测量折叠态 logo 高度与各字符目标位置
 *  ├─ logo 容器高度动画至目标高度（推下方内容）
 *  ├─ 字符按左→右、上→下顺序飞向竖向目标位置
 *
 * Phase 4 — 宽度收缩（~200ms）
 *  ├─ 侧栏宽度 248px → 56px (ios-spring)
 *  ├─ 文字标签 maxWidth → 0, opacity → 0（同速率）
 *  └─ 导航/收藏区域淡出
 *
 * 完成 — 切换 CSS 布局、清理 inline 样式、恢复交互
 */
import { gsap } from '$lib/design/gsap';
import { flushSync, tick } from 'svelte';
import {
  animateLogoSlabInsetVars,
  collectSidebarAnimatorLabelEls,
  chainTimelineComplete,
  getCenterLockTransform,
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
  collapsedLogoLayoutWidth: number
): { targetHeight: number; charTargets: { x: number; y: number }[] } | null {
  const clone = logoContainerEl.cloneNode(true) as HTMLDivElement;
  clone.style.position = 'absolute';
  clone.style.visibility = 'hidden';
  clone.style.height = 'auto';
  clone.style.width = `${collapsedLogoLayoutWidth}px`;
  clone.style.overflow = '';
  clone.style.pointerEvents = 'none';
  clone.style.left = `${logoContainerEl.offsetLeft}px`;
  clone.style.top = `${logoContainerEl.offsetTop}px`;
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

function getCollapsedCollectionsOverlayTop(
  sidebarEl: HTMLElement,
  navRegionEl: HTMLElement
): number {
  const sidebarRect = sidebarEl.getBoundingClientRect();
  const navRect = navRegionEl.getBoundingClientRect();
  return Math.max(0, navRect.bottom - sidebarRect.top);
}

function readCssPixelValue(value: string): number {
  return Number.parseFloat(value) || 0;
}

function getCollapsedLogoLayoutWidth(
  sidebarEl: HTMLElement,
  collapsedWidth: string
): number {
  const collapsedWidthValue = Number.parseFloat(collapsedWidth);
  const styles = getComputedStyle(sidebarEl);
  if (styles.boxSizing !== 'border-box') {
    return collapsedWidthValue;
  }

  const horizontalInsets =
    readCssPixelValue(styles.paddingLeft) +
    readCssPixelValue(styles.paddingRight) +
    readCssPixelValue(styles.borderLeftWidth) +
    readCssPixelValue(styles.borderRightWidth);

  return Math.max(0, collapsedWidthValue - horizontalInsets);
}

export async function runCollapse(id: number, ctx: AnimatorContext) {
  const { config, logoGlyphEls, params, isStale, setTimeline } = ctx;

  config.onContentInteractive(false);

  // Phase 1: 字符原地旋转
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

  await chainTimelineComplete(phase1);
  if (isStale(id)) return;

  // Phase 2+3: 测量折叠态目标位置 → 推高容器 + 字符飞行
  const collapsedLogoLayoutWidth = getCollapsedLogoLayoutWidth(
    config.sidebarEl,
    ctx.constants.COLLAPSED_WIDTH
  );
  const measured = measureCollapsedTargets(
    config.logoContainerEl,
    config.logoCharEls,
    collapsedLogoLayoutWidth
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
  });

  await chainTimelineComplete(flyTl);
  if (isStale(id)) return;

  // Phase 4: 宽度收缩 + 标签收缩 + 内容淡出
  // 不切换 CSS 布局——字符保持 inline x/y 在视觉上的正确位置
  const lockedCharRects = config.logoCharEls.map((el) =>
    el.getBoundingClientRect()
  );
  const lockLogoCharCenters = () => {
    config.logoCharEls.forEach((charEl, index) => {
      const currentTransform = {
        x: Number(gsap.getProperty(charEl, 'x')) || 0,
        y: Number(gsap.getProperty(charEl, 'y')) || 0,
      };
      const nextTransform = getCenterLockTransform(
        lockedCharRects[index],
        charEl.getBoundingClientRect(),
        currentTransform
      );
      gsap.set(charEl, nextTransform);
    });
  };

  const labelEls = collectSidebarAnimatorLabelEls(config);
  gsap.set(config.collectionsCollapsedEl, {
    position: 'absolute',
    top: getCollapsedCollectionsOverlayTop(
      config.sidebarEl,
      config.navRegionEl
    ),
    left: 0,
    right: 0,
    zIndex: 1,
    opacity: 0,
    visibility: 'visible',
  });
  const phase4 = gsap.timeline();
  setTimeline(phase4);

  animateLogoSlabInsetVars(
    phase4,
    config.logoContainerEl,
    'collapsed',
    params.rotateDur,
    'ios-spring',
    0
  );

  phase4.to(
    config.shellEl,
    {
      '--sidebar-width': ctx.constants.COLLAPSED_WIDTH,
      duration: params.rotateDur,
      ease: 'ios-spring',
      onUpdate: lockLogoCharCenters,
      onComplete: lockLogoCharCenters,
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
    config.collectionsRegionEl,
    {
      opacity: 0,
      duration: params.rotateDur,
      ease: 'ios-in',
    },
    0
  );

  phase4.to(
    config.collectionsCollapsedEl,
    {
      opacity: 1,
      duration: params.rotateDur,
      ease: 'ios-out',
    },
    params.rotateDur * 0.35
  );

  await chainTimelineComplete(phase4);
  if (isStale(id)) return;

  // 全部动画完成后切换到 collapsed 内容；collapsed shortcut 保持 overlay
  // 位置，避免在最后一帧从隐藏流重新参与布局。
  config.onLayoutSwitch(true);
  flushSync();
  config.logoCharEls.forEach((el) => {
    gsap.set(el, { clearProps: 'x,y,transform' });
  });
  gsap.set(logoGlyphEls, { rotation: -90 });
  gsap.set(config.logoContainerEl, { clearProps: 'height,overflow,x' });

  config.onContentSwitch(true);
  await tick();
  if (isStale(id)) return;

  ctx.commitState(true);
  config.onContentInteractive(true);
}
