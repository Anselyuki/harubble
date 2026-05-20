<script lang="ts">
  import { animateIn, killTweens } from '$lib/design/gsap';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import MotionPulseBlock from '$lib/components/MotionPulseBlock.svelte';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';

  interface Props {
    reducedMotion: boolean;
  }

  let props: Props = $props();

  let cardEl = $state<HTMLElement | undefined>();
  let heroInfoEl = $state<HTMLElement | undefined>();
  let loadingEl = $state<HTMLElement | undefined>();

  $effect(() => {
    if (!cardEl) return;
    animateIn(cardEl, { opacity: 0 }, { opacity: 1 }, 180, 'ios-out');
    return () => killTweens(cardEl!);
  });

  $effect(() => {
    if (!heroInfoEl) return;
    animateIn(heroInfoEl, { opacity: 0 }, { opacity: 1 }, 220, 'ios-out');
    return () => killTweens(heroInfoEl!);
  });

  $effect(() => {
    if (!loadingEl) return;
    animateIn(loadingEl, { opacity: 0 }, { opacity: 1 }, 200, 'ios-out');
    return () => killTweens(loadingEl!);
  });

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      loadingSongs: m.library_loading_songs(),
    };
  });
</script>

<div class="album-detail-card" bind:this={cardEl}>
  <div class="album-hero">
    <div class="album-hero-info" bind:this={heroInfoEl}>
      <MotionPulseBlock
        className="album-hero-title loading-text"
        reducedMotion={props.reducedMotion}
      />
      <MotionPulseBlock
        className="album-hero-sub loading-text-sub"
        reducedMotion={props.reducedMotion}
        delay={0.14}
      />
    </div>
  </div>
  <div class="loading album-loading" bind:this={loadingEl}>
    <span>{labels.loadingSongs}</span><MotionSpinner
      className="inline-loading-spinner"
      reducedMotion={props.reducedMotion}
    />
  </div>
</div>
