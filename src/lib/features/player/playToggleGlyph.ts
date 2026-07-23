/**
 * 播放切换按钮当前可见的图标类型。
 *
 * - `play`：显示播放三角，代表当前处于暂停 / 未开始状态；
 * - `pause`：显示暂停双竖条，代表当前正在播放；
 * - `loading`：显示加载环，代表命令已发出但后端尚未落定，或后端仍在缓冲。
 */
export type PlayToggleGlyph = 'play' | 'pause' | 'loading';

/**
 * 已有图标动画面对新状态时应采取的动作。
 *
 * - `keep`：当前显示或正在切换的目标已经正确，无需重复动画；
 * - `restore`：正在切向其他图标，但最新状态又回到了当前可见图标，需要取消旧动画；
 * - `swap`：切换到一个新的目标图标。
 */
export type PlayToggleGlyphTransition = 'keep' | 'restore' | 'swap';

/**
 * 决定 {@link selectGlyphAfterCollapse} 输出所需的最小状态切片。
 *
 * - `isPlaying`：后端上报的最新播放态，是「已落定」图标的唯一真相来源；
 * - `isLoading`：后端明确处于缓冲 / 加载中，必须继续显示 loading；
 * - `isPending`：前端已发出命令但尚未收到与目标一致的落定回执，用于覆盖后端一帧延迟。
 */
export interface PlayToggleGlyphState {
  isPlaying: boolean;
  isLoading: boolean;
  isPending: boolean;
}

/**
 * 计算「不考虑 loading / pending」时按钮应显示的落定图标。
 *
 * 只用于稳定态渲染或作为动画中点的目标判定，调用者若还需要考虑加载 / 挂起
 * 状态，请改用 {@link selectGlyphAfterCollapse}。
 *
 * @param isPlaying 后端最新上报的播放态，即 `PlayerState.isPlaying`。
 * @returns 播放中返回 `pause`，其余情况返回 `play`。
 */
export function getSettledPlayToggleGlyph(
  isPlaying: boolean
): Exclude<PlayToggleGlyph, 'loading'> {
  return isPlaying ? 'pause' : 'play';
}

/**
 * 根据当前可见图标、动画中的目标和最新请求，决定如何更新图标。
 *
 * `visibleGlyph` 在出场动画完成前仍是旧图标。如果此时状态快速恢复到旧图标，
 * 只比较 `requestedGlyph === visibleGlyph` 会错误地当成无变化，旧动画随后仍会展示
 * 已经过期的 loading。`restore` 专门处理这个竞争窗口。
 */
export function selectPlayToggleGlyphTransition(
  visibleGlyph: PlayToggleGlyph,
  inFlightGlyph: PlayToggleGlyph | null,
  requestedGlyph: PlayToggleGlyph
): PlayToggleGlyphTransition {
  if (requestedGlyph === visibleGlyph) {
    return inFlightGlyph && inFlightGlyph !== requestedGlyph
      ? 'restore'
      : 'keep';
  }
  if (requestedGlyph === inFlightGlyph) return 'keep';
  return 'swap';
}

/**
 * 折叠动画结束、切换新图标之前，根据当前状态挑选下一帧要显示的图标。
 *
 * 判定分两层，缺一不可：
 * 1. **落定优先**：如果本次折叠出去的不是 loading，且后端已经切到了新的落定态，
 *    直接切到 `settled`，即使一帧后 pending 才被清掉，也不会来回抖。
 * 2. **loading 兜底**：否则（包括 outgoing 本身就是 loading 的情况），
 *    只要 `isLoading` 或 `isPending` 任一为真，就继续显示 loading；两者都为假才切到 `settled`。
 *
 * 这套并列规则同时守住两类边界：
 * - 后端在动画中点前就返回，避免多余的 loading 一闪而过；
 * - 用户在 loading 保持期二次点击时，pending / loading 仍能压过 settled，
 *   防止 loading → pause → loading 的连闪。
 *
 * @param outgoing 折叠动画正要收掉的旧图标。
 * @param state 折叠完成时的最新状态快照，通常由组件即时采样得到。
 * @returns 折叠完成后应立即入场的目标图标。
 */
export function selectGlyphAfterCollapse(
  outgoing: PlayToggleGlyph,
  state: PlayToggleGlyphState
): PlayToggleGlyph {
  const settled = getSettledPlayToggleGlyph(state.isPlaying);

  if (outgoing !== 'loading' && settled !== outgoing) return settled;
  if (state.isLoading || state.isPending) return 'loading';
  return settled;
}
