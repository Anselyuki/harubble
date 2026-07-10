import { tick } from 'svelte';
import { listen } from '@tauri-apps/api/event';
import {
  open as openDialog,
  save as saveDialog,
} from '@tauri-apps/plugin-dialog';
import type { PartialOptions } from 'overlayscrollbars';
import { gsapScrollIntoView } from '$lib/design/gsap';
import {
  getAlbums,
  getAlbumDetail,
  getDefaultOutputDir,
  playSong,
  playNext as playNextTrack,
  playPrevious as playPreviousTrack,
  pausePlayback,
  resumePlayback,
  seekCurrentPlayback,
  getPlayerState,
  setPlaybackVolume,
  clearResponseCache,
  resetHttpClient,
  getSongLyrics,
  recordSongHeat,
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

const MIN_DISPLAY_MS = 120;
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
    warmAlbumArtwork: (coverUrl) => themeManager.warmAlbumArtwork(coverUrl),
    setAlbumStageAspectRatio: (value) =>
      albumStageMotionController.setAspectRatio(value),
    notifyError,
  });

  const playerController = createPlayerController({
    playSong: async (songCid, coverUrl, context) => {
      await playSong(songCid, coverUrl ?? undefined, context ?? undefined);
    },
    playNextTrack: async () => {
      await playNextTrack();
    },
    playPreviousTrack: async () => {
      await playPreviousTrack();
    },
    pausePlayback,
    resumePlayback,
    seekCurrentPlayback: async (positionSecs) => {
      await seekCurrentPlayback(positionSecs);
    },
    setPlaybackVolume,
    getPlayerState,
    getSongLyrics,
    recordSongHeat: (songCid, coverUrl) => {
      // 热度是尽力而为的统计信号（元数据抓取失败、Tauri IPC 断开等都可能触发），
      // 失败时不能弹 toast 打扰用户，也不能变成 unhandled rejection——加一个显式
      // catch 把异常吞掉。真实的播放链路错误由其它更权威的通道呈现。
      void recordSongHeat(songCid, coverUrl)
        .then(() => {
          void homeController.refreshRecentHistory();
        })
        .catch(() => {});
    },
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
    pickSavePath: (defaultName) =>
      saveDialog({
        defaultPath: defaultName,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      }),
    pickOpenPath: () =>
      openDialog({
        filters: [{ name: 'JSON', extensions: ['json'] }],
      }),
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
    getCurrentView: () => shellStore.currentView,
    getFullscreenOpen: () => playerController.fullscreenOpen,
    getSettingsTheme: () => ({
      presetId: settingsState.themePresetId,
      customColors: settingsState.themeCustomColors,
      colorScheme: settingsState.colorScheme,
      dynamicAlbumAccent: settingsState.dynamicAlbumAccent,
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

  const pendingScrollToSongCid = $derived(
    libraryController.pendingScrollToSongCid
  );
  const contentEl = $derived(albumStageMotionController.contentElement);

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
    dynamicAlbumAccent: DEFAULT_THEME_PREFERENCES.dynamicAlbumAccent ?? true,
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
      dynamicAlbumAccent: settingsState.dynamicAlbumAccent,
    }),
  };

  // --- 本地派生 ---

  const activeLyricIndex = $derived.by(() => {
    if (!playerController.lyricsOpen && !playerController.fullscreenOpen)
      return -1;
    let activeIndex = -1;
    const lines = playerController.lyricsLines;
    for (let index = 0; index < lines.length; index += 1) {
      const lineTime = lines[index].time;
      if (lineTime === null) continue;
      if (playerController.progress + 0.08 >= lineTime) {
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
        autoHide: envStore.prefersReducedMotion ? 'leave' : 'move',
        autoHideDelay: envStore.prefersReducedMotion ? 160 : 720,
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
        dynamicAlbumAccent: settingsState.dynamicAlbumAccent,
      },
    });
  }

  function getThemeSettingsSnapshot() {
    return JSON.stringify({
      presetId: settingsState.themePresetId,
      customColors: settingsState.themeCustomColors,
      colorScheme: settingsState.colorScheme,
      dynamicAlbumAccent: settingsState.dynamicAlbumAccent,
    });
  }

  function handleContentWheel(event: WheelEvent) {
    albumStageMotionController.handleContentWheel(event);
  }

  function handleAppErrorEvent(event: AppErrorEvent) {
    notifyError(event.message);
    settingsController.handleAppError(settingsState, shellStore.settingsOpen);
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

  const SCALAR_DIRTY_FIELDS = [
    'format',
    'outputDir',
    'downloadLyrics',
    'notifyOnDownloadComplete',
    'notifyOnPlaybackChange',
    'logLevel',
    'locale',
  ] as const;
  $effect(() => {
    const observed = lastObservedSettings as Record<string, unknown>;
    const state = settingsState as unknown as Record<string, unknown>;
    if (settingsState.suspendDirtyTracking > 0) {
      for (const field of SCALAR_DIRTY_FIELDS) {
        observed[field] = state[field];
      }
      return;
    }
    for (const field of SCALAR_DIRTY_FIELDS) {
      const value = state[field];
      if (value !== observed[field]) {
        settingsState.dirty[field] = true;
        observed[field] = value;
      }
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
    if (
      !pendingScrollToSongCid ||
      !libraryController.selectedAlbum ||
      libraryController.loadingDetail
    ) {
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
      return envStore.isMacOS;
    },
    get currentView() {
      return shellStore.currentView;
    },
    get albums() {
      return libraryController.albums;
    },
    get selectedAlbum() {
      return libraryController.selectedAlbum;
    },
    get selectedAlbumCid() {
      return libraryController.selectedAlbumCid;
    },
    get loadingAlbums() {
      return libraryController.loadingAlbums;
    },
    get loadingDetail() {
      return libraryController.loadingDetail;
    },
    get loadingAlbumCid(): string | null {
      return libraryController.loadingDetail
        ? libraryController.selectedAlbumCid
        : null;
    },
    get errorMsg() {
      return libraryController.errorMsg;
    },
    get librarySearchQuery() {
      return libraryController.librarySearchQuery;
    },
    get librarySearchScope() {
      return libraryController.librarySearchScope;
    },
    get librarySearchLoading() {
      return libraryController.librarySearchLoading;
    },
    get librarySearchResponse() {
      return libraryController.librarySearchResponse;
    },
    get showDetailSkeleton() {
      return libraryController.showDetailSkeleton;
    },
    get albumRequestSeq() {
      return libraryController.albumRequestSeq;
    },
    get selectedAlbumArtworkUrl() {
      return themeManager.selectedAlbumArtworkUrl;
    },
    get currentSong() {
      return playerController.currentSong;
    },
    get isPlaying() {
      return playerController.isPlaying;
    },
    get isPaused() {
      return playerController.isPaused;
    },
    get isLoading() {
      return playerController.isLoading;
    },
    get isPlayTogglePending() {
      return playerController.isPlayTogglePending;
    },
    get progress() {
      return playerController.progress;
    },
    get duration() {
      return playerController.duration;
    },
    get shuffleEnabled() {
      return playerController.shuffleEnabled;
    },
    get repeatMode() {
      return playerController.repeatMode;
    },
    get playbackOrder() {
      return playerController.playbackOrder;
    },
    get playbackFormat() {
      return playerController.playbackFormat;
    },
    get lyricsOpen() {
      return playerController.lyricsOpen;
    },
    get playlistOpen() {
      return playerController.playlistOpen;
    },
    get lyricsLoading() {
      return playerController.lyricsLoading;
    },
    get lyricsError() {
      return playerController.lyricsError;
    },
    get lyricsLines() {
      return playerController.lyricsLines;
    },
    get lyricsUnavailable() {
      return playerController.lyricsUnavailable;
    },
    get activeLyricIndex() {
      return activeLyricIndex;
    },
    get fullscreenOpen() {
      return playerController.fullscreenOpen;
    },
    get playerHasPrevious() {
      return playerController.playerHasPrevious;
    },
    get playerHasNext() {
      return playerController.playerHasNext;
    },
    get downloadingAlbumCid() {
      return downloadController.downloadingAlbumCid;
    },
    get activeDownloadCount() {
      return downloadController.activeDownloadCount;
    },
    get filteredDownloadJobs() {
      return downloadController.filteredJobs;
    },
    get hasDownloadHistory() {
      return downloadController.hasDownloadHistory;
    },
    get prefersReducedMotion() {
      return envStore.prefersReducedMotion;
    },
    get overlayScrollbarOptions() {
      return overlayScrollbarOptions;
    },
    get contentScrollbarEvents() {
      return albumStageMotionController.contentScrollbarEvents;
    },
    get albumStageStyle() {
      return albumStageMotionController.albumStageStyle;
    },
    get albumStageMediaHeight() {
      return albumStageMotionController.albumStageMediaHeight;
    },
    get albumStageScrimOpacity() {
      return albumStageMotionController.albumStageScrimOpacity;
    },
    get albumStageImageOpacity() {
      return albumStageMotionController.albumStageImageOpacity;
    },
    get albumStageImageTransform() {
      return albumStageMotionController.albumStageImageTransform;
    },
    get albumStageSolidifyOpacity() {
      return albumStageMotionController.albumStageSolidifyOpacity;
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
      return shellStore.settingsOpen;
    },
    get downloadPanelOpen() {
      return shellStore.downloadPanelOpen;
    },
    get SettingsSheetView() {
      return shellStore.SettingsSheetView;
    },
    get DownloadTasksSheetView() {
      return shellStore.DownloadTasksSheetView;
    },
    get isRefreshing() {
      return isRefreshing;
    },
    get currentSongDownloadState() {
      return playerController.currentSong
        ? downloadController.getSongDownloadState(
            playerController.currentSong.cid
          )
        : ('idle' as const);
    },
    get currentSongDownloadDisabled() {
      return playerController.currentSong
        ? downloadController.isSongDownloadInteractionBlocked(
            playerController.currentSong.cid
          )
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
