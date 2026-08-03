<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { sliderToGain, gainToSlider } from '$lib/features/player/volume';
  import { createVolumeCapsuleController } from '$lib/features/player/controllers/volumeCapsuleController.svelte';
  import {
    createArkCapsuleAnimator,
    type CapsuleAnimatorRefs,
  } from '../volume-capsule-animator';
  import type { ArkUiThemeFamily } from '$lib/features/shell/visualContract.svelte';
  import * as m from '$lib/paraglide/messages.js';

  interface Props {
    family: ArkUiThemeFamily;
    volume: number;
    muted: boolean;
    open: boolean;
    onopen?: () => void;
    onclose?: () => void;
    onVolumeChange?: (gain: number) => void | Promise<void>;
    onToggleMute?: () => void;
  }

  let {
    family,
    volume,
    muted,
    open,
    onopen,
    onclose,
    onVolumeChange,
    onToggleMute,
  }: Props = $props();

  let sliderRef = $state<HTMLInputElement | null>(null);
  let wrapperRef = $state<HTMLElement | null>(null);
  let trackRef = $state<HTMLElement | null>(null);
  let iconBtnRef = $state<HTMLButtonElement | null>(null);
  let persistent = $state(false);

  const animator = createArkCapsuleAnimator(
    (): CapsuleAnimatorRefs => ({
      track: trackRef,
      badge: null,
      iconBtn: iconBtnRef,
    })
  );

  const controller = createVolumeCapsuleController({
    animator,
    getOpen: () => open || persistent,
    getWrapperEl: () => wrapperRef,
    focusSlider: () => sliderRef?.focus(),
    onopen: () => onopen?.(),
    onclose: () => onclose?.(),
    onVolumeChange: (position) => onVolumeChange?.(sliderToGain(position)),
  });

  const sliderPosition = $derived(gainToSlider(volume));
  const shownPosition = $derived(controller.sliderPreview ?? sliderPosition);
  const displayPercent = $derived(Math.round(shownPosition * 100));
  const volumeIcon = $derived.by(() => {
    if (muted || volume === 0) return 'muted' as const;
    if (gainToSlider(volume) < 0.5) return 'low' as const;
    return 'high' as const;
  });
  $effect(() => {
    if (!trackRef) return;
    controller.syncOpen();
  });

  $effect(() => {
    if (!controller.isDragging) return;
    return controller.installGlobalPointerListeners();
  });

  function handleSliderInput(event: Event) {
    const position = Number((event.currentTarget as HTMLInputElement).value);
    controller.handleSliderInput(position);
  }

  function handleMuteClick() {
    onToggleMute?.();
  }

  onMount(() => {
    const media = window.matchMedia('(hover: none) and (pointer: coarse)');
    const syncPersistent = () => {
      persistent = media.matches;
    };
    syncPersistent();
    media.addEventListener('change', syncPersistent);
    return () => media.removeEventListener('change', syncPersistent);
  });

  onDestroy(() => controller.destroy());
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="volume-hover-zone volume-hover-zone--ark-ui volume-hover-zone--{family}"
  class:dragging={controller.isDragging}
  data-volume-family={family}
  onmouseenter={controller.handleMouseEnter}
  onmouseleave={controller.handleMouseLeave}
>
  <div
    class="volume-wrapper"
    bind:this={wrapperRef}
    data-state={open || persistent ? 'open' : 'closed'}
    onfocusin={controller.handleFocusIn}
    onfocusout={controller.handleFocusOut}
  >
    <button
      type="button"
      class="capsule-icon-btn"
      bind:this={iconBtnRef}
      aria-label={m.player_aria_mute()}
      aria-pressed={muted}
      onclick={handleMuteClick}
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

    <div class="capsule-reveal" bind:this={trackRef}>
      <div class="capsule-track">
        <span class="volume-family-marker" aria-hidden="true"></span>
        <output
          class="volume-readout"
          class:muted-readout={muted}
          aria-hidden="true"
          >{displayPercent}<span class="volume-readout-unit">%</span></output
        >
        <input
          class="capsule-slider"
          class:muted-slider={muted}
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={shownPosition}
          aria-label={m.player_aria_volume_slider()}
          aria-valuetext={`${displayPercent}%`}
          bind:this={sliderRef}
          oninput={handleSliderInput}
          onchange={controller.handleSliderCommit}
          onpointerdown={controller.handleSliderDown}
          onpointerup={controller.handleSliderUp}
          onpointercancel={controller.handleSliderCancel}
          onlostpointercapture={controller.handleSliderCancel}
          style="--volume-percent:{shownPosition * 100}%"
          tabindex={open || persistent ? 0 : -1}
        />
      </div>
    </div>
  </div>
</div>

