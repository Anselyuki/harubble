<script lang="ts">
  import { createAppRuntime } from '$lib/features/shell/appRuntime.svelte';
  import TopToolbar from '$lib/components/app/TopToolbar.svelte';
  import StatusToastHost from '$lib/components/app/StatusToastHost.svelte';
  import AppSidebar from '$lib/components/app/AppSidebar.svelte';
  import LibraryView from '$lib/components/app/LibraryView.svelte';
  import PlayerFlyoutStack from '$lib/components/app/PlayerFlyoutStack.svelte';
  import FullscreenPlayer from '$lib/components/app/FullscreenPlayer.svelte';
  import AppSideSheets from '$lib/components/app/AppSideSheets.svelte';
  import HomeView from '$lib/components/app/HomeView.svelte';
  import TagEditorView from '$lib/components/app/TagEditorView.svelte';
  import CollectionDetailPanel from '$lib/components/app/CollectionDetailPanel.svelte';
  import CollectionFormDialog from '$lib/components/app/CollectionFormDialog.svelte';
  import AlbumOverview from '$lib/components/app/AlbumOverview.svelte';
  import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';

  import {
    createSidebarAnimator,
    type SidebarAnimator,
  } from '$lib/design/sidebar-animator';

  const runtime = createAppRuntime();

  let animator: SidebarAnimator | null = null;
  let logoCharEls: HTMLSpanElement[] = $state([]);
  let shellEl: HTMLElement | null = $state(null);
  let sidebarEl: HTMLElement | null = $state(null);
  let navRegionEl: HTMLElement | null = $state(null);
  let collectionsRegionEl: HTMLElement | null = $state(null);
  let collectionsCollapsedEl: HTMLElement | null = $state(null);
  let bottomLabelEl: HTMLSpanElement | null = $state(null);
  let logoContainerEl: HTMLDivElement | null = $state(null);

  let contentCollapsed = $state(runtime.sidebarCollapsed);
  let contentInteractive = $state(!runtime.sidebarCollapsed);
  let layoutCollapsed = $state(runtime.sidebarCollapsed);

  function handleCharsReady(els: HTMLSpanElement[]) {
    logoCharEls = els;
  }

  function onContentInteractive(interactive: boolean) {
    contentInteractive = interactive;
  }
  function onContentSwitch(collapsed: boolean) {
    contentCollapsed = collapsed;
  }
  function onLayoutSwitch(collapsed: boolean) {
    layoutCollapsed = collapsed;
  }

  /* eslint-disable @typescript-eslint/no-unnecessary-condition -- $state(null) refs are populated by bind:this at runtime */
  $effect(() => {
    if (
      shellEl &&
      sidebarEl &&
      logoContainerEl &&
      bottomLabelEl &&
      navRegionEl &&
      collectionsRegionEl &&
      collectionsCollapsedEl &&
      logoCharEls.length === 12
    ) {
      if (animator) return;
      animator = createSidebarAnimator({
        shellEl,
        sidebarEl,
        logoCharEls,
        logoContainerEl,
        navRegionEl,
        collectionsRegionEl,
        collectionsCollapsedEl,
        bottomLabelEl,
        initialCollapsed: runtime.sidebarCollapsed,
        onContentInteractive,
        onContentSwitch,
        onLayoutSwitch,
      });
    }
  });
  /* eslint-enable @typescript-eslint/no-unnecessary-condition */

  let prevCollapsed: boolean | null = null;
  $effect(() => {
    const curr = runtime.sidebarCollapsed;
    if (!animator) return;
    if (prevCollapsed === null) {
      prevCollapsed = curr;
      return;
    }
    if (curr === prevCollapsed) return;
    prevCollapsed = curr;
    if (curr) {
      animator.collapse();
    } else {
      animator.expand();
    }
  });

  $effect(() => {
    return () => {
      animator?.dispose();
      animator = null;
    };
  });
</script>

