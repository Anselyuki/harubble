// @vitest-environment jsdom

/**
 * Play toggle controller 单元测试（Phase 3 Step 3.2）。
 *
 * 通过 fake animator 验证：
 * - 初次 notify 直接 setImmediately，不排 tween
 * - 状态变化时先 animateOut 再 animateIn
 * - transitionKey 变化触发 command transition
 * - LOADING_GRACE_MS / LOADING_MIN_VISIBLE_MS 定时器路径
 * - restore 路径（inFlightGlyph 与目标一致时）
 * - commandCollapsing 期间普通 notify 被短路
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

/**
 * mock 掉 $lib/design/gsap 的 gsap.delayedCall，用手动 fire 队列替换真实的定时器。
 * 测试可以通过 fireDelayedCalls() 触发所有排队的 delayedCall 回调。
 */
const delayedCallQueue: Array<{ cb: () => void; delay: number }> = [];

function fireDelayedCalls(): void {
  const q = delayedCallQueue.splice(0);
  for (const { cb } of q) cb();
}

vi.mock('$lib/design/gsap', () => ({
  gsap: {
    delayedCall(delay: number, cb: () => void) {
      const entry = { cb, delay };
      delayedCallQueue.push(entry);
      return {
        kill() {
          const idx = delayedCallQueue.indexOf(entry);
          if (idx >= 0) delayedCallQueue.splice(idx, 1);
        },
      };
    },
  },
  MOTION: {
    MICRO: 80,
    FAST: 140,
    BASE: 180,
    SLOW: 260,
    PAGE: 320,
    BASE_OUT: 200,
    SLOW_OUT: 320,
    PAGE_OUT: 380,
    OVERLAY_IN: 220,
  },
}));

beforeEach(() => {
  delayedCallQueue.length = 0;
});

import { createPlayToggleController } from './playToggleController.svelte';
import type { GlyphAnimator } from './playToggleController.svelte';
import type { PlayToggleGlyph } from '$lib/features/player/playToggleGlyph';

function makeFakeAnimator(): GlyphAnimator & {
  events: string[];
  pendingCollapses: Array<() => void>;
} {
  const events: string[] = [];
  const pendingCollapses: Array<() => void> = [];
  return {
    events,
    pendingCollapses,
    animateIn(glyph: PlayToggleGlyph) {
      events.push(`in:${glyph}`);
    },
    animateOut(glyph: PlayToggleGlyph, onDone: () => void) {
      events.push(`out:${glyph}`);
      pendingCollapses.push(onDone);
    },
    setImmediately(glyph: PlayToggleGlyph) {
      events.push(`set:${glyph}`);
    },
    kill() {
      events.push('kill');
    },
  };
}

describe('createPlayToggleController · 初始化与状态机', () => {
  it('首次 notify 走 setImmediately，不排 tween', () => {
    const animator = makeFakeAnimator();
    const ctrl = createPlayToggleController(animator);
    ctrl.notify({
      isPlaying: false,
      isLoading: false,
      isPending: false,
      transitionKey: 0,
    });
    expect(animator.events).toEqual(['set:play']);
    expect(ctrl.visibleGlyph).toBe('play');
  });

  it('首次 loading/pending 直接 setImmediately 到 loading', () => {
    const animator = makeFakeAnimator();
    const ctrl = createPlayToggleController(animator);
    ctrl.notify({
      isPlaying: false,
      isLoading: true,
      transitionKey: 0,
    });
    expect(animator.events).toEqual(['set:loading']);
  });

  it('切到 isPlaying=true 走 animateOut(play) → animateIn(pause)', () => {
    const animator = makeFakeAnimator();
    const ctrl = createPlayToggleController(animator);
    ctrl.notify({ isPlaying: false, transitionKey: 0 });
    animator.events.length = 0;
    ctrl.notify({ isPlaying: true, transitionKey: 0 });
    expect(animator.events).toEqual(['out:play']);
    // 收出完成后自动 in
    animator.pendingCollapses.shift()?.();
    expect(animator.events).toEqual(['out:play', 'in:pause']);
    expect(ctrl.visibleGlyph).toBe('pause');
  });

  it('destroy 调用 animator.kill 并中止后续回调', () => {
    const animator = makeFakeAnimator();
    const ctrl = createPlayToggleController(animator);
    ctrl.notify({ isPlaying: false, transitionKey: 0 });
    ctrl.notify({ isPlaying: true, transitionKey: 0 });
    animator.events.length = 0;
    ctrl.destroy();
    expect(animator.events).toContain('kill');
    // destroy 后即使 animator 完成回调，也不应更新 visibleGlyph（generation 已递增）
    const beforeGlyph = ctrl.visibleGlyph;
    animator.pendingCollapses.shift()?.();
    expect(ctrl.visibleGlyph).toBe(beforeGlyph);
  });
});

describe('createPlayToggleController · command transition', () => {
  it('transitionKey 变化触发 beginCommandTransition', () => {
    const animator = makeFakeAnimator();
    const ctrl = createPlayToggleController(animator);
    ctrl.notify({ isPlaying: false, transitionKey: 0 });
    animator.events.length = 0;
    ctrl.notify({ isPlaying: false, transitionKey: 1 });
    // 应该看到 animateOut（收 play），等待用户 pause/loading 决议
    expect(animator.events[0]?.startsWith('out:')).toBe(true);
  });
});

