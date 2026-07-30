/**
 * MiniPlayerWindow 的受控 IPC 例外层。
 *
 * MiniPlayerWindow 在独立的 secondary window 中运行，无法共享主窗口的 appRuntime。
 * 因此它需要直接订阅 Tauri 事件并调用 API — 这是唯一被允许在普通 UI 组件外
 * 直接使用 listen() 的场所。普通 UI 组件不应复制此模式。
 */
import { listen } from '@tauri-apps/api/event';
import type { AppPreferences, PlayerState } from '$lib/types';
import {
  getPlayerState,
  getPreferences,
  inspectThemePackage,
  pausePlayback,
  resumePlayback,
  playNext,
  playPrevious,
  seekCurrentPlayback,
  showMainWindow,
} from '$lib/api';

export type { PlayerState };

export {
  getPlayerState,
  getPreferences,
  inspectThemePackage,
  pausePlayback,
  resumePlayback,
  playNext,
  playPrevious,
  seekCurrentPlayback,
  showMainWindow,
};

export function listenPlayerStateChanged(
  handler: (state: PlayerState) => void
): Promise<() => void> {
  return listen<PlayerState>('player-state-changed', (event) =>
    handler(event.payload)
  );
}

export function listenPlayerProgress(
  handler: (state: PlayerState) => void
): Promise<() => void> {
  return listen<PlayerState>('player-progress', (event) =>
    handler(event.payload)
  );
}

/**
 * 订阅 preferences_snapshot 广播事件。
 *
 * 后端在偏好写入成功后广播完整快照（含 theme.revision）；Mini Player 通过此订阅
 * 获取主窗口发起的主题变更，并在自身 DOM 上重新应用 token。
 * 消费者应传入 revision-based 单调 reducer，避免乱序事件回滚态。
 */
export function listenPreferencesSnapshot(
  handler: (snapshot: AppPreferences) => void
): Promise<() => void> {
  return listen<AppPreferences>('preferences_snapshot', (event) =>
    handler(event.payload)
  );
}
