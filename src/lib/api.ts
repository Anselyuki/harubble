import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import {
  cacheManager,
  createAlbumCacheTag,
  createInventoryCacheTag,
  createSongCacheTag,
} from './cache';
import type {
  Album,
  AlbumCatalogSnapshot,
  AlbumDetail,
  SongDetail,
  ThemePalette,
  PlayerState,
  PlaybackErrorPayload,
  PlaybackStartResult,
  PlaybackContext,
  CreateDownloadJobRequest,
  DownloadJobSnapshot,
  DownloadManagerSnapshot,
  AppPreferences,
  LocalInventorySnapshot,
  LogViewerPage,
  LogViewerQuery,
  LogFileStatus,
  SearchLibraryRequest,
  SearchLibraryResponse,
  SeriesGroup,
  HistoryEntry,
  HomepageStatus,
  TagDimension,
  TagGroup,
  TagEditorEntityType,
  TagEditorLocalizedValue,
  TagEditorRegistry,
  TagEditorMergeResult,
  ConflictResolution,
  AudioFileMetadata,
  RepeatMode,
  ThemePackageDocument,
  ThemePackageSummary,
} from './types';

const CACHE_KEY_ALBUM_DETAIL = 'album_detail:';
const CACHE_KEY_SONG_DETAIL = 'song_detail:';
const CACHE_KEY_SONG_LYRICS = 'song_lyrics:';
const CACHE_KEY_IMAGE_THEME = 'image_theme:';
const IMAGE_RESOURCE_CONCURRENCY_LIMIT = 1;

const inflightImageThemeRequests = new Map<string, Promise<ThemePalette>>();
const inflightImageSrcRequests = new Map<string, Promise<string>>();
const queuedImageResourceRequests: (() => void)[] = [];
let activeImageResourceRequestCount = 0;

export class PlaybackCommandError extends Error {
  readonly code: PlaybackErrorPayload['code'];
  readonly retryable: boolean;
  readonly sessionId: number | null;

  constructor(payload: PlaybackErrorPayload) {
    super(payload.message);
    this.name = 'PlaybackCommandError';
    this.code = payload.code;
    this.retryable = payload.retryable;
    this.sessionId = payload.sessionId;
  }
}

function isPlaybackErrorPayload(value: unknown): value is PlaybackErrorPayload {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as Partial<PlaybackErrorPayload>;
  return (
    typeof candidate.code === 'string' &&
    typeof candidate.message === 'string' &&
    typeof candidate.retryable === 'boolean'
  );
}

async function invokePlayback<T>(
  command: string,
  args?: Record<string, unknown>
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (isPlaybackErrorPayload(error)) {
      throw new PlaybackCommandError(error);
    }
    throw error;
  }
}

function scheduleImageResourceRequest<T>(task: () => Promise<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const run = () => {
      activeImageResourceRequestCount += 1;
      task()
        .then(resolve, reject)
        .finally(() => {
          activeImageResourceRequestCount = Math.max(
            activeImageResourceRequestCount - 1,
            0
          );
          queuedImageResourceRequests.shift()?.();
        });
    };

    if (activeImageResourceRequestCount < IMAGE_RESOURCE_CONCURRENCY_LIMIT) {
      run();
      return;
    }

    queuedImageResourceRequests.push(run);
  });
}

export async function getAlbums(): Promise<Album[]> {
  return invoke('get_albums');
}

export async function getAlbumCatalog(): Promise<AlbumCatalogSnapshot> {
  return invoke<AlbumCatalogSnapshot>('get_album_catalog');
}

export async function refreshAlbumCatalog(): Promise<AlbumCatalogSnapshot> {
  return invoke<AlbumCatalogSnapshot>('refresh_album_catalog');
}

export async function getAlbumDetail(
  albumCid: string,
  inventoryVersion?: string | null
): Promise<AlbumDetail> {
  const cacheScope = inventoryVersion ?? 'unversioned';
  const cacheKey = `${CACHE_KEY_ALBUM_DETAIL}${cacheScope}:${albumCid}`;
  const cached = await cacheManager.albums.get(cacheKey);
  if (cached.found) {
    return cached.data;
  }

  const data = await invoke<AlbumDetail>('get_album_detail', { albumCid });
  await cacheManager.albums.set(cacheKey, data, [
    createAlbumCacheTag(albumCid),
    createInventoryCacheTag(inventoryVersion),
  ]);
  return data;
}

