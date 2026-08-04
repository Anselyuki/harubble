/**
 * 事件序列跟踪器 —— 为无 revision 字段的 Tauri 事件提供客户端侧过期保护。
 *
 * P1-5 部分二的前端半段：在后端事件载荷补齐单调 revision 前，用监听 arrival
 * 顺序判定何时同一事件的新载荷已经到达、老载荷可安全丢弃。
 *
 * 用法：
 * ```ts
 * const seq = createEventSequence();
 * listen('player-state-changed', (event) => {
 *   const token = seq.next('player-state');
 *   void handle(event.payload).then(() => {
 *     if (!seq.isCurrent('player-state', token)) return;
 *     // apply state to store
 *   });
 * });
 * ```
 *
 * 每个 key 是逻辑事件通道；token 是该通道的单调计数器值。
 * `isCurrent` 返回 true 表示 token 仍是该通道最新分配的值——
 * 表明处理该载荷的异步链没有被更新事件超越。
 */
export interface EventSequence {
  /** 分配下一个 token 并将 key 的当前计数增加。 */
  next(key: string): number;
  /** 判定 token 是否仍等于 key 的当前计数。 */
  isCurrent(key: string, token: number): boolean;
  /** 强制丢弃 key 的所有已发出 token（例如页面切换、controller reset）。 */
  invalidate(key: string): void;
}

/**
 * 创建一个空的 EventSequence 实例。
 *
 * 计数从 1 开始，0 保留给"从未分配"状态，方便调用方在初始化时用 0 作占位。
 * 内部用普通 Map；每个 key 独立版本空间，不同事件通道互不影响。
 */
export function createEventSequence(): EventSequence {
  // eslint-disable-next-line svelte/prefer-svelte-reactivity
  const counters = new Map<string, number>();
  return {
    next(key: string): number {
      const current = counters.get(key) ?? 0;
      const next = current + 1;
      counters.set(key, next);
      return next;
    },
    isCurrent(key: string, token: number): boolean {
      return (counters.get(key) ?? 0) === token;
    },
    invalidate(key: string): void {
      const current = counters.get(key) ?? 0;
      counters.set(key, current + 1);
    },
  };
}
