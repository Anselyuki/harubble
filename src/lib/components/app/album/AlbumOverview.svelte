<script lang="ts">
  import AlbumCard from '$lib/components/AlbumCard.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import type { Album } from '$lib/types';

  interface Props {
    albums: Album[];
    selectedAlbumCid: string | null;
    loadingAlbumCid?: string | null;
    reducedMotion: boolean;
    onSelectAlbum: (album: Album) => void;
  }

  const COVER_PRELOAD_MARGIN = '0px 0px 900px 0px';

  let {
    albums,
    selectedAlbumCid,
    loadingAlbumCid = null,
    reducedMotion,
    onSelectAlbum,
  }: Props = $props();
  let scrollRoot = $state<HTMLElement | null>(null);
</script>

<div class="album-overview">
  <div class="overview-scroll-area" bind:this={scrollRoot}>
    {#if albums.length === 0}
      <div class="overview-empty">
        <div class="overview-empty-icon">♪</div>
        <div class="overview-empty-text">{m.album_overview_empty()}</div>
      </div>
    {:else}
      <div class="overview-section">
        <h2 class="overview-section-title">
          {m.album_overview_all_title({ count: albums.length })}
        </h2>
        <div class="album-grid">
          {#each albums as album (album.cid)}
            <AlbumCard
              {album}
              layout="grid"
              selected={selectedAlbumCid === album.cid}
              loading={loadingAlbumCid === album.cid}
              {reducedMotion}
              coverPreloadRoot={scrollRoot}
              coverPreloadMargin={COVER_PRELOAD_MARGIN}
              onclick={() => onSelectAlbum(album)}
            />
          {/each}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .album-overview {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--surface-workspace);
    overflow: hidden;
  }

  .overview-scroll-area {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: transparent transparent;
  }

  .overview-scroll-area:hover {
    scrollbar-color: rgba(255, 255, 255, 0.28) transparent;
  }

  .overview-section {
    padding: 24px 24px 32px;
  }

  .overview-section-title {
    font-family: var(--font-display);
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 16px;
    padding: 0 4px;
  }

  .album-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 20px;
  }

  /* ── Empty state ── */

  .overview-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 64px 24px;
    color: var(--text-tertiary);
  }

  .overview-empty-icon {
    font-size: 36px;
    opacity: 0.5;
  }

  .overview-empty-text {
    font-size: 14px;
  }
</style>
