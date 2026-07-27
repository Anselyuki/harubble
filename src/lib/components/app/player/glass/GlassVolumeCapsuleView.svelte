<script lang="ts">
  /**
   * Glass family 音量胶囊 view（Phase 3 Step 3.2）。
   *
   * 视觉：iOS 液态玻璃 pill + WaveGlassPanel 波纹 + 展开圆角形变。
   * 与原 VolumeCapsule.svelte 视觉零差异；业务状态机 / 定时器 / 事件处理
   * 通过共享的 state.ts / timer.ts helper 承担。
   */
  import { onDestroy } from 'svelte';
  import { sliderToGain, gainToSlider } from '$lib/features/player/volume';
  import { createGlassCapsuleAnimator } from '../volume-capsule-animator';
  import type { CapsuleAnimatorRefs } from '../volume-capsule-animator';
  import { createVolumeCapsuleController } from '$lib/features/player/controllers/volumeCapsuleController.svelte';
  import WaveGlassPanel from './WaveGlassPanel.svelte';
  import * as m from '$lib/paraglide/messages.js';

  interface Props {
    volume: number;
    muted: boolean;
    open: boolean;
    onopen?: () => void;
    onclose?: () => void;
    onVolumeChange?: (gain: number) => void | Promise<void>;
    onToggleMute?: () => void;
  }

  let {
    volume,
    muted,
    open,
    onopen,
    onclose,
    onVolumeChange,
    onToggleMute,
  }: Props = $props();

  let trackRef = $state<HTMLElement | null>(null);
  let sliderRef = $state<HTMLInputElement | null>(null);
  let wrapperRef = $state<HTMLElement | null>(null);
  let badgeRef = $state<HTMLElement | null>(null);
  let iconBtnRef = $state<HTMLButtonElement | null>(null);

  const animator = createGlassCapsuleAnimator(
    (): CapsuleAnimatorRefs => ({
      track: trackRef,
      badge: badgeRef,
      iconBtn: iconBtnRef,
    })
  );

  const controller = createVolumeCapsuleController({
    animator,
    getOpen: () => open,
    getWrapperEl: () => wrapperRef,
    focusSlider: () => sliderRef?.focus(),
    onopen: () => onopen?.(),
    onclose: () => onclose?.(),
    onVolumeChange: (pos) => onVolumeChange?.(sliderToGain(pos)),
  });

  const sliderPos = $derived(gainToSlider(volume));
  const shownPos = $derived(controller.sliderPreview ?? sliderPos);
  const displayPercent = $derived(muted ? 0 : Math.round(shownPos * 100));
  const isEasterEgg = $derived(displayPercent === 32);
  const displayText = $derived(isEasterEgg ? '32.5' : String(displayPercent));

  const volumeIcon = $derived.by(() => {
    if (muted || volume === 0) return 'muted' as const;
    if (gainToSlider(volume) < 0.5) return 'low' as const;
    return 'high' as const;
  });

  $effect(() => {
    controller.syncOpen();
  });

  $effect(() => {
    if (!controller.isDragging) return;
    return controller.installGlobalPointerListeners();
  });

  function handleSliderInput(event: Event) {
    const pos = Number((event.currentTarget as HTMLInputElement).value);
    controller.handleSliderInput(pos);
  }

  onDestroy(() => controller.destroy());
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="volume-hover-zone"
  onmouseenter={controller.handleMouseEnter}
  onmouseleave={controller.handleMouseLeave}
>
  <WaveGlassPanel open={open && controller.isDragging} />

  <div
    class="volume-wrapper"
    bind:this={wrapperRef}
    role="group"
    aria-label={m.player_aria_volume()}
    data-state={open ? 'open' : 'closed'}
    onfocusout={controller.handleFocusOut}
  >
    <span class="volume-badge-clip">
      <span
        class="volume-badge"
        class:easter-egg={isEasterEgg}
        bind:this={badgeRef}
        aria-hidden="true"
        >{displayText}<span class="volume-badge-unit">%</span></span
      >
    </span>
    <div class="capsule-track" bind:this={trackRef}>
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
        onchange={controller.handleSliderCommit}
        onpointerdown={controller.handleSliderDown}
        onpointerup={controller.handleSliderUp}
        style="--volume-percent:{shownPos * 100}%"
        class:muted-slider={muted}
        tabindex={open ? 0 : -1}
      />
    </div>
    <button
      type="button"
      class="capsule-icon-btn"
      bind:this={iconBtnRef}
      aria-label={muted ? m.player_aria_unmute() : m.player_aria_mute()}
      aria-expanded={open}
      onclick={controller.handleIconClick}
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
</div>

