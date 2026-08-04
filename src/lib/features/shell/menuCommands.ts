/**
 * 应用菜单栏命令分发。
 *
 * 后端 `menu.rs` 在用户点击自定义菜单项时，会通过 `app-menu-command` 事件
 * 广播一个稳定 ID。前端在 `appRuntime` 里只挂一次监听，把事件转交给
 * [`dispatchMenuCommand`]；此函数按 ID 查表调用对应 controller，把菜单命令
 * 集中到一处以便审阅与扩展。
 *
 * ID 命名规范来自 `src-tauri/src/menu.rs::ids`，形如 `app.<group>.<action>`。
 */
import {
  open as openDialog,
  save as saveDialog,
} from '@tauri-apps/plugin-dialog';
import type { AppPreferences, ColorScheme, RepeatMode } from '$lib/types';
import {
  exportPreferences,
  importPreferences,
  rescanLocalInventory,
  sendTestNotification,
} from '$lib/api';
import * as m from '$lib/paraglide/messages.js';
import type { AppView } from '$lib/features/shell/store.svelte';
import type { SettingsSection } from '$lib/components/app/shell/settingsSection';

const VOLUME_STEP = 0.05;
const SEEK_STEP_SECONDS = 10;

/**
 * 菜单命令消费所需的 controller 与 shell 能力。
 *
 * 只列出当前批次的自定义菜单项涉及的方法；后续扩展新命令时按需在此追加。
 */
export interface MenuCommandDeps {
  runtime: {
    handleRefresh: () => Promise<void> | void;
    handleToggleDownloads: () => Promise<void> | void;
    handleToggleSettings: () => Promise<void> | void;
    openSettingsAt: (section: SettingsSection) => Promise<void> | void;
    toggleSidebar: () => void;
    navigateToTop: (view: AppView) => void;
    goBack: () => Promise<void> | void;
    requestClearListeningHistory: () => void;
    readonly canGoBack: boolean;
  };
  playerController: {
    readonly isPlaying: boolean;
    readonly volume: number;
    readonly progress: number;
    readonly duration: number;
    readonly shuffleEnabled: boolean;
    readonly repeatMode: RepeatMode;
    pause: () => Promise<void> | void;
    resume: () => Promise<void> | void;
    playNext: () => Promise<void> | void;
    playPrevious: () => Promise<void> | void;
    seek: (position: number) => Promise<void> | void;
    setVolume: (volume: number) => Promise<void> | void;
    toggleMute: () => Promise<void> | void;
    toggleShuffle: (next: boolean) => void;
    toggleRepeat: (next: RepeatMode) => void;
    toggleLyrics: () => void;
    togglePlaylist: () => void;
    toggleFullscreen: () => void;
  };
  settingsController: {
    state: { colorScheme: ColorScheme };
    applyPreferencesSnapshot: (
      preferences: AppPreferences,
      options?: { force?: boolean }
    ) => void;
  };
  collectionController: {
    readonly selectedCollectionId: string | null;
    openCreateDialog: () => void;
    handleImport: () => Promise<void>;
    handleExport: (id: string) => Promise<void>;
  };
  tagEditorController: {
    importRegistry: () => Promise<void>;
    exportRegistry: () => Promise<void>;
  };
  downloadController: {
    handleClearDownloadHistory: () => Promise<void>;
  };
  notifyError: (message: string) => void;
  notifyInfo: (message: string) => void;
}

const clamp01 = (value: number): number => Math.max(0, Math.min(1, value));

const clampNonNegative = (value: number): number => Math.max(0, value);

/**
 * 按菜单命令 ID 分发到对应 controller。
 *
 * 未识别的 ID 会被静默忽略（后端与前端可能异步版本不一致，忽略比崩溃更稳），
 * 未来引入 telemetry 时可在此增加计数。异步动作抛出的错误统一走
 * `deps.notifyError`。
 */
export async function dispatchMenuCommand(
  id: string,
  deps: MenuCommandDeps
): Promise<void> {
  try {
    await handle(id, deps);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    deps.notifyError(message);
  }
}

