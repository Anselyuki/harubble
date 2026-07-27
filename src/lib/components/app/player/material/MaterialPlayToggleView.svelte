<script lang="ts">
  /**
   * Material family 的 play toggle view（Phase 3 Step 3.2）。
   *
   * 视觉：3 层 glyph 叠放，切换时短程 fade + scale 0.9→1（Material 3 emphasized easing）。
   * 与 glass 差异：无位移、缓动更平滑、圆形 ripple 底色使用 --shape-pill。
   *
   * 业务逻辑通过共享的 [`createPlayToggleController`] 与 glass view 完全一致，
   * 保证 loading grace 与 min visible timing 不受视觉切换影响。
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
            { opacity: 0, scale: 0.9 },
            {
              opacity: 1,
              scale: 1,
              duration: motionDuration(MOTION.FAST),
              ease: 'ios-out',
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
            scale: 0.9,
            duration: motionDuration(MOTION.FAST),
            ease: 'ios-in',
            onComplete: onDone,
          });
        },
        setImmediately(glyph) {
          for (const layer of getLayers()) {
            killTweens(layer);
            gsap.set(layer, { opacity: 0, scale: 0.9 });
          }
          const target = getLayer(glyph);
          if (target) gsap.set(target, { opacity: 1, scale: 1 });
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

<span
  class="play-toggle-glyph play-toggle-glyph--material"
  style:--play-toggle-glyph-size={size}
>
  <span class="glyph-layer" bind:this={playLayer}>
    <svg class="glyph-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M8 5v14l11-7z"></path>
    </svg>
  </span>
  <span class="glyph-layer" bind:this={pauseLayer}>
    <svg class="glyph-icon" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="6" y="5" width="4" height="14" rx="1"></rect>
      <rect x="14" y="5" width="4" height="14" rx="1"></rect>
    </svg>
  </span>
  <span class="glyph-layer" bind:this={loadingLayer}>
    <svg class="glyph-icon loading-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 4a8 8 0 0 1 8 8"></path>
    </svg>
  </span>
</span>

<style>
  .play-toggle-glyph--material {
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
    transform: scale(0.9);
    transform-origin: center;
    will-change: transform, opacity;
  }

  .glyph-icon {
    width: 100%;
    height: 100%;
    fill: currentColor;
    stroke: none;
  }

  .loading-icon {
    fill: none;
    stroke: currentColor;
    stroke-width: 2.4;
    stroke-linecap: round;
    animation: motion-spin var(--motion-spinner) linear infinite;
  }

  @media (prefers-reduced-motion: reduce) {
    .loading-icon {
      animation: none;
    }
  }
</style>
