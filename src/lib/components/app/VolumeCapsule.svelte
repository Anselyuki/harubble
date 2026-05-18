<script lang="ts">
  import { onDestroy } from 'svelte';
  import { gsap, killTweens, getMotionDuration } from '$lib/design/gsap';
  import { sliderToGain, gainToSlider } from '$lib/features/player/volume';
  import * as m from '$lib/paraglide/messages.js';

  const COLLAPSE_DELAY_MS = 250;

  type CapsuleState = 'closed' | 'expanding' | 'open' | 'collapsing';

  interface Props {
    volume: number;
    muted: boolean;
    rightControlsRef: HTMLElement | null;
    onVolumeChange?: (gain: number) => void | Promise<void>;
    onToggleMute?: () => void;
  }

  let { volume, muted, rightControlsRef, onVolumeChange, onToggleMute }: Props =
    $props();

  let capsuleState = $state<CapsuleState>('closed');
  let capsuleRef = $state<HTMLElement | null>(null);
  let sliderRef = $state<HTMLInputElement | null>(null);
  let collapseTimeout: ReturnType<typeof setTimeout> | null = null;
  let isDragging = $state(false);

  const sliderPos = $derived(gainToSlider(volume));
  let sliderPreview = $state<number | null>(null);
  const shownPos = $derived(sliderPreview ?? sliderPos);
  const displayPercent = $derived(muted ? 0 : Math.round(shownPos * 100));

  const volumeIcon = $derived.by(() => {
    if (muted || volume === 0) return 'muted' as const;
    if (gainToSlider(volume) < 0.5) return 'low' as const;
    return 'high' as const;
  });

  const isOpen = $derived(
    capsuleState === 'open' || capsuleState === 'expanding'
  );

  function handleSliderInput(event: Event) {
    sliderPreview = Number((event.currentTarget as HTMLInputElement).value);
  }

  function handleSliderCommit(event: Event) {
    const pos = Number((event.currentTarget as HTMLInputElement).value);
    sliderPreview = null;
    void onVolumeChange?.(sliderToGain(pos));
  }

  function handleSliderDown() {
    isDragging = true;
  }

  function handleSliderUp() {
    isDragging = false;
  }

  function handleMouseEnter() {
    if (collapseTimeout) {
      clearTimeout(collapseTimeout);
      collapseTimeout = null;
    }
    if (capsuleState === 'collapsing') {
      expand();
    }
  }

  function handleMouseLeave() {
    if (isDragging) return;
    if (capsuleState === 'open' || capsuleState === 'expanding') {
      scheduleCollapse();
    }
  }

  export function scheduleCollapse() {
    if (collapseTimeout) clearTimeout(collapseTimeout);
    collapseTimeout = setTimeout(() => {
      collapseTimeout = null;
      collapse();
    }, COLLAPSE_DELAY_MS);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      collapse();
    }
  }

  function handleIconClick() {
    if (capsuleState === 'closed' || capsuleState === 'collapsing') {
      expand();
    } else {
      collapse();
    }
  }

  export function expand() {
    if (capsuleState === 'open' || capsuleState === 'expanding') return;
    if (!capsuleRef || !rightControlsRef) return;

    const targetWidth = rightControlsRef.offsetWidth;
    const duration = getMotionDuration(400);

    killTweens(capsuleRef);
    capsuleState = 'expanding';

    gsap.to(capsuleRef, {
      width: targetWidth,
      duration,
      ease: 'ios-spring',
      onComplete: () => {
        capsuleState = 'open';
        sliderRef?.focus();
      },
    });
  }

  function collapse() {
    if (capsuleState === 'closed' || capsuleState === 'collapsing') return;
    if (!capsuleRef) return;

    const shrinkDuration = getMotionDuration(799);

    killTweens(capsuleRef);
    capsuleState = 'collapsing';

    gsap.to(capsuleRef, {
      width:
        (capsuleRef.querySelector('.capsule-icon-btn') as HTMLElement)
          ?.offsetWidth ?? 34,
      duration: shrinkDuration,
      ease: 'ios',
      onComplete: () => {
        capsuleState = 'closed';
      },
    });
  }

  export function toggle() {
    handleIconClick();
  }

  $effect(() => {
    if (!isDragging) return;
    function handleGlobalPointerUp() {
      isDragging = false;
      if (capsuleRef && !capsuleRef.matches(':hover')) {
        scheduleCollapse();
      }
    }
    document.addEventListener('pointerup', handleGlobalPointerUp);
    return () =>
      document.removeEventListener('pointerup', handleGlobalPointerUp);
  });

  onDestroy(() => {
    if (collapseTimeout) clearTimeout(collapseTimeout);
    if (capsuleRef) killTweens(capsuleRef);
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="volume-capsule"
  bind:this={capsuleRef}
  role="group"
  aria-label={m.player_aria_volume()}
  data-state={capsuleState}
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
  onkeydown={handleKeydown}
>
  <div class="capsule-body" class:capsule-body--hidden={!isOpen}>
    <span class="capsule-value">{displayPercent}%</span>
    <input
      class="capsule-slider"
      type="range"
      min="0"
      max="1"
      step="0.01"
      value={shownPos}
      aria-label={m.player_aria_volume_slider()}
      aria-valuetext={`${displayPercent}%`}
      bind:this={sliderRef}
      oninput={handleSliderInput}
      onchange={handleSliderCommit}
      onpointerdown={handleSliderDown}
      onpointerup={handleSliderUp}
      style="--volume-percent:{shownPos * 100}%"
      class:muted-slider={muted}
      tabindex={isOpen ? 0 : -1}
    />
  </div>
  <button
    type="button"
    class="capsule-icon-btn"
    aria-label={muted ? m.player_aria_unmute() : m.player_aria_mute()}
    aria-expanded={isOpen}
    onclick={handleIconClick}
    ondblclick={() => onToggleMute?.()}
  >
    <svg class="capsule-icon" viewBox="0 0 24 24" aria-hidden="true">
      {#if volumeIcon === 'muted'}
        <path d="M11 5 6 9H2v6h4l5 4V5z"></path>
        <line x1="23" y1="9" x2="17" y2="15"></line>
        <line x1="17" y1="9" x2="23" y2="15"></line>
      {:else if volumeIcon === 'low'}
        <path d="M11 5 6 9H2v6h4l5 4V5z"></path>
        <path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path>
      {:else}
        <path d="M11 5 6 9H2v6h4l5 4V5z"></path>
        <path d="M15.54 8.46a5 5 0 0 1 0 7.07"></path>
        <path d="M19.07 4.93a10 10 0 0 1 0 14.14"></path>
      {/if}
    </svg>
  </button>
</div>

<style>
  .volume-capsule {
    position: relative;
    height: var(--control-button-size, 34px);
    width: var(--control-button-size, 34px);
    display: flex;
    align-items: center;
    border-radius: 999px;
    overflow: hidden;
    flex-shrink: 0;
    background: transparent;
    transition: background-color 150ms ease;
  }

  .volume-capsule[data-state='expanding'],
  .volume-capsule[data-state='open'] {
    background: #ffffff;
  }

  @media (prefers-color-scheme: dark) {
    .volume-capsule[data-state='expanding'],
    .volume-capsule[data-state='open'] {
      background: #1c1c1e;
    }
  }

  .capsule-body {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-left: 12px;
    padding-right: 4px;
    overflow: hidden;
    opacity: 1;
    transition: opacity 100ms ease;
  }

  .capsule-body--hidden {
    opacity: 0;
    pointer-events: none;
  }

  .capsule-icon-btn {
    appearance: none;
    border: 0;
    background: transparent;
    width: var(--control-button-size, 34px);
    height: var(--control-button-size, 34px);
    border-radius: 50%;
    color: var(--icon-default);
    cursor: pointer;
    display: grid;
    place-items: center;
    flex-shrink: 0;
    transition:
      color 150ms ease,
      background-color 150ms ease;
  }

  .capsule-icon-btn:hover {
    color: var(--icon-active);
    background: var(--player-control-hover-bg, rgba(var(--accent-rgb), 0.1));
  }

  .capsule-icon {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.85;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .capsule-value {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-subtle);
    width: 3.5ch;
    text-align: right;
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }

  .capsule-slider {
    -webkit-appearance: none;
    appearance: none;
    flex: 1;
    min-width: 0;
    height: 4px;
    border-radius: 2px;
    background: linear-gradient(
      to right,
      var(--icon-active) 0%,
      var(--icon-active) var(--volume-percent),
      var(--surface-highlight, rgba(120, 120, 128, 0.28)) var(--volume-percent),
      var(--surface-highlight, rgba(120, 120, 128, 0.28)) 100%
    );
    outline: none;
    cursor: pointer;
    transition: opacity 150ms ease;
  }

  .capsule-slider.muted-slider {
    opacity: 0.4;
  }

  .capsule-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--thumb-bg);
    border: 1.5px solid var(--thumb-border);
    box-shadow: 0 1px 3px var(--thumb-shadow);
    cursor: pointer;
  }

  .capsule-slider::-moz-range-thumb {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--thumb-bg);
    border: 1.5px solid var(--thumb-border);
    box-shadow: 0 1px 3px var(--thumb-shadow);
    cursor: pointer;
  }
</style>