export async function refreshAlbumDetail(
  albumCid: string,
  inventoryVersion?: string | null
): Promise<AlbumDetail> {
  await cacheManager.invalidateByTag(createAlbumCacheTag(albumCid));
  const data = await invoke<AlbumDetail>('refresh_album_detail', { albumCid });
  const cacheScope = inventoryVersion ?? 'unversioned';
  const cacheKey = `${CACHE_KEY_ALBUM_DETAIL}${cacheScope}:${albumCid}`;
  await cacheManager.albums.set(cacheKey, data, [
    createAlbumCacheTag(albumCid),
    createInventoryCacheTag(inventoryVersion),
  ]);
  return data;
}

export async function getSongDetail(
  songCid: string,
  inventoryVersion?: string | null
): Promise<SongDetail> {
  const cacheScope = inventoryVersion ?? 'unversioned';
  const cacheKey = `${CACHE_KEY_SONG_DETAIL}${cacheScope}:${songCid}`;
  const cached = await cacheManager.songs.get(cacheKey);
  if (cached.found) {
    return cached.data;
  }

  const data = await invoke<SongDetail>('get_song_detail', { cid: songCid });
  await cacheManager.songs.set(cacheKey, data, [
    createSongCacheTag(songCid),
    createAlbumCacheTag(data.albumCid),
    createInventoryCacheTag(inventoryVersion),
  ]);
  return data;
}

export async function getSongLyrics(songCid: string): Promise<string | null> {
  const cacheKey = `${CACHE_KEY_SONG_LYRICS}${songCid}`;
  const cached = await cacheManager.lyrics.get(cacheKey);
  if (cached.found) {
    return cached.data;
  }

  const songDetail = await getSongDetail(songCid);
  const data = await invoke<string | null>('get_song_lyrics', { cid: songCid });
  await cacheManager.lyrics.set(cacheKey, data, [
    createSongCacheTag(songCid),
    createAlbumCacheTag(songDetail.albumCid),
  ]);
  return data;
}

export async function searchLibrary(
  request: SearchLibraryRequest
): Promise<SearchLibraryResponse> {
  return invoke<SearchLibraryResponse>('search_library', { request });
}

export async function playSong(
  songCid: string,
  coverUrl?: string,
  playbackContext?: PlaybackContext
): Promise<PlaybackStartResult> {
  return invokePlayback('play_song', {
    songCid,
    coverUrl: coverUrl ?? null,
    playbackContext: playbackContext ?? null,
  });
}

export async function pausePlayback(): Promise<void> {
  return invokePlayback<void>('pause_playback');
}

export async function resumePlayback(): Promise<void> {
  return invokePlayback<void>('resume_playback');
}

export async function seekCurrentPlayback(
  positionSecs: number
): Promise<PlaybackStartResult> {
  return invokePlayback('seek_current_playback', { positionSecs });
}

export async function playNext(): Promise<PlaybackStartResult> {
  return invokePlayback('play_next');
}

export async function playPrevious(): Promise<PlaybackStartResult> {
  return invokePlayback('play_previous');
}

export async function getPlayerState(): Promise<PlayerState> {
  return invoke('get_player_state');
}

export async function setPlaybackVolume(volume: number): Promise<number> {
  return invoke('set_playback_volume', { volume });
}

export async function showMainWindow(): Promise<void> {
  return invoke('show_main_window');
}

export async function getDefaultOutputDir(): Promise<string> {
  return invoke('get_default_output_dir');
}

export async function selectDirectory(
  defaultPath?: string
): Promise<string | null> {
  return open({
    directory: true,
    defaultPath,
  });
}

export async function clearAudioCache(): Promise<number> {
  return invoke('clear_audio_cache');
}

export async function clearResponseCache(): Promise<void> {
  return invoke('clear_response_cache');
}

export async function resetHttpClient(): Promise<void> {
  return invoke('reset_http_client');
}

export async function extractImageTheme(
  imageUrl: string
): Promise<ThemePalette> {
  const cacheKey = `${CACHE_KEY_IMAGE_THEME}${imageUrl}`;
  const inflight = inflightImageThemeRequests.get(cacheKey);
  if (inflight) {
    return inflight;
  }

  const request = (async () => {
    const cached = await cacheManager.themes.get(cacheKey);
    if (cached.found) {
      return cached.data;
    }

    return scheduleImageResourceRequest(async () => {
      const queuedCached = await cacheManager.themes.get(cacheKey);
      if (queuedCached.found) {
        return queuedCached.data;
      }

      const data = await invoke<ThemePalette>('extract_image_theme', {
        imageUrl,
      });
      await cacheManager.themes.set(cacheKey, data);
      return data;
    });
  })();
  inflightImageThemeRequests.set(cacheKey, request);

  try {
    return await request;
  } finally {
    if (inflightImageThemeRequests.get(cacheKey) === request) {
      inflightImageThemeRequests.delete(cacheKey);
    }
  }
}

