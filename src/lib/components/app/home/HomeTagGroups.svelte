<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { imageDataSrc } from '$lib/imageDataSrc';
  import type { Album, TagDimension, TagGroup } from '$lib/types';

  interface Props {
    dimensions: TagDimension[];
    groups: TagGroup[];
    selectedDimensionKey: string | null;
    onSelectDimension: (key: string) => void;
    onSelectAlbum: (album: Album) => void | Promise<void>;
  }

  let {
    dimensions,
    groups,
    selectedDimensionKey,
    onSelectDimension,
    onSelectAlbum,
  }: Props = $props();

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      aria: m.home_tags_aria(),
      title: m.home_tags_title(),
      empty: m.home_empty_tags(),
    };
  });
</script>

{#if dimensions.length > 0}
  <section class="tag-groups" aria-label={labels.aria}>
    <h2 class="section-title">{labels.title}</h2>

    <div class="dimension-chips" role="tablist">
      {#each dimensions as dim (dim.key)}
        <button
          class="dimension-chip"
          class:active={selectedDimensionKey === dim.key}
          role="tab"
          aria-selected={selectedDimensionKey === dim.key}
          onclick={() => onSelectDimension(dim.key)}
          type="button"
        >
          {dim.label}
        </button>
      {/each}
    </div>

    {#if groups.length === 0}
      <p class="empty-hint">{labels.empty}</p>
    {:else}
      <ul class="group-list" role="list">
        {#each groups as group (group.value)}
          <li class="group-item">
            <div class="group-header">
              <span class="group-name">{group.value}</span>
              <span class="group-count"
                >{m.home_album_count({ count: group.albums.length })}</span
              >
            </div>

            <div class="group-albums">
              {#each group.albums.slice(0, 8) as album (album.cid)}
                <button
                  class="mini-cover-wrapper"
                  title={album.name}
                  onclick={() => onSelectAlbum(album)}
                  type="button"
                >
                  <img
                    use:imageDataSrc={album.coverUrl}
                    alt={album.name}
                    class="mini-cover"
                    loading="lazy"
                  />
                </button>
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
{/if}

<style>
  .tag-groups {
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

  .dimension-chips {
    display: flex;
    gap: 0.375rem;
    flex-wrap: wrap;
  }

  .dimension-chip {
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    border: 1px solid var(--text-tertiary, rgba(255, 255, 255, 0.2));
    background: none;
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
    font-family: var(--font-body);
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    transition:
      background 0.15s ease,
      color 0.15s ease,
      border-color 0.15s ease;
  }

  .dimension-chip:hover {
    background: var(--surface-secondary, rgba(255, 255, 255, 0.06));
    color: var(--text-primary, #fff);
  }

  .dimension-chip.active {
    background: var(--accent, rgb(250, 45, 72));
    border-color: var(--accent, rgb(250, 45, 72));
    color: #fff;
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
    padding: 0;
    border: none;
    background: none;
    cursor: pointer;
    transition: opacity 0.15s ease;
  }

  .mini-cover-wrapper:hover {
    opacity: 0.8;
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

  .empty-hint {
    font-family: var(--font-body);
    font-size: 0.8125rem;
    color: var(--text-tertiary, rgba(255, 255, 255, 0.4));
    margin: 0;
  }
</style>
