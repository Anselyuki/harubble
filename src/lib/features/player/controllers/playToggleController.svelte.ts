/**
 * Play toggle glyph controller（Phase 3 Step 3.2 · 无缝抽离样板）。
 *
 * # 职责与边界
 *
 * 承接原 `PlayToggleGlyph.svelte` 内的**业务状态机**：
 *
 * - 三态 glyph 切换（`play` / `pause` / `loading`）的决策：由 `selectPlayToggleGlyphTransition`
 *   与 `selectGlyphAfterCollapse` 提供
 * - `LOADING_GRACE_MS`：折叠完成后再等一档才决定是否切到 loading，吞掉抖动
 * - `LOADING_MIN_VISIBLE_MS`：loading 显示后必须停留最短时长，防止连闪
 * - `animationGeneration`：世代号防止跨代 tween 完成回调污染新态
 *
 * # 与 View 层的边界
 *
 * 通过依赖注入的 [`GlyphAnimator`] 把"动画怎么做"完全交给 view 层：
 *
 * - Glass family：iOS spring in / ios-in out（现有行为）
 * - Material family：短程 fade + scale 0.9→1
 * - Terminal family：瞬时替换（无动画）
 *
 * Controller 只调用 `animator.animateOut(glyph, onDone)`、`animator.animateIn(glyph)`、
 * `animator.setImmediately(glyph)`、`animator.kill()`，不直接操作 DOM。
 *
 * # 使用契约
 *
 * ```ts
 * const controller = createPlayToggleController(animator);
 * // 用 $effect 同步 props 到 controller：
 * $effect(() => controller.notify({ isPlaying, isLoading, isPending, transitionKey }));
 * // 组件卸载时释放：
 * onDestroy(() => controller.destroy());
 * ```
 */

import {
  getSettledPlayToggleGlyph,
  selectGlyphAfterCollapse,
  selectPlayToggleGlyphTransition,
  type PlayToggleGlyph,
  type PlayToggleGlyphState,
} from '$lib/features/player/playToggleGlyph';
import { gsap, MOTION } from '$lib/design/gsap';

/**
 * View 层为 controller 提供的动画适配器接口。
 *
 * View 侧知道 DOM refs、缓动曲线、位移方向；controller 侧只关心"该出场了 / 该收了"。
 * 每个 family 提供各自的 `GlyphAnimator` 实现，让 controller 无需感知视觉差异。
 *
 * # 契约
 *
 * - `animateIn(glyph)`：把指定 glyph 淡入 / 弹入到可见态
 * - `animateOut(glyph, onDone)`：把指定 glyph 收起，动画结束调用 onDone
 * - `setImmediately(glyph)`：无动画，直接把指定 glyph 设为可见（其余隐藏）
 * - `kill()`：kill 所有 in-flight tween；组件卸载时调用
 */
export interface GlyphAnimator {
  animateIn(glyph: PlayToggleGlyph): void;
  animateOut(glyph: PlayToggleGlyph, onDone: () => void): void;
  setImmediately(glyph: PlayToggleGlyph): void;
  kill(): void;
}

/** 折叠完成后 grace 期毫秒，与老实现一致：`MOTION.MICRO` */
export const LOADING_GRACE_MS = MOTION.MICRO;
/** loading 一旦显现，最短停留毫秒，防止 loading → pause → loading 连闪 */
export const LOADING_MIN_VISIBLE_MS = MOTION.SLOW + MOTION.MICRO;

export interface PlayToggleControllerInput {
  isPlaying: boolean;
  isLoading?: boolean;
  isPending?: boolean;
  transitionKey?: number;
}

export interface PlayToggleController {
  /**
   * 通知 controller 输入状态变化（play/loading/pending/transitionKey）。
   *
   * 由 view 层在 `$effect` 内调用，controller 内部决策是否发起 tween。
   * 首次调用视为初始化，直接把决议后的 glyph 设为可见（不带动画）。
   */
  notify(input: PlayToggleControllerInput): void;
  /**
   * 释放所有资源（in-flight tween、delayedCall）。
   *
   * 组件卸载时必调，防止 GSAP 动画在 DOM 消失后仍执行回调导致状态泄漏。
   */
  destroy(): void;
  /**
   * 只读的当前可见 glyph（供 view / test 检查）。
   */
  readonly visibleGlyph: PlayToggleGlyph;
}

/**
 * 创建一个 play toggle controller。
 *
 * 传入的 `animator` 决定视觉表现；controller 仅编排状态机与 timer。
 */