export async function getImageSrc(imageUrl: string): Promise<string> {
  const inflight = inflightImageSrcRequests.get(imageUrl);
  if (inflight) {
    return inflight;
  }

  const request = (async () => {
    const cachedPath = await invoke<string>('get_cached_image_path', {
      imageUrl,
    });
    return convertFileSrc(cachedPath);
  })();
  inflightImageSrcRequests.set(imageUrl, request);

  try {
    return await request;
  } finally {
    if (inflightImageSrcRequests.get(imageUrl) === request) {
      inflightImageSrcRequests.delete(imageUrl);
    }
  }
}

export async function createDownloadJob(
  request: CreateDownloadJobRequest
): Promise<DownloadJobSnapshot> {
  return invoke('create_download_job', { request });
}

export async function listDownloadJobs(): Promise<DownloadManagerSnapshot> {
  return invoke('list_download_jobs');
}

export async function cancelDownloadJob(
  jobId: string
): Promise<DownloadJobSnapshot | null> {
  return invoke('cancel_download_job', { jobId });
}

export async function cancelDownloadTask(
  jobId: string,
  taskId: string
): Promise<DownloadJobSnapshot | null> {
  return invoke('cancel_download_task', { jobId, taskId });
}

export async function retryDownloadJob(
  jobId: string
): Promise<DownloadJobSnapshot | null> {
  return invoke('retry_download_job', { jobId });
}

export async function retryDownloadTask(
  jobId: string,
  taskId: string
): Promise<DownloadJobSnapshot | null> {
  return invoke('retry_download_task', { jobId, taskId });
}

export async function clearDownloadHistory(): Promise<number> {
  return invoke('clear_download_history');
}

export async function sendTestNotification(): Promise<void> {
  return invoke('send_test_notification');
}

export type NotificationPermissionState =
  | 'granted'
  | 'denied'
  | 'prompt'
  | 'prompt-with-rationale';

export async function getNotificationPermissionState(): Promise<NotificationPermissionState> {
  return invoke('get_notification_permission_state');
}

export async function requestNotificationPermission(): Promise<NotificationPermissionState> {
  return invoke('request_notification_permission');
}

export async function getLocalInventorySnapshot(): Promise<LocalInventorySnapshot> {
  return invoke<LocalInventorySnapshot>('get_local_inventory_snapshot');
}

export async function getAudioMetadata(
  albumName: string,
  songName: string
): Promise<AudioFileMetadata | null> {
  return invoke<AudioFileMetadata | null>('get_audio_metadata', {
    albumName,
    songName,
  });
}

export async function getPreferences(): Promise<AppPreferences> {
  return invoke<AppPreferences>('get_preferences');
}

export async function setPreferences(
  preferences: AppPreferences,
  expectedRevision: number
): Promise<AppPreferences> {
  return invoke<AppPreferences>('set_preferences', {
    preferences,
    expectedRevision,
  });
}

export async function listLogRecords(
  query: LogViewerQuery
): Promise<LogViewerPage> {
  return invoke<LogViewerPage>('list_log_records', { query });
}

export async function getLogFileStatus(): Promise<LogFileStatus> {
  return invoke<LogFileStatus>('get_log_file_status');
}

export async function getLatestAlbums(limit: number): Promise<Album[]> {
  return invoke<Album[]>('get_latest_albums', { limit });
}

export async function getAlbumsBySeriesGroup(): Promise<SeriesGroup[]> {
  return invoke<SeriesGroup[]>('get_albums_by_series');
}

export async function recordSongHeat(
  songCid: string,
  coverUrl: string | null
): Promise<void> {
  return invoke('record_song_heat', { songCid, coverUrl });
}

export async function getRecentHistory(limit: number): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>('get_recent_history', { limit });
}

export async function clearListeningHistory(): Promise<number> {
  return invoke<number>('clear_listening_history');
}

export async function getHomepageStatus(): Promise<HomepageStatus> {
  return invoke<HomepageStatus>('get_homepage_status');
}

