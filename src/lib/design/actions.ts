import { gsap, getMotionDuration, killTweens, MOTION } from './gsap';

/**
 * Svelte action：以 GSAP 驱动 `.fullscreen-lyric-line` 的高亮进度变量。
 *
 * 该 action 通过 tween `--lyric-progress`（0 → 1）让 CSS 侧的 font-size、
 * transform、color 派生表达式产生平滑过渡；不直接操纵 font-size / transform，
 * 因此 responsive media query 覆盖 `--lyric-size-{base,active}` 时无需 JS 感知。
 *
 * 初次挂载不做 tween，直接 set 到目标值，避免首屏出现无意义的推入动画；
 * 每次 `update` 比对目标值，仅在变化时执行 tween；`destroy` 会 kill 未完成的
 * tween，防止组件卸载后仍持续修改 inline style。
 *
 * @param node 目标歌词行元素
 * @param active 当前是否为活动行
 * @returns Svelte action 的 update / destroy 生命周期钩子
 */
export function lyricActiveTween(
  node: HTMLElement,
  active: boolean
): { update(active: boolean): void; destroy(): void } {
  let currentTarget = active ? 1 : 0;
  gsap.set(node, { '--lyric-progress': currentTarget });

  return {
    update(nextActive: boolean) {
      const target = nextActive ? 1 : 0;
      if (target === currentTarget) return;
      currentTarget = target;
      killTweens(node);
      gsap.to(node, {
        '--lyric-progress': target,
        duration: getMotionDuration(MOTION.SLOW),
        ease: 'ios-spring',
      });
    },
    destroy() {
      killTweens(node);
    },
  };
}

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
