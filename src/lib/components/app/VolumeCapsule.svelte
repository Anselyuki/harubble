<script lang="ts">
  import { onDestroy } from 'svelte';
  import { gsap, killTweens, getMotionDuration } from '$lib/design/gsap';
  import { sliderToGain, gainToSlider } from '$lib/features/player/volume';
  import { getVolumeStripExtension } from './volume-capsule-strip';
  import { createWaveLoop, WAVE_LAYERS } from './volume-capsule-wave';
  import type { WaveController } from './volume-capsule-wave';
  import * as m from '$lib/paraglide/messages.js';

  const COLLAPSE_DELAY_MS = 799;
  const CAPSULE_WIDTH = 200;

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
  let trackRef = $state<HTMLElement | null>(null);
  let sliderRef = $state<HTMLInputElement | null>(null);
  let wrapperRef = $state<HTMLElement | null>(null);
  let badgeRef = $state<HTMLElement | null>(null);
  let stripRef = $state<HTMLElement | null>(null);
  let iconBtnRef = $state<HTMLButtonElement | null>(null);
  let glassRef = $state<HTMLDivElement | null>(null);
  let waveGroupRef = $state<SVGGElement | null>(null);
  let collapseTimeout: ReturnType<typeof setTimeout> | null = null;
  let badgeVisible = false;
  let isDragging = $state(false);

  const sliderPos = $derived(gainToSlider(volume));
  let sliderPreview = $state<number | null>(null);
  const shownPos = $derived(sliderPreview ?? sliderPos);
  const displayPercent = $derived(muted ? 0 : Math.round(shownPos * 100));
  const isEasterEgg = $derived(displayPercent === 32);
  const displayText = $derived(isEasterEgg ? '32.5' : String(displayPercent));

  const volumeIcon = $derived.by(() => {
    if (muted || volume === 0) return 'muted' as const;
    if (gainToSlider(volume) < 0.5) return 'low' as const;
    return 'high' as const;
  });

  const isOpen = $derived(
    capsuleState === 'open' ||
      capsuleState === 'expanding' ||
      capsuleState === 'collapsing'
  );

  // --- Wave animation ---

  let waveCtrl: WaveController | null = null;

  $effect(() => {
    waveCtrl = createWaveLoop(() => waveGroupRef);
    return () => waveCtrl?.stop();
  });

  $effect(() => {
    const open = capsuleState === 'open' || capsuleState === 'expanding';
    if (open) {
      waveCtrl?.start();
    } else {
      waveCtrl?.stop();
    }
  });

  // --- Glass reveal/hide ---

  function revealGlass() {
    if (!glassRef || !rightControlsRef) return;
    killTweens(glassRef);
    gsap.set(glassRef, { visibility: 'visible', width: CAPSULE_WIDTH });
    gsap.fromTo(
      glassRef,
      { height: 0 },
      { height: 50, duration: getMotionDuration(300), ease: 'ios-out' }
    );
  }

  function hideGlass() {
    if (!glassRef) return;
    killTweens(glassRef);
    gsap.to(glassRef, {
      height: 0,
      duration: getMotionDuration(180),
      ease: 'ios-in',
      onComplete: () => {
        gsap.set(glassRef, { visibility: 'hidden' });
      },
    });
  }

  // --- Badge (merged showBadge + showBadgePersistent) ---

  function showBadge(persistent = false) {
    if (!badgeRef || !trackRef) return;
    const stripExtension = getVolumeStripExtension({
      badgeTop: badgeRef.getBoundingClientRect().top,
      trackTop: trackRef.getBoundingClientRect().top,
    });

    if (!badgeVisible) {
      badgeVisible = true;
      killTweens(badgeRef);
      gsap.fromTo(
        badgeRef,
        { y: '100%' },
        { y: '0%', duration: getMotionDuration(200), ease: 'ios-out' }
      );
      if (stripRef) {
        killTweens(stripRef);
        gsap.to(stripRef, {
          top: -stripExtension,
          duration: getMotionDuration(200),
          ease: 'ios-out',
        });
      }
      revealGlass();
    } else if (!persistent && stripRef) {
      killTweens(stripRef);
      gsap.to(stripRef, {
        top: -stripExtension,
        duration: getMotionDuration(120),
        ease: 'ios-out',
      });
    }
  }

  function hideBadge() {
    if (!badgeRef || !badgeVisible) return;
    killTweens(badgeRef);
    gsap.to(badgeRef, {
      y: '100%',
      duration: getMotionDuration(180),
      ease: 'ios-in',
      onComplete: () => {
        badgeVisible = false;
      },
    });
    if (stripRef) {
      killTweens(stripRef);
      gsap.to(stripRef, {
        top: 0,
        duration: getMotionDuration(180),
        ease: 'ios-in',
      });
    }
    hideGlass();
  }

  // --- Expand / Collapse ---

  export function expand() {
    if (capsuleState === 'open' || capsuleState === 'expanding') return;
    if (!trackRef || !rightControlsRef) return;

    const targetWidth = CAPSULE_WIDTH;
    const duration = getMotionDuration(400);
    const bg = window.matchMedia('(prefers-color-scheme: dark)').matches
      ? '#2c2c2e'
      : '#f2f2f7';

    capsuleState = 'expanding';
    killTweens(trackRef);

    if (stripRef) {
      killTweens(stripRef);
      gsap.to(stripRef, {
        opacity: 1,
        duration: getMotionDuration(150),
        ease: 'ios-out',
      });
    }

    gsap.set(trackRef, { backgroundColor: bg });
    gsap.to(trackRef, {
      width: targetWidth,
      opacity: 1,
      borderTopLeftRadius: 0,
      borderTopRightRadius: 0,
      borderBottomRightRadius: 16,
      borderBottomLeftRadius: 16,
      duration,
      ease: 'ios-spring',
      onComplete: () => {
        capsuleState = 'open';
        sliderRef?.focus();
      },
    });

    showBadge(true);
  }

  function collapse() {
    if (capsuleState === 'closed' || capsuleState === 'collapsing') return;
    if (!trackRef) return;

    const shrinkDuration = getMotionDuration(799);
    const badgeHideDuration = getMotionDuration(180);

    capsuleState = 'collapsing';
    killTweens(trackRef);
    hideBadge();

    if (stripRef) {
      killTweens(stripRef);
      gsap.to(stripRef, {
        opacity: 0,
        duration: getMotionDuration(200),
        delay: badgeHideDuration * 0.5,
        ease: 'ios-in',
      });
    }

    gsap.to(trackRef, {
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
        capsuleState = 'closed';
        gsap.set(trackRef!, { backgroundColor: 'transparent' });
      },
    });
  }

  export function toggle() {
    if (capsuleState === 'closed' || capsuleState === 'collapsing') {
      expand();
    } else {
      collapse();
    }
  }

  export function scheduleCollapse() {
    if (collapseTimeout) clearTimeout(collapseTimeout);
    collapseTimeout = setTimeout(() => {
      collapseTimeout = null;
      collapse();
    }, COLLAPSE_DELAY_MS);
  }

  // --- Interaction handlers ---

  function handleSliderInput(event: Event) {
    const pos = Number((event.currentTarget as HTMLInputElement).value);
    sliderPreview = pos;
    void onVolumeChange?.(sliderToGain(pos));
    showBadge();
  }

  function handleSliderCommit() {
    sliderPreview = null;
  }

  function handleSliderDown() {
    isDragging = true;
  }

  function handleSliderUp() {
    isDragging = false;
  }

  function handleFocusOut(event: FocusEvent) {
    if (!wrapperRef) return;
    const related = event.relatedTarget as Node | null;
    if (related && wrapperRef.contains(related)) return;
    if (capsuleState === 'open' || capsuleState === 'expanding') {
      scheduleCollapse();
    }
  }

  function handleMouseLeave() {
    if (isDragging) return;
    if (capsuleState === 'open' || capsuleState === 'expanding') {
      scheduleCollapse();
    }
  }

  function handleMouseEnter() {
    if (collapseTimeout) {
      clearTimeout(collapseTimeout);
      collapseTimeout = null;
    }
  }

  function handleKeydown(_event: KeyboardEvent) {}

  function handleIconClick() {
    if (capsuleState === 'closed' || capsuleState === 'collapsing') {
      expand();
    }
  }

  function handleIconPointerEnter() {
    if (!iconBtnRef) return;
    gsap.to(iconBtnRef, {
      color: 'var(--icon-active)',
      backgroundColor:
        'var(--player-control-hover-bg, rgba(var(--accent-rgb), 0.1))',
      duration: getMotionDuration(150),
      ease: 'ios-out',
    });
  }

  function handleIconPointerLeave() {
    if (!iconBtnRef) return;
    gsap.to(iconBtnRef, {
      color: 'var(--icon-default)',
      backgroundColor: 'transparent',
      duration: getMotionDuration(150),
      ease: 'ios-in',
    });
  }

  // --- Drag escape hatch ---

  $effect(() => {
    if (!isDragging) return;
    function handleGlobalPointerUp() {
      isDragging = false;
      if (wrapperRef && !wrapperRef.matches(':hover')) {
        scheduleCollapse();
      }
    }
    document.addEventListener('pointerup', handleGlobalPointerUp);
    return () =>
      document.removeEventListener('pointerup', handleGlobalPointerUp);
  });

  // --- Cleanup ---

  onDestroy(() => {
    waveCtrl?.stop();
    if (collapseTimeout) clearTimeout(collapseTimeout);
    if (trackRef) killTweens(trackRef);
    if (badgeRef) killTweens(badgeRef);
    if (stripRef) killTweens(stripRef);
    if (iconBtnRef) killTweens(iconBtnRef);
    if (glassRef) killTweens(glassRef);
  });