<style>
  .volume-hover-zone {
    --volume-family-signal: var(
      --theme-accent,
      var(--accent, var(--icon-active))
    );
    --volume-family-signal-alt: var(
      --theme-custom-signal-alt,
      var(--volume-family-signal)
    );
    --volume-control-size: max(40px, var(--control-button-size, 34px));
    --volume-control-overhang: max(
      0px,
      calc(var(--volume-control-size) - var(--control-button-size, 34px))
    );
    --volume-context-signal: var(
      --icon-active,
      var(--album-accent, var(--volume-family-signal))
    );
    --volume-track-background: var(--bg-elevated);
    --volume-track-border: var(--border);
    --volume-track-shadow: var(--elevation-sm);
    --volume-track-radius: var(--shape-sm);
    --volume-track-padding-left: 24px;
    --volume-track-gap: 8px;
    --volume-rail-rest: var(--bg-tertiary);
    --volume-rail-size: 4px;
    --volume-thumb-width: 10px;
    --volume-thumb-height: 14px;
    --volume-thumb-radius: var(--shape-xs);
    --volume-thumb-background: var(--volume-context-signal);
    --volume-thumb-border: 0;
    --volume-thumb-shadow: none;
    --volume-readout-color: var(--text-secondary);
    --volume-readout-font: var(--font-display);
    --volume-readout-size: 11px;
    --volume-readout-width: 34px;
    position: absolute;
    inset-block: 0;
    left: 0;
    right: calc(0px - var(--volume-control-overhang));
    pointer-events: auto;
  }

  .volume-wrapper {
    position: absolute;
    inset-block: 0;
    left: 0;
    right: 0;
    height: var(--volume-control-size);
    margin-block: auto;
    display: flex;
    align-items: center;
    z-index: 20;
  }

  .capsule-reveal {
    position: relative;
    width: 0;
    max-width: min(
      calc(100% - var(--volume-control-overhang)),
      calc(100vw - 56px)
    );
    height: var(--volume-control-size);
    margin-left: auto;
    overflow: clip;
    visibility: hidden;
    opacity: 0;
    pointer-events: none;
    will-change: width, opacity;
  }

  .capsule-track {
    position: absolute;
    inset-block: 0;
    right: 0;
    isolation: isolate;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--volume-track-gap);
    width: 200px;
    height: var(--volume-control-size);
    padding-left: var(--volume-track-padding-left);
    padding-right: calc(var(--volume-control-size) + 8px);
    white-space: nowrap;
    border: 1px solid var(--volume-track-border);
    border-radius: var(--volume-track-radius);
    background: var(--volume-track-background);
    box-shadow: var(--volume-track-shadow);
  }

  .volume-family-marker {
    position: absolute;
    left: 7px;
    top: 50%;
    pointer-events: none;
    opacity: 1;
  }

  .capsule-icon-btn {
    position: absolute;
    right: 0;
    z-index: 2;
    appearance: none;
    width: var(--volume-control-size);
    height: var(--volume-control-size);
    padding: 0;
    border: 0;
    border-radius: var(--volume-track-radius);
    background: transparent;
    color: var(--icon-default);
    cursor: pointer;
    display: grid;
    grid-template-columns: var(--control-button-size, 34px) minmax(0, 1fr);
    align-items: center;
    justify-items: center;
    transition: color 120ms cubic-bezier(0.2, 0, 0, 1);
  }

  .capsule-icon-btn:hover,
  .capsule-icon-btn[aria-pressed='true'],
  .volume-wrapper[data-state='open'] .capsule-icon-btn {
    color: var(--volume-family-signal);
  }

  .capsule-icon-btn:focus-visible {
    outline: 2px solid var(--volume-family-signal);
    outline-offset: 2px;
  }

  :global(:root[data-ark-theme]) .capsule-slider:focus-visible {
    outline: none;
  }

  .capsule-reveal:has(.capsule-slider:focus) {
    width: 200px !important;
    visibility: visible !important;
    opacity: 1 !important;
    pointer-events: auto !important;
  }

  .capsule-track:has(.capsule-slider:focus-visible)::after {
    content: '';
    position: absolute;
    inset: 2px;
    z-index: 3;
    box-sizing: border-box;
    border: 2px solid var(--volume-family-signal);
    border-radius: inherit;
    pointer-events: none;
  }

  .capsule-icon {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.9;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .volume-readout {
    flex: 0 0 var(--volume-readout-width);
    display: inline-flex;
    justify-content: flex-end;
    align-items: baseline;
    min-width: 0;
    padding: 0;
    border: 0;
    color: var(--volume-readout-color);
    background: transparent;
    font-family: var(--volume-readout-font);
    font-size: var(--volume-readout-size);
    font-variant-numeric: tabular-nums;
    font-weight: 700;
    line-height: 1;
    letter-spacing: 0;
    pointer-events: none;
  }

  .volume-readout.muted-readout {
    opacity: 0.56;
  }

  .volume-readout-unit {
    margin-left: 1px;
    font-size: 0.72em;
  }

  .capsule-slider {
    -webkit-appearance: none;
    appearance: none;
    flex: 1;
    min-width: 0;
    height: var(--volume-control-size);
    border: 0;
    border-radius: 0;
    background: linear-gradient(
        to right,
        var(--volume-context-signal) 0%,
        var(--volume-context-signal) var(--volume-percent),
        var(--volume-rail-rest) var(--volume-percent),
        var(--volume-rail-rest) 100%
      )
      center / 100% var(--volume-rail-size) no-repeat;
    outline: none;
    cursor: pointer;
  }

  .capsule-slider.muted-slider {
    opacity: 0.42;
  }

  .capsule-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: var(--volume-thumb-width);
    height: var(--volume-thumb-height);
    border: var(--volume-thumb-border);
    border-radius: var(--volume-thumb-radius);
    background: var(--volume-thumb-background);
    box-shadow: var(--volume-thumb-shadow);
    cursor: grab;
  }

  .capsule-slider::-moz-range-thumb {
    width: var(--volume-thumb-width);
    height: var(--volume-thumb-height);
    border: var(--volume-thumb-border);
    border-radius: var(--volume-thumb-radius);
    background: var(--volume-thumb-background);
    box-shadow: var(--volume-thumb-shadow);
    cursor: grab;
  }

  .dragging .capsule-slider,
  .dragging .capsule-slider::-webkit-slider-thumb,
  .dragging .capsule-slider::-moz-range-thumb {
    cursor: grabbing;
  }

  .volume-hover-zone--ark {
    --volume-track-background: color-mix(
      in srgb,
      var(--bg-primary) 88%,
      #080a0b 12%
    );
    --volume-track-border: color-mix(
      in srgb,
      var(--volume-family-signal) 42%,
      var(--border)
    );
    --volume-track-shadow: inset 3px 0 0 var(--volume-family-signal);
    --volume-track-radius: var(--shape-sm, 0);
    --volume-rail-rest: color-mix(
      in srgb,
      var(--text-primary) 22%,
      transparent
    );
    --volume-rail-size: 3px;
    --volume-thumb-width: 8px;
    --volume-thumb-height: 15px;
    --volume-thumb-radius: 0;
    --volume-readout-color: var(--volume-family-signal);
    --volume-readout-font: var(--font-mono);
  }

  .volume-hover-zone--ark .volume-family-marker {
    width: 2px;
    height: 16px;
    margin-top: -8px;
    background: var(--volume-family-signal);
    box-shadow: 5px 0 0
      color-mix(in srgb, var(--volume-family-signal) 34%, transparent);
  }

  .volume-hover-zone--endfield {
    --volume-track-background: var(
      --ark-field-dock-raised,
      color-mix(in srgb, var(--bg-primary) 84%, #191919 16%)
    );
    --volume-track-border: color-mix(
      in srgb,
      var(--text-primary) 34%,
      transparent
    );
    --volume-track-shadow: 4px 4px 0
      color-mix(in srgb, var(--volume-family-signal) 34%, transparent);
    --volume-track-radius: var(--shape-sm, 2px);
    --volume-track-padding-left: 27px;
    --volume-rail-rest: color-mix(
      in srgb,
      var(--text-primary) 28%,
      transparent
    );
    --volume-rail-size: 6px;
    --volume-thumb-width: 10px;
    --volume-thumb-height: 18px;
    --volume-thumb-radius: 0;
    --volume-thumb-background: var(--volume-context-signal);
    --volume-readout-color: var(--volume-family-signal);
    --volume-readout-font: var(--font-display);
  }

  .volume-hover-zone--endfield .capsule-track {
    clip-path: polygon(8px 0, 100% 0, 100% 100%, 0 100%, 0 8px);
  }

  .volume-hover-zone--endfield .volume-family-marker {
    left: 0;
    top: 0;
    width: 15px;
    height: 100%;
    background: var(--volume-family-signal);
    clip-path: polygon(0 0, 100% 0, 55% 100%, 0 100%);
  }

  .volume-hover-zone--exa {
    --volume-track-background: color-mix(
      in srgb,
      var(--bg-primary) 82%,
      transparent
    );
    --volume-track-border: color-mix(
      in srgb,
      var(--volume-family-signal) 46%,
      transparent
    );
    --volume-track-shadow:
      0 0 0 1px color-mix(in srgb, var(--volume-family-signal) 8%, transparent),
      0 8px 24px color-mix(in srgb, #080914 34%, transparent);
    --volume-track-radius: var(--shape-pill);
    --volume-track-padding-left: 30px;
    --volume-rail-rest: color-mix(
      in srgb,
      var(--text-primary) 18%,
      transparent
    );
    --volume-rail-size: 4px;
    --volume-thumb-width: 12px;
    --volume-thumb-height: 12px;
    --volume-thumb-radius: var(--shape-circle);
    --volume-thumb-border: 1px solid var(--volume-family-signal);
    --volume-thumb-background: var(--volume-context-signal);
    --volume-thumb-shadow: 0 0 10px
      color-mix(in srgb, var(--volume-family-signal) 36%, transparent);
    --volume-readout-color: var(--volume-family-signal);
    --volume-readout-font: var(--font-display);
  }

  .volume-hover-zone--exa .capsule-track {
    backdrop-filter: blur(var(--blur-md));
    -webkit-backdrop-filter: blur(var(--blur-md));
  }

  .volume-hover-zone--exa .volume-family-marker {
    width: 18px;
    height: 18px;
    margin-top: -9px;
    border: 1px solid var(--volume-family-signal);
    border-right-color: transparent;
    border-radius: var(--shape-circle);
    transform: rotate(-24deg);
  }

  .volume-hover-zone--exa .volume-family-marker::after {
    content: '';
    position: absolute;
    right: -2px;
    top: 2px;
    width: 4px;
    height: 4px;
    border-radius: var(--shape-circle);
    background: var(--volume-family-signal-alt);
  }

  .volume-hover-zone--popucom {
    --volume-track-background: color-mix(
      in srgb,
      var(--bg-primary) 92%,
      var(--volume-family-signal) 8%
    );
    --volume-track-border: var(--text-primary);
    --volume-track-shadow: 4px 5px 0
      var(--theme-custom-action-alt, var(--volume-family-signal-alt));
    --volume-track-radius: var(--shape-pill);
    --volume-track-padding-left: 29px;
    --volume-rail-rest: color-mix(
      in srgb,
      var(--text-primary) 24%,
      transparent
    );
    --volume-rail-size: 8px;
    --volume-thumb-width: 15px;
    --volume-thumb-height: 15px;
    --volume-thumb-radius: var(--shape-circle);
    --volume-thumb-border: 2px solid var(--text-primary);
    --volume-thumb-background: var(--volume-context-signal);
    --volume-thumb-shadow: 2px 2px 0 var(--text-primary);
    --volume-readout-color: var(--text-primary);
    --volume-readout-font: var(--font-display);
  }

  .volume-hover-zone--popucom .capsule-track {
    border-width: 2px;
  }

  .volume-hover-zone--popucom .volume-family-marker {
    width: 13px;
    height: 13px;
    margin-top: -7px;
    border: 2px solid var(--text-primary);
    border-radius: var(--shape-circle);
    background: var(--theme-custom-action-alt, var(--volume-family-signal-alt));
    box-shadow: 2px 2px 0 var(--text-primary);
    transform: rotate(-12deg);
  }

  .volume-hover-zone--corporate {
    --volume-track-background: color-mix(
      in srgb,
      var(--bg-primary) 94%,
      transparent
    );
    --volume-track-border: color-mix(
      in srgb,
      var(--text-primary) 52%,
      transparent
    );
    --volume-track-shadow: var(--elevation-xs);
    --volume-track-radius: var(--shape-sm, 0);
    --volume-track-padding-left: 27px;
    --volume-rail-rest: color-mix(
      in srgb,
      var(--text-primary) 24%,
      transparent
    );
    --volume-rail-size: 2px;
    --volume-thumb-width: 7px;
    --volume-thumb-height: 16px;
    --volume-thumb-radius: 0;
    --volume-thumb-background: var(--volume-context-signal);
    --volume-readout-color: var(--volume-family-signal);
    --volume-readout-font: var(--font-display);
  }

  .volume-hover-zone--corporate .volume-family-marker {
    width: 13px;
    height: 2px;
    margin-top: -1px;
    background: var(--volume-family-signal);
  }

  @media (max-width: 480px) {
    .volume-hover-zone {
      --volume-readout-width: 31px;
    }

    .capsule-reveal {
      max-width: min(
        calc(100% - var(--volume-control-overhang)),
        calc(100vw - 72px)
      );
    }
  }

  @media (hover: none) and (pointer: coarse) {
    .capsule-reveal {
      width: 200px !important;
      visibility: visible !important;
      opacity: 1 !important;
      pointer-events: auto !important;
      will-change: auto;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .capsule-icon-btn {
      transition: none;
    }

    .capsule-reveal {
      will-change: auto;
    }
  }
</style>
