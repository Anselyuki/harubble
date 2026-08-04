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

  const LIBRARY_COVER_PRELOAD_MARGIN = '0px 0px 900px 0px';

  let { runtime }: Props = $props();
  let libraryScrollRoot = $state<HTMLElement | null>(null);

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      loadingAlbums: m.library_loading_albums(),
      loadFailed: m.library_load_failed(),
    };
  });
</script>

{#if runtime.selectedAlbum || runtime.selectedAlbumCid || (runtime.loadingDetail && runtime.showDetailSkeleton)}
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
      isViewTransitioning={runtime.isViewTransitioning}
    />
  </AlbumWorkspace>
{:else}
  <div class="content library-browse" bind:this={libraryScrollRoot}>
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
            loading={runtime.loadingAlbumCid === album.cid}
            reducedMotion={runtime.prefersReducedMotion}
            coverPreloadRoot={libraryScrollRoot}
            coverPreloadMargin={LIBRARY_COVER_PRELOAD_MARGIN}
            onclick={() => runtime.handleSelectAlbum(album)}
          />
        {/each}
      </div>
    {/if}
  </div>
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
</style>
