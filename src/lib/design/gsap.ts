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