export async function getTagDimensions(): Promise<TagDimension[]> {
  return invoke<TagDimension[]>('get_tag_dimensions');
}

export async function getAlbumsByTagDimension(
  dimensionKey: string
): Promise<TagGroup[]> {
  return invoke<TagGroup[]>('get_albums_by_tag_dimension', { dimensionKey });
}

// ─── Tag Editor ──────────────────────────────────────────────────────────────

export async function getTagEditorMerged(): Promise<TagEditorRegistry> {
  return invoke<TagEditorRegistry>('get_tag_editor_merged');
}

export async function getTagEditorLocalOverlay(): Promise<TagEditorRegistry> {
  return invoke<TagEditorRegistry>('get_tag_editor_local_overlay');
}

export async function setTagEditorEntityTag(
  entityType: TagEditorEntityType,
  cid: string,
  dimensionKey: string,
  values: TagEditorLocalizedValue[]
): Promise<void> {
  return invoke('set_tag_editor_entity_tag', {
    entityType,
    cid,
    dimensionKey,
    values,
  });
}

export async function removeTagEditorEntityTag(
  entityType: TagEditorEntityType,
  cid: string,
  dimensionKey: string
): Promise<void> {
  return invoke('remove_tag_editor_entity_tag', {
    entityType,
    cid,
    dimensionKey,
  });
}

export async function addTagEditorDimension(
  key: string,
  labelZh: string,
  labelEn: string
): Promise<void> {
  return invoke('add_tag_editor_dimension', { key, labelZh, labelEn });
}

export async function removeTagEditorDimension(key: string): Promise<void> {
  return invoke('remove_tag_editor_dimension', { key });
}

export async function applyTagEditorRemoteUpdate(
  newRemote: TagEditorRegistry
): Promise<TagEditorMergeResult> {
  return invoke<TagEditorMergeResult>('apply_tag_editor_remote_update', {
    newRemote,
  });
}

export async function resolveTagEditorConflict(
  entityType: TagEditorEntityType,
  cid: string,
  dimensionKey: string,
  keep: ConflictResolution
): Promise<void> {
  return invoke('resolve_tag_editor_conflict', {
    entityType,
    cid,
    dimensionKey,
    keep,
  });
}

export async function exportTagEditorRegistry(path: string): Promise<void> {
  return invoke('export_tag_editor_registry', { path });
}

export async function importTagEditorRegistry(
  path: string
): Promise<TagEditorMergeResult> {
  return invoke<TagEditorMergeResult>('import_tag_editor_registry', { path });
}

/**
 * 同步循环 / 随机播放的勾选态到系统菜单。
 *
 * 前端在 `playerController.repeatMode` 或 `playerController.shuffleEnabled`
 * 变化时调用；后端负责在已挂载的菜单里调用对应 `CheckMenuItem::set_checked`。
 * 菜单尚未构建（例如启动早期）时返回错误字符串，前端可忽略。
 */
export async function syncPlaybackMenuState(
  repeatMode: RepeatMode,
  shuffleEnabled: boolean
): Promise<void> {
  return invoke('sync_playback_menu_state', {
    repeatMode,
    shuffleEnabled,
  });
}

/**
 * 本地库存扫描的校验强度。
 *
 * 与后端 `harubble_core::local_inventory::VerificationMode` 保持一致；
 * 前端调用方通常传 `undefined`，让后端沿用当前偏好中的默认值。
 */
export type LocalInventoryVerificationMode =
  | 'none'
  | 'whenAvailable'
  | 'strict';

/**
 * 触发一次本地库存重扫。
 *
 * 命令是异步启动扫描，立即返回**当前**快照；真正的扫描结果通过
 * `local-inventory-state-changed` 事件推送到前端订阅方。菜单入口通常
 * 忽略返回值，仅利用副作用触发扫描。
 */
export async function rescanLocalInventory(
  verificationMode?: LocalInventoryVerificationMode
): Promise<LocalInventorySnapshot> {
  return invoke<LocalInventorySnapshot>('rescan_local_inventory', {
    verificationMode: verificationMode ?? null,
  });
}

/**
 * 导出当前偏好到指定文件路径。
 *
 * 调用方需先弹出保存对话框拿到路径；后端会以 TOML 写盘。
 */
export async function exportPreferences(
  outputPath: string
): Promise<AppPreferences> {
  return invoke<AppPreferences>('export_preferences', { outputPath });
}

