/**
 * 侧栏拖曳宽度控制器
 *
 * 在侧栏右边缘提供拖曳把手，允许用户自由调整侧栏宽度。
 * 拖曳期间实时更新 `--sidebar-width` CSS 变量并通知布局状态变化，
 * 松手后吸附至展开态（248px）或折叠态（56px）。
 *
 * ## 交互规则
 *
 * - 拖曳范围：[collapsedWidth, expandedWidth]
 * - 拖曳期间实时通知宽度变化与阈值交叉
 * - 松手时根据当前宽度是否超过阈值决定吸附方向
 * - 使用 Pointer Capture 确保拖出把手区域仍正常响应
 */
import { gsap } from '$lib/design/gsap';

export interface SidebarResizeConfig {
  /** 应用壳层根元素（设置 --sidebar-width 的容器） */
  shellEl: HTMLElement;
  /** 拖曳把手元素 */
  handleEl: HTMLElement;
  /** 折叠态宽度 */
  collapsedWidth: number;
  /** 展开态宽度 */
  expandedWidth: number;
  /** 切换阈值——低于此宽度视为折叠意图 */
  threshold: number;
  /** 获取当前折叠状态 */
  getCollapsed: () => boolean;
  /** 拖曳中实时宽度变化回调 */
  onWidthChange: (width: number) => void;
  /** 拖曳中跨越阈值时触发的布局切换回调 */
  onCrossThreshold: (collapsed: boolean) => void;
  /** 拖曳结束回调——finalWidth 为松手时宽度，shouldCollapse 为吸附方向 */
  onDragEnd: (finalWidth: number, shouldCollapse: boolean) => void;
  /** 键盘每次调整的宽度 */
  keyboardStep?: number;
}

export interface SidebarResizeHandle {
  /** 销毁事件监听，清理资源 */
  dispose: () => void;
}

/**
 * 创建侧栏拖曳控制器。
 *
 * 绑定 pointer 事件到 handleEl，拖曳期间实时调用回调，松手后由外部决定吸附动画。
 */
export function createSidebarResize(
  config: SidebarResizeConfig
): SidebarResizeHandle {
  const {
    shellEl,
    handleEl,
    collapsedWidth,
    expandedWidth,
    threshold,
    getCollapsed,
    onWidthChange,
    onCrossThreshold,
    onDragEnd,
    keyboardStep = 16,
  } = config;

  let isDragging = false;
  let startX = 0;
  let startWidth = 0;
  /** 拖曳期间追踪的折叠态，用于检测阈值交叉 */
  let dragCollapsed = false;

  function getCurrentWidth(): number {
    const raw = shellEl.style.getPropertyValue('--sidebar-width');
    if (raw) return Number.parseFloat(raw);
    return getCollapsed() ? collapsedWidth : expandedWidth;
  }

  function clampWidth(w: number): number {
    return Math.min(Math.max(w, collapsedWidth), expandedWidth);
  }

  function handlePointerDown(e: PointerEvent) {
    // 仅响应主键
    if (e.button !== 0) return;
    e.preventDefault();
    isDragging = true;
    startX = e.clientX;
    startWidth = getCurrentWidth();
    dragCollapsed = getCollapsed();
    handleEl.setPointerCapture(e.pointerId);
    // 拖曳期间禁用文本选择
    document.body.style.userSelect = 'none';
    document.body.style.cursor = 'col-resize';
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging) return;
    const deltaX = e.clientX - startX;
    const newWidth = clampWidth(startWidth + deltaX);

    onWidthChange(newWidth);

    // 检测阈值交叉
    const shouldBeCollapsed = newWidth < threshold;
    if (shouldBeCollapsed !== dragCollapsed) {
      dragCollapsed = shouldBeCollapsed;
      onCrossThreshold(shouldBeCollapsed);
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (!isDragging) return;
    isDragging = false;
    handleEl.releasePointerCapture(e.pointerId);
    document.body.style.userSelect = '';
    document.body.style.cursor = '';

    const finalWidth = getCurrentWidth();
    const shouldCollapse = finalWidth < threshold;
    onDragEnd(finalWidth, shouldCollapse);
  }

  function handleKeyDown(e: KeyboardEvent) {
    const currentWidth = getCurrentWidth();
    let nextWidth: number | null = null;
    if (e.key === 'ArrowLeft') nextWidth = currentWidth - keyboardStep;
    if (e.key === 'ArrowRight') {
      nextWidth = getCollapsed()
        ? Math.max(threshold, currentWidth + keyboardStep)
        : currentWidth + keyboardStep;
    }
    if (e.key === 'Home') nextWidth = collapsedWidth;
    if (e.key === 'End') nextWidth = expandedWidth;
    if (nextWidth === null) return;

    e.preventDefault();
    nextWidth = clampWidth(nextWidth);
    const shouldCollapse = nextWidth < threshold;
    onWidthChange(nextWidth);
    if (shouldCollapse !== getCollapsed()) {
      onCrossThreshold(shouldCollapse);
    }
    onDragEnd(nextWidth, shouldCollapse);
  }

  // 绑定事件
  handleEl.addEventListener('pointerdown', handlePointerDown);
  handleEl.addEventListener('pointermove', handlePointerMove);
  handleEl.addEventListener('pointerup', handlePointerUp);
  // 防止拖曳取消（如浏览器触摸滑动）
  handleEl.addEventListener('pointercancel', handlePointerUp);
  handleEl.addEventListener('keydown', handleKeyDown);

  function dispose() {
    handleEl.removeEventListener('pointerdown', handlePointerDown);
    handleEl.removeEventListener('pointermove', handlePointerMove);
    handleEl.removeEventListener('pointerup', handlePointerUp);
    handleEl.removeEventListener('pointercancel', handlePointerUp);
    handleEl.removeEventListener('keydown', handleKeyDown);
    if (isDragging) {
      document.body.style.userSelect = '';
      document.body.style.cursor = '';
    }
  }

  return { dispose };
}

/**
 * 侧栏吸附动画——从当前宽度动画到目标宽度。
 *
 * 用于拖曳松手后的弹性吸附效果，不触发 Logo FLIP 等复杂动画。
 *
 * @param shellEl 壳层元素（持有 --sidebar-width 变量）
 * @param targetWidth 吸附目标宽度（px）
 * @param durationMs 动画时长（ms），默认 200
 * @param onComplete 动画完成回调
 * @returns GSAP Tween 实例，可用于外部 kill
 */
export function animateSnapToWidth(
  shellEl: HTMLElement,
  targetWidth: number,
  durationMs = 200,
  onComplete?: () => void
): gsap.core.Tween {
  const currentWidth =
    Number.parseFloat(shellEl.style.getPropertyValue('--sidebar-width')) ||
    targetWidth;

  const proxy = { width: currentWidth };
  return gsap.to(proxy, {
    width: targetWidth,
    duration: durationMs / 1000,
    ease: 'ios-spring',
    onUpdate: () => {
      shellEl.style.setProperty('--sidebar-width', `${proxy.width}px`);
    },
    onComplete,
  });
}
