<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import MotionPulseBlock from '$lib/components/MotionPulseBlock.svelte';

  interface Props {
    albumName?: string;
    artworkUrl?: string | null;
    loading?: boolean;
    reducedMotion?: boolean;
    stageStyle: string;
    mediaHeight: string;
    scrimOpacity: number;
    imageOpacity?: number;
    imageTransform?: string;
    solidifyOpacity?: number;
    element?: HTMLElement | null;
  }

  let {
    albumName = m.library_album_stage_default_name(),
    artworkUrl = null,
    loading = false,
    reducedMotion = false,
    stageStyle,
    mediaHeight,
    scrimOpacity,
    imageOpacity = 1,
    imageTransform = 'translateZ(0) scale(1)',
    solidifyOpacity = 0,
    element = $bindable<HTMLElement | null>(null),
  }: Props = $props();
</script>

<div class="album-stage" bind:this={element} style={stageStyle}>
  <div class="album-stage-frame">
    <div
      class={`album-stage-media${loading ? ' album-stage-media-loading' : ''}`}
      style:height={mediaHeight}
    >
      <div class="album-stage-media-content">
        {#if loading}
          <MotionPulseBlock
            className="album-stage-skeleton loading-cover"
            {reducedMotion}
          />
        {:else}
          <img
            class="album-stage-image"
            src={artworkUrl ?? undefined}
            alt={m.library_album_stage_alt({ name: albumName })}
            loading="eager"
            style:opacity={imageOpacity}
            style:transform={imageTransform}
          />
          <div
            class="album-stage-solidify"
            aria-hidden="true"
            style:opacity={solidifyOpacity}
          ></div>
        {/if}
      </div>
      <div
        class="album-stage-media-scrim"
        aria-hidden="true"
        style:opacity={scrimOpacity}
      ></div>
      <div class="album-stage-media-border" aria-hidden="true"></div>
      <div class="album-stage-divider" aria-hidden="true"></div>
    </div>
  </div>
</div>