async function handle(id: string, deps: MenuCommandDeps): Promise<void> {
  const {
    runtime,
    playerController,
    settingsController,
    collectionController,
    tagEditorController,
    downloadController,
    notifyError,
    notifyInfo,
  } = deps;
  switch (id) {
    case 'app.app.preferences':
      await runtime.handleToggleSettings();
      return;
    case 'app.app.test_notification':
      await sendTestNotification();
      notifyInfo(m.menu_test_notification_sent());
      return;
    case 'app.file.new_collection':
      runtime.navigateToTop('collection');
      collectionController.openCreateDialog();
      return;
    case 'app.file.import_collection':
      await collectionController.handleImport();
      return;
    case 'app.file.export_collection': {
      const id = collectionController.selectedCollectionId;
      if (!id) {
        notifyError(m.menu_export_collection_no_selection());
        return;
      }
      await collectionController.handleExport(id);
      return;
    }
    case 'app.file.import_tag_registry':
      await tagEditorController.importRegistry();
      return;
    case 'app.file.export_tag_registry':
      await tagEditorController.exportRegistry();
      return;
    case 'app.file.import_preferences': {
      const path = await openDialog({
        filters: [{ name: 'TOML', extensions: ['toml'] }],
      });
      if (typeof path !== 'string') return;
      const preferences = await importPreferences(path);
      settingsController.applyPreferencesSnapshot(preferences, { force: true });
      notifyInfo(m.menu_preferences_imported());
      return;
    }
    case 'app.file.export_preferences': {
      const path = await saveDialog({
        defaultPath: 'harubble-preferences.toml',
        filters: [{ name: 'TOML', extensions: ['toml'] }],
      });
      if (typeof path !== 'string') return;
      await exportPreferences(path);
      notifyInfo(m.menu_preferences_exported());
      return;
    }
    case 'app.file.clear_listening_history':
      runtime.requestClearListeningHistory();
      return;
    case 'app.file.clear_download_history':
      await downloadController.handleClearDownloadHistory();
      return;
    case 'app.view.home':
      runtime.navigateToTop('home');
      return;
    case 'app.view.search':
      runtime.navigateToTop('search');
      return;
    case 'app.view.overview':
      runtime.navigateToTop('overview');
      return;
    case 'app.view.library':
      runtime.navigateToTop('library');
      return;
    case 'app.view.collection':
      runtime.navigateToTop('collection');
      return;
    case 'app.view.tag_editor':
      runtime.navigateToTop('tagEditor');
      return;
    case 'app.view.go_back':
      if (runtime.canGoBack) await runtime.goBack();
      return;
    case 'app.view.toggle_sidebar':
      runtime.toggleSidebar();
      return;
    case 'app.view.toggle_downloads':
      await runtime.handleToggleDownloads();
      return;
    case 'app.view.refresh':
      await runtime.handleRefresh();
      return;
    case 'app.view.rescan_inventory':
      await rescanLocalInventory();
      notifyInfo(m.menu_rescan_inventory_started());
      return;
    case 'app.view.logs':
      await runtime.openSettingsAt('logs');
      return;
    case 'app.view.appearance.auto':
      settingsController.state.colorScheme = 'auto';
      return;
    case 'app.view.appearance.light':
      settingsController.state.colorScheme = 'light';
      return;
    case 'app.view.appearance.dark':
      settingsController.state.colorScheme = 'dark';
      return;
    case 'app.playback.toggle':
      if (playerController.isPlaying) {
        await playerController.pause();
      } else {
        await playerController.resume();
      }
      return;
    case 'app.playback.next':
      await playerController.playNext();
      return;
    case 'app.playback.previous':
      await playerController.playPrevious();
      return;
    case 'app.playback.seek_forward': {
      if (playerController.duration <= 0) return;
      const target = Math.min(
        playerController.duration,
        clampNonNegative(playerController.progress + SEEK_STEP_SECONDS)
      );
      await playerController.seek(target);
      return;
    }
    case 'app.playback.seek_backward': {
      if (playerController.duration <= 0) return;
      const target = clampNonNegative(
        playerController.progress - SEEK_STEP_SECONDS
      );
      await playerController.seek(target);
      return;
    }
    case 'app.playback.volume_up':
      await playerController.setVolume(
        clamp01(playerController.volume + VOLUME_STEP)
      );
      return;
    case 'app.playback.volume_down':
      await playerController.setVolume(
        clamp01(playerController.volume - VOLUME_STEP)
      );
      return;
    case 'app.playback.toggle_mute':
      await playerController.toggleMute();
      return;
    case 'app.playback.toggle_shuffle':
      playerController.toggleShuffle(!playerController.shuffleEnabled);
      return;
    case 'app.playback.repeat.off':
      if (playerController.repeatMode !== 'off') {
        playerController.toggleRepeat('off');
      }
      return;
    case 'app.playback.repeat.all':
      if (playerController.repeatMode !== 'all') {
        playerController.toggleRepeat('all');
      }
      return;
    case 'app.playback.repeat.one':
      if (playerController.repeatMode !== 'one') {
        playerController.toggleRepeat('one');
      }
      return;
    case 'app.playback.toggle_lyrics':
      playerController.toggleLyrics();
      return;
    case 'app.playback.toggle_playlist':
      playerController.togglePlaylist();
      return;
    case 'app.playback.toggle_fullscreen':
      playerController.toggleFullscreen();
      return;
    default:
      return;
  }
}