describe('createPlayToggleController · Svelte 5 兼容性', () => {
  it('visibleGlyph getter 是响应式（不能被外部改）', () => {
    const animator = makeFakeAnimator();
    const ctrl = createPlayToggleController(animator);
    ctrl.notify({ isPlaying: false, transitionKey: 0 });
    expect(ctrl.visibleGlyph).toBe('play');
    // 尝试外部改写：只读属性应静默失败或抛错
    expect(() => {
      (ctrl as unknown as { visibleGlyph: PlayToggleGlyph }).visibleGlyph =
        'pause';
    }).toThrow();
  });

  it('回调不同的 animator 实现不同视觉但共享状态机', () => {
    const glass = makeFakeAnimator();
    const material = makeFakeAnimator();
    const g = createPlayToggleController(glass);
    const m = createPlayToggleController(material);
    g.notify({ isPlaying: false, transitionKey: 0 });
    m.notify({ isPlaying: false, transitionKey: 0 });
    g.notify({ isPlaying: true, transitionKey: 0 });
    m.notify({ isPlaying: true, transitionKey: 0 });
    // 状态机行为一致：都会先 out(play)
    expect(glass.events).toEqual(['set:play', 'out:play']);
    expect(material.events).toEqual(['set:play', 'out:play']);
    vi.clearAllMocks();
  });
});

describe('createPlayToggleController · 定时器分支（HIGH review）', () => {
  it('LOADING_GRACE_MS：command transition 后决议为 loading 走 grace delayedCall', () => {
    const animator = makeFakeAnimator();
    const ctrl = createPlayToggleController(animator);
    // 初始化为 play 态
    ctrl.notify({ isPlaying: false, isLoading: false, transitionKey: 0 });
    animator.events.length = 0;
    // 切换 transitionKey + 变 loading（比如用户点了播放，等待缓冲）
    ctrl.notify({ isPlaying: false, isLoading: true, transitionKey: 1 });
    // 应先 animateOut(play)
    expect(animator.events).toEqual(['out:play']);
    expect(delayedCallQueue.length).toBe(0);
    // 收出完成 → 应排入 grace delayedCall（loading 决议延迟）
    animator.pendingCollapses.shift()?.();
    expect(delayedCallQueue.length).toBe(1);
    // grace 触发 → reveal loading + 再排 min visible delayedCall
    fireDelayedCalls();
    expect(animator.events).toEqual(['out:play', 'in:loading']);
    expect(ctrl.visibleGlyph).toBe('loading');
    expect(delayedCallQueue.length).toBe(1);
  });

  it('LOADING_MIN_VISIBLE_MS：loading 显示后必须停留最短时长', () => {
    const animator = makeFakeAnimator();
    const ctrl = createPlayToggleController(animator);
    ctrl.notify({ isPlaying: false, transitionKey: 0 });
    ctrl.notify({ isPlaying: false, isLoading: true, transitionKey: 1 });
    animator.pendingCollapses.shift()?.();
    fireDelayedCalls(); // grace 触发，loading 显示，排入 min visible
    animator.events.length = 0;
    // 期间即使 loading=false（后端"清抖动"）也不应立即切走
    ctrl.notify({ isPlaying: true, isLoading: false, transitionKey: 1 });
    expect(animator.events).toEqual([]); // commandCollapsing 仍为 true，普通 notify 被短路
    // min visible 触发 → 决议下一 glyph
    fireDelayedCalls();
    expect(animator.events).toContain('out:loading');
  });

  it('grace 期间 loading 变假：直接 reveal 非 loading 状态', () => {
    const animator = makeFakeAnimator();
    const ctrl = createPlayToggleController(animator);
    ctrl.notify({ isPlaying: false, transitionKey: 0 });
    ctrl.notify({ isPlaying: false, isLoading: true, transitionKey: 1 });
    animator.pendingCollapses.shift()?.();
    // grace 排入队后，用户 loading 决议前，isLoading 已经变 false 且 isPlaying=true
    ctrl.notify({ isPlaying: true, isLoading: false, transitionKey: 1 });
    // grace 触发 → 决议 pause，跳过 loading，不排 min visible
    fireDelayedCalls();
    expect(animator.events).toContain('in:pause');
    expect(delayedCallQueue.length).toBe(0);
  });

  it('commandCollapsing 期间普通 notify 被短路', () => {
    const animator = makeFakeAnimator();
    const ctrl = createPlayToggleController(animator);
    ctrl.notify({ isPlaying: false, transitionKey: 0 });
    // 触发 command transition，进入 commandCollapsing=true
    ctrl.notify({ isPlaying: true, transitionKey: 1 });
    animator.events.length = 0;
    // 收出还未完成，此时同 transitionKey 的普通 notify 应被短路
    ctrl.notify({ isPlaying: true, transitionKey: 1 });
    expect(animator.events).toEqual([]);
  });

  it('restore 路径：inFlightGlyph 与目标一致时 animateIn 恢复', () => {
    const animator = makeFakeAnimator();
    const ctrl = createPlayToggleController(animator);
    ctrl.notify({ isPlaying: false, transitionKey: 0 });
    // 触发 out(play) → 收出中，inFlightGlyph 变成 pause
    ctrl.notify({ isPlaying: true, transitionKey: 0 });
    expect(animator.events).toEqual(['set:play', 'out:play']);
    // 收出未完成前用户又切回 play（撤销）：应 restore visibleGlyph=play
    ctrl.notify({ isPlaying: false, transitionKey: 0 });
    // restore 分支：animateIn(visibleGlyph=play)
    expect(animator.events[animator.events.length - 1]).toBe('in:play');
  });
});
