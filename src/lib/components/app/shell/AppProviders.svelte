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
    toggleSidebar: runtime.toggleSidebar,
    handleRefresh: runtime.handleRefresh,
    handleToggleSettings: runtime.handleToggleSettings,
    handleToggleDownloads: runtime.handleToggleDownloads,
    notifyInfo: runtime.notifyInfo,
    notifyError: runtime.notifyError,
    navigate: (view) => {
      runtime.shellStore.currentView = view;
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
    pause: runtime.playerController.pause,
    resume: runtime.playerController.resume,
    seek: runtime.playerController.seek,
    playPrevious: runtime.playerController.playPrevious,
    playNext: runtime.playerController.playNext,
    toggleShuffle: runtime.playerController.toggleShuffle,
    toggleRepeat: runtime.playerController.toggleRepeat,
    toggleLyrics: runtime.playerController.toggleLyrics,
    togglePlaylist: runtime.playerController.togglePlaylist,
    toggleFullscreen: runtime.playerController.toggleFullscreen,
    setVolume: runtime.playerController.setVolume,
    toggleMute: runtime.playerController.toggleMute,
    playQueueEntry: runtime.playerController.playQueueEntry,
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
    canClearDownloadHistory: runtime.downloadController.canClearDownloadHistory,
    getJobProgress: runtime.downloadController.getJobProgress,
    getJobProgressText: runtime.downloadController.getJobProgressText,
    getJobStatusLabel: runtime.downloadController.getJobStatusLabel,
    getJobKindLabel: runtime.downloadController.getJobKindLabel,
    getJobSummaryLabel: runtime.downloadController.getJobSummaryLabel,
    getJobDisplayTitle: runtime.downloadController.getJobDisplayTitle,
    getJobErrorSummary: runtime.downloadController.getJobErrorSummary,
    isJobActive: runtime.downloadController.isJobActive,
    canCancelTask: runtime.downloadController.canCancelTask,
    canRetryTask: runtime.downloadController.canRetryTask,
    getTaskErrorLabel: runtime.downloadController.getTaskErrorLabel,
    getTaskStatusLabel: runtime.downloadController.getTaskStatusLabel,
    handleClearDownloadHistory:
      runtime.downloadController.handleClearDownloadHistory,
    handleCancelDownloadJob: runtime.downloadController.handleCancelDownloadJob,
    handleRetryDownloadJob: runtime.downloadController.handleRetryDownloadJob,
    handleCancelDownloadTask:
      runtime.downloadController.handleCancelDownloadTask,
    handleRetryDownloadTask: runtime.downloadController.handleRetryDownloadTask,
    handleSongDownload: runtime.downloadController.handleSongDownload,
    getSongDownloadState: runtime.downloadController.getSongDownloadState,
    isSongDownloadInteractionBlocked:
      runtime.downloadController.isSongDownloadInteractionBlocked,
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
    get librarySearchQuery() {
      return runtime.librarySearchQuery;
    },
    get librarySearchScope() {
      return runtime.librarySearchScope;
    },
    get librarySearchLoading() {
      return runtime.librarySearchLoading;
    },
    get librarySearchResponse() {
      return runtime.librarySearchResponse;
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
    setSearchQuery: runtime.libraryController.setSearchQuery,
    setSearchScope: runtime.libraryController.setSearchScope,
    handleSelectAlbum: runtime.handleSelectAlbum,
    handleSelectSearchResult: runtime.handleSelectSearchResult,
    toggleSelectionMode: runtime.toggleSelectionMode,
    selectAllSongs: runtime.selectAllSongs,
    deselectAllSongs: runtime.deselectAllSongs,
    invertSongSelection: runtime.invertSongSelection,
    toggleSongSelection: runtime.toggleSongSelection,
    isSongSelected: runtime.isSongSelected,
    handleDownloadSelection: runtime.handleDownloadSelection,
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
    selectCollection: runtime.collectionController.selectCollection,
    openCreateDialog: runtime.collectionController.openCreateDialog,
    openEditDialog: runtime.collectionController.openEditDialog,
    closeFormDialog: runtime.collectionController.closeFormDialog,
    handleCreate: runtime.collectionController.handleCreate,
    handleUpdate: runtime.collectionController.handleUpdate,
    handleDelete: runtime.collectionController.handleDelete,
    handleExport: runtime.collectionController.handleExport,
    handleRemoveSongs: runtime.collectionController.handleRemoveSongs,
    handleReorderSongs: runtime.collectionController.handleReorderSongs,
    handleAddSongs: runtime.collectionController.handleAddSongs,
    loadCollections: runtime.collectionController.loadCollections,
  });
</script>

{@render children()}
