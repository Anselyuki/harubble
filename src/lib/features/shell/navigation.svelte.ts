/**
 * 导航历史栈模块
 *
 * 提供类浏览器的后退导航能力，维护一个 NavigationEntry 的有序栈。
 * 本模块为纯数据层，不依赖任何 UI 框架组件，供 appRuntime.svelte.ts 消费。
 *
 * 使用约束：
 * - 连续 push 相同 entry（由 isSameEntry 判定）时自动去重，不会产生重复栈帧
 * - pop/peek 在栈为空时安全返回 undefined，不抛出异常
 * - 所有操作均为不可变风格（immutable），不直接修改原数组
 */

/** 应用内各视图的导航入口，使用判别联合类型以携带视图专属参数 */
export type NavigationEntry =
  | { view: 'home' }
  | { view: 'search' }
  | { view: 'overview' }
  | { view: 'library'; albumCid: string | null }
  | { view: 'collection'; collectionId: string }
  | { view: 'tagEditor'; albumCid: string | null; songCid: string | null };

/**
 * 判断两个 NavigationEntry 是否指向同一个视图状态。
 *
 * 用于 push 时的去重判断：若栈顶与待入栈 entry 相同，则跳过入栈。
 *
 * @param a 第一个导航入口
 * @param b 第二个导航入口
 * @returns 两者指向同一视图状态时返回 true
 */
export function isSameEntry(a: NavigationEntry, b: NavigationEntry): boolean {
  if (a.view !== b.view) return false;
  switch (a.view) {
    case 'home':
    case 'search':
    case 'overview':
      return true;
    case 'library':
      return a.albumCid === (b as typeof a).albumCid;
    case 'collection':
      return a.collectionId === (b as typeof a).collectionId;
    case 'tagEditor':
      return (
        a.albumCid === (b as typeof a).albumCid &&
        a.songCid === (b as typeof a).songCid
      );
  }
}

const MAX_STACK_SIZE = 50;

let stack = $state<NavigationEntry[]>([]);

/**
 * 将新的导航入口压入栈顶。
 *
 * 若栈顶与 entry 相同（由 isSameEntry 判定），则跳过，不产生重复栈帧。
 * 栈深度超过 MAX_STACK_SIZE 时自动丢弃最早的条目。
 *
 * @param entry 待入栈的导航入口
 */
function push(entry: NavigationEntry): void {
  if (stack.length > 0 && isSameEntry(stack[stack.length - 1]!, entry)) return;
  const next = [...stack, entry];
  stack = next.length > MAX_STACK_SIZE ? next.slice(-MAX_STACK_SIZE) : next;
}

/**
 * 弹出栈顶的导航入口并返回。
 *
 * @returns 弹出的导航入口；栈为空时返回 undefined
 */
function pop(): NavigationEntry | undefined {
  if (stack.length === 0) return undefined;
  const top = stack[stack.length - 1];
  stack = stack.slice(0, -1);
  return top;
}

/**
 * 查看栈顶的导航入口，不移除。
 *
 * @returns 栈顶的导航入口；栈为空时返回 undefined
 */
function peek(): NavigationEntry | undefined {
  return stack[stack.length - 1];
}

/**
 * 清空导航历史栈。
 */
function clear(): void {
  stack = [];
}

/**
 * 导航历史栈，提供类浏览器的后退导航能力。
 *
 * 典型使用场景：
 * - 用户切换视图时调用 push 记录历史
 * - 用户点击后退时调用 pop 回到上一个视图
 * - 通过 canGoBack 判断是否显示后退按钮
 *
 * 调用约束：
 * - 本模块为单例，全局共享同一个栈实例
 * - 应在应用初始化时调用 clear 重置状态
 */
export const navigationStack = {
  /** 栈非空时为 true，可用于控制后退按钮的可见性 */
  get canGoBack() {
    return stack.length > 0;
  },
  /** 当前栈中的条目数量 */
  get size() {
    return stack.length;
  },
  push,
  pop,
  peek,
  clear,
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    navigationStack.clear();
  });
}
