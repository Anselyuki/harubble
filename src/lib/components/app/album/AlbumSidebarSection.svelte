<script lang="ts">
  import AlbumCard from '$lib/components/AlbumCard.svelte';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';
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
    loadingAlbums?: boolean;
    errorMsg?: string;
    searchQuery?: string;
    searchLoading?: boolean;
    searchResponse?: SearchLibraryResponse | null;
    onSelect: (album: Album) => void;
    onSelectSearchResult: (item: SearchLibraryResultItem) => void;
  }

  let {
    albums,
    selectedAlbumCid,
    reducedMotion,
    loadingAlbums = false,
    errorMsg = '',
    searchQuery = '',
    searchLoading = false,
    searchResponse = null,
    onSelect,
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
      loadingAlbums: m.library_loading_albums(),
      loadFailed: m.library_load_failed(),
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

  let scrollAreaEl: HTMLElement | undefined = $state();
  let scrollTimer: ReturnType<typeof setTimeout> | undefined;

  function handleScroll() {
    scrollAreaEl?.classList.add('is-scrolling');
    clearTimeout(scrollTimer);
    scrollTimer = setTimeout(() => {
      scrollAreaEl?.classList.remove('is-scrolling');
    }, 1200);
  }
</script>

<div class="album-sidebar-section">
  <div
    class="sidebar-scroll-area"
    bind:this={scrollAreaEl}
    onscroll={handleScroll}
  >
    {#if loadingAlbums}
      <div class="loading">
        <span>{labels.loadingAlbums}</span><MotionSpinner
          className="inline-loading-spinner"
          {reducedMotion}
        />
      </div>
    {:else if errorMsg && albums.length === 0}
      <div class="empty-state">
        <div class="empty-icon">⚠️</div>
        <div class="empty-text">{labels.loadFailed}</div>
        <div class="empty-text" style="margin-top: 8px; font-size: 12px;">
          {errorMsg}
        </div>
      </div>
    {:else if isSearchMode}
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
        <div class="empty-state">
          <div class="empty-text">{searchStatusMessage}</div>
        </div>
      {:else if searchResponse && searchResponse.items.length > 0}
        <div class="album-list">
          {#each searchResponse.items as item, index (`${item.kind}:${item.albumCid}:${item.songCid ?? 'album'}:${index}`)}
            <button
              type="button"
              class={`search-result${selectedAlbumCid === item.albumCid ? ' is-selected' : ''}`}
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
              <div class="search-result-kind">
                {item.kind === 'album'
                  ? labels.resultKindAlbum
                  : labels.resultKindSong}
              </div>
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
            </button>
          {/each}
        </div>
      {:else}
        <div class="empty-state">
          <div class="empty-text">{labels.noResults}</div>
        </div>
      {/if}
    {:else}
      <div class="album-list">
        {#each albums as album (album.cid)}
          <AlbumCard
            {album}
            selected={selectedAlbumCid === album.cid}
            {reducedMotion}
            onclick={() => onSelect(album)}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .album-sidebar-section {
    display: flex;
    flex-direction: column;
    width: 100%;
    min-width: 0;
    height: 100%;
  }

  .sidebar-scroll-area {
    flex: 1;
    width: 100%;
    min-width: 0;
    overflow-y: auto;
    min-height: 0;
    scrollbar-width: thin;
    scrollbar-color: transparent transparent;
    padding-right: 8px;
  }

  .sidebar-scroll-area:hover,
  .sidebar-scroll-area.is-scrolling {
    scrollbar-color: rgba(255, 255, 255, 0.28) transparent;
  }

  .sidebar-scroll-area::-webkit-scrollbar {
    width: 4px;
  }

  .sidebar-scroll-area::-webkit-scrollbar-track {
    background: transparent;
    margin: 4px 0;
  }

  .sidebar-scroll-area::-webkit-scrollbar-thumb {
    border-radius: 4px;
    background: rgba(255, 255, 255, 0);
    transition: background 0.3s ease;
  }

  .sidebar-scroll-area:hover::-webkit-scrollbar-thumb,
  .sidebar-scroll-area.is-scrolling::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.28);
  }

  .sidebar-scroll-area::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.42);
  }

  .album-sidebar-section :global(.album-list) {
    width: 100%;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }

  .album-sidebar-section :global(.album-card) {
    width: 100%;
    min-width: 0;
  }

  .search-status-card {
    display: grid;
    gap: 10px;
    padding: 16px 14px;
    border-radius: 20px;
    border: 1px solid rgba(255, 255, 255, 0.22);
    background: rgba(255, 255, 255, 0.16);
  }

  .search-status-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .search-status-hint {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .search-status-progress {
    position: relative;
    overflow: hidden;
    height: 8px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.16);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.12);
  }

  .search-status-progress-bar {
    position: absolute;
    inset: 0;
    width: 42%;
    border-radius: inherit;
    background: linear-gradient(
      90deg,
      rgba(var(--accent-rgb), 0.28) 0%,
      rgba(var(--accent-rgb), 0.9) 45%,
      rgba(var(--accent-rgb), 0.32) 100%
    );
    animation: search-progress-slide 1.2s ease-in-out infinite;
  }

  .search-status-progress-bar.is-reduced-motion {
    width: 100%;
    opacity: 0.72;
    animation: none;
  }

  @keyframes search-progress-slide {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(240%);
    }
  }

  .search-result {
    width: 100%;
    display: grid;
    gap: 4px;
    padding: 12px 14px;
    border-radius: 18px;
    border: 1px solid rgba(255, 255, 255, 0.28);
    background: rgba(255, 255, 255, 0.22);
    text-align: left;
  }

  .search-result:not(.is-selected) {
    transition:
      background-color 0.16s ease,
      border-color 0.16s ease;
  }

  .search-result:hover:not(.is-selected),
  .search-result.is-selected {
    background: rgba(var(--accent-rgb), 0.1);
    border-color: rgba(var(--accent-rgb), 0.22);
  }

  .search-result.is-selected {
    transition: none;
  }

  .search-result-kind {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .search-result-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .search-result-subtitle {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 12px;
    color: var(--text-secondary);
  }
</style>
