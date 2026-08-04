<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { lazyLoad } from '$lib/lazyLoad';
  import MotionPulseBlock from '$lib/components/MotionPulseBlock.svelte';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import {
    ChevronRight,
    Disc3,
    Music2,
    RefreshCw,
    SearchX,
  } from '@lucide/svelte';
  import type {
    Album,
    LibraryIndexState,
    LibrarySearchHitField,
    SearchLibraryResponse,
    SearchLibraryResultItem,
  } from '$lib/types';

  interface Props {
    response: SearchLibraryResponse | null;
    indexState: LibraryIndexState;
    searchLoading: boolean;
    loadingMore: boolean;
    searchError: string | null;
    albums: Album[];
    reducedMotion?: boolean;
    onSelectResult: (item: SearchLibraryResultItem) => void | Promise<void>;
    onRetry: () => void | Promise<void>;
    onLoadMore: () => void | Promise<void>;
  }

  let {
    response,
    indexState,
    searchLoading,
    loadingMore,
    searchError,
    albums,
    reducedMotion = false,
    onSelectResult,
    onRetry,
    onLoadMore,
  }: Props = $props();

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      count: (count: number) => m.search_results_count({ count }),
      updating: m.search_results_updating(),
      updatingHint: m.search_results_updating_hint(),
      notReadyTitle: m.search_results_not_ready_title(),
      notReadyHint: m.search_results_not_ready_hint(),
      buildingTitle: m.library_search_index_building_title(),
      buildingHint: m.library_search_index_building_hint(),
      noResults: m.library_search_no_results(),
      noResultsHint: m.search_results_no_results_hint(),
      loadMore: m.search_results_load_more(),
      loadingMore: m.search_results_loading_more(),
      retry: m.search_results_retry(),
      kindAlbum: m.library_search_result_kind_album(),
      kindSong: m.library_search_result_kind_song(),
      matchTitle: m.search_match_title(),
      matchArtist: m.search_match_artist(),
      matchIntro: m.search_match_intro(),
      matchBelong: m.search_match_belong(),
      matchTagValues: m.search_match_tag_values(),
    };
  });

  const effectiveIndexState = $derived(response?.indexState ?? indexState);
  const hasResults = $derived((response?.items.length ?? 0) > 0);
  const hasMore = $derived(
    Boolean(response && response.items.length < response.total)
  );
  const isUpdating = $derived(
    hasResults &&
      (effectiveIndexState === 'building' || effectiveIndexState === 'stale')
  );

  function albumFor(albumCid: string): Album | undefined {
    return albums.find((album) => album.cid === albumCid);
  }

  function matchLabel(field: LibrarySearchHitField): string {
    switch (field) {
      case 'title':
        return labels.matchTitle;
      case 'artist':
        return labels.matchArtist;
      case 'intro':
        return labels.matchIntro;
      case 'belong':
        return labels.matchBelong;
      case 'tagValues':
        return labels.matchTagValues;
    }
  }
</script>

