<script lang="ts">
  import { killTweens } from '$lib/design/gsap';
  import { runLayeredIn } from '$lib/design/view-transition';
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

  // 骨架屏与真实详情面板同构的分层进入，避免加载态与就绪态之间的观感跳变。
  $effect(() => {
    if (!cardEl) return;
    const tl = runLayeredIn([
      { target: cardEl },
      { target: heroInfoEl },
      { target: loadingEl },
    ]);
    return () => {
      tl.kill();
      killTweens(cardEl!);
      if (heroInfoEl) killTweens(heroInfoEl);
      if (loadingEl) killTweens(loadingEl);
    };
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
