/**
 * 侧栏展开动画
 *
 * Phase 1 — 宽度展开 + 旋转 + slab 同步（并行，300ms）
 *  ├─ 侧栏宽度 56px → 248px (ios-spring)
 *  ├─ 字符旋转 -90° → 0° (ios-spring, stagger 50ms)
 *  ├─ slab 宽度通过 --slab-width CSS 变量同步展开至 logo 容器宽度
 *  └─ 文字标签：可用空间 ≥ 标签宽度×50% 时开始同速展开
 *
 * Phase 2 — FLIP 堆栈弹出（240ms/字符，stagger 50ms，底部优先）
 *  └─ 容器高度同步过渡至目标高度
 */
import { tick } from 'svelte';
import { gsap } from '$lib/design/gsap';
import {
  getCenterLockTransform,
  collectSidebarAnimatorLabelEls,
  chainTimelineComplete,
} from './sidebar-animator';
import type { AnimatorContext } from './sidebar-animator';

export async function runExpand(id: number, ctx: AnimatorContext) {
  const { config, logoGlyphEls, params, isStale, setTimeline } = ctx;
  const { COLLAPSED_WIDTH_VALUE, EXPANDED_WIDTH_VALUE } = ctx.constants;

  config.onContentInteractive(false);

  await ctx.awaitReady();
  if (isStale(id)) return;

  const collapsedLogoWidth =
    config.logoContainerEl.getBoundingClientRect().width ||
    COLLAPSED_WIDTH_VALUE;

  ctx.applyExpandedWidthFrame(collapsedLogoWidth, COLLAPSED_WIDTH_VALUE);
  gsap.set(config.logoContainerEl, { overflow: 'hidden' });
  gsap.set(config.collectionsCollapsedEl, { opacity: 0, visibility: 'hidden' });

  config.onContentSwitch(false);
  await tick();
  if (isStale(id)) return;

  gsap.set(config.collectionsRegionEl, { clearProps: 'opacity' });
  gsap.set(config.navRegionEl, { clearProps: 'opacity' });
  const labelEls = collectSidebarAnimatorLabelEls(config);
  gsap.set(labelEls, { maxWidth: 0, opacity: 0 });

  const measuredLabelWidths = labelEls.map((el) => el.scrollWidth);

  const lockedGlyphRects = logoGlyphEls.map((el) => el.getBoundingClientRect());
  const lockLogoGlyphCenters = () => {
    logoGlyphEls.forEach((glyphEl, index) => {
      const currentTransform = {
        x: Number(gsap.getProperty(glyphEl, 'x')) || 0,
        y: Number(gsap.getProperty(glyphEl, 'y')) || 0,
      };
      const nextTransform = getCenterLockTransform(
        lockedGlyphRects[index],
        glyphEl.getBoundingClientRect(),
        currentTransform
      );
      gsap.set(glyphEl, nextTransform);
    });
  };

  // Phase 1：字母仍处于竖向堆叠态，slab 宽度保持折叠态不变；
  // slab 宽度展开移到 flipPhase，随字母散开同步进行、同时结束。
  const widthFrame = { width: COLLAPSED_WIDTH_VALUE };
  const phase1 = gsap.timeline();
  setTimeline(phase1);

  phase1.to(
    widthFrame,
    {
      width: EXPANDED_WIDTH_VALUE,
      duration: params.widthDur,
      ease: 'ios-spring',
      onUpdate: () => {
        ctx.applyExpandedWidthFrame(collapsedLogoWidth, widthFrame.width);
        lockLogoGlyphCenters();

        const currentExpansion = widthFrame.width - COLLAPSED_WIDTH_VALUE;
        labelEls.forEach((el, index) => {
          const labelWidth = measuredLabelWidths[index] || 120;
          const trigger = labelWidth * 0.5;
          if (currentExpansion >= trigger) {
            const progress = Math.min(
              (currentExpansion - trigger) / labelWidth,
              1
            );
            gsap.set(el, {
              maxWidth: progress * labelWidth,
              opacity: progress,
            });
          }
        });
      },
      onComplete: () => {
        ctx.applyExpandedWidthFrame(collapsedLogoWidth, EXPANDED_WIDTH_VALUE);
        labelEls.forEach((el, index) => {
          gsap.set(el, {
            maxWidth: measuredLabelWidths[index] || 120,
            opacity: 1,
          });
        });
      },
    },
    0
  );

  phase1.to(
    logoGlyphEls,
    {
      rotation: 0,
      duration: params.widthDur,
      stagger: params.stagger,
      ease: 'ios-spring',
      onUpdate: lockLogoGlyphCenters,
    },
    0
  );

  await chainTimelineComplete(phase1);
  if (isStale(id)) return;

  // slab 宽度展开在 flipPhase 中随字母散开同步进行
  const flipResult = await ctx.flipPhase(id, false);
  if (!flipResult || isStale(id)) return;

  await chainTimelineComplete(flipResult);
  if (isStale(id)) return;

  config.onContentInteractive(true);
  ctx.commitState(false);
}
