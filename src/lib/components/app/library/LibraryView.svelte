<script lang="ts">
  import AlbumCard from '$lib/components/AlbumCard.svelte';
  import AlbumWorkspace from '$lib/components/app/album/AlbumWorkspace.svelte';
  import AlbumWorkspaceContent from '$lib/components/app/album/AlbumWorkspaceContent.svelte';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import type { AppRuntime } from '$lib/features/shell/appRuntime.svelte';

  interface Props {
    runtime: AppRuntime;
  }

  let { runtime }: Props = $props();

  const trimmedSearchQuery = $derived.by(() =>
    runtime.librarySearchQuery.trim()
  );
  const isSearchMode = $derived.by(() => trimmedSearchQuery.length > 0);

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      loadingAlbums: m.library_loading_albums(),
      loadFailed: m.library_load_failed(),
      noResults: m.library_search_no_results(),
      resultKindAlbum: m.library_search_result_kind_album(),
      resultKindSong: m.library_search_result_kind_song(),
      indexBuildingTitle: m.library_search_index_building_title(),
      indexBuildingAria: m.library_search_index_building_aria(),
      indexBuildingValuetext: m.library_search_index_building_valuetext(),
      indexBuildingHint: m.library_search_index_building_hint(),
    };
  });

  const searchIndexState = $derived.by(
    () => runtime.librarySearchResponse?.indexState ?? 'notReady'
  );
  const isSearchIndexBuilding = $derived.by(
    () =>
      isSearchMode &&
      !runtime.librarySearchLoading &&
      searchIndexState === 'building'
  );
  const searchStatusMessage = $derived.by(() => {
    if (!isSearchMode) return '';
    void localeState.current;
    if (runtime.librarySearchLoading) return m.library_search_searching();
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

{#if runtime.selectedAlbum || (runtime.loadingDetail && runtime.showDetailSkeleton)}
  <AlbumWorkspace
    currentSong={runtime.currentSong}
    loadingDetail={runtime.loadingDetail}
    selectedAlbum={runtime.selectedAlbum}
  >
    <AlbumWorkspaceContent
      loadingDetail={runtime.loadingDetail}
      showDetailSkeleton={runtime.showDetailSkeleton}
      albumRequestSeq={runtime.albumRequestSeq}
      selectedAlbum={runtime.selectedAlbum}
      selectedAlbumArtworkUrl={runtime.selectedAlbumArtworkUrl}
      currentSongCid={runtime.currentSong?.cid ?? null}
      isPlaybackActive={runtime.isPlaying || runtime.isPaused}
      isPlaybackPaused={runtime.isPaused}
      downloadingAlbumCid={runtime.downloadingAlbumCid}
      selectionModeEnabled={runtime.selectionModeEnabled}
      selectedSongCids={runtime.selectedSongCids}
      reducedMotion={runtime.prefersReducedMotion}
      overlayScrollbarOptions={runtime.overlayScrollbarOptions}
      contentScrollbarEvents={runtime.contentScrollbarEvents}
      onContentWheel={runtime.handleContentWheel}
      albumStageStyle={runtime.albumStageStyle}
      albumStageMediaHeight={runtime.albumStageMediaHeight}
      albumStageScrimOpacity={runtime.albumStageScrimOpacity}
      albumStageImageOpacity={runtime.albumStageImageOpacity}
      albumStageImageTransform={runtime.albumStageImageTransform}
      albumStageSolidifyOpacity={runtime.albumStageSolidifyOpacity}
      bind:albumStageElement={runtime.albumStageElement}
      onToggleSelectionMode={runtime.toggleSelectionMode}
      onSelectAllSongs={runtime.selectAllSongs}
      onDeselectAllSongs={runtime.deselectAllSongs}
      onInvertSongSelection={runtime.invertSongSelection}
      onDownloadAlbum={runtime.downloadController.handleAlbumDownload}
      onDownloadSelection={runtime.handleDownloadSelection}
      onPlaySong={runtime.handlePlay}
      onTogglePlay={runtime.isPlaying
        ? runtime.playerController.pause
        : runtime.playerController.resume}
      onDownloadSong={runtime.downloadController.handleSongDownload}
      onToggleSongSelection={runtime.toggleSongSelection}
      isSongSelected={runtime.isSongSelected}
      getSongDownloadState={runtime.downloadController.getSongDownloadState}
      isSongDownloadInteractionBlocked={runtime.downloadController
        .isSongDownloadInteractionBlocked}
      hasAlbumDownloadJob={runtime.hasAlbumDownloadJob}
      isSelectionDownloadDisabled={runtime.downloadController
        .isSelectionDownloadActionDisabled}
      isCurrentSelectionCreating={runtime.downloadController
        .isCurrentSelectionCreating}
      hasCurrentSelectionJob={runtime.hasCurrentSelectionJob}
      collections={runtime.collectionController.collections}
      onAddToCollection={(colId, songCid) =>
        runtime.collectionController.handleAddSongs(colId, [songCid])}
      onBack={runtime.canGoBack
        ? () => runtime.goBack()
        : runtime.handleDeselectAlbum}
    />
  </AlbumWorkspace>
{:else if isSearchMode}
  <main class="content library-browse">
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
            class={`search-status-progress-bar${runtime.prefersReducedMotion ? ' is-reduced-motion' : ''}`}
          ></div>
        </div>
        <div class="search-status-hint">{labels.indexBuildingHint}</div>
      </div>
    {:else if searchStatusMessage}
      <div class="library-empty-state">{searchStatusMessage}</div>
    {:else if runtime.librarySearchResponse && runtime.librarySearchResponse.items.length > 0}
      <div class="library-results-grid">
        {#each runtime.librarySearchResponse.items as item, index (`${item.kind}:${item.albumCid}:${item.songCid ?? 'album'}:${index}`)}
          <button
            type="button"
            class="search-result"
            class:is-selected={runtime.selectedAlbumCid === item.albumCid}
            onclick={() => runtime.handleSelectSearchResult(item)}
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
      <div class="library-empty-state">{labels.noResults}</div>
    {/if}
  </main>
{:else}
  <main class="content library-browse">
    {#if runtime.loadingAlbums}
      <div class="library-loading">
        <span>{labels.loadingAlbums}</span>
        <MotionSpinner
          className="inline-loading-spinner"
          reducedMotion={runtime.prefersReducedMotion}
        />
      </div>
    {:else if runtime.errorMsg && runtime.albums.length === 0}
      <div class="library-empty-state">
        <div>⚠️</div>
        <div>{labels.loadFailed}</div>
        <div class="library-error-detail">{runtime.errorMsg}</div>
      </div>
    {:else}
      <div class="library-album-grid">
        {#each runtime.albums as album (album.cid)}
          <AlbumCard
            {album}
            selected={runtime.selectedAlbumCid === album.cid}
            reducedMotion={runtime.prefersReducedMotion}
            onclick={() => runtime.handleSelectAlbum(album)}
          />
        {/each}
      </div>
    {/if}
  </main>
{/if}

<style>
  .library-browse {
    padding: 24px 28px;
    overflow-y: auto;
  }

  .library-album-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 16px;
  }

  .library-results-grid {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .library-loading {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 24px;
    color: var(--text-secondary);
    font-size: 14px;
  }

  .library-empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 48px 24px;
    color: var(--text-secondary);
    font-size: 14px;
    text-align: center;
  }

  .library-error-detail {
    margin-top: 8px;
    font-size: 12px;
    color: var(--text-tertiary);
  }

  .search-result {
    width: 100%;
    display: grid;
    gap: 4px;
    padding: 12px 16px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--hover-bg);
    text-align: left;
    cursor: pointer;
  }

  .search-result:hover,
  .search-result.is-selected {
    background: var(--surface-state);
    border-color: rgba(var(--accent-rgb), 0.22);
  }

  .search-result-kind {
    font-size: 11px;
    color: var(--text-tertiary);
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

  .search-status-card {
    display: grid;
    gap: 10px;
    padding: 16px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--hover-bg);
    max-width: 400px;
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
    height: 6px;
    border-radius: 999px;
    background: var(--hover-bg-elevated);
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
</style>
