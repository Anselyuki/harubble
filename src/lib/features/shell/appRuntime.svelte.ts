import { tick } from 'svelte';
import { listen } from '@tauri-apps/api/event';
import type { PartialOptions } from 'overlayscrollbars';
import {
  getAlbums,
  getAlbumDetail,
  getDefaultOutputDir,
  playSong,
  pausePlayback,
  resumePlayback,
  seekCurrentPlayback,
  getPlayerState,
  clearResponseCache,
  extractImageTheme,
  getImageDataUrl,
  getSongLyrics,
  createDownloadJob,
  listDownloadJobs,
  cancelDownloadJob,
  cancelDownloadTask,
  retryDownloadJob,
  retryDownloadTask,
  clearDownloadHistory,
  getPreferences,
  setPreferences,
  getLocalInventorySnapshot,
  searchLibrary,
  getLatestAlbums,
  getAlbumsBySeriesGroup,
  getRecentHistory,
  getHomepageStatus,
  clearListeningHistory,
  getTagDimensions,
  getAlbumsByTagDimension,
  getTagEditorMerged,
  getTagEditorLocalOverlay,
  setTagEditorEntityTag,
  removeTagEditorEntityTag,
  addTagEditorDimension,
  removeTagEditorDimension,
  applyTagEditorRemoteUpdate,
  resolveTagEditorConflict,
} from '$lib/api';
import {
  clearCache,
  createInventoryCacheTag,
  invalidateByTag,
  warmCacheManager,
} from '$lib/cache';
import type {
  Album,
  AlbumDetail,
  OutputFormat,
  SongEntry,
  PlayerState,
  PlaybackQueueEntry,
  DownloadJobSnapshot,
  DownloadManagerSnapshot,
  DownloadTaskProgressEvent,
  LocalInventorySnapshot,
  AppErrorEvent,
  LogLevel,
  SearchLibraryResultItem,
  ThemePalette,
} from '$lib/types';
import { applyThemePalette, DEFAULT_THEME_PALETTE } from '$lib/theme';
import { envStore } from '$lib/features/env/store.svelte';
import { shellStore } from '$lib/features/shell/store.svelte';
import { createSettingsController } from '$lib/features/shell/settings.svelte';
import { createAlbumStageMotionController } from '$lib/features/shell/albumStageMotion.svelte';
import { createLibraryController } from '$lib/features/library/controller.svelte';
import { createPlayerController } from '$lib/features/player/controller.svelte';
import { createDownloadController } from '$lib/features/download/controller.svelte';
import { createHomeController } from '$lib/features/home/controller.svelte';
import { createTagEditorController } from '$lib/features/tagEditor/controller.svelte';
import { createCollectionController } from '$lib/features/collection/controller.svelte';
import {
  listCollections,
  getCollection,
  createCollection,
  updateCollection,
  deleteCollection,
  addSongsToCollection,
  removeSongsFromCollection,
  reorderCollectionSongs,
  exportCollection,
  importCollection,
} from '$lib/collectionApi';
import { preloadImage } from '$lib/features/library/helpers';
import {
  buildAlbumPlaybackEntries,
  getSelectedAlbumCoverUrl,
} from '$lib/features/library/selectors';
import { localeState, type Locale } from '$lib/i18n';
import * as m from '$lib/paraglide/messages.js';
import { toast } from 'svelte-sonner';

const MIN_DISPLAY_MS = 260;
const DETAIL_SKELETON_DELAY_MS = 140;

const delay = (ms: number): Promise<void> =>
  new Promise((resolve) => {
    setTimeout(resolve, ms);
  });

