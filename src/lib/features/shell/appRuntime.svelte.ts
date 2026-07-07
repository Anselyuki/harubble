import { tick } from 'svelte';
import { listen } from '@tauri-apps/api/event';
import type { PartialOptions } from 'overlayscrollbars';
import { gsapScrollIntoView } from '$lib/design/gsap';
import {
  getAlbums,
  getAlbumDetail,
  getDefaultOutputDir,
  playSong,
  pausePlayback,
  resumePlayback,
  seekCurrentPlayback,
  getPlayerState,
  setPlaybackVolume,
  clearResponseCache,
  resetHttpClient,
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
  exportTagEditorRegistry,
  importTagEditorRegistry,
} from '$lib/api';
import {
  clearCache,
  createInventoryCacheTag,
  invalidateByTag,
  warmCacheManager,
} from '$lib/cache';
import type { AppErrorEvent, LogLevel, OutputFormat } from '$lib/types';
import { DEFAULT_THEME_PREFERENCES } from '$lib/themePresets';
import { envStore } from '$lib/features/env/store.svelte';
import { shellStore } from '$lib/features/shell/store.svelte';
import { navigationStack } from './navigation.svelte';
import { createSettingsController } from '$lib/features/shell/settings.svelte';
import { createAlbumStageMotionController } from '$lib/features/shell/albumStageMotion.svelte';
import { createLibraryController } from '$lib/features/library/controller.svelte';
import { createPlayerController } from '$lib/features/player/controller.svelte';
import { createDownloadController } from '$lib/features/download/controller.svelte';
import { createHomeController } from '$lib/features/home/controller.svelte';
import { createTagEditorController } from '$lib/features/tagEditor/controller.svelte';
import { createCollectionController } from '$lib/features/collection/controller.svelte';
import { createSearchController } from '$lib/features/search/controller.svelte';
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
import {
  bootstrapApp,
  subscribeToTauriEvents,
} from '$lib/features/shell/appRuntimeBootstrap.svelte';
import { localeState, type Locale } from '$lib/i18n';
import * as m from '$lib/paraglide/messages.js';
import { toast } from 'svelte-sonner';
import { createSelectionManager } from './selectionManager.svelte';
import { createThemeManager } from './themeManager.svelte';
import { createNavigationManager } from './navigationManager.svelte';
import { createDownloadBridge } from './downloadBridge.svelte';

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

  // --- 子控制器创建 ---

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
    getIsViewTransitioning: () => navigationManager.isViewTransitioning,
  });

  const libraryController = createLibraryController({
    delay,
    detailSkeletonDelayMs: DETAIL_SKELETON_DELAY_MS,
    minDetailDisplayMs: MIN_DISPLAY_MS,
    getAlbums,
    getAlbumDetail,
    searchLibrary,
    preloadAlbumArtwork: (album) => themeManager.preloadAlbumArtwork(album),
    setAlbumStageAspectRatio: (value) =>
      albumStageMotionController.setAspectRatio(value),
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
    setPlaybackVolume,
    getPlayerState,
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
    exportTagEditorRegistry,
    importTagEditorRegistry,
    getAlbumDetail: (albumCid: string) => getAlbumDetail(albumCid),
    getAlbums: () => libraryController.albums,
    notifyError,
  });

  const searchController = createSearchController({
    getRecentHistory,
    getAlbums: () => libraryController.albums,
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
    notifyInfo,
    notifyError,
  });

  // --- 子模块创建 ---

  const selectionManager = createSelectionManager({
    getSelectedAlbum: () => libraryController.selectedAlbum,
  });

  const themeManager = createThemeManager({
    getSelectedAlbum: () => libraryController.selectedAlbum,
    getFullscreenOpen: () => playerController.fullscreenOpen,
    getSettingsTheme: () => ({
      presetId: settingsState.themePresetId,
      customColors: settingsState.themeCustomColors,
      colorScheme: settingsState.colorScheme,
    }),
  });

  const navigationManager = createNavigationManager({
    libraryController,
    playerController,
    collectionController,
    tagEditorController,
    albumStageMotionController,
    clearSongSelection: () => selectionManager.clearSongSelection(),
    setSelectionModeEnabled: (value) =>
      selectionManager.setSelectionModeEnabled(value),
    notifyError,
    getAlbums: () => libraryController.albums,
    getSelectedAlbum: () => libraryController.selectedAlbum,
    getShuffleEnabled: () => playerController.shuffleEnabled,
    getPlaybackOrder: () => playerController.playbackOrder,
  });

  const downloadBridge = createDownloadBridge({
    downloadController,
    playerController,
    clearSongSelection: () => selectionManager.clearSongSelection(),
    setSelectionModeEnabled: (value) =>
      selectionManager.setSelectionModeEnabled(value),
    notifyError,
  });

  // --- 本地状态 ---

  let playerStateInitSeq = 0;
  let playerStateHydratedFromEvent = false;
  let albumStageElement = $state<HTMLElement | null>(null);
  let isRefreshing = $state(false);

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
  const selectedAlbumCid = $derived(libraryController.selectedAlbumCid);
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
  const playbackFormat = $derived(playerController.playbackFormat);
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
  const playerHasPrevious = $derived(playerController.playerHasPrevious);
  const playerHasNext = $derived(playerController.playerHasNext);

  // --- 设置状态 ---

  const settingsState = $state({
    format: 'flac' as OutputFormat,
    outputDir: '',
    downloadLyrics: true,
    notifyOnDownloadComplete: true,
    notifyOnPlaybackChange: true,
    logLevel: 'error' as LogLevel,
    locale: 'zh-CN' as Locale,
    volume: 1,
    themePresetId: DEFAULT_THEME_PREFERENCES.presetId,
    themeCustomColors: {},
    colorScheme: DEFAULT_THEME_PREFERENCES.colorScheme ?? 'auto',
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
      theme: false,
    },
    suspendDirtyTracking: 0,
  });

  const lastObservedSettings = {
    format: settingsState.format,
    outputDir: settingsState.outputDir,
    downloadLyrics: settingsState.downloadLyrics,
    notifyOnDownloadComplete: settingsState.notifyOnDownloadComplete,
    notifyOnPlaybackChange: settingsState.notifyOnPlaybackChange,
    logLevel: settingsState.logLevel,
    locale: settingsState.locale,
    theme: JSON.stringify({
      presetId: settingsState.themePresetId,
      customColors: settingsState.themeCustomColors,
      colorScheme: settingsState.colorScheme,
    }),
  };

  // --- 本地派生 ---

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

  // --- 本地辅助 ---

  function getSettingsSnapshot() {
    return JSON.stringify({
      format: settingsState.format,
      outputDir: settingsState.outputDir,
      downloadLyrics: settingsState.downloadLyrics,
      notifyOnDownloadComplete: settingsState.notifyOnDownloadComplete,
      notifyOnPlaybackChange: settingsState.notifyOnPlaybackChange,
      logLevel: settingsState.logLevel,
      locale: settingsState.locale,
      theme: {
        presetId: settingsState.themePresetId,
        customColors: settingsState.themeCustomColors,
        colorScheme: settingsState.colorScheme,
      },
    });
  }

  function getThemeSettingsSnapshot() {
    return JSON.stringify({
      presetId: settingsState.themePresetId,
      customColors: settingsState.themeCustomColors,
      colorScheme: settingsState.colorScheme,
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

  function handleOutputDirChange() {
    return settingsController.savePreferences(settingsState);
  }

  async function handleRefresh() {
    if (isRefreshing) return;
    isRefreshing = true;
    selectionManager.clearSongSelection();
    selectionManager.setSelectionModeEnabled(false);
    try {
      await clearCache();
      await clearResponseCache();
      await libraryController.reloadAlbumsAndRefreshCurrentSelection({
        afterSelect: async () => {
          await tick();
          navigationManager.resetContentScroll();
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

  // --- 设置 dirty tracking effects ---

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
    const value = getThemeSettingsSnapshot();
    if (settingsState.suspendDirtyTracking > 0) {
      lastObservedSettings.theme = value;
      return;
    }
    if (value !== lastObservedSettings.theme) {
      settingsState.dirty.theme = true;
      lastObservedSettings.theme = value;
    }
  });

  $effect(() => {
    const { persistedSnapshot, isSaving, lastSaveFailedSnapshot, prefsReady } =
      settingsState;
    if (!prefsReady || isSaving) return;
    const currentSnapshot = getSettingsSnapshot();
    if (currentSnapshot === persistedSnapshot) return;
    if (currentSnapshot === lastSaveFailedSnapshot) return;
    void settingsController.savePreferences(settingsState);
  });

  $effect(() => {
    albumStageMotionController.albumStageElement = albumStageElement;
  });

  // --- 生命周期 ---

  async function doBootstrapApp(shouldDispose: () => boolean) {
    await bootstrapApp(
      {
        warmCacheManager,
        settingsController,
        settingsState,
        libraryController,
        getDefaultOutputDir,
        getLocalInventorySnapshot,
        clearSongSelection: () => selectionManager.clearSongSelection(),
        setSelectionModeEnabled: (value) =>
          selectionManager.setSelectionModeEnabled(value),
        resetContentScroll: () => navigationManager.resetContentScroll(),
        tick,
        downloadController,
        listDownloadJobs,
        playerController,
        getPlayerState,
        getPlayerStateInitSeq: () => playerStateInitSeq,
        incrementPlayerStateInitSeq: () => ++playerStateInitSeq,
        getPlayerStateHydratedFromEvent: () => playerStateHydratedFromEvent,
        homeController,
      },
      shouldDispose
    );
  }

  async function doSubscribeToTauriEvents(shouldDispose: () => boolean) {
    return subscribeToTauriEvents(
      {
        listen,
        playerController,
        downloadController,
        libraryController,
        homeController,
        handleAppErrorEvent,
        clearSongSelection: () => selectionManager.clearSongSelection(),
        setSelectionModeEnabled: (value) =>
          selectionManager.setSelectionModeEnabled(value),
        invalidateInventoryCaches,
        setPlayerStateHydratedFromEvent: (value) => {
          playerStateHydratedFromEvent = value;
        },
      },
      shouldDispose
    );
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
    searchController.dispose();
    navigationStack.clear();
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

    const handleOnline = () => {
      void resetHttpClient().catch(() => {});
    };
    window.addEventListener('online', handleOnline);

    void (async () => {
      try {
        const nextUnsubscribe = await doSubscribeToTauriEvents(() => disposed);
        // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- async race guard
        if (disposed) {
          nextUnsubscribe();
          return;
        }
        unsubscribe = nextUnsubscribe;

        await doBootstrapApp(() => disposed);
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
      window.removeEventListener('online', handleOnline);
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

      gsapScrollIntoView(contentEl, row, 'center');
      libraryController.clearPendingScrollToSong(expectedSongCid);
    });
  });

  let sidebarStateBeforeTagEditor: boolean | null = null;

  $effect(() => {
    const view = shellStore.currentView;
    if (view === 'tagEditor') {
      if (sidebarStateBeforeTagEditor === null) {
        sidebarStateBeforeTagEditor = shellStore.sidebarCollapsed;
      }
      if (!shellStore.sidebarCollapsed) {
        shellStore.setSidebarCollapsedTransient(true);
      }
    } else if (sidebarStateBeforeTagEditor !== null) {
      shellStore.setSidebarCollapsedTransient(sidebarStateBeforeTagEditor);
      sidebarStateBeforeTagEditor = null;
    }
  });

  // --- 公共 API ---

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
      return themeManager.selectedAlbumArtworkUrl;
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
    get isPlayTogglePending() {
      return playerController.isPlayTogglePending;
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
    get playbackFormat() {
      return playbackFormat;
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
      return selectionManager.selectionModeEnabled;
    },
    get selectedSongCids() {
      return selectionManager.selectedSongCids;
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
    searchController,
    notifyInfo,
    notifyError,
    handleSelectAlbum: navigationManager.handleSelectAlbum,
    handleDeselectAlbum: libraryController.deselectAlbum,
    handleSelectSearchResult: navigationManager.handleSelectSearchResult,
    handlePlay: navigationManager.handlePlay,
    handlePlayCollectionSong: navigationManager.handlePlayCollectionSong,
    handleRefresh,
    handleContentWheel,
    handleToggleDownloads: downloadBridge.handleToggleDownloads,
    handleToggleSettings: downloadBridge.handleToggleSettings,
    handleOutputDirChange,
    handleDownloadSelection: downloadBridge.handleDownloadSelection,
    handleCurrentSongDownload: downloadBridge.handleCurrentSongDownload,
    hasAlbumDownloadJob: downloadBridge.hasAlbumDownloadJob,
    hasCurrentSelectionJob: downloadBridge.hasCurrentSelectionJob,
    toggleSelectionMode: selectionManager.toggleSelectionMode,
    selectAllSongs: selectionManager.selectAllSongs,
    deselectAllSongs: selectionManager.deselectAllSongs,
    invertSongSelection: selectionManager.invertSongSelection,
    toggleSongSelection: selectionManager.toggleSongSelection,
    isSongSelected: selectionManager.isSongSelected,
    get canGoBack() {
      return navigationManager.canGoBack;
    },
    get isNavigating() {
      return navigationManager.isNavigating;
    },
    get isViewTransitioning() {
      return navigationManager.isViewTransitioning;
    },
    handleTransitionStart: navigationManager.handleTransitionStart,
    handleTransitionEnd: navigationManager.handleTransitionEnd,
    get navigationDirection() {
      return navigationManager.navigationDirection;
    },
    navigateToTop: navigationManager.navigateToTop,
    openAlbum: navigationManager.openAlbum,
    openCollection: navigationManager.openCollection,
    openTagEditor: navigationManager.openTagEditor,
    goBack: navigationManager.goBack,
  };
}

export type AppRuntime = ReturnType<typeof createAppRuntime>;
