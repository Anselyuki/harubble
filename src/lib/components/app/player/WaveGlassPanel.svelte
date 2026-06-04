<script lang="ts">
  import { onDestroy } from 'svelte';
  import { createWaveLoop, WAVE_LAYERS } from './volume-capsule-wave';
  import type { WaveController } from './volume-capsule-wave';
  import { gsap, killTweens, getMotionDuration } from '$lib/design/gsap';

  const GLASS_WIDTH = 200;

  interface Props {
    open: boolean;
  }

  let { open }: Props = $props();

  let glassEl = $state<HTMLDivElement | null>(null);
  let waveGroupRef = $state<SVGGElement | null>(null);
  let waveCtrl: WaveController | null = null;

  $effect(() => {
    waveCtrl = createWaveLoop(() => waveGroupRef);
    return () => waveCtrl?.stop();
  });

  $effect(() => {
    if (!glassEl) return;
    if (open) {
      waveCtrl?.start();
      killTweens(glassEl);
      gsap.set(glassEl, { visibility: 'visible', width: GLASS_WIDTH });
      gsap.fromTo(
        glassEl,
        { height: 0 },
        { height: 50, duration: getMotionDuration(300), ease: 'ios-out' }
      );
    } else {
      waveCtrl?.stop();
      killTweens(glassEl);
      gsap.to(glassEl, {
        height: 0,
        duration: getMotionDuration(180),
        ease: 'ios-in',
        onComplete: () => {
          gsap.set(glassEl!, { visibility: 'hidden' });
        },
      });
    }
  });

  onDestroy(() => {
    waveCtrl?.stop();
    if (glassEl) killTweens(glassEl);
  });
</script>

<div class="wave-glass" bind:this={glassEl} aria-hidden="true">
  <div class="wave-glass-blur"></div>
  <svg class="ripple-backdrop" viewBox="0 0 200 50" preserveAspectRatio="none">
    <defs>
      <radialGradient
        id="bottom-glow"
        cx="50%"
        cy="100%"
        r="60%"
        fx="50%"
        fy="100%"
      >
        <stop offset="0%" stop-color="rgba(var(--album-accent-rgb), 0.4)" />
        <stop offset="100%" stop-color="rgba(var(--album-accent-rgb), 0)" />
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
    stroke: rgba(var(--album-accent-rgb), 0.6);
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
    vector-effect: non-scaling-stroke;
  }

  .dot {
    fill: rgba(var(--album-accent-rgb), 0.5);
  }
</style>
