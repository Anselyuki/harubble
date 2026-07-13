import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import {
  cacheManager,
  createAlbumCacheTag,
  createInventoryCacheTag,
  createSongCacheTag,
} from './cache';
import type {
  Album,
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
} from './types';

const CACHE_KEY_ALBUM_DETAIL = 'album_detail:';
const CACHE_KEY_SONG_DETAIL = 'song_detail:';
const CACHE_KEY_SONG_LYRICS = 'song_lyrics:';
const CACHE_KEY_IMAGE_THEME = 'image_theme:';
const CACHE_KEY_IMAGE_DATA_URL = 'image_data_url:';
const IMAGE_RESOURCE_CONCURRENCY_LIMIT = 1;

const inflightImageThemeRequests = new Map<string, Promise<ThemePalette>>();
const inflightImageDataUrlRequests = new Map<string, Promise<string>>();
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

export async function getImageDataUrl(imageUrl: string): Promise<string> {
  const cacheKey = `${CACHE_KEY_IMAGE_DATA_URL}${imageUrl}`;
  const inflight = inflightImageDataUrlRequests.get(cacheKey);
  if (inflight) {
    return inflight;
  }

  const request = (async () => {
    const cached = await cacheManager.covers.get(cacheKey);
    if (cached.found) {
      return cached.data;
    }

    return scheduleImageResourceRequest(async () => {
      const queuedCached = await cacheManager.covers.get(cacheKey);
      if (queuedCached.found) {
        return queuedCached.data;
      }

      const data = await invoke<string>('get_image_data_url', { imageUrl });
      await cacheManager.covers.set(cacheKey, data);
      return data;
    });
  })();
  inflightImageDataUrlRequests.set(cacheKey, request);

  try {
    return await request;
  } finally {
    if (inflightImageDataUrlRequests.get(cacheKey) === request) {
      inflightImageDataUrlRequests.delete(cacheKey);
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
  preferences: AppPreferences
): Promise<AppPreferences> {
  return invoke<AppPreferences>('set_preferences', { preferences });
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