{#if searchLoading && !hasResults}
  <div class="search-skeleton" aria-live="polite">
    {#each Array(6) as _, index (index)}
      <div class="skeleton-row">
        <MotionPulseBlock
          className="skeleton-cover"
          {reducedMotion}
          delay={index * 0.04}
        />
        <div class="skeleton-copy">
          <MotionPulseBlock
            className="skeleton-line skeleton-line--title"
            {reducedMotion}
            delay={index * 0.04}
          />
          <MotionPulseBlock
            className="skeleton-line skeleton-line--meta"
            {reducedMotion}
            delay={index * 0.04}
          />
        </div>
      </div>
    {/each}
  </div>
{:else if searchError && !hasResults}
  <div class="search-empty-state" role="alert">
    <SearchX size={30} strokeWidth={1.6} />
    <div class="empty-title">{searchError}</div>
    <Button variant="outline" size="sm" onclick={onRetry}>
      <RefreshCw data-icon="inline-start" />
      {labels.retry}
    </Button>
  </div>
{:else if !hasResults && effectiveIndexState !== 'ready'}
  <div class="search-empty-state" aria-live="polite">
    <MotionSpinner className="index-spinner" {reducedMotion} />
    <div class="empty-title">
      {effectiveIndexState === 'building'
        ? labels.buildingTitle
        : labels.notReadyTitle}
    </div>
    <div class="empty-hint">
      {effectiveIndexState === 'building'
        ? labels.buildingHint
        : labels.notReadyHint}
    </div>
  </div>
{:else if response && !hasResults}
  <div class="search-empty-state">
    <SearchX size={30} strokeWidth={1.6} />
    <div class="empty-title">{labels.noResults}</div>
    <div class="empty-hint">{labels.noResultsHint}</div>
  </div>
{:else if response}
  <section class="search-results" data-testid="search-results">
    <header class="results-header">
      <div class="results-count">{labels.count(response.total)}</div>
      <div class="results-status" aria-live="polite">
        {#if searchLoading}
          <MotionSpinner className="results-spinner" {reducedMotion} />
        {/if}
        {#if isUpdating}
          <span class="updating-dot" aria-hidden="true"></span>
          <span>{labels.updating}</span>
        {/if}
      </div>
    </header>

    {#if isUpdating}
      <div class="updating-note">{labels.updatingHint}</div>
    {/if}

    <div class="results-list">
      {#each response.items as item (`${item.kind}:${item.albumCid}:${item.songCid ?? 'album'}`)}
        {@const album = albumFor(item.albumCid)}
        {@const resultName =
          item.kind === 'song' && item.songTitle
            ? item.songTitle
            : item.albumTitle}
        <button
          type="button"
          class="result-row"
          data-testid="search-result-item"
          aria-label={m.library_search_result_aria({
            kind: item.kind === 'album' ? labels.kindAlbum : labels.kindSong,
            name: resultName,
          })}
          onclick={() => onSelectResult(item)}
        >
          <div
            class="result-cover"
            use:lazyLoad={{ rootMargin: '180px', reducedMotion }}
            data-src={album?.coverUrl}
          >
            <div class="album-cover-placeholder" aria-hidden="true">
              {#if item.kind === 'album'}
                <Disc3 size={22} strokeWidth={1.6} />
              {:else}
                <Music2 size={22} strokeWidth={1.6} />
              {/if}
            </div>
            <img alt={item.albumTitle} />
          </div>

          <div class="result-main">
            <div class="result-heading">
              <span class="result-kind" data-kind={item.kind}>
                {item.kind === 'album' ? labels.kindAlbum : labels.kindSong}
              </span>
              <span class="result-title">{resultName}</span>
            </div>
            <div class="result-meta">
              {#if item.kind === 'song'}
                <span>{item.albumTitle}</span>
              {/if}
              {#if item.artistLine}
                <span>{item.artistLine}</span>
              {/if}
            </div>
            {#if item.matchedFields.length > 0}
              <div
                class="match-fields"
                aria-label={item.matchedFields.map(matchLabel).join(', ')}
              >
                {#each item.matchedFields as field (field)}
                  <span class="match-field">{matchLabel(field)}</span>
                {/each}
              </div>
            {/if}
          </div>

          <ChevronRight class="result-chevron" size={18} aria-hidden="true" />
        </button>
      {/each}
    </div>

    {#if hasMore}
      <div class="load-more-region">
        <Button
          variant="outline"
          size="sm"
          disabled={loadingMore}
          onclick={onLoadMore}
        >
          {#if loadingMore}
            <MotionSpinner className="load-more-spinner" {reducedMotion} />
            {labels.loadingMore}
          {:else}
            {labels.loadMore}
          {/if}
        </Button>
      </div>
    {/if}
  </section>
{/if}

<style>
  .search-skeleton,
  .results-list {
    display: flex;
    flex-direction: column;
  }

  .search-skeleton {
    gap: 1px;
    overflow: hidden;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }

  .skeleton-row {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 78px;
    padding: 11px 12px;
    border-bottom: 1px solid var(--border);
  }

  .skeleton-row:last-child {
    border-bottom: none;
  }

  .skeleton-row :global(.skeleton-cover) {
    width: 54px;
    height: 54px;
    flex: 0 0 54px;
    border-radius: var(--shape-md);
    background: var(--surface-state);
  }

  .skeleton-copy {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 9px;
  }

  .skeleton-copy :global(.skeleton-line) {
    height: 10px;
    border-radius: var(--shape-xs);
    background: var(--surface-state);
  }

  .skeleton-copy :global(.skeleton-line--title) {
    width: min(42%, 280px);
  }

  .skeleton-copy :global(.skeleton-line--meta) {
    width: min(28%, 180px);
  }

  .search-empty-state {
    display: flex;
    min-height: 320px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 48px 24px;
    color: var(--text-tertiary);
    text-align: center;
  }

  .search-empty-state :global(.index-spinner) {
    width: 28px;
    height: 28px;
  }

  .empty-title {
    max-width: 440px;
    color: var(--text-secondary);
    font-size: 14px;
    font-weight: 600;
  }

  .empty-hint {
    max-width: 440px;
    color: var(--text-tertiary);
    font-size: 12px;
  }

  .search-results {
    display: flex;
    flex-direction: column;
  }

  .results-header {
    display: flex;
    min-height: 30px;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 12px 10px;
  }

  .results-count {
    color: var(--text-secondary);
    font-family: var(--font-display);
    font-size: 12px;
    font-weight: 600;
  }

  .results-status {
    display: flex;
    min-height: 20px;
    align-items: center;
    gap: 6px;
    color: var(--text-tertiary);
    font-size: 11px;
  }

  .results-status :global(.results-spinner),
  .load-more-region :global(.load-more-spinner) {
    width: 14px;
    height: 14px;
  }

  .updating-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.1);
  }

  .updating-note {
    margin: 0 12px 10px;
    padding: 8px 10px;
    border-left: 2px solid rgba(var(--accent-rgb), 0.5);
    color: var(--text-secondary);
    background: rgba(var(--accent-rgb), 0.055);
    font-size: 11px;
  }

  .results-list {
    overflow: hidden;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
  }

  .result-row {
    appearance: none;
    display: grid;
    grid-template-columns: 54px minmax(0, 1fr) 20px;
    min-height: 78px;
    align-items: center;
    gap: 14px;
    width: 100%;
    padding: 11px 12px;
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    color: inherit;
    cursor: pointer;
    font: inherit;
    text-align: left;
    transition: var(--motion-hover);
  }

  .result-row:last-child {
    border-bottom: none;
  }

  .result-row:hover,
  .result-row:focus-visible {
    background: var(--hover-bg-elevated);
  }

  .result-row:focus-visible {
    position: relative;
    z-index: 1;
    outline: 2px solid rgba(var(--accent-rgb), 0.34);
    outline-offset: -2px;
  }

  .result-cover {
    position: relative;
    display: flex;
    width: 54px;
    height: 54px;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    border-radius: var(--shape-md);
    background: var(--surface-state);
    color: var(--text-tertiary);
  }

  .result-cover img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transform: scale(1.04);
  }

  .album-cover-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .result-main {
    min-width: 0;
  }

  .result-heading,
  .result-meta,
  .match-fields {
    display: flex;
    align-items: center;
  }

  .result-heading {
    gap: 8px;
  }

  .result-kind {
    flex: 0 0 auto;
    padding: 2px 5px;
    border-radius: var(--shape-xs);
    color: var(--accent);
    background: rgba(var(--accent-rgb), 0.09);
    font-family: var(--font-wide);
    font-size: 9px;
    font-weight: 700;
    line-height: 1.35;
  }

  .result-kind[data-kind='song'] {
    color: oklch(from var(--accent) l c calc(h - 28));
    background: oklch(from var(--accent) l c calc(h - 28) / 0.09);
  }

  .result-title {
    min-width: 0;
    overflow: hidden;
    color: var(--text-primary);
    font-size: 14px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .result-meta {
    gap: 7px;
    min-height: 18px;
    margin-top: 3px;
    overflow: hidden;
    color: var(--text-secondary);
    font-size: 12px;
    white-space: nowrap;
  }

  .result-meta span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .result-meta span + span::before {
    content: '·';
    margin-right: 7px;
    color: var(--text-tertiary);
  }

  .match-fields {
    gap: 5px;
    margin-top: 5px;
    overflow: hidden;
  }

  .match-field {
    flex: 0 0 auto;
    padding: 2px 5px;
    border: 1px solid var(--border);
    border-radius: var(--shape-xs);
    color: var(--text-tertiary);
    font-size: 9px;
    line-height: 1.2;
  }

  .result-row :global(.result-chevron) {
    color: var(--text-tertiary);
    opacity: 0.55;
  }

  .result-row:hover :global(.result-chevron),
  .result-row:focus-visible :global(.result-chevron) {
    color: var(--accent);
    opacity: 1;
  }

  .load-more-region {
    display: flex;
    justify-content: center;
    padding: 20px 0 4px;
  }

  @media (max-width: 720px) {
    .result-row {
      grid-template-columns: 48px minmax(0, 1fr) 18px;
      gap: 11px;
      min-height: 70px;
      padding-right: 6px;
      padding-left: 6px;
    }

    .result-cover {
      width: 48px;
      height: 48px;
    }

    .results-header,
    .updating-note {
      margin-right: 6px;
      margin-left: 6px;
    }
  }
</style>
