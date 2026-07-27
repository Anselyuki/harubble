/**
 * Volume capsule 通用 controller（Phase 3 Step 3.2 · Fix 6）。
 *
 * 收敛 GlassVolumeCapsuleView / MaterialVolumeCapsuleView 完全一致的事件层
 * 代码：状态机推进、collapse timer、slider input / mouse leave enter / focusout /
 * 全局 pointerup 五个事件的编排。
 *
 * # 与 animator 的边界
 *
 * animator（`CapsuleAnimator` 接口的 glass / material 实现）由 view 层根据当前
 * visualContract.family 选择注入，负责 tween 参数与 DOM 副作用；controller 只关心
 * 业务：什么时候展开、什么时候收起、如何处理外部输入。
 */
import {
  CapsuleState,
  transition,
  type CapsuleEvent,
} from '$lib/components/app/player/volume-capsule-state';
import { createCollapseTimer } from '$lib/components/app/player/volume-capsule-timer';
import type { CapsuleAnimator } from '$lib/components/app/player/volume-capsule-animator';

const COLLAPSE_DELAY_MS = 799;

export interface VolumeCapsuleControllerOpts {
  /** 由父组件传入的 open 目标态。controller 在其变化时驱动 animator。 */
  getOpen(): boolean;
  /** 用于 focusout 判断 relatedTarget 是否仍在包裹内。 */
  getWrapperEl(): HTMLElement | null;
  /** 展开完成后 controller 会调用它把焦点交给 slider。 */
  focusSlider(): void;
  /** 用户操作触发的向上广播事件。 */
  onopen(): void;
  onclose(): void;
  /** slider 拖动过程中的音量更新，值为增益（0..1）。 */
  onVolumeChange(gain: number): void | Promise<void>;
  /** 供 animator 使用的引用 getter；由 view 组件按需实现。 */
  animator: CapsuleAnimator;
}

export interface VolumeCapsuleController {
  /** view 层通过 `$effect` 观察 open 变化并调用它，controller 内决策是否触发 tween。 */
  syncOpen(): void;
  handleSliderInput(pos: number): void;
  handleSliderCommit(): void;
  handleSliderDown(): void;
  handleSliderUp(): void;
  handleFocusOut(event: FocusEvent): void;
  handleMouseLeave(): void;
  handleMouseEnter(): void;
  handleIconClick(): void;
  installGlobalPointerListeners(): () => void;
  destroy(): void;
  /** 供 UI 层观察拖动态：拖动中显示实时预览。 */
  readonly isDragging: boolean;
  /** 供测试观察状态。 */
  readonly state: CapsuleState;
  /** 拖动过程中的滑块预览值（0..1）。null 表示不覆盖 volume 派生值。 */
  readonly sliderPreview: number | null;
}

export function createVolumeCapsuleController(
  opts: VolumeCapsuleControllerOpts
): VolumeCapsuleController {
  const animator = opts.animator;
  let state: CapsuleState = CapsuleState.Closed;
  let isDragging = false;
  let sliderPreview: number | null = null;

  function send(event: CapsuleEvent): void {
    state = transition(state, event);
  }

  const collapseTimer = createCollapseTimer(COLLAPSE_DELAY_MS, () => {
    opts.onclose();
  });

  return {
    get isDragging() {
      return isDragging;
    },
    get state() {
      return state;
    },
    get sliderPreview() {
      return sliderPreview;
    },
    syncOpen() {
      if (opts.getOpen() && state === CapsuleState.Closed) {
        send('OPEN');
        animator.expand(() => {
          send('EXPANDED');
          opts.focusSlider();
        });
      } else if (!opts.getOpen() && state === CapsuleState.Open) {
        send('CLOSE');
        animator.collapse(() => send('COLLAPSED'));
      }
    },
    handleSliderInput(pos: number) {
      sliderPreview = pos;
      void opts.onVolumeChange(pos);
      animator.showBadge();
    },
    handleSliderCommit() {
      sliderPreview = null;
    },
    handleSliderDown() {
      isDragging = true;
    },
    handleSliderUp() {
      isDragging = false;
    },

    handleFocusOut(event: FocusEvent) {
      const wrapper = opts.getWrapperEl();
      if (!wrapper) return;
      const related = event.relatedTarget as Node | null;
      if (related && wrapper.contains(related)) return;
      if (state === CapsuleState.Open) collapseTimer.schedule();
    },
    handleMouseLeave() {
      if (isDragging) return;
      if (state === CapsuleState.Open) collapseTimer.schedule();
    },
    handleMouseEnter() {
      collapseTimer.cancel();
      // 用 prop 值（getOpen）而不是内部 state：在 Expanding 阶段 state≠Open 但父组件
      // 已收到 open=true，避免重复触发 onopen。与原始 view 严格一致。
      if (!opts.getOpen()) opts.onopen();
    },
    handleIconClick() {
      if (!opts.getOpen()) opts.onopen();
    },
    installGlobalPointerListeners() {
      // 与原始 glass/material view 严格一致：拖到胶囊外松开时，只要指针不 hover
      // 就 schedule collapse。不检查 state（原始行为无条件 schedule）。
      function handleGlobalPointerUp() {
        isDragging = false;
        const wrapper = opts.getWrapperEl();
        if (wrapper && !wrapper.matches(':hover')) {
          collapseTimer.schedule();
        }
      }
      document.addEventListener('pointerup', handleGlobalPointerUp);
      return () =>
        document.removeEventListener('pointerup', handleGlobalPointerUp);
    },
    destroy() {
      collapseTimer.destroy();
      animator.destroy();
    },
  };
}
