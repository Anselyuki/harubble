<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { imageDataSrc } from '$lib/imageDataSrc';
  import MotionPulseBlock from '$lib/components/MotionPulseBlock.svelte';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';
  import type { Album } from '$lib/types';

  interface Props {
    albums: Album[];
    loading: boolean;
    loadingAlbumCid?: string | null;
    reducedMotion?: boolean;
    onSelect: (album: Album) => void | Promise<void>;
  }

  let {
    albums,
    loading,
    loadingAlbumCid = null,
    reducedMotion = false,
    onSelect,
  }: Props = $props();

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      title: m.home_latest_albums_title(),
      empty: m.home_empty_albums(),
    };
  });
</script>

<section class="latest-albums" aria-label={labels.title}>
  <h2 class="section-title">{labels.title}</h2>

  {#if loading && albums.length === 0}
    <div class="skeleton-row">
      {#each Array(6) as _, i (i)}
        <MotionPulseBlock className="skeleton-card" {reducedMotion} />
      {/each}
    </div>
  {:else if albums.length === 0}
    <p class="empty-hint">{labels.empty}</p>
  {:else}
    <div class="album-scroll">
      {#each albums as album, index (album.cid)}
        <button
          class="album-card-wrapper"
          onclick={() => onSelect(album)}
          type="button"
        >
          <div class="album-cover-frame">
            <img
              use:imageDataSrc={{
                src: album.coverUrl,
                loading: index < 6 ? 'eager' : 'lazy',
                rootMargin: '700px',
              }}
              alt={album.name}
              class="album-cover"
              loading={index < 6 ? 'eager' : 'lazy'}
            />
            {#if loadingAlbumCid === album.cid}
              <div class="album-cover-loading" aria-hidden="true">
                <MotionSpinner
                  className="album-cover-spinner"
                  {reducedMotion}
                />
              </div>
            {/if}
          </div>
          <span class="album-name">{album.name}</span>
          <span class="album-artists">{album.artists.join(', ')}</span>
        </button>
      {/each}
    </div>
  {/if}
</section>

<style>
  .latest-albums {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary, #fff);
    margin: 0;
  }

  .album-scroll {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    overflow-x: auto;
    padding-bottom: 0.5rem;
    scrollbar-width: thin;
  }

  .album-card-wrapper {
    flex-shrink: 0;
    width: 140px;
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
    color: inherit;
    border-radius: 8px;
    transition: var(--motion-hover);
  }

  .album-card-wrapper:hover {
    opacity: 0.82;
  }

  .album-cover-frame {
    position: relative;
    width: 140px;
    height: 140px;
    border-radius: 8px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .album-cover {
    display: block;
    width: 140px;
    height: 140px;
    object-fit: cover;
    border-radius: 8px;
    background: var(--surface-secondary, rgba(255, 255, 255, 0.06));
  }

  .album-cover-loading {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.35);
    backdrop-filter: blur(2px);
  }

  .album-cover-loading :global(.album-cover-spinner) {
    width: 22px;
    height: 22px;
    color: white;
  }

  .album-name {
    font-family: var(--font-body);
    font-size: 0.8125rem;
    font-weight: 500;
    line-height: 1.25rem;
    color: var(--text-primary, #fff);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .album-artists {
    font-family: var(--font-body);
    font-size: 0.6875rem;
    line-height: 1rem;
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .skeleton-row {
    display: flex;
    gap: 0.75rem;
  }

  .skeleton-row :global(.skeleton-card) {
    flex-shrink: 0;
    width: 140px;
    height: 188px;
    border-radius: 8px;
    background: var(--surface-secondary, rgba(255, 255, 255, 0.06));
  }

  .empty-hint {
    font-family: var(--font-body);
    font-size: 0.8125rem;
    color: var(--text-tertiary, rgba(255, 255, 255, 0.4));
    margin: 0;
  }
</style>
