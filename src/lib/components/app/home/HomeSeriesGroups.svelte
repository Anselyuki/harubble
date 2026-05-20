<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { imageDataSrc } from '$lib/imageDataSrc';
  import type { SeriesGroup } from '$lib/types';

  interface Props {
    groups: SeriesGroup[];
    belongReady: boolean;
    onSelectSeries: (series: string) => void;
  }

  let { groups, belongReady, onSelectSeries }: Props = $props();

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      aria: m.home_series_aria(),
      title: m.home_series_title(),
      empty: m.home_empty_series(),
    };
  });
</script>

<section class="series-groups" aria-label={labels.aria}>
  <h2 class="section-title">{labels.title}</h2>

  {#if !belongReady}
    <div class="skeleton-list">
      {#each Array(3) as _, i (i)}
        <div class="skeleton-row"></div>
      {/each}
    </div>
  {:else if groups.length === 0}
    <p class="empty-hint">{labels.empty}</p>
  {:else}
    <ul class="group-list" role="list">
      {#each groups as group (group.series)}
        <li class="group-item">
          <button
            class="group-header"
            type="button"
            onclick={() => onSelectSeries(group.series)}
          >
            <span class="group-name">{group.series}</span>
            <span class="group-count"
              >{m.home_album_count({ count: group.albums.length })}</span
            >
          </button>

          <div class="group-albums">
            {#each group.albums.slice(0, 8) as album (album.cid)}
              <div class="mini-cover-wrapper" title={album.name}>
                <img
                  use:imageDataSrc={album.coverUrl}
                  alt={album.name}
                  class="mini-cover"
                  loading="lazy"
                />
              </div>
            {/each}
            {#if group.albums.length > 8}
              <span class="overflow-badge">+{group.albums.length - 8}</span>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .series-groups {
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

  .group-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .group-item {
    background: var(--surface-secondary, rgba(255, 255, 255, 0.04));
    border-radius: 10px;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }

  .group-header {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    text-align: left;
    color: inherit;
  }

  .group-name {
    font-family: var(--font-body);
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-primary, #fff);
  }

  .group-count {
    font-family: var(--font-body);
    font-size: 0.75rem;
    color: var(--text-tertiary, rgba(255, 255, 255, 0.4));
  }

  .group-albums {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    flex-wrap: wrap;
  }

  .mini-cover-wrapper {
    width: 40px;
    height: 40px;
    border-radius: 5px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .mini-cover {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .overflow-badge {
    font-family: var(--font-wide);
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
    padding: 0 0.25rem;
  }

  .skeleton-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .skeleton-row {
    height: 80px;
    border-radius: 10px;
    background: var(--surface-secondary, rgba(255, 255, 255, 0.04));
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
