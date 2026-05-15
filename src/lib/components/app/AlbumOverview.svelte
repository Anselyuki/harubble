<script lang="ts">
  import AlbumCard from '$lib/components/AlbumCard.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import type {
    Album,
    LibraryIndexState,
    SearchLibraryResponse,
    SearchLibraryResultItem,
  } from '$lib/types';

  interface Props {
    albums: Album[];
    selectedAlbumCid: string | null;
    reducedMotion: boolean;
    searchQuery: string;
    searchLoading: boolean;
    searchResponse: SearchLibraryResponse | null;
    onSelectAlbum: (album: Album) => void;
    onSelectSearchResult: (item: SearchLibraryResultItem) => void;
  }

  let {
    albums,
    selectedAlbumCid,
    reducedMotion,
    searchQuery,
    searchLoading,
    searchResponse,
    onSelectAlbum,
    onSelectSearchResult,
  }: Props = $props();

  const trimmedSearchQuery = $derived.by(() => searchQuery.trim());
  const isSearchMode = $derived.by(() => trimmedSearchQuery.length > 0);
  const searchIndexState = $derived.by<LibraryIndexState>(
    () => searchResponse?.indexState ?? 'notReady'
  );
  const isSearchIndexBuilding = $derived.by(
    () => isSearchMode && !searchLoading && searchIndexState === 'building'
  );

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      indexBuildingTitle: m.library_search_index_building_title(),
      indexBuildingAria: m.library_search_index_building_aria(),
      indexBuildingValuetext: m.library_search_index_building_valuetext(),
      indexBuildingHint: m.library_search_index_building_hint(),
      resultKindAlbum: m.library_search_result_kind_album(),
      resultKindSong: m.library_search_result_kind_song(),
      noResults: m.library_search_no_results(),
    };
  });

  const searchStatusMessage = $derived.by(() => {
    if (!isSearchMode) return '';
    void localeState.current;
    if (searchLoading) return m.library_search_searching();
    switch (searchIndexState) {
      case 'stale':
        return m.library_search_index_stale();
      case 'notReady':
        return m.library_search_index_not_ready();
      default:
        return '';
    }
  });
</script>

<div class="album-overview">
  {#if isSearchMode}
    <!-- 搜索模式：展示搜索结果 -->
    <div class="overview-scroll-area">
      {#if isSearchIndexBuilding}
        <div class="search-status-card" aria-live="polite">
          <div class="search-status-title">{labels.indexBuildingTitle}</div>
          <div
            class="search-status-progress"
            role="progressbar"
            aria-label={labels.indexBuildingAria}
            aria-valuetext={labels.indexBuildingValuetext}
          >
            <div
              class={`search-status-progress-bar${reducedMotion ? ' is-reduced-motion' : ''}`}
            ></div>
          </div>
          <div class="search-status-hint">{labels.indexBuildingHint}</div>
        </div>
      {:else if searchStatusMessage}
        <div class="overview-empty">
          <div class="overview-empty-text">{searchStatusMessage}</div>
        </div>
      {:else if searchResponse && searchResponse.items.length > 0}
        <div class="search-results">
          {#each searchResponse.items as item, index (`${item.kind}:${item.albumCid}:${item.songCid ?? 'album'}:${index}`)}
            <button
              type="button"
              class={`search-result-card${selectedAlbumCid === item.albumCid ? ' is-selected' : ''}`}
              aria-label={m.library_search_result_aria({
                kind:
                  item.kind === 'album'
                    ? labels.resultKindAlbum
                    : labels.resultKindSong,
                name:
                  item.kind === 'song' && item.songTitle
                    ? item.songTitle
                    : item.albumTitle,
              })}
              onclick={() => onSelectSearchResult(item)}
            >
              <div class="search-result-kind-badge">
                {item.kind === 'album'
                  ? labels.resultKindAlbum
                  : labels.resultKindSong}
              </div>
              <div class="search-result-info">
                <div class="search-result-title">
                  {item.kind === 'song' && item.songTitle
                    ? item.songTitle
                    : item.albumTitle}
                </div>
                <div class="search-result-subtitle">
                  {#if item.kind === 'song'}
                    <span>{item.albumTitle}</span>
                  {/if}
                  {#if item.artistLine}
                    <span>{item.artistLine}</span>
                  {/if}
                </div>
              </div>
            </button>
          {/each}
        </div>
      {:else}
        <div class="overview-empty">
          <div class="overview-empty-text">{labels.noResults}</div>
        </div>
      {/if}
    </div>
  {:else}
    <!-- 浏览模式：全量专辑卡片网格 -->
    <div class="overview-scroll-area">
      {#if albums.length === 0}
        <div class="overview-empty">
          <div class="overview-empty-icon">♪</div>
          <div class="overview-empty-text">暂无专辑</div>
        </div>
      {:else}
        <div class="overview-section">
          <h2 class="overview-section-title">全部专辑 ({albums.length})</h2>
          <div class="album-grid">
            {#each albums as album (album.cid)}
              <AlbumCard
                {album}
                layout="grid"
                selected={selectedAlbumCid === album.cid}
                {reducedMotion}
                onclick={() => onSelectAlbum(album)}
              />
            {/each}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .album-overview {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--surface-workspace);
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

  /* ── Search results ── */

  .search-results {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 16px 24px 32px;
  }

  .search-result-card {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 10px 12px;
    border: none;
    border-radius: 10px;
    background: none;
    color: var(--text-primary);
    font-family: var(--font-body);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    transition:
      background-color 0.15s ease,
      box-shadow 0.15s ease;
  }

  .search-result-card:hover {
    background: var(--hover-bg-elevated);
  }

  .search-result-card.is-selected {
    background: rgba(var(--accent-rgb), 0.1);
  }

  .search-result-kind-badge {
    flex-shrink: 0;
    padding: 2px 8px;
    border-radius: 6px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--accent);
    background: rgba(var(--accent-rgb), 0.1);
  }

  .search-result-info {
    flex: 1;
    min-width: 0;
  }

  .search-result-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .search-result-subtitle {
    display: flex;
    gap: 8px;
    margin-top: 2px;
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ── Search index building status ── */

  .search-status-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 24px;
    padding: 16px;
    border-radius: 12px;
    background: rgba(var(--accent-rgb), 0.06);
    border: 1px solid rgba(var(--accent-rgb), 0.1);
  }

  .search-status-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .search-status-progress {
    height: 4px;
    border-radius: 2px;
    background: rgba(var(--accent-rgb), 0.12);
    overflow: hidden;
  }

  .search-status-progress-bar {
    height: 100%;
    width: 40%;
    border-radius: 2px;
    background: var(--accent);
    animation: index-progress-slide 1.6s ease-in-out infinite;
  }

  .search-status-progress-bar.is-reduced-motion {
    animation: none;
    width: 100%;
    opacity: 0.5;
  }

  .search-status-hint {
    font-size: 12px;
    color: var(--text-secondary);
  }

  @keyframes index-progress-slide {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(350%);
    }
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