export function createPlayToggleController(
  animator: GlyphAnimator
): PlayToggleController {
  let visibleGlyph: PlayToggleGlyph = 'play';
  let previousTransitionKey = 0;
  let initialized = false;
  let commandCollapsing = false;
  let animationGeneration = 0;
  let inFlightGlyph: PlayToggleGlyph | null = null;
  let loadingDelay: gsap.core.Tween | null = null;
  let loadingMinimumDelay: gsap.core.Tween | null = null;
  let lastInput: PlayToggleControllerInput = {
    isPlaying: false,
    isLoading: false,
    isPending: false,
    transitionKey: 0,
  };

  function snapshot(): PlayToggleGlyphState {
    return {
      isPlaying: lastInput.isPlaying,
      isLoading: lastInput.isLoading ?? false,
      isPending: lastInput.isPending ?? false,
    };
  }

  function cancelLoadingDelay() {
    loadingDelay?.kill();
    loadingDelay = null;
  }

  function cancelLoadingMinimumDelay() {
    loadingMinimumDelay?.kill();
    loadingMinimumDelay = null;
  }

  function reveal(glyph: PlayToggleGlyph, generation: number) {
    if (generation !== animationGeneration) return;
    visibleGlyph = glyph;
    inFlightGlyph = null;
    animator.animateIn(glyph);
  }

  function swapTo(glyph: PlayToggleGlyph) {
    const transition = selectPlayToggleGlyphTransition(
      visibleGlyph,
      inFlightGlyph,
      glyph
    );
    if (transition === 'keep') return;

    const generation = ++animationGeneration;
    if (transition === 'restore') {
      inFlightGlyph = null;
      animator.animateIn(visibleGlyph);
      return;
    }

    inFlightGlyph = glyph;
    animator.animateOut(visibleGlyph, () => reveal(glyph, generation));
  }

  function holdLoadingBeforeSettling(
    outgoingGlyph: PlayToggleGlyph,
    generation: number
  ) {
    loadingMinimumDelay = gsap.delayedCall(
      LOADING_MIN_VISIBLE_MS / 1000,
      () => {
        loadingMinimumDelay = null;
        if (generation !== animationGeneration) return;
        commandCollapsing = false;
        const nextGlyph = selectGlyphAfterCollapse(outgoingGlyph, snapshot());
        if (nextGlyph !== 'loading') swapTo(nextGlyph);
      }
    );
  }

  function beginCommandTransition() {
    const outgoingGlyph = visibleGlyph;
    const generation = ++animationGeneration;
    cancelLoadingDelay();
    cancelLoadingMinimumDelay();
    commandCollapsing = true;
    inFlightGlyph = null;

    animator.animateOut(outgoingGlyph, () => {
      if (generation !== animationGeneration) return;
      const nextGlyph = selectGlyphAfterCollapse(outgoingGlyph, snapshot());
      if (nextGlyph === 'loading') {
        loadingDelay = gsap.delayedCall(LOADING_GRACE_MS / 1000, () => {
          loadingDelay = null;
          if (generation !== animationGeneration) return;
          const delayedGlyph = selectGlyphAfterCollapse(
            outgoingGlyph,
            snapshot()
          );
          reveal(delayedGlyph, generation);
          if (delayedGlyph === 'loading') {
            holdLoadingBeforeSettling(outgoingGlyph, generation);
          } else {
            commandCollapsing = false;
          }
        });
        return;
      }
      commandCollapsing = false;
      reveal(nextGlyph, generation);
    });
  }

  return {
    notify(input) {
      lastInput = input;
      const state = snapshot();
      if (!initialized) {
        initialized = true;
        previousTransitionKey = input.transitionKey ?? 0;
        const initialGlyph =
          state.isLoading || state.isPending
            ? 'loading'
            : getSettledPlayToggleGlyph(state.isPlaying);
        visibleGlyph = initialGlyph;
        animator.setImmediately(initialGlyph);
        return;
      }
      if ((input.transitionKey ?? 0) !== previousTransitionKey) {
        previousTransitionKey = input.transitionKey ?? 0;
        beginCommandTransition();
        return;
      }
      if (commandCollapsing) return;
      const nextGlyph =
        state.isLoading || state.isPending
          ? 'loading'
          : getSettledPlayToggleGlyph(state.isPlaying);
      swapTo(nextGlyph);
    },
    destroy() {
      animationGeneration += 1;
      inFlightGlyph = null;
      cancelLoadingDelay();
      cancelLoadingMinimumDelay();
      animator.kill();
    },
    get visibleGlyph() {
      return visibleGlyph;
    },
  };
}
