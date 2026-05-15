import gsap from 'gsap';
import { Flip } from 'gsap/Flip';
import { CustomEase } from 'gsap/CustomEase';

gsap.registerPlugin(Flip, CustomEase);

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
