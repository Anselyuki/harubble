<script lang="ts">
  /**
   * Glass family 的 play toggle view（Phase 3 Step 3.2）。
   *
   * 视觉：3 层 glyph（play / pause / loading）叠放，切换时 iOS spring 弹入 / ios-in 收出。
   * 保留原 `PlayToggleGlyph.svelte` 的全部视觉行为，行为零变化。
   *
   * 业务逻辑（LOADING_GRACE_MS / MIN_VISIBLE / 世代号）由外部注入的 controller 承担；
   * 本组件只暴露 GlyphAnimator 适配器（tween 参数、DOM refs）给它。
   */
  import { onDestroy } from 'svelte';
  import {
    gsap,
    getMotionDuration,
    killTweens,
    MOTION,
    shouldSkipMotion,
  } from '$lib/design/gsap';
  import {
    createPlayToggleController,
    type PlayToggleController,
  } from '$lib/features/player/controllers/playToggleController.svelte';
  import type { PlayToggleGlyph } from '$lib/features/player/playToggleGlyph';

  interface Props {
    isPlaying: boolean;
    isLoading?: boolean;
    isPending?: boolean;
    transitionKey?: number;
    reducedMotion?: boolean;
    size?: string;
  }

  let {
    isPlaying,
    isLoading = false,
    isPending = false,
    transitionKey = 0,
    reducedMotion = false,
    size = '24px',
  }: Props = $props();

  let playLayer = $state<HTMLSpanElement | null>(null);
  let pauseLayer = $state<HTMLSpanElement | null>(null);
  let loadingLayer = $state<HTMLSpanElement | null>(null);
  let controller: PlayToggleController | null = null;

  function motionDuration(durationMs: number): number {
    return reducedMotion || shouldSkipMotion()
      ? 0
      : getMotionDuration(durationMs);
  }

  function getLayer(glyph: PlayToggleGlyph): HTMLSpanElement | null {
    switch (glyph) {
      case 'pause':
        return pauseLayer;
      case 'loading':
        return loadingLayer;
      default:
        return playLayer;
    }
  }

  function getLayers(): HTMLSpanElement[] {
    return [playLayer, pauseLayer, loadingLayer].filter(
      (layer): layer is HTMLSpanElement => layer !== null
    );
  }

  $effect(() => {
    if (!playLayer || !pauseLayer || !loadingLayer) return;
    if (!controller) {
      controller = createPlayToggleController({
        animateIn(glyph) {
          const target = getLayer(glyph);
          if (!target) return;
          killTweens(target);
          gsap.fromTo(
            target,
            { opacity: 0, scale: 0, y: 2 },
            {
              opacity: 1,
              scale: 1,
              y: 0,
              duration: motionDuration(MOTION.SLOW),
              ease: 'ios-spring',
            }
          );
        },
        animateOut(glyph, onDone) {
          const target = getLayer(glyph);
          if (!target) {
            onDone();
            return;
          }
          killTweens(target);
          gsap.to(target, {
            opacity: 0,
            scale: 0,
            y: -1,
            duration: motionDuration(MOTION.SLOW_OUT),
            ease: 'ios-in',
            onComplete: onDone,
          });
        },
        setImmediately(glyph) {
          for (const layer of getLayers()) {
            killTweens(layer);
            gsap.set(layer, { opacity: 0, scale: 0, y: 2 });
          }
          const target = getLayer(glyph);
          if (target) gsap.set(target, { opacity: 1, scale: 1, y: 0 });
        },
        kill() {
          for (const layer of getLayers()) killTweens(layer);
        },
      });
    }
    controller.notify({ isPlaying, isLoading, isPending, transitionKey });
  });

  onDestroy(() => {
    controller?.destroy();
    controller = null;
  });
</script>

<span class="play-toggle-glyph" style:--play-toggle-glyph-size={size}>
  <span class="glyph-layer" bind:this={playLayer}>
    <svg class="glyph-icon play-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M8.2 6.3v11.4L17.35 12z"></path>
    </svg>
  </span>
  <span class="glyph-layer" bind:this={pauseLayer}>
    <svg class="glyph-icon" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="7.15" y="5.95" width="3.4" height="12.1" rx="1.25"></rect>
      <rect x="13.45" y="5.95" width="3.4" height="12.1" rx="1.25"></rect>
    </svg>
  </span>
  <span class="glyph-layer" bind:this={loadingLayer}>
    <svg class="glyph-icon loading-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 5a7 7 0 1 1-6.3 4"></path>
    </svg>
  </span>
</span>

<style>
  .play-toggle-glyph {
    position: relative;
    width: var(--play-toggle-glyph-size);
    height: var(--play-toggle-glyph-size);
    display: grid;
    place-items: center;
    flex: 0 0 auto;
  }

  .glyph-layer {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    opacity: 0;
    transform: translateY(2px) scale(0);
    transform-origin: center;
    will-change: transform, opacity;
  }

  .glyph-icon {
    width: 100%;
    height: 100%;
    fill: currentColor;
    stroke: none;
  }

  .play-icon {
    transform: translateX(0.5px);
  }

  .loading-icon {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.2;
    stroke-linecap: round;
    animation: motion-spin var(--motion-spinner) linear infinite;
  }

  @media (prefers-reduced-motion: reduce) {
    .loading-icon {
      animation: none;
    }
  }
</style>
