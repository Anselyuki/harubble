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
    onClose?: () => void;
  }

  let { volume, muted, rightControlsRef, onVolumeChange, onClose }: Props =
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
    scheduleCollapse();
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

  export function expand() {
    if (capsuleState === 'open' || capsuleState === 'expanding') return;
    if (!capsuleRef || !rightControlsRef) return;

    const targetWidth = rightControlsRef.offsetWidth;
    const duration = getMotionDuration(400);
    const fadeDuration = getMotionDuration(150);

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
    gsap.to(capsuleRef, {
      opacity: 1,
      duration: fadeDuration,
      ease: 'ios-out',
    });
  }

  function collapse() {
    if (capsuleState === 'closed' || capsuleState === 'collapsing') return;
    if (!capsuleRef) return;

    const fadeDuration = getMotionDuration(100);
    const shrinkDuration = getMotionDuration(300);

    killTweens(capsuleRef);
    capsuleState = 'collapsing';

    const tl = gsap.timeline({
      onComplete: () => {
        capsuleState = 'closed';
        onClose?.();
      },
    });

    tl.to(capsuleRef, {
      opacity: 0,
      duration: fadeDuration,
      ease: 'ios-in',
    });
    tl.to(capsuleRef, {
      width: 0,
      duration: shrinkDuration,
      ease: 'ios',
    });
  }

  export function toggle() {
    if (capsuleState === 'closed' || capsuleState === 'collapsing') {
      expand();
    } else {
      collapse();
    }
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
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
  onkeydown={handleKeydown}
>
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
  />
</div>

<style>
  .volume-capsule {
    position: absolute;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    height: var(--control-button-size, 34px);
    width: 0;
    opacity: 0;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 calc(var(--control-button-size, 34px) + 4px) 0 12px;
    border-radius: 999px;
    background: var(--player-shell-bg, var(--surface));
    border: 1px solid var(--surface-border);
    z-index: 10;
    white-space: nowrap;
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