<style>
  .volume-hover-zone {
    --capsule-track-bg: color-mix(
      in srgb,
      var(--surface, var(--player-shell-bg)) 96%,
      transparent
    );
    --volume-shell-surface: color-mix(
      in srgb,
      var(--surface, var(--player-shell-bg)) 28%,
      transparent
    );
    --volume-shell-border: var(--surface-border, var(--player-shell-border));
    --volume-shell-shadow:
      0 4px 16px color-mix(in srgb, var(--surface-border) 62%, transparent),
      0 1px 4px color-mix(in srgb, var(--surface-border) 40%, transparent);
    --volume-shell-highlight: color-mix(
      in srgb,
      var(--surface-highlight) 22%,
      transparent
    );
    --volume-shell-highlight-soft: color-mix(
      in srgb,
      var(--surface-highlight) 6%,
      transparent
    );
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    pointer-events: auto;
  }

  .volume-wrapper {
    position: absolute;
    right: 0;
    top: 0;
    bottom: 0;
    margin-top: auto;
    margin-bottom: auto;
    height: var(--control-button-size, 34px);
    display: flex;
    align-items: center;
    z-index: 20;
  }

  .capsule-track {
    position: relative;
    display: flex;
    align-items: center;
    height: var(--control-button-size, 34px);
    width: 0;
    opacity: 0;
    overflow-x: clip;
    padding-left: 12px;
    padding-right: calc(var(--control-button-size, 34px) + 4px);
    border-radius: var(--shape-pill);
    white-space: nowrap;
    box-shadow: var(--volume-shell-shadow);
  }

  .capsule-icon-btn {
    position: absolute;
    right: 0;
    z-index: 2;
    appearance: none;
    border: 0;
    background: transparent;
    width: var(--control-button-size, 34px);
    height: var(--control-button-size, 34px);
    border-radius: var(--shape-circle);
    color: var(--icon-default);
    cursor: pointer;
    display: grid;
    place-items: center;
    flex-shrink: 0;
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

  .volume-badge-clip {
    position: absolute;
    left: 0;
    bottom: 100%;
    margin-bottom: -4px;
    overflow: hidden;
    height: 52px;
    padding-right: 4px;
    pointer-events: none;
    z-index: 1;
  }

  .volume-badge {
    display: flex;
    align-items: baseline;
    margin-left: 6px;
    padding: 4px 4px 4px 0;
    font-family: var(--font-brand);
    font-size: 36px;
    letter-spacing: 1px;
    color: var(--album-accent-readable-foreground);
    -webkit-text-stroke: 4px var(--icon-active);
    paint-order: stroke fill;
    -webkit-text-fill-color: var(--album-accent-readable-foreground);
    pointer-events: none;
    transform: translateY(100%);
  }

  .volume-badge-unit {
    font-size: 18px;
    margin-left: 5px;
  }

  .capsule-slider {
    -webkit-appearance: none;
    appearance: none;
    flex: 1;
    min-width: 0;
    height: 10px;
    border-radius: 5px;
    background: linear-gradient(
      to right,
      var(--icon-active) 0%,
      var(--icon-active) var(--volume-percent),
      var(--capsule-track-bg) var(--volume-percent),
      var(--capsule-track-bg) 100%
    );
    outline: none;
    cursor: pointer;
  }

  .capsule-slider.muted-slider {
    opacity: 0.4;
  }

  .capsule-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 0;
    height: 0;
  }

  .capsule-slider::-moz-range-thumb {
    width: 0;
    height: 0;
    border: none;
    background: transparent;
  }
</style>
