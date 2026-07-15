<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { AppRuntime } from '$lib/features/shell/appRuntime.svelte';
  import {
    setShellContext,
    setPlayerContext,
    setDownloadContext,
    setLibraryContext,
    setCollectionContext,
  } from '$lib/contexts';

  interface Props {
    runtime: AppRuntime;
    children: Snippet;
  }

  let { runtime, children }: Props = $props();

  // Wire runtime properties into typed contexts using getters for reactivity.
  // Plain property assignment would snapshot the value at call time; getters
  // forward reads to the live reactive source on every access.

  setShellContext({
    get currentView() {
      return runtime.shellStore.currentView;
    },
    get isMacOS() {
      return runtime.isMacOS;
    },
    get prefersReducedMotion() {
      return runtime.prefersReducedMotion;
    },
    get settingsOpen() {
      return runtime.settingsOpen;
    },
    get downloadPanelOpen() {
      return runtime.downloadPanelOpen;
    },
    get isRefreshing() {
      return runtime.isRefreshing;
    },
    get overlayScrollbarOptions() {
      return runtime.overlayScrollbarOptions;
    },
    get sidebarCollapsed() {
      return runtime.sidebarCollapsed;
    },
    toggleSidebar: () => runtime.toggleSidebar(),
    handleRefresh: () => runtime.handleRefresh(),
    handleToggleSettings: () => runtime.handleToggleSettings(),
    handleToggleDownloads: () => runtime.handleToggleDownloads(),
    notifyInfo: (message) => runtime.notifyInfo(message),
    notifyError: (message) => runtime.notifyError(message),
    navigate: (view) => {
      runtime.navigateToTop(view);
    },
  });

  setPlayerContext({
    get currentSong() {
      return runtime.currentSong;
    },
    get isPlaying() {
      return runtime.isPlaying;
    },
    get isPaused() {
      return runtime.isPaused;
    },
    get isLoading() {
      return runtime.isLoading;
    },
    get isPlayTogglePending() {
      return runtime.isPlayTogglePending;
    },
    get hasPrevious() {
      return runtime.playerHasPrevious;
    },
    get hasNext() {
      return runtime.playerHasNext;
    },
    get progress() {
      return runtime.progress;
    },
    get duration() {
      return runtime.duration;
    },
    get shuffleEnabled() {
      return runtime.shuffleEnabled;
    },
    get repeatMode() {
      return runtime.repeatMode;
    },
    get playbackOrder() {
      return runtime.playbackOrder;
    },
    get lyricsOpen() {
      return runtime.lyricsOpen;
    },
    get playlistOpen() {
      return runtime.playlistOpen;
    },
    get lyricsLoading() {
      return runtime.lyricsLoading;
    },
    get lyricsError() {
      return runtime.lyricsError;
    },
    get lyricsLines() {
      return runtime.lyricsLines;
    },
    get lyricsUnavailable() {
      return runtime.lyricsUnavailable;
    },
    get activeLyricIndex() {
      return runtime.activeLyricIndex;
    },
    get fullscreenOpen() {
      return runtime.fullscreenOpen;
    },
    get volume() {
      return runtime.playerController.volume;
    },
    get muted() {
      return runtime.playerController.muted;
    },
    get playbackFormat() {
      return runtime.playbackFormat;
    },
    pause: () => {
      void runtime.playerController.pause();
    },
    resume: () => {
      void runtime.playerController.resume();
    },
    seek: (positionSecs) => {
      void runtime.playerController.seek(positionSecs);
    },
    playPrevious: () => {
      void runtime.playerController.playPrevious();
    },
    playNext: () => {
      void runtime.playerController.playNext();
    },
    toggleShuffle: (next) => runtime.playerController.toggleShuffle(next),
    toggleRepeat: (next) => runtime.playerController.toggleRepeat(next),
    toggleLyrics: () => runtime.playerController.toggleLyrics(),
    togglePlaylist: () => runtime.playerController.togglePlaylist(),
    toggleFullscreen: () => runtime.playerController.toggleFullscreen(),
    setVolume: (volume) => {
      void runtime.playerController.setVolume(volume);
    },
    toggleMute: () => runtime.playerController.toggleMute(),
    playQueueEntry: (entry, order, index) =>
      runtime.playerController.playQueueEntry(entry, order, index),
  });

  setDownloadContext({
    get activeDownloadCount() {
      return runtime.activeDownloadCount;
    },
    get filteredJobs() {
      return runtime.filteredDownloadJobs;
    },
    get hasDownloadHistory() {
      return runtime.hasDownloadHistory;
    },
    get searchQuery() {
      return runtime.downloadController.searchQuery;
    },
    set searchQuery(v) {
      runtime.downloadController.searchQuery = v;
    },
    get scopeFilter() {
      return runtime.downloadController.scopeFilter;
    },
    set scopeFilter(v) {
      runtime.downloadController.scopeFilter = v;
    },
    get statusFilter() {
      return runtime.downloadController.statusFilter;
    },
    set statusFilter(v) {
      runtime.downloadController.statusFilter = v;
    },
    get kindFilter() {
      return runtime.downloadController.kindFilter;
    },
    set kindFilter(v) {
      runtime.downloadController.kindFilter = v;
    },
    canClearDownloadHistory: () =>
      runtime.downloadController.canClearDownloadHistory(),
    getJobProgress: (job) => runtime.downloadController.getJobProgress(job),
    getJobProgressText: (job) =>
      runtime.downloadController.getJobProgressText(job),
    getJobStatusLabel: (job) =>
      runtime.downloadController.getJobStatusLabel(job),
    getJobKindLabel: (job) => runtime.downloadController.getJobKindLabel(job),
    getJobSummaryLabel: (job) =>
      runtime.downloadController.getJobSummaryLabel(job),
    getJobDisplayTitle: (job) =>
      runtime.downloadController.getJobDisplayTitle(job),
    getJobErrorSummary: (job) =>
      runtime.downloadController.getJobErrorSummary(job),
    isJobActive: (jobId) => runtime.downloadController.isJobActive(jobId),
    canCancelTask: (task) => runtime.downloadController.canCancelTask(task),
    canRetryTask: (task) => runtime.downloadController.canRetryTask(task),
    getTaskErrorLabel: (task) =>
      runtime.downloadController.getTaskErrorLabel(task),
    getTaskStatusLabel: (task) =>
      runtime.downloadController.getTaskStatusLabel(task),
    handleClearDownloadHistory: () =>
      runtime.downloadController.handleClearDownloadHistory(),
    handleCancelDownloadJob: (jobId) =>
      runtime.downloadController.handleCancelDownloadJob(jobId),
    handleRetryDownloadJob: (jobId) =>
      runtime.downloadController.handleRetryDownloadJob(jobId),
    handleCancelDownloadTask: (jobId, taskId) =>
      runtime.downloadController.handleCancelDownloadTask(jobId, taskId),
    handleRetryDownloadTask: (jobId, taskId) =>
      runtime.downloadController.handleRetryDownloadTask(jobId, taskId),
    handleSongDownload: (songCid) =>
      runtime.downloadController.handleSongDownload(songCid),
    getSongDownloadState: (songCid) =>
      runtime.downloadController.getSongDownloadState(songCid),
    isSongDownloadInteractionBlocked: (songCid) =>
      runtime.downloadController.isSongDownloadInteractionBlocked(songCid),
  });

  setLibraryContext({
    get albums() {
      return runtime.albums;
    },
    get selectedAlbum() {
      return runtime.selectedAlbum;
    },
    get selectedAlbumCid() {
      return runtime.selectedAlbumCid;
    },
    get loadingAlbums() {
      return runtime.loadingAlbums;
    },
    get loadingDetail() {
      return runtime.loadingDetail;
    },
    get errorMsg() {
      return runtime.errorMsg;
    },
    get showDetailSkeleton() {
      return runtime.showDetailSkeleton;
    },
    get albumRequestSeq() {
      return runtime.albumRequestSeq;
    },
    get selectedAlbumArtworkUrl() {
      return runtime.selectedAlbumArtworkUrl;
    },
    get selectionModeEnabled() {
      return runtime.selectionModeEnabled;
    },
    get selectedSongCids() {
      return runtime.selectedSongCids;
    },
    handleSelectAlbum: (album) => runtime.handleSelectAlbum(album),
    toggleSelectionMode: () => runtime.toggleSelectionMode(),
    selectAllSongs: () => runtime.selectAllSongs(),
    deselectAllSongs: () => runtime.deselectAllSongs(),
    invertSongSelection: () => runtime.invertSongSelection(),
    toggleSongSelection: (songCid) => runtime.toggleSongSelection(songCid),
    isSongSelected: (songCid) => runtime.isSongSelected(songCid),
  });

  setCollectionContext({
    get collections() {
      return runtime.collectionController.collections;
    },
    get selectedCollectionId() {
      return runtime.collectionController.selectedCollectionId;
    },
    get selectedCollection() {
      return runtime.collectionController.selectedCollection;
    },
    get isLoading() {
      return runtime.collectionController.isLoading;
    },
    get isDetailLoading() {
      return runtime.collectionController.isDetailLoading;
    },
    get formDialogOpen() {
      return runtime.collectionController.formDialogOpen;
    },
    get formDialogMode() {
      return runtime.collectionController.formDialogMode;
    },
    selectCollection: (id) => {
      void runtime.collectionController.selectCollection(id);
    },
    openCreateDialog: () => runtime.collectionController.openCreateDialog(),
    openEditDialog: () => runtime.collectionController.openEditDialog(),
    closeFormDialog: () => runtime.collectionController.closeFormDialog(),
    handleCreate: (name, description) =>
      runtime.collectionController.handleCreate(name, description),
    handleUpdate: (id, name, description) =>
      runtime.collectionController.handleUpdate(id, name, description),
    handleDelete: (id) => runtime.collectionController.handleDelete(id),
    handleExport: (id) => runtime.collectionController.handleExport(id),
    handleRemoveSongs: (collectionId, songCids) =>
      runtime.collectionController.handleRemoveSongs(collectionId, songCids),
    handleReorderSongs: (collectionId, songCids) =>
      runtime.collectionController.handleReorderSongs(collectionId, songCids),
    handleAddSongs: (collectionId, songCids) =>
      runtime.collectionController.handleAddSongs(collectionId, songCids),
    loadCollections: () => runtime.collectionController.loadCollections(),
  });
</script>

{@render children()}
