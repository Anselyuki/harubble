/**
 * MiniPlayerWindow 的受控 IPC 例外层。
 *
 * MiniPlayerWindow 在独立的 secondary window 中运行，无法共享主窗口的 appRuntime。
 * 因此它需要直接订阅 Tauri 事件并调用 API — 这是唯一被允许在普通 UI 组件外
 * 直接使用 listen() 的场所。普通 UI 组件不应复制此模式。
 */
import { listen } from '@tauri-apps/api/event';
import type { PlayerState } from '$lib/types';
import {
  getPlayerState,
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
