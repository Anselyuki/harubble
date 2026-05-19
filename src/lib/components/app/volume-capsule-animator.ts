import { gsap, killTweens, getMotionDuration } from '$lib/design/gsap';

const CAPSULE_WIDTH = 200;

export interface CapsuleAnimatorRefs {
  track: HTMLElement | null;
  badge: HTMLElement | null;
  iconBtn: HTMLButtonElement | null;
}

export interface CapsuleAnimator {
  expand(onComplete: () => void): void;
  collapse(onComplete: () => void): void;
  showBadge(): void;
  hideBadge(): void;
  hoverIconIn(): void;
  hoverIconOut(): void;
  destroy(): void;
}

export function createCapsuleAnimator(
  getRefs: () => CapsuleAnimatorRefs
): CapsuleAnimator {
  let badgeVisible = false;

  function showBadgeInternal(refs: CapsuleAnimatorRefs) {
    if (!refs.badge || !refs.track) return;

    if (!badgeVisible) {
      badgeVisible = true;
      killTweens(refs.badge);
      gsap.fromTo(
        refs.badge,
        { y: '100%' },
        { y: '0%', duration: getMotionDuration(200), ease: 'ios-out' }
      );
    }
  }

  function hideBadgeInternal(refs: CapsuleAnimatorRefs) {
    if (!refs.badge || !badgeVisible) return;
    killTweens(refs.badge);
    gsap.to(refs.badge, {
      y: '100%',
      duration: getMotionDuration(180),
      ease: 'ios-in',
      onComplete: () => {
        badgeVisible = false;
      },
    });
  }

  return {
    expand(onComplete: () => void) {
      const refs = getRefs();
      if (!refs.track) return;

      const duration = getMotionDuration(400);
      const bg = getComputedStyle(refs.track)
        .getPropertyValue('--capsule-track-bg')
        .trim();

      killTweens(refs.track);
      gsap.set(refs.track, { backgroundColor: bg || '#f2f2f7' });
      gsap.to(refs.track, {
        width: CAPSULE_WIDTH,
        opacity: 1,
        borderTopLeftRadius: 0,
        borderTopRightRadius: 0,
        borderBottomRightRadius: 16,
        borderBottomLeftRadius: 16,
        duration,
        ease: 'ios-spring',
        onComplete,
      });

      showBadgeInternal(refs);
    },

    collapse(onComplete: () => void) {
      const refs = getRefs();
      if (!refs.track) return;

      const shrinkDuration = getMotionDuration(799);
      const badgeHideDuration = getMotionDuration(180);

      killTweens(refs.track);
      hideBadgeInternal(refs);

      gsap.to(refs.track, {
        width: 0,
        opacity: 0,
        borderTopLeftRadius: 999,
        borderTopRightRadius: 999,
        borderBottomRightRadius: 999,
        borderBottomLeftRadius: 999,
        duration: shrinkDuration,
        delay: badgeHideDuration * 0.5,
        ease: 'ios',
        onComplete: () => {
          gsap.set(refs.track!, { backgroundColor: 'transparent' });
          onComplete();
        },
      });
    },

    showBadge() {
      showBadgeInternal(getRefs());
    },

    hideBadge() {
      hideBadgeInternal(getRefs());
    },

    hoverIconIn() {
      const { iconBtn } = getRefs();
      if (!iconBtn) return;
      gsap.to(iconBtn, {
        color: 'var(--icon-active)',
        backgroundColor:
          'var(--player-control-hover-bg, rgba(var(--accent-rgb), 0.1))',
        duration: getMotionDuration(150),
        ease: 'ios-out',
      });
    },

    hoverIconOut() {
      const { iconBtn } = getRefs();
      if (!iconBtn) return;
      gsap.to(iconBtn, {
        color: 'var(--icon-default)',
        backgroundColor: 'transparent',
        duration: getMotionDuration(150),
        ease: 'ios-in',
      });
    },

    destroy() {
      const refs = getRefs();
      if (refs.track) killTweens(refs.track);
      if (refs.badge) killTweens(refs.badge);
      if (refs.iconBtn) killTweens(refs.iconBtn);
    },
  };
}
