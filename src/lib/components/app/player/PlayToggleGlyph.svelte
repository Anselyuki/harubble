<script lang="ts">
  import { onDestroy } from 'svelte';
  import {
    gsap,
    getMotionDuration,
    killTweens,
    MOTION,
  } from '$lib/design/gsap';
  import {
    getSettledPlayToggleGlyph,
    selectGlyphAfterCollapse,
    selectPlayToggleGlyphTransition,
    type PlayToggleGlyph,
    type PlayToggleGlyphState,
  } from '$lib/features/player/playToggleGlyph';

  // 折叠完成后再等 MICRO 一档才决定是否切到 loading，用来吞掉常见的
  // 「一帧内 pending 就被后端清掉」的抖动，避免瞬间闪一下 loading 图标。
  const LOADING_GRACE_MS = MOTION.MICRO;
  // 一旦 loading 真的显示出来，必须至少停留一次入场 + 一档缓冲的时长再收，
  // 防止用户在缓冲期二次点击时出现 loading → pause → loading 的连闪。
  const LOADING_MIN_VISIBLE_MS = MOTION.SLOW + MOTION.MICRO;

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
  let visibleGlyph: PlayToggleGlyph = 'play';
  let previousTransitionKey = 0;
  let initialized = false;
  let commandCollapsing = false;
  let animationGeneration = 0;
  let inFlightGlyph: PlayToggleGlyph | null = null;
  let loadingDelay: gsap.core.Tween | null = null;
  let loadingMinimumDelay: gsap.core.Tween | null = null;

  function motionDuration(durationMs: number): number {
    return reducedMotion ? 0 : getMotionDuration(durationMs);
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

  function snapshot(): PlayToggleGlyphState {
    return { isPlaying, isLoading, isPending };
  }

  function cancelLoadingDelay() {
    loadingDelay?.kill();
    loadingDelay = null;
  }

  function cancelLoadingMinimumDelay() {
    loadingMinimumDelay?.kill();
    loadingMinimumDelay = null;
  }

  function setVisibleImmediately(glyph: PlayToggleGlyph) {
    for (const layer of getLayers()) {
      killTweens(layer);
      gsap.set(layer, { opacity: 0, scale: 0, y: 2 });
    }
    const target = getLayer(glyph);
    if (target) gsap.set(target, { opacity: 1, scale: 1, y: 0 });
    visibleGlyph = glyph;
    inFlightGlyph = null;
  }

  function reveal(glyph: PlayToggleGlyph, generation: number) {
    if (generation !== animationGeneration) return;
    const target = getLayer(glyph);
    if (!target) return;
    visibleGlyph = glyph;
    inFlightGlyph = null;
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
  }

  function swapTo(glyph: PlayToggleGlyph) {
    const transition = selectPlayToggleGlyphTransition(
      visibleGlyph,
      inFlightGlyph,
      glyph
    );
    if (transition === 'keep') return;

    const outgoing = getLayer(visibleGlyph);
    const generation = ++animationGeneration;
    if (transition === 'restore') {
      inFlightGlyph = null;
      for (const layer of getLayers()) {
        killTweens(layer);
        if (layer !== outgoing) {
          gsap.set(layer, { opacity: 0, scale: 0, y: 2 });
        }
      }
      if (!outgoing) return;
      gsap.to(outgoing, {
        opacity: 1,
        scale: 1,
        y: 0,
        duration: motionDuration(MOTION.SLOW_OUT),
        ease: 'ios-out',
      });
      return;
    }

    inFlightGlyph = glyph;
    if (!outgoing) {
      reveal(glyph, generation);
      return;
    }
    killTweens(outgoing);
    gsap.to(outgoing, {
      opacity: 0,
      scale: 0,
      y: -1,
      duration: motionDuration(MOTION.SLOW_OUT),
      ease: 'ios-in',
      onComplete: () => reveal(glyph, generation),
    });
  }

  function holdLoadingBeforeSettling(
    outgoingGlyph: PlayToggleGlyph,
    generation: number
  ) {
    loadingMinimumDelay = gsap.delayedCall(
      motionDuration(LOADING_MIN_VISIBLE_MS),
      () => {
        loadingMinimumDelay = null;
        if (generation !== animationGeneration) return;
        commandCollapsing = false;
        const nextGlyph = selectGlyphAfterCollapse(outgoingGlyph, snapshot());
        if (nextGlyph !== 'loading') swapTo(nextGlyph);
      }
    );
  }

  function beginCommandTransition() {
    const outgoingGlyph = visibleGlyph;
    const outgoing = getLayer(outgoingGlyph);
    const generation = ++animationGeneration;
    cancelLoadingDelay();
    cancelLoadingMinimumDelay();
    commandCollapsing = true;
    inFlightGlyph = null;

    const finishCollapse = () => {
      if (generation !== animationGeneration) return;
      const nextGlyph = selectGlyphAfterCollapse(outgoingGlyph, snapshot());
      if (nextGlyph === 'loading' && !reducedMotion) {
        loadingDelay = gsap.delayedCall(
          motionDuration(LOADING_GRACE_MS),
          () => {
            loadingDelay = null;
            if (generation !== animationGeneration) return;
            const delayedGlyph = selectGlyphAfterCollapse(
              outgoingGlyph,
              snapshot()
            );
            reveal(delayedGlyph, generation);
            if (delayedGlyph === 'loading') {
              holdLoadingBeforeSettling(outgoingGlyph, generation);
            } else {
              commandCollapsing = false;
            }
          }
        );
        return;
      }
      commandCollapsing = false;
      reveal(nextGlyph, generation);
    };

    if (!outgoing) {
      finishCollapse();
      return;
    }

    killTweens(outgoing);
    gsap.to(outgoing, {
      opacity: 0,
      scale: 0,
      y: -1,
      duration: motionDuration(MOTION.SLOW_OUT),
      ease: 'ios-in',
      onComplete: finishCollapse,
    });
  }

  $effect(() => {
    const currentTransitionKey = transitionKey;
    const currentState = snapshot();
    if (!playLayer || !pauseLayer || !loadingLayer) return;

    if (!initialized) {
      initialized = true;
      previousTransitionKey = currentTransitionKey;
      setVisibleImmediately(
        currentState.isLoading || currentState.isPending
          ? 'loading'
          : getSettledPlayToggleGlyph(currentState.isPlaying)
      );
      return;
    }

    if (currentTransitionKey !== previousTransitionKey) {
      previousTransitionKey = currentTransitionKey;
      beginCommandTransition();
      return;
    }

    if (commandCollapsing) return;
    swapTo(
      currentState.isLoading || currentState.isPending
        ? 'loading'
        : getSettledPlayToggleGlyph(currentState.isPlaying)
    );
  });

  onDestroy(() => {
    animationGeneration += 1;
    inFlightGlyph = null;
    cancelLoadingDelay();
    cancelLoadingMinimumDelay();
    for (const layer of getLayers()) killTweens(layer);
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
