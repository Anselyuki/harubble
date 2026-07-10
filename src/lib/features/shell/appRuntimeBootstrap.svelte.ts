import type { listen as tauriListen } from '@tauri-apps/api/event';
import type {
  Album,
  PlayerState,
  PlaybackEndedEvent,
  DownloadManagerSnapshot,
  DownloadJobSnapshot,
  DownloadTaskProgressEvent,
  LocalInventorySnapshot,
  AppErrorEvent,
} from '$lib/types';
import type { AppEventMap } from '$lib/appEvents';
import type { SettingsState } from '$lib/features/shell/settings.svelte';
import * as m from '$lib/paraglide/messages.js';

/**
 * `bootstrapApp` 所需的外部依赖。
 *
 * 由 `createAppRuntime` 在调用时组装并传入，
 * 使 bootstrap 逻辑可独立于 Svelte 5 rune 作用域进行测试与维护。
 */
export interface BootstrapDeps {
  warmCacheManager: () => Promise<void>;
  settingsController: {
    hydratePreferences: (
      state: SettingsState,
      options: { shouldDispose?: () => boolean }
    ) => Promise<void>;
    applyDefaultOutputDir: (state: SettingsState, dir: string) => void;
  };
  settingsState: SettingsState;
  libraryController: {
    loadAlbums: (options: {
      shouldDispose?: () => boolean;
    }) => Promise<Album[]>;
    selectedAlbumCid: string | null;
    initializeInventory: (snapshot: LocalInventorySnapshot | null) => void;
    selectAlbum: (
      album: Album,
      options?: {
        shouldDispose?: () => boolean;
        afterSelect?: () => Promise<void>;
      }
    ) => Promise<void>;
  };
  getDefaultOutputDir: () => Promise<string>;
  getLocalInventorySnapshot: () => Promise<LocalInventorySnapshot>;
  clearSongSelection: () => void;
  setSelectionModeEnabled: (value: boolean) => void;
  resetContentScroll: () => void;
  tick: () => Promise<void>;
  downloadController: {
    beginHydrationAttempt: () => number;
    applyManagerSnapshot: (
      manager: DownloadManagerSnapshot,
      seq: number
    ) => void;
  };
  listDownloadJobs: () => Promise<DownloadManagerSnapshot>;
  playerController: {
    syncPlayerState: (state: PlayerState) => void;
  };
  getPlayerState: () => Promise<PlayerState>;
  getPlayerStateInitSeq: () => number;
  incrementPlayerStateInitSeq: () => number;
  getPlayerStateHydratedFromEvent: () => boolean;
  homeController: {
    loadHomepageData: () => Promise<void>;
  };
}

/**
 * `subscribeToTauriEvents` 所需的外部依赖。
 *
 * 封装了所有 Tauri 事件订阅所需的 controller 方法与回调，
 * 使事件注册逻辑可独立于 Svelte 5 rune 作用域。
 */
export interface EventSubscriptionDeps {
  listen: typeof tauriListen;
  playerController: {
    syncPlayerState: (state: PlayerState) => void;
    syncPlayerProgress: (state: PlayerState) => void;
    syncPlaybackEnded: (event: PlaybackEndedEvent) => void;
  };
  downloadController: {
    applyManagerEvent: (snapshot: DownloadManagerSnapshot) => void;
    applyJobUpdate: (snapshot: DownloadJobSnapshot) => void;
    applyTaskProgress: (event: DownloadTaskProgressEvent) => void;
  };
  libraryController: {
    handleInventoryStateChanged: (
      snapshot: LocalInventorySnapshot,
      options: {
        shouldDispose: () => boolean;
        invalidateInventoryCaches: (
          version: string | null | undefined
        ) => Promise<void>;
        onSelectionInvalidated: () => void;
      }
    ) => Promise<void>;
  };
  homeController: {
    handleBelongReady: () => void;
  };
  handleAppErrorEvent: (event: AppErrorEvent) => void;
  clearSongSelection: () => void;
  setSelectionModeEnabled: (value: boolean) => void;
  invalidateInventoryCaches: (
    version: string | null | undefined
  ) => Promise<void>;
  setPlayerStateHydratedFromEvent: (value: boolean) => void;
}
/**
 * 应用启动引导流程。
 *
 * 按顺序完成缓存预热、偏好加载、库存初始化、下载状态恢复与播放器状态同步，
 * 最后触发首页数据加载。每一步均检查 `shouldDispose` 以支持提前中断。
 */
