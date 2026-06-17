import gsap from 'gsap';
import { Flip } from 'gsap/Flip';
import { CustomEase } from 'gsap/CustomEase';
import { ScrollToPlugin } from 'gsap/ScrollToPlugin';

gsap.registerPlugin(Flip, CustomEase, ScrollToPlugin);

CustomEase.create('ios', '0.25, 0.1, 0.25, 1.0');
CustomEase.create('ios-in', '0.42, 0, 1, 1');
CustomEase.create('ios-out', '0, 0, 0.58, 1');
CustomEase.create('ios-spring', '0.22, 0.61, 0.36, 1');

export { gsap, Flip };

export const reducedMotionQuery =
  typeof window !== 'undefined'
    ? window.matchMedia('(prefers-reduced-motion: reduce)')
    : null;

export function getMotionDuration(baseMs: number): number {
  return reducedMotionQuery?.matches ? 0 : baseMs / 1000;
}

/**
 * 统一动效时长令牌（毫秒）。
 *
 * 作为前端 GSAP 动画时长的单一真相来源，主档位与 `src/app.css` 中的
 * `--motion-fast / --motion-base / --motion-slow / --motion-page` CSS 变量
 * 一一对应；新增动画请从这里取值，不要在组件内硬编码毫秒数。
 *
 * 进出语义档位：
 * - `MICRO`：极快微交互（按下回弹、小浮层关闭），快于 `FAST`
 * - `FAST`：hover / 细粒度状态反馈（对应 `--motion-fast`）
 * - `BASE`：常规元素进出（对应 `--motion-base`）
 * - `SLOW`：覆盖层 / 较大元素进出（对应 `--motion-slow`）
 * - `PAGE`：主视图页面级转场（对应 `--motion-page`）
 *
 * 出场普遍略短于入场，以营造“快速让位、从容入场”的 iOS 观感，
 * 因此常规档位提供独立的 `*_OUT` 值。
 *
 * 专用入场档：
 * - `OVERLAY_IN`：浮层（dialog / select / tooltip / popover）的统一入场时长，
 *   与浮层出场（`BASE_OUT` / `MICRO`）配对使用，使各类浮层进出节奏一致。
 *
 * 注：少数经权衡的有意特例（如音量胶囊收缩的非对称时长）不走令牌，
 * 在调用点就地以注释说明，不强行并入档位。
 */
export const MOTION = {
  MICRO: 100,
  FAST: 140,
  BASE: 180,
  SLOW: 260,
  PAGE: 320,
  BASE_OUT: 150,
  SLOW_OUT: 200,
  PAGE_OUT: 280,
  OVERLAY_IN: 200,
} as const;

/**
 * 页面级转场的水平位移量（百分比）。
 *
 * 入场视图从屏幕外整屏滑入（`100`），离场视图反向小幅位移（`18`）并淡出，
 * 形成方向一致、互不遮挡的“推进 / 后退”观感。
 */
export const VIEW_SHIFT = {
  IN: 100,
  OUT: 18,
} as const;

export function killTweens(targets: gsap.TweenTarget): void {
  gsap.killTweensOf(targets);
}

export function animateIn(
  target: gsap.TweenTarget,
  fromVars: gsap.TweenVars,
  toVars: gsap.TweenVars,
  durationMs: number,
  ease?: string
): gsap.core.Tween {
  killTweens(target);
  return gsap.fromTo(target, fromVars, {
    ...toVars,
    duration: getMotionDuration(durationMs),
    ease: ease ?? 'ios-spring',
  });
}

export function animateOut(
  target: gsap.TweenTarget,
  toVars: gsap.TweenVars,
  durationMs: number,
  opts?: { ease?: string; onComplete?: () => void }
): gsap.core.Tween {
  killTweens(target);
  return gsap.to(target, {
    ...toVars,
    duration: getMotionDuration(durationMs),
    ease: opts?.ease ?? 'ios-in',
    onComplete: opts?.onComplete,
  });
}

/**
 * 等待文档字体加载完成。
 *
 * 用于动画启动前确保字体度量稳定，避免 scrollWidth 等测量值因字体 fallback 而偏差。
 * 内置 500ms 超时保护，超时后静默 resolve，不阻塞动画流程。
 */
export function awaitFontsReady(): Promise<void> {
  if (typeof document === 'undefined') return Promise.resolve();
  return Promise.race([
    document.fonts.ready.then(() => {}),
    new Promise<void>((resolve) => setTimeout(resolve, 500)),
  ]);
}

/**
 * 等待浏览器完成首次 paint（双帧 rAF）+ 字体就绪。
 *
 * 用于 FLIP 动画首次触发前，确保 getBoundingClientRect() 返回准确坐标。
 * 合并双帧等待与字体加载，取两者中较晚完成的时机。
 */
export function awaitPaintReady(): Promise<void> {
  if (typeof window === 'undefined') return Promise.resolve();
  const frameDone = new Promise<void>((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });
  return Promise.all([frameDone, awaitFontsReady()]).then(() => {});
}

export function gsapScrollIntoView(
  container: HTMLElement,
  target: HTMLElement,
  block: 'center' | 'nearest' = 'nearest'
): void {
  const containerRect = container.getBoundingClientRect();
  const targetRect = target.getBoundingClientRect();
  const currentScrollTop = container.scrollTop;

  const targetTop = currentScrollTop + (targetRect.top - containerRect.top);
  const targetBottom = targetTop + targetRect.height;

  let scrollTarget: number;

  if (block === 'center') {
    scrollTarget = targetTop - (container.clientHeight - targetRect.height) / 2;
  } else {
    const isAbove = targetRect.top < containerRect.top;
    const isBelow = targetRect.bottom > containerRect.bottom;

    if (!isAbove && !isBelow) return;

    if (isAbove) {
      scrollTarget = targetTop;
    } else {
      scrollTarget = targetBottom - container.clientHeight;
    }
  }

  gsap.to(container, {
    scrollTo: { y: scrollTarget },
    duration: getMotionDuration(MOTION.PAGE),
    ease: 'ios-out',
  });
}
