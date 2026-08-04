import type {
  PlayerState,
  PlaybackEndedEvent,
  DownloadManagerSnapshot,
  DownloadJobSnapshot,
  DownloadTaskProgressEvent,
  LocalInventorySnapshot,
  LocalInventoryScanProgressEvent,
  AppErrorEvent,
  LibraryIndexState,
  AlbumCatalogRefreshedEvent,
  AppPreferences,
} from '$lib/types';

/**
 * 应用全局 Tauri 事件名到载荷类型的映射表。
 *
 * 通过 `typedListen` 使用此映射，让事件名拼写错误在编译期暴露。
 */
export interface AppEventMap {
  'player-state-changed': PlayerState;
  'player-progress': PlayerState;
  'player-ended': PlaybackEndedEvent;
  'download-manager-state-changed': DownloadManagerSnapshot;
  'download-job-updated': DownloadJobSnapshot;
  'download-task-progress': DownloadTaskProgressEvent;
  'app-error-recorded': AppErrorEvent;
  'local-inventory-state-changed': LocalInventorySnapshot;
  'local-inventory-scan-progress': LocalInventoryScanProgressEvent;
  'library-search-index-state-changed': LibraryIndexState;
  'album-catalog-refreshed': AlbumCatalogRefreshedEvent;
  'homepage-belong-ready': void;
  'app-menu-command': { id: string };
  preferences_snapshot: AppPreferences;
}

export type AppEventName = keyof AppEventMap;
