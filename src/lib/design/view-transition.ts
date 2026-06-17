/**
 * 主视图页面级转场的底层动画原语。
 *
 * 负责把“离开的视图”冻结为一层静态 DOM 快照，并与“进入的视图”用 GSAP
 * 时间线协调地完成方向一致的推进 / 后退动画。冻结采用真实 DOM 节点的
 * 深拷贝（`cloneNode(true)`）而非 `innerHTML` 字符串，以保留已渲染的图片、
 * 滚动位置与布局度量，避免重新加载导致的闪白与错位。
 *
 * 该模块只处理“一进一出”的转场编排，不感知具体视图内容，也不持有任何
 * 业务状态；视图的挂载 / 卸载时序仍由调用方（`ViewTransition.svelte`）控制。
 */
import { gsap, getMotionDuration, MOTION, VIEW_SHIFT } from '$lib/design/gsap';

/** 转场方向：`forward` 为推进（新视图从右入），`back` 为后退（新视图从左入）。 */
export type TransitionDirection = 'forward' | 'back';

/**
 * 把一个已渲染的视图节点冻结为静态快照元素。
 *
 * 对 `source` 做深拷贝，并把其内部所有可滚动容器的 `scrollTop` / `scrollLeft`
 * 同步到对应的克隆节点上，使快照在视觉上与冻结前完全一致。返回的元素未挂载，
 * 由调用方决定插入位置；其 `pointer-events` 已被禁用，不会拦截交互。
 *
 * @param source 待冻结的真实视图根节点
 * @returns 可直接插入 DOM 的静态快照元素
 */
export function freezeViewSnapshot(source: HTMLElement): HTMLElement {
  const clone = source.cloneNode(true) as HTMLElement;

  // 在 source 侧收集所有存在滚动偏移的节点，记录其在扁平 DOM 树中的索引，
  // 再通过相同索引定位 clone 侧对应节点并还原偏移量。clone 的 scrollTop
  // 初始为 0，不能用”非零过滤”在 clone 侧查找。
  syncScrollPositions(source, clone);

  clone.style.pointerEvents = 'none';
  return clone;
}

/**
 * 将 source 树中有滚动偏移的节点的 scrollTop/scrollLeft 还原到 clone 树的
 * 对应节点上。通过扁平 querySelectorAll 索引一一对应，保证结构一致时定位准确。
 */
function syncScrollPositions(source: HTMLElement, clone: HTMLElement): void {
  // 根节点本身
  if (source.scrollTop > 0 || source.scrollLeft > 0) {
    clone.scrollTop = source.scrollTop;
    clone.scrollLeft = source.scrollLeft;
  }

  const sourceAll = source.querySelectorAll<HTMLElement>('*');
  const cloneAll = clone.querySelectorAll<HTMLElement>('*');
  const count = Math.min(sourceAll.length, cloneAll.length);
  for (let i = 0; i < count; i++) {
    const s = sourceAll[i];
    if (s.scrollTop > 0 || s.scrollLeft > 0) {
      cloneAll[i].scrollTop = s.scrollTop;
      cloneAll[i].scrollLeft = s.scrollLeft;
    }
  }
}

/** {@link runViewTransition} 的入参。 */
export interface ViewTransitionParams {
  /** 进入的视图根节点（live 真实组件）。 */
  incoming: HTMLElement;
  /** 离开的视图快照（由 {@link freezeViewSnapshot} 生成并已挂载）。 */
  outgoing: HTMLElement;
  /** 转场方向，决定水平位移的正负。 */
  direction: TransitionDirection;
  /** 入场时长（毫秒），默认 {@link MOTION.PAGE}。 */
  durationIn?: number;
  /** 出场时长（毫秒），默认 {@link MOTION.PAGE_OUT}。 */
  durationOut?: number;
  /** 时间线开始时回调（用于标记转场进行中）。 */
  onStart?: () => void;
  /** 时间线结束 / 被打断后回调（用于清理与状态复位）。 */
  onComplete?: () => void;
}

