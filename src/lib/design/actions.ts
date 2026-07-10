/**
 * Svelte action：将 node 元素的实际高度同步到 CSS 自定义属性 --brand-region-height。
 *
 * 用途：挂载在 .brand-region 浮层根节点上，替代 App.svelte 中手写的 $effect + ResizeObserver。
 * .sidebar-brand-spacer 通过该变量预留等高安全区，把导航推到浮层下方。
 * 折叠态下字母竖排会显著增高，必须实时跟随，否则 spacer 停在兜底值，
 * logo/slab 会直接覆盖在导航之上。
 *
 * 实现细节：
 * - 挂载时立即同步一次；
 * - 通过 ResizeObserver 监听后续尺寸变化；
 * - 延迟到下一帧（rAF）再写回，避免回调内同步改动布局触发
 *   "ResizeObserver loop completed with undelivered notifications" 警告；
 * - destroy 时断开 observer 并取消未执行的 rAF。
 *
 * @param node - 被观测的 HTMLElement（.brand-region 根节点）
 * @returns Svelte action 销毁句柄
 */
export function syncBrandHeight(node: HTMLElement): { destroy(): void } {
  const syncHeight = () => {
    document.documentElement.style.setProperty(
      '--brand-region-height',
      `${node.offsetHeight}px`
    );
  };

  syncHeight();

  if (typeof ResizeObserver === 'undefined') {
    return { destroy() {} };
  }

  let rafId = 0;
  const observer = new ResizeObserver(() => {
    // 延迟到下一帧再写回，避免回调内同步改动布局触发
    // "ResizeObserver loop completed with undelivered notifications" 警告
    if (rafId) cancelAnimationFrame(rafId);
    rafId = requestAnimationFrame(() => {
      rafId = 0;
      syncHeight();
    });
  });
  observer.observe(node);

  return {
    destroy() {
      if (rafId) cancelAnimationFrame(rafId);
      observer.disconnect();
    },
  };
}
