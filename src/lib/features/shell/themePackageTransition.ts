import { flushSync } from 'svelte';
import {
  gsap,
  getMotionDuration,
  MOTION,
  shouldSkipMotion,
} from '$lib/design/gsap';
import { runCssVariableWriteTransaction } from '$lib/theme';

export type ThemePackageTransitionReason =
  | 'activate'
  | 'preview'
  | 'dismiss-preview';

export interface ThemePackageTransitionOptions {
  animate?: boolean;
  reason?: ThemePackageTransitionReason;
  targetPackageId?: string | null;
}

interface ActiveThemePackageTransition {
  overlay: HTMLDivElement;
  timeline: gsap.core.Timeline | null;
  committed: boolean;
  targetPackageId: string | null | undefined;
  resolve: () => void;
  reject: (reason: unknown) => void;
}

let activeTransition: ActiveThemePackageTransition | null = null;

function swallowTransitionPointerEvent(event: Event): void {
  event.preventDefault();
  event.stopImmediatePropagation();
}

/** 同步提交主题包 token；用于首次 hydrate 和无需可见转场的恢复路径。 */
export function commitThemePackageUpdate(commit: () => void): void {
  runCssVariableWriteTransaction(() => {
    flushSync(commit);
  });
}

function createTransitionOverlay(
  reason: ThemePackageTransitionReason
): HTMLDivElement {
  const overlay = document.createElement('div');
  overlay.className = 'theme-package-transition';
  overlay.dataset.reason = reason;
  overlay.dataset.phase = 'cover';
  overlay.dataset.testid = 'theme-package-transition';
  overlay.setAttribute('aria-hidden', 'true');
  for (const eventName of [
    'pointerdown',
    'pointerup',
    'mousedown',
    'mouseup',
    'click',
    'dblclick',
    'contextmenu',
  ]) {
    overlay.addEventListener(eventName, swallowTransitionPointerEvent);
  }
  overlay.addEventListener('wheel', swallowTransitionPointerEvent, {
    passive: false,
  });

  const veil = document.createElement('div');
  veil.className = 'theme-package-transition__veil';
  const signal = document.createElement('div');
  signal.className = 'theme-package-transition__signal';
  overlay.append(veil, signal);
  return overlay;
}

function takeActiveTransition(
  killTimeline: boolean
): ActiveThemePackageTransition | null {
  const active = activeTransition;
  if (!active) return null;
  activeTransition = null;
  if (killTimeline) {
    active.timeline?.kill();
    gsap.killTweensOf(active.overlay);
  }
  active.overlay.remove();
  delete document.documentElement.dataset.themePackageTransition;
  return active;
}

function removeActiveTransition(killTimeline: boolean): void {
  takeActiveTransition(killTimeline)?.resolve();
}

/** 取消飞行中的主题包转场并清理其 DOM，不提交尚未到达中点的旧主题。 */
export function cancelThemePackageTransition(): void {
  removeActiveTransition(true);
}

/**
 * Reconcile an already-rendered target with the in-flight mask.
 *
 * A different or not-yet-committed transition is obsolete and must not reach
 * its midpoint later. A committed reveal for this same target remains valid and
 * should finish instead of disappearing as soon as its snapshot event arrives.
 */
export function cancelObsoleteThemePackageTransition(
  targetPackageId: string | null
): void {
  const active = activeTransition;
  if (
    active?.committed &&
    active.targetPackageId !== undefined &&
    active.targetPackageId === targetPackageId
  ) {
    return;
  }
  cancelThemePackageTransition();
}

/**
 * 运行统一的主题包切换遮罩。
 *
 * 遮罩从右侧扫入，在完全覆盖画面的中点原子提交主题，再继续向左侧退出。
 * 它只吞掉已覆盖区域的指针事件、不进入焦点顺序；新操作会立即中断并清理旧时间线。
 */
export function runThemePackageTransition(
  commit: () => void,
  options: ThemePackageTransitionOptions = {}
): Promise<void> {
  if (
    options.animate === false &&
    options.targetPackageId !== undefined &&
    !shouldSkipMotion() &&
    activeTransition?.committed &&
    activeTransition.targetPackageId === options.targetPackageId
  ) {
    // An idempotent snapshot or same-package scheme refresh may arrive while
    // this target is already revealing. Commit the fresh values immediately,
    // but let the valid reveal finish instead of snapping its mask away.
    commitThemePackageUpdate(commit);
    return Promise.resolve();
  }
  cancelThemePackageTransition();
  const reason = options.reason ?? 'activate';

  if (
    options.animate === false ||
    typeof document === 'undefined' ||
    shouldSkipMotion()
  ) {
    commitThemePackageUpdate(commit);
    return Promise.resolve();
  }

  // A package switch is a full-stage reveal, so use the normal element-in
  // rhythm and its paired shorter exit. FAST + MICRO was too fleeting to read
  // on a large window, especially for high-contrast family changes.
  const enterDuration = getMotionDuration(MOTION.BASE);
  const exitDuration = getMotionDuration(MOTION.BASE_OUT);
  if (enterDuration <= 0 && exitDuration <= 0) {
    commitThemePackageUpdate(commit);
    return Promise.resolve();
  }

  const overlay = createTransitionOverlay(reason);
  document.body.append(overlay);
  document.documentElement.dataset.themePackageTransition = 'cover';
  gsap.set(overlay, { xPercent: 100, force3D: true });

  let resolveFinished!: () => void;
  let rejectFinished!: (reason: unknown) => void;
  const finished = new Promise<void>((resolve, reject) => {
    resolveFinished = resolve;
    rejectFinished = reject;
  });
  const transition: ActiveThemePackageTransition = {
    overlay,
    timeline: null,
    committed: false,
    targetPackageId: options.targetPackageId,
    resolve: resolveFinished,
    reject: rejectFinished,
  };
  const timeline = gsap.timeline({
    onComplete: () => {
      if (activeTransition !== transition) return;
      removeActiveTransition(false);
    },
  });
  transition.timeline = timeline;
  activeTransition = transition;

  timeline
    .to(overlay, {
      xPercent: 0,
      duration: enterDuration,
      ease: 'ios-in',
    })
    .call(() => {
      if (activeTransition !== transition || transition.committed) return;
      transition.committed = true;
      overlay.dataset.phase = 'reveal';
      document.documentElement.dataset.themePackageTransition = 'reveal';
      try {
        commitThemePackageUpdate(commit);
      } catch (error) {
        takeActiveTransition(true)?.reject(error);
      }
    })
    .to(overlay, {
      xPercent: -100,
      duration: exitDuration,
      ease: 'ios-out',
    });

  return finished;
}
