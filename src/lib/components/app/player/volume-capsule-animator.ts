import {
  gsap,
  killTweens,
  getMotionDuration,
  MOTION,
  shouldSkipMotion,
} from '$lib/design/gsap';

const CAPSULE_WIDTH = 200;
const ARK_CAPSULE_EXPAND_SECONDS = 0.18;
const ARK_CAPSULE_COLLAPSE_SECONDS = 0.12;

export interface CapsuleAnimatorRefs {
  track: HTMLElement | null;
  badge: HTMLElement | null;
  iconBtn: HTMLButtonElement | null;
}

/**
 * 音量胶囊的家族无关动画接口（Phase 3 Step 3.2）。
 *
 * 由 view 层根据当前 visualContract.family 选择实现：
 * - `createGlassCapsuleAnimator`：iOS spring + 圆角形变（现有行为）
 * - `createMaterialCapsuleAnimator`：Material 3 easing + elevation
 *
 * 业务逻辑（state 机、collapse timer）由 view 层的 controller 承担，
 * animator 只关心 tween 参数与 DOM 副作用。
 */
export interface CapsuleAnimator {
  expand(onComplete: () => void): void;
  collapse(onComplete: () => void): void;
  showBadge(): void;
  hideBadge(): void;
  destroy(): void;
}

/**
 * Glass family 音量胶囊动画器：iOS spring 展开 + 圆角回收 + 波形按钮。
 *
 * 现有 VolumeCapsule.svelte 的默认行为，Phase 3 之前是唯一的 animator 实现。
 */
export function createGlassCapsuleAnimator(
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
        { y: '0%', duration: getMotionDuration(MOTION.BASE), ease: 'ios-out' }
      );
    }
  }

  function hideBadgeInternal(refs: CapsuleAnimatorRefs) {
    if (!refs.badge || !badgeVisible) return;
    killTweens(refs.badge);
    gsap.to(refs.badge, {
      y: '100%',
      duration: getMotionDuration(MOTION.BASE_OUT),
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

      // 胶囊展开/收缩是一对经权衡的非对称特例：展开 400ms 从容铺开，
      // 收缩 799ms 慢速回收，节奏刻意不对称，故不并入通用时长令牌。
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
    },

    collapse(onComplete: () => void) {
      const refs = getRefs();
      if (!refs.track) return;

      const shrinkDuration = getMotionDuration(799);
      const badgeHideDuration = getMotionDuration(MOTION.BASE_OUT);

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

    destroy() {
      const refs = getRefs();
      if (refs.track) killTweens(refs.track);
      if (refs.badge) killTweens(refs.badge);
      if (refs.iconBtn) killTweens(refs.iconBtn);
    },
  };
}

/**
 * Material family 音量胶囊动画器：直接 fade-in 宽度扩展，不做圆角形变。
 *
 * Material 3 惯例是 elevation + emphasized easing。胶囊两端保持标准圆角（--shape-lg），
 * 展开 / 收缩节奏对称（BASE 时长），不复用 glass 的 799ms 非对称配置。
 */
export function createMaterialCapsuleAnimator(
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
        { y: '100%', opacity: 0 },
        {
          y: '0%',
          opacity: 1,
          duration: getMotionDuration(MOTION.BASE),
          ease: 'ios-out',
        }
      );
    }
  }

  function hideBadgeInternal(refs: CapsuleAnimatorRefs) {
    if (!refs.badge || !badgeVisible) return;
    killTweens(refs.badge);
    gsap.to(refs.badge, {
      y: '100%',
      opacity: 0,
      duration: getMotionDuration(MOTION.BASE_OUT),
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
      killTweens(refs.track);
      gsap.to(refs.track, {
        width: CAPSULE_WIDTH,
        opacity: 1,
        duration: getMotionDuration(MOTION.BASE),
        ease: 'ios-out',
        onComplete,
      });
    },

    collapse(onComplete: () => void) {
      const refs = getRefs();
      if (!refs.track) return;
      killTweens(refs.track);
      hideBadgeInternal(refs);
      gsap.to(refs.track, {
        width: 0,
        opacity: 0,
        duration: getMotionDuration(MOTION.BASE_OUT),
        ease: 'ios-in',
        onComplete,
      });
    },

    showBadge() {
      showBadgeInternal(getRefs());
    },

    hideBadge() {
      hideBadgeInternal(getRefs());
    },

    destroy() {
      const refs = getRefs();
      if (refs.track) killTweens(refs.track);
      if (refs.badge) killTweens(refs.badge);
      if (refs.iconBtn) killTweens(refs.iconBtn);
    },
  };
}

/**
 * The five Ark UI families share one HCI motion model. Family identity stays in
 * CSS geometry and tokens; timing and control response stay predictable.
 */
export function createArkCapsuleAnimator(
  getRefs: () => CapsuleAnimatorRefs
): CapsuleAnimator {
  let initialized = false;

  function prepareTrack(refs: CapsuleAnimatorRefs): void {
    if (!refs.track || initialized) return;
    killTweens(refs.track);
    gsap.set(refs.track, {
      width: 0,
      opacity: 0,
      visibility: 'hidden',
      pointerEvents: 'none',
    });
    initialized = true;
  }

  function runTrack(targetOpen: boolean, onComplete: () => void): void {
    const refs = getRefs();
    if (!refs.track) {
      onComplete();
      return;
    }

    prepareTrack(refs);
    killTweens(refs.track);
    if (targetOpen) {
      gsap.set(refs.track, {
        visibility: 'visible',
        pointerEvents: 'auto',
      });
    }
    const terminal = targetOpen
      ? { width: CAPSULE_WIDTH, opacity: 1 }
      : { width: 0, opacity: 0 };
    if (shouldSkipMotion()) {
      gsap.set(refs.track, terminal);
      if (!targetOpen) {
        gsap.set(refs.track, { visibility: 'hidden', pointerEvents: 'none' });
      }
      onComplete();
      return;
    }

    gsap.to(refs.track, {
      ...terminal,
      // This compact control keeps one interaction tempo across theme packages.
      // Package-level motion overrides still style larger family surfaces.
      duration: targetOpen
        ? ARK_CAPSULE_EXPAND_SECONDS
        : ARK_CAPSULE_COLLAPSE_SECONDS,
      ease: targetOpen ? 'ios-out' : 'ios-in',
      onComplete: () => {
        if (!targetOpen && refs.track) {
          gsap.set(refs.track, {
            visibility: 'hidden',
            pointerEvents: 'none',
          });
        }
        onComplete();
      },
    });
  }

  return {
    expand(onComplete) {
      runTrack(true, onComplete);
    },
    collapse(onComplete) {
      runTrack(false, onComplete);
    },
    showBadge() {
      // Ark families use the stable inline readout instead of a transient badge.
    },
    hideBadge() {
      // Ark families use the stable inline readout instead of a transient badge.
    },
    destroy() {
      const refs = getRefs();
      if (refs.track) killTweens(refs.track);
      if (refs.badge) killTweens(refs.badge);
      if (refs.iconBtn) killTweens(refs.iconBtn);
      initialized = false;
    },
  };
}

/**
 * @deprecated 使用 `createGlassCapsuleAnimator`。保留仅为过渡期回退，Phase 3 完成后移除。
 */
export const createCapsuleAnimator = createGlassCapsuleAnimator;