export async function bootstrapApp(
  deps: BootstrapDeps,
  shouldDispose: () => boolean
): Promise<void> {
  try {
    await deps.warmCacheManager();
  } catch {
    // Keep startup usable if IndexedDB warm start is unavailable.
  }

  if (shouldDispose()) {
    return;
  }

  try {
    await deps.settingsController.hydratePreferences(deps.settingsState, {
      shouldDispose,
    });
  } catch {
    // Preferences hydration failure is already tolerated in controller.
  }

  const defaultDirPromise = deps.settingsState.outputDir
    ? Promise.resolve('')
    : deps.getDefaultOutputDir().catch(() => '');

  try {
    const albumList = await deps.libraryController.loadAlbums({
      shouldDispose,
    });

    const defaultDir = await defaultDirPromise;
    if (shouldDispose()) {
      return;
    }
    if (defaultDir) {
      deps.settingsController.applyDefaultOutputDir(
        deps.settingsState,
        defaultDir
      );
    }

    try {
      const snapshot = await deps.getLocalInventorySnapshot();
      if (shouldDispose()) {
        return;
      }
      deps.libraryController.initializeInventory(snapshot);
    } catch {
      if (!shouldDispose()) {
        deps.libraryController.initializeInventory(null);
      }
    }

    if (albumList.length > 0 && !deps.libraryController.selectedAlbumCid) {
      deps.clearSongSelection();
      deps.setSelectionModeEnabled(false);
      await deps.libraryController.selectAlbum(albumList[0], {
        shouldDispose,
        afterSelect: async () => {
          await deps.tick();
          deps.resetContentScroll();
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
      deps.settingsController.applyDefaultOutputDir(
        deps.settingsState,
        defaultDir
      );
    }
  }

  try {
    const requestSeq = deps.downloadController.beginHydrationAttempt();
    const manager = await deps.listDownloadJobs();
    if (shouldDispose()) {
      return;
    }
    deps.downloadController.applyManagerSnapshot(manager, requestSeq);
  } catch {
    // Download manager not available
  }

  try {
    const requestSeq = deps.incrementPlayerStateInitSeq();
    const playerState = await deps.getPlayerState();
    if (shouldDispose()) {
      return;
    }
    if (
      requestSeq === deps.getPlayerStateInitSeq() &&
      !deps.getPlayerStateHydratedFromEvent()
    ) {
      deps.playerController.syncPlayerState(playerState);
    }
  } catch {
    // Player not playing on startup
  }

  void deps.homeController.loadHomepageData();
}
/**
 * 注册所有 Tauri 后端事件监听器。
 *
 * 返回一个清理函数，调用后会取消所有已注册的事件订阅。
 * 每次注册后均检查 `shouldDispose` 以支持组件卸载时的提前中断。
 */
export async function subscribeToTauriEvents(
  deps: EventSubscriptionDeps,
  shouldDispose: () => boolean
): Promise<() => void> {
  const unlisteners: (() => void)[] = [];

  const cleanup = () => {
    while (unlisteners.length > 0) {
      unlisteners.pop()?.();
    }
  };

  async function register<K extends keyof AppEventMap>(
    eventName: K,
    handler: (event: { payload: AppEventMap[K] }) => void | Promise<void>
  ) {
    const unlisten = await deps.listen<AppEventMap[K]>(
      eventName,
      async (event) => {
        if (shouldDispose()) {
          return;
        }
        await handler(event);
      }
    );

    if (shouldDispose()) {
      unlisten();
      return false;
    }

    unlisteners.push(unlisten);
    return true;
  }

  try {
    if (
      !(await register('player-state-changed', (event) => {
        deps.setPlayerStateHydratedFromEvent(true);
        deps.playerController.syncPlayerState(event.payload);
      }))
    ) {
      return cleanup;
    }

    if (
      !(await register('player-progress', (event) => {
        deps.playerController.syncPlayerProgress(event.payload);
      }))
    ) {
      return cleanup;
    }
    if (
      !(await register('player-ended', (event) => {
        deps.playerController.syncPlaybackEnded(event.payload);
      }))
    ) {
      return cleanup;
    }
    if (
      !(await register('download-manager-state-changed', (event) => {
        deps.downloadController.applyManagerEvent(event.payload);
      }))
    ) {
      return cleanup;
    }

    if (
      !(await register('download-job-updated', (event) => {
        deps.downloadController.applyJobUpdate(event.payload);
      }))
    ) {
      return cleanup;
    }

    if (
      !(await register('download-task-progress', (event) => {
        deps.downloadController.applyTaskProgress(event.payload);
      }))
    ) {
      return cleanup;
    }

    if (
      !(await register('app-error-recorded', (event) => {
        deps.handleAppErrorEvent(event.payload);
      }))
    ) {
      return cleanup;
    }

    if (
      !(await register('local-inventory-state-changed', async (event) => {
        await deps.libraryController.handleInventoryStateChanged(
          event.payload,
          {
            shouldDispose,
            invalidateInventoryCaches: deps.invalidateInventoryCaches,
            onSelectionInvalidated: () => {
              deps.clearSongSelection();
              deps.setSelectionModeEnabled(false);
            },
          }
        );
      }))
    ) {
      return cleanup;
    }
    if (
      !(await register('homepage-belong-ready', () => {
        deps.homeController.handleBelongReady();
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
