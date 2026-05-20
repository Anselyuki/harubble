<script lang="ts">
  import HomeView from '$lib/components/app/home/HomeView.svelte';
  import LibraryView from '$lib/components/app/library/LibraryView.svelte';
  import TagEditorView from '$lib/components/app/tag-editor/TagEditorView.svelte';
  import CollectionDetailPanel from '$lib/components/app/collection/CollectionDetailPanel.svelte';
  import AlbumOverview from '$lib/components/app/album/AlbumOverview.svelte';
  import type { AppRuntime } from '$lib/features/shell/appRuntime.svelte';

  interface Props {
    runtime: AppRuntime;
  }

  let { runtime }: Props = $props();
</script>

{#if runtime.currentView === 'home'}
  <HomeView {runtime} />
{:else if runtime.currentView === 'tagEditor'}
  <TagEditorView {runtime} />
{:else if runtime.currentView === 'collection'}
  <CollectionDetailPanel
    collection={runtime.collectionController.selectedCollection}
    isLoading={runtime.collectionController.isDetailLoading}
    reducedMotion={runtime.prefersReducedMotion}
    currentSongCid={runtime.currentSong?.cid ?? null}
    isPlaybackActive={runtime.isPlaying || runtime.isPaused}
    isPlaybackPaused={runtime.isPaused}
    onEdit={runtime.collectionController.openEditDialog}
    onDelete={runtime.collectionController.handleDelete}
    onExport={runtime.collectionController.handleExport}
    onRemoveSongs={runtime.collectionController.handleRemoveSongs}
    onReorderSongs={runtime.collectionController.handleReorderSongs}
    onPlaySong={runtime.handlePlayCollectionSong}
    onTogglePlay={runtime.isPlaying
      ? runtime.playerController.pause
      : runtime.playerController.resume}
    onDownloadSong={runtime.downloadController.handleSongDownload}
    getSongDownloadState={runtime.downloadController.getSongDownloadState}
    isSongDownloadInteractionBlocked={runtime.downloadController
      .isSongDownloadInteractionBlocked}
    collections={runtime.collectionController.collections}
    onAddToCollection={(colId, songCid) =>
      runtime.collectionController.handleAddSongs(colId, [songCid])}
  />
{:else if runtime.currentView === 'overview'}
  <AlbumOverview
    albums={runtime.albums}
    selectedAlbumCid={runtime.selectedAlbumCid}
    reducedMotion={runtime.prefersReducedMotion}
    searchQuery={runtime.librarySearchQuery}
    searchLoading={runtime.librarySearchLoading}
    searchResponse={runtime.librarySearchResponse}
    onSelectAlbum={runtime.handleSelectAlbum}
    onSelectSearchResult={runtime.handleSelectSearchResult}
  />
{:else}
  <LibraryView {runtime} />
{/if}
