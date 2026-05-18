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
    onClose?: () => void;
  }

  let {
    volume,
    muted,
    rightControlsRef,
    onVolumeChange,
    onToggleMute,
    onClose,
  }: Props = $props();

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
  <button
    type="button"
    class="capsule-mute"
    aria-label={muted ? m.player_aria_unmute() : m.player_aria_mute()}
    onclick={() => onToggleMute?.()}
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
