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
    duration: getMotionDuration(300),
    ease: 'ios-out',
  });
}
