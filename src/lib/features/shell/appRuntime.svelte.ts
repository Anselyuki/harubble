import { tick } from 'svelte';
import { listen } from '@tauri-apps/api/event';
import type { PartialOptions } from 'overlayscrollbars';
import { gsapScrollIntoView } from '$lib/design/gsap';
import {
  getDefaultOutputDir,
  getPlayerState,
  resetHttpClient,
  listDownloadJobs,
  getLocalInventorySnapshot,
  syncPlaybackMenuState,
} from '$lib/api';
import {
  createInventoryCacheTag,
  invalidateByTag,
  warmCacheManager,
} from '$lib/cache';
import type { AppErrorEvent } from '$lib/types';
import { envStore } from '$lib/features/env/store.svelte';
import { shellStore } from '$lib/features/shell/store.svelte';
import { navigationStack } from './navigation.svelte';
import {
  bootstrapApp,
  subscribeToTauriEvents,
} from '$lib/features/shell/appRuntimeBootstrap.svelte';
import { createRuntimeComposites } from '$lib/features/shell/appRuntimeComposites.svelte';
import { createAlbumCatalogRefreshScheduler } from '$lib/features/library/albumCatalogRefreshScheduler';
import * as m from '$lib/paraglide/messages.js';
import { toast } from 'svelte-sonner';
import { dispatchMenuCommand } from '$lib/features/shell/menuCommands';

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

  const {
    albumCatalogController,
    settingsController,
    libraryController,
    playerController,
    downloadController,
    homeController,
    tagEditorController,
    searchController,
    collectionController,
    albumStageMotionController,
    selectionManager,
    themeManager,
    navigationManager,
    downloadBridge,
  } = createRuntimeComposites(notifyInfo, notifyError);

  const albumCatalogRefreshScheduler = createAlbumCatalogRefreshScheduler({
    ensureFresh: () => albumCatalogController.ensureFresh(),
    isDocumentVisible: () => document.visibilityState === 'visible',
    timers: window,
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

  function handleContentWheel(event: WheelEvent) {
    albumStageMotionController.handleContentWheel(event);
  }

  function handleAppErrorEvent(event: AppErrorEvent) {
    notifyError(event.message);
    settingsController.handleAppError(shellStore.settingsOpen);
  }

  async function invalidateInventoryCaches(
    inventoryVersion: string | null | undefined
  ) {
    await invalidateByTag(createInventoryCacheTag(inventoryVersion));
  }

  function handleOutputDirChange() {
    return settingsController.savePreferences();
  }

  async function handleRefresh() {
    if (isRefreshing) return;
    isRefreshing = true;
    selectionManager.clearSongSelection();
    selectionManager.setSelectionModeEnabled(false);
    try {
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

  $effect(() => {
    albumStageMotionController.albumStageElement = albumStageElement;
  });

  // --- 生命周期 ---

  async function doBootstrapApp(shouldDispose: () => boolean) {
    await bootstrapApp(
      {
        warmCacheManager,
        settingsController,
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
        searchController,
        albumCatalogController,
        homeController,
        handleAppErrorEvent,
        clearSongSelection: () => selectionManager.clearSongSelection(),
        setSelectionModeEnabled: (value) =>
          selectionManager.setSelectionModeEnabled(value),
        invalidateInventoryCaches,
        setPlayerStateHydratedFromEvent: (value) => {
          playerStateHydratedFromEvent = value;
        },
        handleMenuCommand,
      },
      shouldDispose
    );
  }

  function handleMenuCommand(id: string): Promise<void> {
    return dispatchMenuCommand(id, {
      runtime: {
        handleRefresh,
        handleToggleDownloads: downloadBridge.handleToggleDownloads,
        handleToggleSettings: downloadBridge.handleToggleSettings,
        openSettingsAt: async (section) => {
          await shellStore.openSettings({ notifyError, section });
        },
        toggleSidebar: () => shellStore.toggleSidebar(),
        navigateToTop: navigationManager.navigateToTop,
        goBack: navigationManager.goBack,
        get canGoBack() {
          return navigationManager.canGoBack;
        },
      },
      playerController,
      settingsController,
      collectionController: {
        get selectedCollectionId() {
          return collectionController.selectedCollectionId;
        },
        openCreateDialog: collectionController.openCreateDialog,
        handleImport: collectionController.handleImport,
        handleExport: collectionController.handleExport,
      },
      tagEditorController: {
        importRegistry: tagEditorController.importRegistry,
        exportRegistry: tagEditorController.exportRegistry,
      },
      homeController: {
        handleClearHistory: homeController.handleClearHistory,
      },
      downloadController: {
        handleClearDownloadHistory:
          downloadController.handleClearDownloadHistory,
      },
      notifyError,
      notifyInfo,
    });
  }

  function teardownAppRuntime(unsubscribe: (() => void) | null) {
    shellStore.dispose();
    envStore.dispose();
    libraryController.dispose();
    albumCatalogController.dispose();
    playerController.dispose();
    downloadController.dispose();
    albumStageMotionController.dispose();
    homeController.dispose();
    tagEditorController.dispose();
    searchController.dispose();
    settingsController.dispose();
    navigationStack.clear();
    playerStateInitSeq += 1;
    playerStateHydratedFromEvent = false;
    unsubscribe?.();
  }

  $effect(() => {
    settingsController.init();
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
      void resetHttpClient()
        .catch(() => {})
        .finally(() => albumCatalogRefreshScheduler.handleOnline());
    };
    const handleVisibilityChange = () => {
      albumCatalogRefreshScheduler.handleVisibilityChange();
    };
    const handleFocus = () => {
      albumCatalogRefreshScheduler.handleFocus();
    };
    window.addEventListener('online', handleOnline);
    window.addEventListener('focus', handleFocus);
    document.addEventListener('visibilitychange', handleVisibilityChange);
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
      window.removeEventListener('focus', handleFocus);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      albumCatalogRefreshScheduler.dispose();
      teardownAppRuntime(unsubscribe);
    };
  });

  $effect(() => {
    albumCatalogRefreshScheduler.setActive(
      shellStore.currentView === 'home' || shellStore.currentView === 'library'
    );
  });

  $effect(() => {
    playerController.syncPlaybackLifecycle();
  });

  $effect(() => {
    const repeat = playerController.repeatMode;
    const shuffle = playerController.shuffleEnabled;
    // 依赖 locale 以便菜单在语言切换重建后立即同步勾选态。
    void settingsController.state.locale;
    void syncPlaybackMenuState(repeat, shuffle).catch(() => {});
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
    get settingsState() {
      return settingsController.state;
    },
    shellStore,
    albumCatalogController,
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