</script>

<!-- Hover zone: covers glass + track so mouse between them doesn't trigger leave -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="volume-hover-zone"
  onmouseenter={handleMouseEnter}
  onmouseleave={handleMouseLeave}
>
  <!-- Glass pane: backdrop-filter isolated from animated SVG to avoid recomposite -->
  <div class="wave-glass" bind:this={glassRef} aria-hidden="true">
    <div class="wave-glass-blur"></div>
    <svg
      class="ripple-backdrop"
      viewBox="0 0 200 50"
      preserveAspectRatio="none"
    >
      <defs>
        <radialGradient
          id="bottom-glow"
          cx="50%"
          cy="100%"
          r="60%"
          fx="50%"
          fy="100%"
        >
          <stop offset="0%" stop-color="rgba(var(--accent-rgb), 0.4)" />
          <stop offset="100%" stop-color="rgba(var(--accent-rgb), 0)" />
        </radialGradient>
      </defs>
      <rect x="0" y="18" width="200" height="32" fill="url(#bottom-glow)" />
      <g class="dot-grid">
        {#each { length: 7 } as _, row (row)}
          {#each { length: 24 } as _, col (col)}
            <circle
              cx={4 + col * 8}
              cy={50 - 4 - row * 6}
              r={1.5 - row * 0.15}
              class="dot"
            />
          {/each}
        {/each}
      </g>
      <g bind:this={waveGroupRef}>
        {#each WAVE_LAYERS as layer, i (i)}
          <path
            d="M0,80 L200,80"
            class="siri-wave"
            style="opacity:{layer.opacity}"
          />
        {/each}
      </g>
    </svg>
  </div>

  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="volume-wrapper"
    bind:this={wrapperRef}
    role="group"
    aria-label={m.player_aria_volume()}
    data-state={capsuleState}
    onfocusout={handleFocusOut}
    onkeydown={handleKeydown}
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
    <span class="capsule-strip" bind:this={stripRef}></span>
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
        onchange={handleSliderCommit}
        onpointerdown={handleSliderDown}
        onpointerup={handleSliderUp}
        style="--volume-percent:{shownPos * 100}%"
        class:muted-slider={muted}
        tabindex={capsuleState === 'open' || capsuleState === 'expanding'
          ? 0
          : -1}
      />
    </div>
    <button
      type="button"
      class="capsule-icon-btn"
      bind:this={iconBtnRef}
      aria-label={muted ? m.player_aria_unmute() : m.player_aria_mute()}
      aria-expanded={isOpen}
      onclick={handleIconClick}
      ondblclick={() => onToggleMute?.()}
      onpointerenter={handleIconPointerEnter}
      onpointerleave={handleIconPointerLeave}
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
  .wave-glass {
    position: absolute;
    right: 0;
    bottom: var(--control-button-size, 34px);
    height: 0;
    pointer-events: none;
    visibility: hidden;
    overflow: hidden;
    border-radius: 16px 16px 0 0;
    z-index: 10;
    border: 1px solid rgba(255, 255, 255, 0.18);
    border-bottom: none;
    box-shadow:
      0 4px 16px rgba(0, 0, 0, 0.12),
      0 1px 4px rgba(0, 0, 0, 0.08);
  }

  .wave-glass-blur {
    position: absolute;
    inset: 0;
    z-index: 0;
    border-radius: inherit;
    background:
      linear-gradient(
        180deg,
        rgba(255, 255, 255, 0.18),
        rgba(255, 255, 255, 0.06)
      ),
      color-mix(in srgb, var(--player-shell-bg) 28%, transparent);
    backdrop-filter: blur(22px) saturate(1.75);
    -webkit-backdrop-filter: blur(22px) saturate(1.75);
  }

  .wave-glass-blur::before {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.2),
      rgba(255, 255, 255, 0)
    );
    pointer-events: none;
  }

  @media (prefers-color-scheme: dark) {
    .wave-glass {
      border-color: rgba(255, 255, 255, 0.12);
      box-shadow:
        0 4px 16px rgba(0, 0, 0, 0.28),
        0 1px 4px rgba(0, 0, 0, 0.18);
    }

    .wave-glass-blur {
      background:
        linear-gradient(
          180deg,
          rgba(255, 255, 255, 0.12),
          rgba(255, 255, 255, 0.035)
        ),
        color-mix(in srgb, var(--player-shell-bg) 18%, transparent);
    }

    .wave-glass-blur::before {
      background: linear-gradient(
        180deg,
        rgba(255, 255, 255, 0.12),
        rgba(255, 255, 255, 0)
      );
    }
  }

  .ripple-backdrop {
    position: relative;
    z-index: 1;
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
    will-change: contents;
  }

  .siri-wave {
    fill: none;
    stroke: rgba(var(--accent-rgb), 0.6);
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }

  .dot {
    fill: rgba(var(--accent-rgb), 0.5);
  }

  .volume-hover-zone {
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
    border-radius: 999px;
    white-space: nowrap;
    box-shadow:
      0 4px 16px rgba(0, 0, 0, 0.12),
      0 1px 4px rgba(0, 0, 0, 0.08);
  }

  .capsule-strip {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 4px;
    background: var(--icon-active);
    opacity: 0;
    pointer-events: none;
    z-index: 1;
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
    border-radius: 50%;
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
    margin-bottom: 0;
    overflow: hidden;
    height: 44px;
    pointer-events: none;
    z-index: 1;
  }

  .volume-badge {
    display: flex;
    align-items: baseline;
    margin-left: 6px;
    font-family: var(--font-brand);
    font-size: 36px;
    color: var(--icon-active);
    pointer-events: none;
    transform: translateY(100%);
  }

  .volume-badge-unit {
    font-size: 18px;
    margin-left: 1px;
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
      var(--surface-highlight, rgba(120, 120, 128, 0.28)) var(--volume-percent),
      var(--surface-highlight, rgba(120, 120, 128, 0.28)) 100%
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
