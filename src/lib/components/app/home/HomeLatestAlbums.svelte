<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { imageDataSrc } from '$lib/imageDataSrc';
  import type { Album } from '$lib/types';

  interface Props {
    albums: Album[];
    loading: boolean;
    onSelect: (album: Album) => void | Promise<void>;
  }

  let { albums, loading, onSelect }: Props = $props();

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
        <div class="skeleton-card"></div>
      {/each}
    </div>
  {:else if albums.length === 0}
    <p class="empty-hint">{labels.empty}</p>
  {:else}
    <div class="album-scroll">
      {#each albums as album (album.cid)}
        <button
          class="album-card-wrapper"
          onclick={() => onSelect(album)}
          type="button"
        >
          <img
            use:imageDataSrc={album.coverUrl}
            alt={album.name}
            class="album-cover"
            loading="lazy"
          />
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
    transition: opacity 0.15s ease;
  }

  .album-card-wrapper:hover {
    opacity: 0.8;
  }

  .album-cover {
    width: 140px;
    height: 140px;
    object-fit: cover;
    border-radius: 8px;
    background: var(--surface-secondary, rgba(255, 255, 255, 0.06));
  }

  .album-name {
    font-family: var(--font-body);
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-primary, #fff);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .album-artists {
    font-family: var(--font-body);
    font-size: 0.6875rem;
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .skeleton-row {
    display: flex;
    gap: 0.75rem;
  }

  .skeleton-card {
    flex-shrink: 0;
    width: 140px;
    height: 180px;
    border-radius: 8px;
    background: var(--surface-secondary, rgba(255, 255, 255, 0.06));
    animation: pulse 1.5s ease-in-out infinite;
  }

  .empty-hint {
    font-family: var(--font-body);
    font-size: 0.8125rem;
    color: var(--text-tertiary, rgba(255, 255, 255, 0.4));
    margin: 0;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
  }
</style>
