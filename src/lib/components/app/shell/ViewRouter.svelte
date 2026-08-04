<script lang="ts">
  import HomeView from '$lib/components/app/home/HomeView.svelte';
  import LibraryView from '$lib/components/app/library/LibraryView.svelte';
  import TagEditorView from '$lib/components/app/tag-editor/TagEditorView.svelte';
  import CollectionDetailPanel from '$lib/components/app/collection/CollectionDetailPanel.svelte';
  import AlbumOverview from '$lib/components/app/album/AlbumOverview.svelte';
  import SearchView from '$lib/components/app/search/SearchView.svelte';
  import ViewTransition from '$lib/components/ViewTransition.svelte';
  import { createResolvedSongsStore } from '$lib/features/collection/resolvedSongs.svelte';
  import { getSongDetail, getAlbumDetail } from '$lib/api';
  import type { AppRuntime } from '$lib/features/shell/appRuntime.svelte';
  import { tick } from 'svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';

  interface Props {
    runtime: AppRuntime;
  }

  let { runtime }: Props = $props();

  const resolvedSongsStore = createResolvedSongsStore({
    getSongDetail,
    getAlbumDetail,
  });

  $effect(() => {
    resolvedSongsStore.resolve(runtime.collectionController.selectedCollection);
  });

  const viewTitle = $derived.by(() => {
    void localeState.current;
    switch (runtime.currentView) {
      case 'home':
        return m.shell_nav_home();
      case 'search':
        return m.shell_nav_search();
      case 'overview':
        return m.album_overview_all_title({ count: runtime.albums.length });
      case 'library':
        return runtime.selectedAlbum?.name ?? m.shell_nav_library();
      case 'collection':
        return (
          runtime.collectionController.selectedCollection?.name ??
          m.shell_nav_collections()
        );
      case 'tagEditor':
        return m.shell_nav_tags();
    }
  });

  let previousView: string | null = null;
  $effect(() => {
    const currentView = runtime.currentView;
    if (previousView === null) {
      previousView = currentView;
      return;
    }
    if (currentView === previousView) return;
    previousView = currentView;
    void tick().then(() => {
      const target =
        currentView === 'search'
          ? document.querySelector<HTMLElement>('[data-testid="search-input"]')
          : document.querySelector<HTMLElement>('[data-view-heading]');
      target?.focus({ preventScroll: true });
    });
  });
</script>

<ViewTransition
  viewKey={runtime.currentView}
  direction={runtime.navigationDirection}
  reducedMotion={runtime.prefersReducedMotion}
  onTransitionStart={runtime.handleTransitionStart}
  onTransitionEnd={runtime.handleTransitionEnd}
>
  <h1 class="sr-only" tabindex="-1" data-view-heading>{viewTitle}</h1>
  {#if runtime.currentView === 'home'}
    <HomeView {runtime} />
  {:else if runtime.currentView === 'search'}
    <SearchView
      runtime={{
        searchController: runtime.searchController,
        handleSelectAlbum: runtime.handleSelectAlbum,
        handleSelectSearchResult: runtime.handleSelectSearchResult,
        albums: runtime.albums,
        prefersReducedMotion: runtime.prefersReducedMotion,
        loadingAlbumCid: runtime.loadingAlbumCid,
      }}
    />
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
      resolvedSongs={resolvedSongsStore.resolvedSongs}
      isResolvingSongs={resolvedSongsStore.isResolvingSongs}
      playbackQueue={resolvedSongsStore.playbackQueue}
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
      loadingAlbumCid={runtime.loadingAlbumCid}
      reducedMotion={runtime.prefersReducedMotion}
      onSelectAlbum={runtime.handleSelectAlbum}
    />
  {:else if runtime.currentView === 'library'}
    <LibraryView {runtime} />
  {/if}
</ViewTransition>
