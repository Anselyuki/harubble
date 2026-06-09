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
  collectSidebarAnimatorLabelEls,
  chainTimelineComplete,
  getSlabTargetHeight,
} from './sidebar-animator';
import type { AnimatorContext } from './sidebar-animator';

/**
 * 用隐藏克隆元素测量折叠态 logo 的目标高度和每个字符的目标位置。
 *
 * 克隆当前 logo 容器，将其切换到 collapsed 布局（通过添加 .collapsed class），
 * 在不影响当前 DOM 的情况下测量折叠态各字符位置和容器高度。
 */
function measureCollapsedTargets(
  sidebarEl: HTMLElement,
  logoContainerEl: HTMLDivElement,
  charEls: HTMLSpanElement[],
  collapsedSidebarWidth: string
): { targetHeight: number; charTargets: { x: number; y: number }[] } | null {
  const measurementSidebar = document.createElement('aside');
  const clone = logoContainerEl.cloneNode(true) as HTMLDivElement;

  const sidebarRect = sidebarEl.getBoundingClientRect();
  measurementSidebar.className = sidebarEl.className;
  measurementSidebar.classList.add('collapsed');
  measurementSidebar.setAttribute('aria-hidden', 'true');
  measurementSidebar.style.position = 'fixed';
  measurementSidebar.style.visibility = 'hidden';
  measurementSidebar.style.pointerEvents = 'none';
  measurementSidebar.style.left = `${sidebarRect.left}px`;
  measurementSidebar.style.top = `${sidebarRect.top}px`;
  measurementSidebar.style.width = collapsedSidebarWidth;
  measurementSidebar.style.height = 'auto';
  measurementSidebar.style.overflow = 'visible';

  clone.style.height = 'auto';
  clone.style.width = '';
  clone.style.overflow = '';
  clone.style.pointerEvents = 'none';
  clone.classList.add('collapsed');

  // 清除克隆内 glyph 的 inline transform（旋转），以获得准确的布局位置
  clone.querySelectorAll<HTMLElement>('[data-logo-glyph]').forEach((el) => {
    el.style.transform = '';
  });

  measurementSidebar.appendChild(clone);
  sidebarEl.parentElement!.appendChild(measurementSidebar);

  const cloneChars = clone.querySelectorAll<HTMLSpanElement>('.brand-char');
  if (cloneChars.length !== charEls.length) {
    measurementSidebar.remove();
    return null;
  }

  const targetHeight = clone.offsetHeight;

  const charTargets = Array.from(cloneChars).map((cloneChar, i) => {
    const cloneRect = cloneChar.getBoundingClientRect();
    const originalRect = charEls[i].getBoundingClientRect();
    return {
      x: cloneRect.left - originalRect.left,
      y: cloneRect.top - originalRect.top,
    };
  });

  measurementSidebar.remove();
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

export async function runCollapse(id: number, ctx: AnimatorContext) {
  const { config, logoGlyphEls, params, isStale, setTimeline } = ctx;

  config.onContentInteractive(false);

  // 确保首次 paint 完成 + 字体就绪，避免 getBoundingClientRect 度量偏差
  await ctx.awaitReady();
  if (isStale(id)) return;

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
  const measured = measureCollapsedTargets(
    config.sidebarEl,
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

  // slab 高度与 logo 容器同步动画（显式 GSAP 驱动）
  flyTl.to(
    config.logoSlabEl,
    {
      height: getSlabTargetHeight(config.logoSlabEl, targetHeight),
      duration: params.moveDur + totalStagger,
      ease: 'ios-spring',
    },
    0
  );

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

  // Phase 4: 标签收缩 + 内容淡出 + slab 收缩
  // slab 宽度收缩帧：从展开态实际宽度过渡到折叠态宽度
  const slabExpandedWidth = config.logoSlabEl.offsetWidth;
  const slabCollapsedWidth = ctx.constants.COLLAPSED_WIDTH_VALUE - 10;
  const slabFrame = { width: slabExpandedWidth };

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

  // slab 宽度从展开态收缩到折叠态（通过 --slab-width CSS 变量）
  phase4.to(
    slabFrame,
    {
      width: slabCollapsedWidth,
      duration: params.rotateDur,
      ease: 'ios-spring',
      onUpdate: () => {
        config.logoSlabEl.style.setProperty(
          '--slab-width',
          `${slabFrame.width}px`
        );
      },
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

  // Phase 4 的 lockLogoCharTopLeft 已经将字符锁在真实 CSS 流位置上，
  // 直接切换布局并清理 inline 样式即可，无偏差。
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