export function createAppRuntime() {
  function notifyInfo(message: string) {
    toast(message);
  }

  function notifyError(message: string) {
    toast.error(message);
  }

  async function preloadAlbumArtwork(
    album: AlbumDetail
  ): Promise<number | null> {
    const sourceUrl = album.coverDeUrl ?? album.coverUrl;
    const resolvedUrl = await getImageDataUrl(sourceUrl).catch(() => sourceUrl);
    const meta = await preloadImage(resolvedUrl);
    return meta?.aspectRatio ?? null;
  }

  function setAlbumStageAspectRatio(value: number | null | undefined) {
    albumStageMotionController.setAspectRatio(value);
  }

  const libraryController = createLibraryController({
    delay,
    detailSkeletonDelayMs: DETAIL_SKELETON_DELAY_MS,
    minDetailDisplayMs: MIN_DISPLAY_MS,
    getAlbums,
    getAlbumDetail,
    searchLibrary,
    preloadAlbumArtwork,
    setAlbumStageAspectRatio,
    notifyError,
  });

  const playerController = createPlayerController({
    playSong: async (songCid, coverUrl, context) => {
      await playSong(songCid, coverUrl ?? undefined, context ?? undefined);
    },
    pausePlayback,
    resumePlayback,
    seekCurrentPlayback: async (positionSecs) => {
      await seekCurrentPlayback(positionSecs);
    },
    getSongLyrics,
    notifyError,
  });

  const downloadController = createDownloadController({
    createDownloadJob,
    cancelDownloadJob,
    cancelDownloadTask,
    retryDownloadJob,
    retryDownloadTask,
    clearDownloadHistory,
    openDownloadPanel: async (resetFilters = false) => {
      await shellStore.openDownloads({
        notifyError,
        beforeOpen: resetFilters
          ? () => {
              downloadController.resetFilters();
            }
          : undefined,
      });
    },
    getDownloadOptions: () => ({
      outputDir: settingsState.outputDir,
      format: settingsState.format,
      downloadLyrics: settingsState.downloadLyrics,
    }),
    notifyInfo,
    notifyError,
  });

  const settingsController = createSettingsController({
    getPreferences,
    setPreferences,
    notifyError,
    onLocaleChanged: (locale) => localeState.applyBackendLocale(locale),
  });

  const albumStageMotionController = createAlbumStageMotionController({
    getReducedMotion: () => envStore.prefersReducedMotion,
    getViewportHeight: () => envStore.viewportHeight,
    getLoadingDetail: () => libraryController.loadingDetail,
  });

  const homeController = createHomeController({
    getLatestAlbums,
    getAlbumsBySeriesGroup,
    getRecentHistory,
    getHomepageStatus,
    clearListeningHistory,
    getTagDimensions,
    getAlbumsByTagDimension,
    notifyError,
  });

  const tagEditorController = createTagEditorController({
    getTagEditorMerged,
    getTagEditorLocalOverlay,
    setTagEditorEntityTag,
    removeTagEditorEntityTag,
    addTagEditorDimension,
    removeTagEditorDimension,
    applyTagEditorRemoteUpdate,
    resolveTagEditorConflict,
    getAlbumDetail: (albumCid: string) => getAlbumDetail(albumCid),
    notifyError,
  });

  const collectionController = createCollectionController({
    listCollections,
    getCollection,
    createCollection,
    updateCollection,
    deleteCollection,
    addSongsToCollection,
    removeSongsFromCollection,
    reorderCollectionSongs,
    exportCollection,
    importCollection,
    navigateToCollection: shellStore.navigateToCollection,
    notifyInfo,
    notifyError,
  });

  let selectedSongCids = $state<string[]>([]);
  let selectionModeEnabled = $state(false);
  let themeRequestSeq = 0;
  let artworkRequestSeq = 0;
  let playerStateInitSeq = 0;
  let playerStateHydratedFromEvent = false;

  const settingsOpen = $derived(shellStore.settingsOpen);
  const downloadPanelOpen = $derived(shellStore.downloadPanelOpen);
  const SettingsSheetView = $derived(shellStore.SettingsSheetView);
  const DownloadTasksSheetView = $derived(shellStore.DownloadTasksSheetView);
  const contentScrollbarEvents = $derived(
    albumStageMotionController.contentScrollbarEvents
  );
  const albumStageStyle = $derived(albumStageMotionController.albumStageStyle);
  const albumStageMediaHeight = $derived(
    albumStageMotionController.albumStageMediaHeight
  );
  const albumStageScrimOpacity = $derived(
    albumStageMotionController.albumStageScrimOpacity
  );
  const albumStageImageOpacity = $derived(
    albumStageMotionController.albumStageImageOpacity
  );
  const albumStageImageTransform = $derived(
    albumStageMotionController.albumStageImageTransform
  );
  const albumStageSolidifyOpacity = $derived(
    albumStageMotionController.albumStageSolidifyOpacity
  );
  const prefersReducedMotion = $derived(envStore.prefersReducedMotion);
  const albums = $derived(libraryController.albums);
  const selectedAlbum = $derived(libraryController.selectedAlbum);
  const selectedAlbumCid = $derived(
    shellStore.currentView === 'tagEditor'
      ? (tagEditorController.editingAlbum?.cid ?? null)
      : libraryController.selectedAlbumCid
  );
  const loadingAlbums = $derived(libraryController.loadingAlbums);
  const loadingDetail = $derived(libraryController.loadingDetail);
  const errorMsg = $derived(libraryController.errorMsg);
  const librarySearchQuery = $derived(libraryController.librarySearchQuery);
  const librarySearchScope = $derived(libraryController.librarySearchScope);
  const librarySearchLoading = $derived(libraryController.librarySearchLoading);
  const librarySearchResponse = $derived(
    libraryController.librarySearchResponse
  );
  const pendingScrollToSongCid = $derived(
    libraryController.pendingScrollToSongCid
  );
  const showDetailSkeleton = $derived(libraryController.showDetailSkeleton);
  const albumRequestSeq = $derived(libraryController.albumRequestSeq);
  const currentSong = $derived(playerController.currentSong);
  const isPlaying = $derived(playerController.isPlaying);
  const isPaused = $derived(playerController.isPaused);
  const isLoading = $derived(playerController.isLoading);
  const progress = $derived(playerController.progress);
  const duration = $derived(playerController.duration);
  const shuffleEnabled = $derived(playerController.shuffleEnabled);
  const repeatMode = $derived(playerController.repeatMode);
  const playbackOrder = $derived(playerController.playbackOrder);
  const lyricsOpen = $derived(playerController.lyricsOpen);
  const playlistOpen = $derived(playerController.playlistOpen);
  const lyricsLoading = $derived(playerController.lyricsLoading);
  const lyricsError = $derived(playerController.lyricsError);
  const lyricsLines = $derived(playerController.lyricsLines);
  const lyricsUnavailable = $derived(playerController.lyricsUnavailable);
  const fullscreenOpen = $derived(playerController.fullscreenOpen);
  const downloadingAlbumCid = $derived(downloadController.downloadingAlbumCid);
  const activeDownloadCount = $derived(downloadController.activeDownloadCount);
  const filteredDownloadJobs = $derived(downloadController.filteredJobs);
  const hasDownloadHistory = $derived(downloadController.hasDownloadHistory);
  const contentEl = $derived(albumStageMotionController.contentElement);
  const isMacOS = $derived(envStore.isMacOS);

  const settingsState = $state({
    format: 'flac' as OutputFormat,
    outputDir: '',
    downloadLyrics: true,
    notifyOnDownloadComplete: true,
    notifyOnPlaybackChange: true,
    logLevel: 'error' as LogLevel,
    locale: 'zh-CN' as Locale,
    settingsLogRefreshToken: 0,
    prefsReady: false,
    isSaving: false,
    persistedSnapshot: '',
    lastSaveFailedSnapshot: '',
    dirty: {
      format: false,
      outputDir: false,
      downloadLyrics: false,
      notifyOnDownloadComplete: false,
      notifyOnPlaybackChange: false,
      logLevel: false,
      locale: false,
    },
    suspendDirtyTracking: 0,
  });

  let selectedAlbumArtworkUrl = $state<string | null>(null);
  let albumStageElement = $state<HTMLElement | null>(null);
  let activeThemeArtworkUrl: string | null = null;
  let cachedAlbumPalette = $state<ThemePalette | null>(null);
  let activeAlbumStageArtworkUrl: string | null = null;
  const lastObservedSettings = {
    format: settingsState.format,
    outputDir: settingsState.outputDir,
    downloadLyrics: settingsState.downloadLyrics,
    notifyOnDownloadComplete: settingsState.notifyOnDownloadComplete,
    notifyOnPlaybackChange: settingsState.notifyOnPlaybackChange,
    logLevel: settingsState.logLevel,
    locale: settingsState.locale,
  };

  const playerHasPrevious = $derived(playerController.playerHasPrevious);
  const playerHasNext = $derived(playerController.playerHasNext);

  const activeLyricIndex = $derived.by(() => {
    if (!lyricsOpen && !fullscreenOpen) return -1;
    let activeIndex = -1;
    for (let index = 0; index < lyricsLines.length; index += 1) {
      const lineTime = lyricsLines[index].time;
      if (lineTime === null) continue;
      if (progress + 0.08 >= lineTime) {
        activeIndex = index;
      } else {
        break;
      }
    }
    return activeIndex;
  });

  const overlayScrollbarOptions = $derived.by(
    (): PartialOptions => ({
      scrollbars: {
        theme: 'os-theme-app',
        autoHide: prefersReducedMotion ? 'leave' : 'move',
        autoHideDelay: prefersReducedMotion ? 160 : 720,
        autoHideSuspend: true,
        dragScroll: true,
        clickScroll: false,
      },
    })
  );

  let isRefreshing = $state(false);

  // --- PLACEHOLDER_HELPERS ---

  function resetContentScroll() {
    albumStageMotionController.resetContentScroll();
  }

  function isSongSelected(songCid: string): boolean {
    return selectedSongCids.includes(songCid);
  }

  function toggleSongSelection(songCid: string) {
    if (selectedSongCids.includes(songCid)) {
      selectedSongCids = selectedSongCids.filter((cid) => cid !== songCid);
      return;
    }
    selectedSongCids = [...selectedSongCids, songCid];
  }

  function clearSongSelection() {
    selectedSongCids = [];
  }

  function selectAllSongs() {
    if (!selectedAlbum) return;
    selectedSongCids = selectedAlbum.songs.map((s: SongEntry) => s.cid);
  }

  function deselectAllSongs() {
    selectedSongCids = [];
  }

  function invertSongSelection() {
    if (!selectedAlbum) return;
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- ephemeral, non-reactive lookup
    const allCids = new Set(selectedAlbum.songs.map((s: SongEntry) => s.cid));
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- ephemeral, non-reactive lookup
    const currentSelected = new Set(selectedSongCids);
    selectedSongCids = [...allCids].filter((cid) => !currentSelected.has(cid));
  }

  function toggleSelectionMode() {
    selectionModeEnabled = !selectionModeEnabled;
    if (!selectionModeEnabled) {
      clearSongSelection();
    }
  }

  function getSettingsSnapshot() {
    return JSON.stringify({
      format: settingsState.format,
      outputDir: settingsState.outputDir,
      downloadLyrics: settingsState.downloadLyrics,
      notifyOnDownloadComplete: settingsState.notifyOnDownloadComplete,
      notifyOnPlaybackChange: settingsState.notifyOnPlaybackChange,
      logLevel: settingsState.logLevel,
      locale: settingsState.locale,
    });
  }

  function handleContentWheel(event: WheelEvent) {
    albumStageMotionController.handleContentWheel(event);
  }

  function handleAppErrorEvent(event: AppErrorEvent) {
    notifyError(event.message);
    settingsController.handleAppError(settingsState, settingsOpen);
  }

  async function invalidateInventoryCaches(
    inventoryVersion: string | null | undefined
  ) {
    await invalidateByTag(createInventoryCacheTag(inventoryVersion));
  }

  async function handleSelectAlbum(album: Album) {
    if (shellStore.currentView === 'tagEditor') {
      await tagEditorController.selectAlbumForEdit(album);
      return;
    }
    shellStore.navigateToLibrary();
    clearSongSelection();
    selectionModeEnabled = false;
    await libraryController.selectAlbum(album, {
      afterSelect: async () => {
        await tick();
        resetContentScroll();
      },
    });
  }

  async function handleSelectSearchResult(item: SearchLibraryResultItem) {
    const album = albums.find((candidate) => candidate.cid === item.albumCid);
    if (!album) {
      notifyError(m.app_error_album_not_found());
      return;
    }
    shellStore.navigateToLibrary();
    libraryController.setPendingScrollToSong(
      item.kind === 'song' ? item.songCid : null
    );
    clearSongSelection();
    selectionModeEnabled = false;
    await libraryController.selectAlbum(album, {
      afterSelect: async () => {
        await tick();
        resetContentScroll();
      },
    });
  }

  async function handlePlay(song: SongEntry) {
    const sourceEntries = buildAlbumPlaybackEntries(selectedAlbum);
    const fallbackEntry: PlaybackQueueEntry = {
      cid: song.cid,
      name: song.name,
      artists: song.artists,
      coverUrl: getSelectedAlbumCoverUrl(selectedAlbum),
    };
    const entries = sourceEntries.length ? sourceEntries : [fallbackEntry];
    playerController.applyPlaybackQueue(entries, song.cid);
    const nextOrder = shuffleEnabled ? [...playbackOrder] : [...entries];
    const nextIndex = nextOrder.findIndex((entry) => entry.cid === song.cid);
    if (nextIndex < 0) return;
    await playerController.playQueueEntry(
      nextOrder[nextIndex],
      nextOrder,
      nextIndex
    );
  }

  async function handlePlayCollectionSong(
    song: SongEntry,
    queue: PlaybackQueueEntry[]
  ) {
    const entries = queue.length
      ? queue
      : [
          {
            cid: song.cid,
            name: song.name,
            artists: song.artists,
            coverUrl: null,
          },
        ];
    playerController.applyPlaybackQueue(entries, song.cid);
    const nextOrder = shuffleEnabled ? [...playbackOrder] : [...entries];
    const nextIndex = nextOrder.findIndex((entry) => entry.cid === song.cid);
    if (nextIndex < 0) return;
    await playerController.playQueueEntry(
      nextOrder[nextIndex],
      nextOrder,
      nextIndex
    );
  }

  async function handleRefresh() {
    if (isRefreshing) return;
    isRefreshing = true;
    clearSongSelection();
    selectionModeEnabled = false;
    try {
      await clearCache();
      await clearResponseCache();
      await libraryController.reloadAlbumsAndRefreshCurrentSelection({
        afterSelect: async () => {
          await tick();
          resetContentScroll();
        },
      });
    } catch (e) {
      notifyError(
        m.app_error_refresh_failed({
          error: e instanceof Error ? e.message : String(e),
        })
      );
    } finally {
      await delay(400);
      isRefreshing = false;
    }
  }

  function handleToggleDownloads() {
    void shellStore.toggleDownloads({ notifyError });
  }

  function handleToggleSettings() {
    void shellStore.toggleSettings({ notifyError });
  }

  function handleOutputDirChange() {
    return settingsController.savePreferences(settingsState);
  }

  function handleDownloadSelection(songCids: string[]) {
    void downloadController.handleSelectionDownload(songCids, {
      afterCreated: () => {
        clearSongSelection();
        selectionModeEnabled = false;
      },
    });
  }

  function handleCurrentSongDownload() {
    if (currentSong) {
      void downloadController.handleSongDownload(currentSong.cid);
    }
  }

  function hasAlbumDownloadJob(albumCid: string): boolean {
    return !!downloadController.findAlbumDownloadJob(albumCid);
  }

  function hasCurrentSelectionJob(songCids: string[]): boolean {
    return !!downloadController.getCurrentSelectionJob(songCids);
  }

  $effect(() => {
    const artworkUrl =
      selectedAlbum?.coverUrl ?? selectedAlbum?.coverDeUrl ?? null;
    if (artworkUrl === activeThemeArtworkUrl) {
      return;
    }

    activeThemeArtworkUrl = artworkUrl;
    const paletteRequestSeq = ++themeRequestSeq;

    if (!artworkUrl) {
      cachedAlbumPalette = null;
      return;
    }

    void (async () => {
      try {
        const palette = await extractImageTheme(artworkUrl);
        if (paletteRequestSeq !== themeRequestSeq) return;
        cachedAlbumPalette = palette;
      } catch {
        if (paletteRequestSeq !== themeRequestSeq) return;
        cachedAlbumPalette = null;
      }
    })();
  });

  $effect(() => {
    const shouldApplyAlbumTheme =
      shellStore.currentView === 'library' || fullscreenOpen;
    if (shouldApplyAlbumTheme && cachedAlbumPalette) {
      applyThemePalette(cachedAlbumPalette);
    } else {
      applyThemePalette(DEFAULT_THEME_PALETTE);
    }
  });

  $effect(() => {
    const sourceUrl =
      selectedAlbum?.coverDeUrl ?? selectedAlbum?.coverUrl ?? null;
    if (sourceUrl === activeAlbumStageArtworkUrl) {
      return;
    }

    activeAlbumStageArtworkUrl = sourceUrl;
    const requestSeq = ++artworkRequestSeq;

    if (!sourceUrl) {
      selectedAlbumArtworkUrl = null;
      return;
    }

    void (async () => {
      try {
        const dataUrl = await getImageDataUrl(sourceUrl);
        if (requestSeq !== artworkRequestSeq) return;
        selectedAlbumArtworkUrl = dataUrl;
      } catch {
        if (requestSeq !== artworkRequestSeq) return;
        selectedAlbumArtworkUrl = null;
      }
    })();
  });

  $effect(() => {
    const value = settingsState.format;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.format = value;
      return;
    }
    if (value !== lastObservedSettings.format) {
      settingsState.dirty.format = true;
      lastObservedSettings.format = value;
    }
  });

  $effect(() => {
    const value = settingsState.outputDir;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.outputDir = value;
      return;
    }
    if (value !== lastObservedSettings.outputDir) {
      settingsState.dirty.outputDir = true;
      lastObservedSettings.outputDir = value;
    }
  });

  $effect(() => {
    const value = settingsState.downloadLyrics;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.downloadLyrics = value;
      return;
    }
    if (value !== lastObservedSettings.downloadLyrics) {
      settingsState.dirty.downloadLyrics = true;
      lastObservedSettings.downloadLyrics = value;
    }
  });

  $effect(() => {
    const value = settingsState.notifyOnDownloadComplete;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.notifyOnDownloadComplete = value;
      return;
    }
    if (value !== lastObservedSettings.notifyOnDownloadComplete) {
      settingsState.dirty.notifyOnDownloadComplete = true;
      lastObservedSettings.notifyOnDownloadComplete = value;
    }
  });

  $effect(() => {
    const value = settingsState.notifyOnPlaybackChange;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.notifyOnPlaybackChange = value;
      return;
    }
    if (value !== lastObservedSettings.notifyOnPlaybackChange) {
      settingsState.dirty.notifyOnPlaybackChange = true;
      lastObservedSettings.notifyOnPlaybackChange = value;
    }
  });

  $effect(() => {
    const value = settingsState.logLevel;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.logLevel = value;
      return;
    }
    if (value !== lastObservedSettings.logLevel) {
      settingsState.dirty.logLevel = true;
      lastObservedSettings.logLevel = value;
    }
  });

  $effect(() => {
    const value = settingsState.locale;
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.locale = value;
      return;
    }
    if (value !== lastObservedSettings.locale) {
      settingsState.dirty.locale = true;
      lastObservedSettings.locale = value;
    }
  });

  $effect(() => {
    const { persistedSnapshot, isSaving, lastSaveFailedSnapshot, prefsReady } =
      settingsState;
    if (!prefsReady || isSaving) {
      return;
    }

    const currentSnapshot = getSettingsSnapshot();

    if (currentSnapshot === persistedSnapshot) {
      return;
    }

    if (currentSnapshot === lastSaveFailedSnapshot) {
      return;
    }

    void settingsController.savePreferences(settingsState);
  });

  $effect(() => {
    albumStageMotionController.albumStageElement = albumStageElement;
  });

  async function bootstrapApp(shouldDispose: () => boolean) {
    try {
      await warmCacheManager();
    } catch {
      // Keep startup usable if IndexedDB warm start is unavailable.
    }

    if (shouldDispose()) {
      return;
    }

    try {
      await settingsController.hydratePreferences(settingsState, {
        shouldDispose,
      });
    } catch {
      // Preferences hydration failure is already tolerated in controller.
    }

    const defaultDirPromise = settingsState.outputDir
      ? Promise.resolve('')
      : getDefaultOutputDir().catch(() => '');

    try {
      const albumList = await libraryController.loadAlbums({ shouldDispose });

      const defaultDir = await defaultDirPromise;
      if (shouldDispose()) {
        return;
      }
      if (defaultDir) {
        settingsController.applyDefaultOutputDir(settingsState, defaultDir);
      }

      try {
        const snapshot = await getLocalInventorySnapshot();
        if (shouldDispose()) {
          return;
        }
        libraryController.initializeInventory(snapshot);
      } catch {
        if (!shouldDispose()) {
          libraryController.initializeInventory(null);
        }
      }

      if (albumList.length > 0 && !libraryController.selectedAlbumCid) {
        clearSongSelection();
        selectionModeEnabled = false;
        await libraryController.selectAlbum(albumList[0], {
          shouldDispose,
          afterSelect: async () => {
            await tick();
            resetContentScroll();
          },
        });
        if (shouldDispose()) {
          return;
        }
      }
    } catch {
      const defaultDir = await defaultDirPromise;
      if (shouldDispose()) {
        return;
      }
      if (defaultDir) {
        settingsController.applyDefaultOutputDir(settingsState, defaultDir);
      }
    }

    try {
      const requestSeq = downloadController.beginHydrationAttempt();
      const manager = await listDownloadJobs();
      if (shouldDispose()) {
        return;
      }
      downloadController.applyManagerSnapshot(manager, requestSeq);
    } catch {
      // Download manager not available
    }

    try {
      const requestSeq = ++playerStateInitSeq;
      const playerState = await getPlayerState();
      if (shouldDispose()) {
        return;
      }
      if (requestSeq === playerStateInitSeq && !playerStateHydratedFromEvent) {
        playerController.syncPlayerState(playerState);
      }
    } catch {
      // Player not playing on startup
    }

    void homeController.loadHomepageData();
  }

  async function subscribeToTauriEvents(shouldDispose: () => boolean) {
    const unlisteners: (() => void)[] = [];

    const cleanup = () => {
      while (unlisteners.length > 0) {
        unlisteners.pop()?.();
      }
    };

    async function register<T>(
      eventName: string,
      handler: (event: { payload: T }) => void | Promise<void>
    ) {
      const unlisten = await listen<T>(eventName, async (event) => {
        if (shouldDispose()) {
          return;
        }
        await handler(event);
      });

      if (shouldDispose()) {
        unlisten();
        return false;
      }

      unlisteners.push(unlisten);
      return true;
    }

    try {
      if (
        !(await register<PlayerState>('player-state-changed', (event) => {
          playerStateHydratedFromEvent = true;
          playerController.syncPlayerState(event.payload);
        }))
      ) {
        return cleanup;
      }

      if (
        !(await register<PlayerState>('player-progress', (event) => {
          playerController.syncPlayerProgress(event.payload);
        }))
      ) {
        return cleanup;
      }

      if (
        !(await register<DownloadManagerSnapshot>(
          'download-manager-state-changed',
          (event) => {
            downloadController.applyManagerEvent(event.payload);
          }
        ))
      ) {
        return cleanup;
      }

      if (
        !(await register<DownloadJobSnapshot>(
          'download-job-updated',
          (event) => {
            downloadController.applyJobUpdate(event.payload);
          }
        ))
      ) {
        return cleanup;
      }

      if (
        !(await register<DownloadTaskProgressEvent>(
          'download-task-progress',
          (event) => {
            downloadController.applyTaskProgress(event.payload);
          }
        ))
      ) {
        return cleanup;
      }

      if (
        !(await register<AppErrorEvent>('app-error-recorded', (event) => {
          handleAppErrorEvent(event.payload);
        }))
      ) {
        return cleanup;
      }

      if (
        !(await register<LocalInventorySnapshot>(
          'local-inventory-state-changed',
          async (event) => {
            await libraryController.handleInventoryStateChanged(event.payload, {
              shouldDispose,
              invalidateInventoryCaches,
              onSelectionInvalidated: () => {
                clearSongSelection();
                selectionModeEnabled = false;
              },
            });
          }
        ))
      ) {
        return cleanup;
      }

      if (
        !(await register<void>('homepage-belong-ready', () => {
          homeController.handleBelongReady();
        }))
      ) {
        return cleanup;
      }

      return cleanup;
    } catch (error) {
      cleanup();
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(m.app_error_event_subscribe_failed({ error: message }), {
        cause: error,
      });
    }
  }

  function teardownAppRuntime(unsubscribe: (() => void) | null) {
    shellStore.dispose();
    envStore.dispose();
    libraryController.dispose();
    playerController.dispose();
    downloadController.dispose();
    albumStageMotionController.dispose();
    homeController.dispose();
    tagEditorController.dispose();
    playerStateInitSeq += 1;
    playerStateHydratedFromEvent = false;
    unsubscribe?.();
  }

  $effect(() => {
    libraryController.init();
    playerController.init();
    downloadController.init();
    envStore.init();
    shellStore.init();
    homeController.init();
    void collectionController.loadCollections();

    let disposed = false;
    let unsubscribe: (() => void) | null = null;

    void (async () => {
      try {
        const nextUnsubscribe = await subscribeToTauriEvents(() => disposed);
        // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- async race guard
        if (disposed) {
          nextUnsubscribe();
          return;
        }
        unsubscribe = nextUnsubscribe;

        await bootstrapApp(() => disposed);
      } catch (error) {
        // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- async race guard
        if (disposed) {
          return;
        }
        notifyError(
          m.app_error_init_failed({
            error: error instanceof Error ? error.message : String(error),
          })
        );
      }
    })();

    return () => {
      disposed = true;
      teardownAppRuntime(unsubscribe);
    };
  });

  $effect(() => {
    playerController.syncPlaybackLifecycle();
  });

  $effect(() => {
    if (!pendingScrollToSongCid || !selectedAlbum || loadingDetail) {
      return;
    }

    const expectedSongCid = pendingScrollToSongCid;
    void tick().then(() => {
      if (pendingScrollToSongCid !== expectedSongCid || !contentEl) {
        return;
      }

      const row = contentEl.querySelector<HTMLElement>(
        `[data-song-cid="${CSS.escape(expectedSongCid)}"]`
      );
      if (!row) {
        return;
      }

      row.scrollIntoView({
        behavior: prefersReducedMotion ? 'auto' : 'smooth',
        block: 'center',
      });
      libraryController.clearPendingScrollToSong(expectedSongCid);
    });
  });

  return {
    get isMacOS() {
      return isMacOS;
    },
    get currentView() {
      return shellStore.currentView;
    },
    get albums() {
      return albums;
    },
    get selectedAlbum() {
      return selectedAlbum;
    },
    get selectedAlbumCid() {
      return selectedAlbumCid;
    },
    get loadingAlbums() {
      return loadingAlbums;
    },
    get loadingDetail() {
      return loadingDetail;
    },
    get errorMsg() {
      return errorMsg;
    },
    get librarySearchQuery() {
      return librarySearchQuery;
    },
    get librarySearchScope() {
      return librarySearchScope;
    },
    get librarySearchLoading() {
      return librarySearchLoading;
    },
    get librarySearchResponse() {
      return librarySearchResponse;
    },
    get showDetailSkeleton() {
      return showDetailSkeleton;
    },
    get albumRequestSeq() {
      return albumRequestSeq;
    },
    get selectedAlbumArtworkUrl() {
      return selectedAlbumArtworkUrl;
    },
    get currentSong() {
      return currentSong;
    },
    get isPlaying() {
      return isPlaying;
    },
    get isPaused() {
      return isPaused;
    },
    get isLoading() {
      return isLoading;
    },
    get progress() {
      return progress;
    },
    get duration() {
      return duration;
    },
    get shuffleEnabled() {
      return shuffleEnabled;
    },
    get repeatMode() {
      return repeatMode;
    },
    get playbackOrder() {
      return playbackOrder;
    },
    get lyricsOpen() {
      return lyricsOpen;
    },
    get playlistOpen() {
      return playlistOpen;
    },
    get lyricsLoading() {
      return lyricsLoading;
    },
    get lyricsError() {
      return lyricsError;
    },
    get lyricsLines() {
      return lyricsLines;
    },
    get lyricsUnavailable() {
      return lyricsUnavailable;
    },
    get activeLyricIndex() {
      return activeLyricIndex;
    },
    get fullscreenOpen() {
      return fullscreenOpen;
    },
    get playerHasPrevious() {
      return playerHasPrevious;
    },
    get playerHasNext() {
      return playerHasNext;
    },
    get downloadingAlbumCid() {
      return downloadingAlbumCid;
    },
    get activeDownloadCount() {
      return activeDownloadCount;
    },
    get filteredDownloadJobs() {
      return filteredDownloadJobs;
    },
    get hasDownloadHistory() {
      return hasDownloadHistory;
    },
    get prefersReducedMotion() {
      return prefersReducedMotion;
    },
    get overlayScrollbarOptions() {
      return overlayScrollbarOptions;
    },
    get contentScrollbarEvents() {
      return contentScrollbarEvents;
    },
    get albumStageStyle() {
      return albumStageStyle;
    },
    get albumStageMediaHeight() {
      return albumStageMediaHeight;
    },
    get albumStageScrimOpacity() {
      return albumStageScrimOpacity;
    },
    get albumStageImageOpacity() {
      return albumStageImageOpacity;
    },
    get albumStageImageTransform() {
      return albumStageImageTransform;
    },
    get albumStageSolidifyOpacity() {
      return albumStageSolidifyOpacity;
    },
    get albumStageElement() {
      return albumStageElement;
    },
    set albumStageElement(value: HTMLElement | null) {
      albumStageElement = value;
    },
    get selectionModeEnabled() {
      return selectionModeEnabled;
    },
    get selectedSongCids() {
      return selectedSongCids;
    },
    get settingsOpen() {
      return settingsOpen;
    },
    get downloadPanelOpen() {
      return downloadPanelOpen;
    },
    get SettingsSheetView() {
      return SettingsSheetView;
    },
    get DownloadTasksSheetView() {
      return DownloadTasksSheetView;
    },
    get isRefreshing() {
      return isRefreshing;
    },
    get currentSongDownloadState() {
      return currentSong
        ? downloadController.getSongDownloadState(currentSong.cid)
        : ('idle' as const);
    },
    get currentSongDownloadDisabled() {
      return currentSong
        ? downloadController.isSongDownloadInteractionBlocked(currentSong.cid)
        : false;
    },
    get sidebarCollapsed() {
      return shellStore.sidebarCollapsed;
    },
    toggleSidebar() {
      shellStore.toggleSidebar();
    },
    settingsState,
    shellStore,
    libraryController,
    playerController,
    downloadController,
    homeController,
    tagEditorController,
    collectionController,
    notifyInfo,
    notifyError,
    handleSelectAlbum,
    handleSelectSearchResult,
    handlePlay,
    handlePlayCollectionSong,
    handleRefresh,
    handleContentWheel,
    handleToggleDownloads,
    handleToggleSettings,
    handleOutputDirChange,
    handleDownloadSelection,
    handleCurrentSongDownload,
    hasAlbumDownloadJob,
    hasCurrentSelectionJob,
    toggleSelectionMode,
    selectAllSongs,
    deselectAllSongs,
    invertSongSelection,
    toggleSongSelection,
    isSongSelected,
  };
}

export type AppRuntime = ReturnType<typeof createAppRuntime>;
