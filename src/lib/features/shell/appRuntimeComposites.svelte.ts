/**
 * 运行时复合体工厂模块。
 *
 * 负责在 `createAppRuntime` 生命周期内统一创建所有子控制器与子模块实例，
 * 并通过懒引用闭包安全处理控制器间的前向依赖（如 playerController 引用
 * homeController、libraryController 引用 themeManager 等）。
 *
 * 调用方（appRuntime.svelte.ts）只需传入两个通知回调，即可获得所有
 * 已组装好的控制器与模块引用。
 */

import {
  open as openDialog,
  save as saveDialog,
} from '@tauri-apps/plugin-dialog';
import {
  getAlbums,
  getAlbumDetail,
  playSong,
  playNext as playNextTrack,
  playPrevious as playPreviousTrack,
  pausePlayback,
  resumePlayback,
  seekCurrentPlayback,
  getPlayerState,
  setPlaybackVolume,
  getSongLyrics,
  recordSongHeat,
  createDownloadJob,
  cancelDownloadJob,
  cancelDownloadTask,
  retryDownloadJob,
  retryDownloadTask,
  clearDownloadHistory,
  getPreferences,
  setPreferences,
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
import { createSearchController } from '$lib/features/search/controller.svelte';
import { createSelectionManager } from './selectionManager.svelte';
import { createThemeManager } from './themeManager.svelte';
import { createNavigationManager } from './navigationManager.svelte';
import { createDownloadBridge } from './downloadBridge.svelte';
import { localeState } from '$lib/i18n';

export interface RuntimeComposites {
  settingsController: ReturnType<typeof createSettingsController>;
  libraryController: ReturnType<typeof createLibraryController>;
  playerController: ReturnType<typeof createPlayerController>;
  downloadController: ReturnType<typeof createDownloadController>;
  homeController: ReturnType<typeof createHomeController>;
  tagEditorController: ReturnType<typeof createTagEditorController>;
  searchController: ReturnType<typeof createSearchController>;
  collectionController: ReturnType<typeof createCollectionController>;
  albumStageMotionController: ReturnType<
    typeof createAlbumStageMotionController
  >;
  selectionManager: ReturnType<typeof createSelectionManager>;
  themeManager: ReturnType<typeof createThemeManager>;
  navigationManager: ReturnType<typeof createNavigationManager>;
  downloadBridge: ReturnType<typeof createDownloadBridge>;
}

/**
 * 创建应用运行时所需的全部子控制器与子模块实例。
 *
 * @param notifyInfo - 展示普通提示 toast 的回调
 * @param notifyError - 展示错误 toast 的回调
 * @returns 包含所有已初始化控制器与模块的 {@link RuntimeComposites} 对象
 */
export function createRuntimeComposites(
  notifyInfo: (message: string) => void,
  notifyError: (message: string) => void
): RuntimeComposites {
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

  // homeController を前方参照するため、playerController より先に let 宣言する。
  // playerController の recordSongHeat クロージャは呼び出し時点で homeController が
  // 確実に代入済みとなるため、非 null アサーション付き宣言で安全に参照できる。
  // eslint-disable-next-line svelte/prefer-const -- forward reference: assigned below before first use
  let homeController!: ReturnType<typeof createHomeController>;

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
      outputDir: settingsController.state.outputDir,
      format: settingsController.state.format,
      downloadLyrics: settingsController.state.downloadLyrics,
    }),
    notifyInfo,
    notifyError,
  });

  homeController = createHomeController({
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
      presetId: settingsController.state.themePresetId,
      customColors: settingsController.state.themeCustomColors,
      colorScheme: settingsController.state.colorScheme,
      dynamicAlbumAccent: settingsController.state.dynamicAlbumAccent,
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

  return {
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
  };
}

// --- モジュールプライベート定数（appRuntime.svelte.ts と共有） ---

const MIN_DISPLAY_MS = 120;
const DETAIL_SKELETON_DELAY_MS = 140;

const delay = (ms: number): Promise<void> =>
  new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