{#if runtime.isMacOS}
  <div
    class="macos-window-drag-region"
    data-tauri-drag-region
    aria-hidden="true"
  ></div>
{/if}

<StatusToastHost />

<div
  class="app-shell"
  class:macos-overlay={runtime.isMacOS}
  bind:this={shellEl}
>
  <AppSidebar
    isMacOS={runtime.isMacOS}
    currentView={runtime.currentView}
    {contentCollapsed}
    {contentInteractive}
    {layoutCollapsed}
    onNavigate={(view) => {
      runtime.shellStore.currentView = view;
    }}
    collections={runtime.collectionController.collections}
    selectedCollectionId={runtime.collectionController.selectedCollectionId}
    isCollectionsLoading={runtime.collectionController.isLoading}
    onSelectCollection={runtime.collectionController.selectCollection}
    onCreateCollection={runtime.collectionController.openCreateDialog}
    onRequestExpand={runtime.toggleSidebar}
    bind:sidebarEl
    bind:navRegionEl
    bind:collectionsRegionEl
    bind:collectionsCollapsedEl
    bind:bottomLabelEl
    bind:logoContainerEl
    onCharsReady={handleCharsReady}
  />

  <button
    type="button"
    class="sidebar-toggle-btn"
    onclick={runtime.toggleSidebar}
    aria-label={runtime.sidebarCollapsed
      ? 'Expand sidebar'
      : 'Collapse sidebar'}
  >
    {#if runtime.sidebarCollapsed}
      <ChevronRightIcon size={14} />
    {:else}
      <ChevronLeftIcon size={14} />
    {/if}
  </button>

  <section class="main-region">
    {#if runtime.isMacOS}
      <div
        class="main-drag-region"
        data-tauri-drag-region
        aria-hidden="true"
      ></div>
    {/if}

    <TopToolbar
      activeDownloadCount={runtime.activeDownloadCount}
      isRefreshing={runtime.isRefreshing}
      settingsOpen={runtime.settingsOpen}
      downloadPanelOpen={runtime.downloadPanelOpen}
      searchQuery={runtime.librarySearchQuery}
      searchScope={runtime.librarySearchScope}
      currentView={runtime.currentView}
      onRefresh={runtime.handleRefresh}
      onOpenDownloads={runtime.handleToggleDownloads}
      onOpenSettings={runtime.handleToggleSettings}
      onSearchQueryChange={runtime.libraryController.setSearchQuery}
      onSearchScopeChange={runtime.libraryController.setSearchScope}
      onNavigate={(view) => {
        runtime.shellStore.currentView = view;
      }}
    />

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

    <PlayerFlyoutStack
      song={runtime.currentSong}
      isPlaying={runtime.isPlaying}
      isPaused={runtime.isPaused}
      hasPrevious={runtime.playerHasPrevious}
      hasNext={runtime.playerHasNext}
      progress={runtime.progress}
      duration={runtime.duration}
      isLoading={runtime.isLoading}
      reducedMotion={runtime.prefersReducedMotion}
      isShuffled={runtime.shuffleEnabled}
      repeatMode={runtime.repeatMode}
      lyricsOpen={runtime.lyricsOpen}
      playlistOpen={runtime.playlistOpen}
      lyricsLoading={runtime.lyricsLoading}
      lyricsError={runtime.lyricsError}
      lyricsLines={runtime.lyricsLines}
      lyricsUnavailable={runtime.lyricsUnavailable}
      activeLyricIndex={runtime.activeLyricIndex}
      playbackOrder={runtime.playbackOrder}
      downloadState={runtime.currentSongDownloadState}
      downloadDisabled={runtime.currentSongDownloadDisabled}
      onPrevious={runtime.playerController.playPrevious}
      onTogglePlay={runtime.isPlaying
        ? runtime.playerController.pause
        : runtime.playerController.resume}
      onSeek={runtime.playerController.seek}
      onNext={runtime.playerController.playNext}
      onShuffleChange={runtime.playerController.toggleShuffle}
      onRepeatModeChange={runtime.playerController.toggleRepeat}
      onToggleLyrics={runtime.playerController.toggleLyrics}
      onTogglePlaylist={runtime.playerController.togglePlaylist}
      onToggleFullscreen={runtime.playerController.toggleFullscreen}
      onDownload={runtime.handleCurrentSongDownload}
      onPlayQueueEntry={runtime.playerController.playQueueEntry}
    />

    {#if runtime.fullscreenOpen && runtime.currentSong}
      <FullscreenPlayer
        song={runtime.currentSong}
        isPlaying={runtime.isPlaying}
        isPaused={runtime.isPaused}
        isLoading={runtime.isLoading}
        hasPrevious={runtime.playerHasPrevious}
        hasNext={runtime.playerHasNext}
        progress={runtime.progress}
        duration={runtime.duration}
        isShuffled={runtime.shuffleEnabled}
        repeatMode={runtime.repeatMode}
        lyricsLoading={runtime.lyricsLoading}
        lyricsError={runtime.lyricsError}
        lyricsLines={runtime.lyricsLines}
        activeLyricIndex={runtime.activeLyricIndex}
        reducedMotion={runtime.prefersReducedMotion}
        onPrevious={runtime.playerController.playPrevious}
        onTogglePlay={runtime.isPlaying
          ? runtime.playerController.pause
          : runtime.playerController.resume}
        onSeek={runtime.playerController.seek}
        onNext={runtime.playerController.playNext}
        onShuffleChange={runtime.playerController.toggleShuffle}
        onRepeatModeChange={runtime.playerController.toggleRepeat}
        onDownload={runtime.handleCurrentSongDownload}
        downloadState={runtime.currentSongDownloadState}
        downloadDisabled={runtime.currentSongDownloadDisabled}
        onClose={runtime.playerController.toggleFullscreen}
      />
    {/if}

    <AppSideSheets
      SettingsSheetView={runtime.SettingsSheetView}
      DownloadTasksSheetView={runtime.DownloadTasksSheetView}
      bind:settingsOpen={runtime.shellStore.settingsOpen}
      bind:downloadPanelOpen={runtime.shellStore.downloadPanelOpen}
      bind:format={runtime.settingsState.format}
      bind:outputDir={runtime.settingsState.outputDir}
      bind:downloadLyrics={runtime.settingsState.downloadLyrics}
      bind:notifyOnDownloadComplete={
        runtime.settingsState.notifyOnDownloadComplete
      }
      bind:notifyOnPlaybackChange={runtime.settingsState.notifyOnPlaybackChange}
      bind:logLevel={runtime.settingsState.logLevel}
      bind:locale={runtime.settingsState.locale}
      settingsLogRefreshToken={runtime.settingsState.settingsLogRefreshToken}
      notifyInfo={runtime.notifyInfo}
      notifyError={runtime.notifyError}
      onOutputDirChange={runtime.handleOutputDirChange}
      jobs={runtime.filteredDownloadJobs}
      hasDownloadHistory={runtime.hasDownloadHistory}
      bind:searchQuery={runtime.downloadController.searchQuery}
      bind:scopeFilter={runtime.downloadController.scopeFilter}
      bind:statusFilter={runtime.downloadController.statusFilter}
      bind:kindFilter={runtime.downloadController.kindFilter}
      canClearDownloadHistory={runtime.downloadController
        .canClearDownloadHistory}
      getJobProgress={runtime.downloadController.getJobProgress}
      getJobProgressText={runtime.downloadController.getJobProgressText}
      getJobStatusLabel={runtime.downloadController.getJobStatusLabel}
      getJobKindLabel={runtime.downloadController.getJobKindLabel}
      getJobSummaryLabel={runtime.downloadController.getJobSummaryLabel}
      getJobDisplayTitle={runtime.downloadController.getJobDisplayTitle}
      getJobErrorSummary={runtime.downloadController.getJobErrorSummary}
      isJobActive={runtime.downloadController.isJobActive}
      canCancelTask={runtime.downloadController.canCancelTask}
      canRetryTask={runtime.downloadController.canRetryTask}
      getTaskErrorLabel={runtime.downloadController.getTaskErrorLabel}
      getTaskStatusLabel={runtime.downloadController.getTaskStatusLabel}
      onClearDownloadHistory={runtime.downloadController
        .handleClearDownloadHistory}
      onCancelDownloadJob={runtime.downloadController.handleCancelDownloadJob}
      onRetryDownloadJob={runtime.downloadController.handleRetryDownloadJob}
      onCancelDownloadTask={runtime.downloadController.handleCancelDownloadTask}
      onRetryDownloadTask={runtime.downloadController.handleRetryDownloadTask}
    />
  </section>
</div>

<CollectionFormDialog
  bind:open={runtime.collectionController.formDialogOpen}
  mode={runtime.collectionController.formDialogMode}
  initialName={runtime.collectionController.selectedCollection?.name ?? ''}
  initialDescription={runtime.collectionController.selectedCollection
    ?.description ?? ''}
  onSubmit={(name, description) => {
    if (runtime.collectionController.formDialogMode === 'create') {
      return runtime.collectionController.handleCreate(name, description);
    }
    const id = runtime.collectionController.selectedCollectionId;
    if (id) {
      return runtime.collectionController.handleUpdate(id, name, description);
    }
  }}
  onClose={runtime.collectionController.closeFormDialog}
/>