/**
 * 执行一次主视图页面级转场。
 *
 * 入场视图从屏幕外整屏滑入并保持不透明，离场快照同向小幅位移并淡出，
 * 两者使用统一的 iOS 缓动以避免重叠期的“重影”。`reduced motion` 下
 * `getMotionDuration` 会把时长归零，转场退化为瞬时切换。
 *
 * @returns 创建的 GSAP 时间线；调用方可在卸载 / 打断时对其 `kill()`。
 *   `onComplete` 在自然结束时触发；若被 `kill()`，调用方需自行调用清理逻辑。
 */
export function runViewTransition(
  params: ViewTransitionParams
): gsap.core.Timeline {
  const {
    incoming,
    outgoing,
    direction,
    durationIn = MOTION.PAGE,
    durationOut = MOTION.PAGE_OUT,
    onStart,
    onComplete,
  } = params;

  const isForward = direction === 'forward';
  const inFromPercent = isForward ? VIEW_SHIFT.IN : -VIEW_SHIFT.IN;
  const outToPercent = isForward ? -VIEW_SHIFT.OUT : VIEW_SHIFT.OUT;
  const durIn = getMotionDuration(durationIn);
  const durOut = getMotionDuration(durationOut);

  const tl = gsap.timeline({
    onStart,
    onComplete,
  });

  tl.fromTo(
    incoming,
    { xPercent: inFromPercent, opacity: 1 },
    { xPercent: 0, opacity: 1, duration: durIn, ease: 'ios-spring' },
    0
  );
  tl.fromTo(
    outgoing,
    { xPercent: 0, opacity: 1 },
    { xPercent: outToPercent, opacity: 0, duration: durOut, ease: 'ios-out' },
    0
  );

  return tl;
}

/** {@link runLayeredIn} 中单个层的动画描述。 */
export interface LayeredInLayer {
  /** 目标元素；为空（组件尚未挂载）时该层被跳过。 */
  target: HTMLElement | null | undefined;
  /** 入场起始的纵向位移（像素），默认 0（仅淡入）。 */
  fromY?: number;
}

/**
 * 以统一节奏为一组层叠元素编排“分层进入”动画。
 *
 * 各层共享同一时间线，按数组顺序以固定 `stagger` 依次淡入并上移归位，
 * 形成详情面板（封面 → 标题信息 → 列表）那种由整体到局部的进入观感。
 * 相比为每个区块各写一个 `$effect`，这里保证了节奏、曲线与时长的一致性，
 * 也便于在卸载时统一 `kill()`。`reduced motion` 下时长归零，退化为瞬时显示。
 *
 * @param layers 自上而下的层描述；`target` 为空的层会被静默跳过
 * @param opts.duration 单层入场时长（毫秒），默认 {@link MOTION.SLOW}
 * @param opts.stagger 相邻层之间的启动间隔（秒），默认 0.05
 * @returns 创建的 GSAP 时间线；调用方应在卸载时 `kill()`
 */
export function runLayeredIn(
  layers: LayeredInLayer[],
  opts?: { duration?: number; stagger?: number }
): gsap.core.Timeline {
  const duration = getMotionDuration(opts?.duration ?? MOTION.SLOW);
  // reduced motion 下 duration 归零，stagger 也必须归零，
  // 否则各层仍按间隔依次显示，不是真正的"瞬时同时出现"。
  const stagger = duration === 0 ? 0 : (opts?.stagger ?? 0.05);

  const tl = gsap.timeline();
  let index = 0;
  for (const layer of layers) {
    if (!layer.target) continue;
    tl.fromTo(
      layer.target,
      { opacity: 0, y: layer.fromY ?? 0 },
      { opacity: 1, y: 0, duration, ease: 'ios-spring' },
      index * stagger
    );
    index++;
  }
  return tl;
}