/**
 * 从指定文件导入偏好。
 *
 * 后端校验文件后覆盖当前偏好并同步落盘；若下载目录变化会自动触发一次库存重扫。
 */
export async function importPreferences(
  inputPath: string
): Promise<AppPreferences> {
  return invoke<AppPreferences>('import_preferences', { inputPath });
}

// ---------------------------------------------------------------------------
// Theme package commands (Phase 1 MVP)
// ---------------------------------------------------------------------------

/**
 * 列出所有已安装的主题包摘要。
 *
 * 后端按 id 字典序返回；返回值仅包含 manifest 精简字段。
 * 完整 slots/variants 需通过 `inspectThemePackage(id)` 按需读取。
 */
export async function listThemePackages(): Promise<ThemePackageSummary[]> {
  return invoke<ThemePackageSummary[]>('list_theme_packages');
}

/**
 * 读取指定主题包的完整文档（含 slots/variants/warnings）。
 *
 * 返回 `null` 表示 id 不存在于 committed 目录。
 */
export async function inspectThemePackage(
  id: string
): Promise<ThemePackageDocument | null> {
  return invoke<ThemePackageDocument | null>('inspect_theme_package', { id });
}

/**
 * 从本地文件路径安装主题包。
 *
 * 入参 `path` 必须是绝对路径，指向可读的 `.json` 文件（≤ 512 KiB）。
 * 后端会走 sanitize + hash + atomic commit 流程。
 * 若同 id 已存在则覆盖，返回值为新安装的摘要。
 */
export async function installThemePackageFromFile(
  path: string
): Promise<ThemePackageSummary> {
  return invoke<ThemePackageSummary>('install_theme_package_from_file', {
    path,
  });
}

/**
 * 从远程 https URL 下载并安装主题包。
 *
 * 后端做全套 SSRF 防护：仅接受 https（端口 443）、拒绝私有 / loopback / CGNAT / multicast /
 * 保留段 IP、禁用重定向、总耗时 ≤ 15s、大小 ≤ 512 KiB、Content-Type 必须为 JSON。
 * 下载成功后走与 `installThemePackageFromFile` 相同的 sanitize 流水线。
 */
export async function installThemePackageFromUrl(
  url: string
): Promise<ThemePackageSummary> {
  return invoke<ThemePackageSummary>('install_theme_package_from_url', {
    url,
  });
}

/**
 * 卸载指定主题包（原子搬到 pending-delete，启动扫描时清理）。
 *
 * 对不存在的 id 幂等成功。若被卸载的 id 恰好是当前 active_package_id，
 * 后端会同步清空激活状态并广播 `preferences_snapshot` 事件。
 */
export async function uninstallThemePackage(id: string): Promise<void> {
  return invoke<void>('uninstall_theme_package', { id });
}

/**
 * 通过 CAS 激活指定主题包（或传 null 清空激活状态）。
 *
 * `expectedRevision` 为客户端上次读取到的 `theme.revision`，后端在写锁内比对：
 * 匹配 → 更新 activePackageId 并 revision+1，返回新快照；
 * 不匹配 → 抛出 `RevisionMismatch`，前端应重新 getPreferences 后再决定。
 *
 * 成功路径会通过 `preferences_snapshot` 事件广播到所有窗口（含 Mini Player）。
 */
export async function setActiveThemePackage(
  id: string | null,
  expectedRevision: number
): Promise<AppPreferences> {
  return invoke<AppPreferences>('set_active_theme_package', {
    id,
    expectedRevision,
  });
}

/**
 * 进入指定主题包的预览态（内存中，不持久化）。
 *
 * 返回主题包完整文档；前端应基于其派生 token 应用到 DOM，
 * 但不写 preferences。调用 `dismissThemePreview` 恢复到 committed 态。
 */
export async function previewThemePackage(
  id: string
): Promise<ThemePackageDocument> {
  return invoke<ThemePackageDocument>('preview_theme_package', { id });
}

/**
 * 关闭主题包预览态，恢复到 committed / preferences 派生。
 *
 * 对未处于预览态的调用幂等成功。
 */
export async function dismissThemePreview(): Promise<void> {
  return invoke<void>('dismiss_theme_preview');
}

/**
 * 将指定主题包的原始 JSON 导出到本地路径。
 *
 * `outputPath` 必须为绝对路径；父目录必须存在。目标已存在时被覆盖。
 */
export async function exportThemePackage(
  id: string,
  outputPath: string
): Promise<void> {
  return invoke<void>('export_theme_package', { id, outputPath });
}
