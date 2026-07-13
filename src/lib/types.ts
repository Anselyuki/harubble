import type { Locale } from './i18n/types';

export type LocalTrackDownloadStatus =
  | 'missing'
  | 'detected'
  | 'verified'
  | 'mismatch'
  | 'partial'
  | 'unverifiable'
  | 'unknown';

export interface TrackDownloadBadge {
  isDownloaded: boolean;
  downloadStatus: LocalTrackDownloadStatus;
  inventoryVersion: string;
}

export type LocalInventoryStatus = 'idle' | 'scanning' | 'completed' | 'failed';

export interface LocalInventorySnapshot {
  rootOutputDir: string;
  status: LocalInventoryStatus;
  inventoryVersion: string;
  startedAt: string | null;
  finishedAt: string | null;
  scannedFileCount: number;
  matchedTrackCount: number;
  verifiedTrackCount: number;
  lastError: string | null;
}

/**
 * 本地库存扫描过程中由后端推送到前端的进度事件载荷。
 *
 * 对应 Rust 侧 `LocalInventoryScanProgressEvent`（`crates/harubble-core/src/local_inventory/mod.rs`），
 * 通过 `local-inventory-scan-progress` 事件发布。
 */
export interface LocalInventoryScanProgressEvent {
  rootOutputDir: string;
  inventoryVersion: string;
  filesScanned: number;
  matchedTrackCount: number;
  verifiedTrackCount: number;
  currentPath: string | null;
}

export interface AlbumDownloadBadge {
  isDownloaded: boolean;
  downloadStatus: LocalTrackDownloadStatus;
  inventoryVersion: string;
}

export interface TagEntry {
  dimension: string;
  values: string[];
  colors?: (string | null)[];
}

export interface TagDimension {
  key: string;
  label: string;
}

export interface Album {
  cid: string;
  name: string;
  coverUrl: string;
  artists: string[];
  download: AlbumDownloadBadge;
  tags: TagEntry[];
}

export interface SongEntry {
  cid: string;
  name: string;
  artists: string[];
  download: TrackDownloadBadge;
  tags: TagEntry[];
}

export interface PlaybackQueueEntry {
  cid: string;
  name: string;
  artists: string[];
  coverUrl: string | null;
}

export interface PlaybackContext {
  entries: PlaybackQueueEntry[];
  currentIndex: number;
}

export interface SongDetail {
  cid: string;
  name: string;
  albumCid: string;
  sourceUrl: string;
  lyricUrl: string | null;
  mvUrl: string | null;
  mvCoverUrl: string | null;
  artists: string[];
  download: TrackDownloadBadge;
  tags: TagEntry[];
}

export interface AlbumDetail {
  cid: string;
  name: string;
  intro: string | null;
  belong: string;
  coverUrl: string;
  coverDeUrl: string | null;
  artists: string[] | null;
  download: AlbumDownloadBadge;
  tags: TagEntry[];
  songs: SongEntry[];
}

export interface AudioFileMetadata {
  format: string;
  sampleRate: number;
  channels: number;
  bitsPerSample: number | null;
  durationSecs: number;
  bitrateKbps: number | null;
  fileSize: number;
}

export type LibrarySearchScope = 'all' | 'albums' | 'songs';

export type LibrarySearchHitField =
  | 'title'
  | 'artist'
  | 'intro'
  | 'belong'
  | 'tagValues';

export type LibraryIndexState = 'notReady' | 'building' | 'stale' | 'ready';

export type SearchLibraryResultKind = 'album' | 'song';

export interface SearchLibraryRequest {
  query: string;
  scope: LibrarySearchScope;
  limit?: number;
  offset?: number;
}

export interface SearchLibraryResultItem {
  kind: SearchLibraryResultKind;
  albumCid: string;
  songCid: string | null;
  albumTitle: string;
  songTitle: string | null;
  artistLine: string | null;
  matchedFields: LibrarySearchHitField[];
}

export interface SearchLibraryResponse {
  items: SearchLibraryResultItem[];
  total: number;
  query: string;
  scope: LibrarySearchScope;
  indexState: LibraryIndexState;
}

export interface ThemePalette {
  accentHex: string;
  accentHoverHex: string;
  accentRgb: [number, number, number];
  accentHoverRgb: [number, number, number];
  waveColors: [number, number, number][];
  surfaceHex: string;
  textPrimaryHex: string;
  textSecondaryHex: string;
  tintHex: string;
  dangerHex: string;
}

export type ThemeColorSlot =
  | 'accent'
  | 'surface'
  | 'textPrimary'
  | 'textSecondary'
  | 'tint'
  | 'danger';

export type ThemeColorSlots = Record<ThemeColorSlot, string>;

export type ColorScheme = 'auto' | 'light' | 'dark';

export interface ThemePreferences {
  presetId: string;
  customColors: Partial<ThemeColorSlots>;
  colorScheme?: ColorScheme;
  dynamicAlbumAccent?: boolean;
}

export interface ThemeTokenSet {
  accent: string;
  accentHover: string;
  accentRgb: string;
  accentHoverRgb: string;
  accentReadableForeground: string;
  accentHoverReadableForeground: string;
  bgPrimary: string;
  bgSecondary: string;
  bgTertiary: string;
  bgElevated: string;
  textPrimary: string;
  textSecondary: string;
  textTertiary: string;
  border: string;
  ring: string;
  destructive: string;
  destructiveRgb: string;
  surfaceState: string;
  surfaceBase: string;
  surfaceSidebar: string;
  surfaceOverlay: string;
}

export type OutputFormat = 'flac' | 'wav' | 'mp3';

// ---------------------------------------------------------------------------
// Download job types (mirrors harubble-core/src/download/model.rs)
// ---------------------------------------------------------------------------

export type DownloadJobKind = 'song' | 'album' | 'selection';

export type DownloadJobStatus =
  | 'queued'
  | 'running'
  | 'completed'
  | 'partiallyFailed'
  | 'failed'
  | 'cancelled';

export type DownloadHistoryScopeFilter = 'all' | 'active' | 'history';

export type DownloadHistoryStatusFilter = 'all' | DownloadJobStatus;

export type DownloadHistoryKindFilter = 'all' | DownloadJobKind;

export type DownloadTaskStatus =
  | 'queued'
  | 'preparing'
  | 'downloading'
  | 'writing'
  | 'completed'
  | 'failed'
  | 'cancelled';

export type DownloadErrorCode =
  | 'network'
  | 'api'
  | 'io'
  | 'decode'
  | 'tagging'
  | 'lyrics'
  | 'cancelled'
  | 'invalidRequest'
  | 'internal';

export interface DownloadErrorInfo {
  code: DownloadErrorCode;
  message: string;
  retryable: boolean;
  details: string | null;
}

export interface DownloadOptions {
  outputDir: string;
  format: OutputFormat;
  downloadLyrics: boolean;
}

export interface DownloadTaskSnapshot {
  id: string;
  jobId: string;
  songCid: string;
  songName: string;
  artists: string[];
  albumCid: string;
  albumName: string;
  status: DownloadTaskStatus;
  bytesDone: number;
  bytesTotal: number | null;
  outputPath: string | null;
  error: DownloadErrorInfo | null;
  attempt: number;
  songIndex: number;
  songCount: number;
}

export interface DownloadJobSnapshot {
  id: string;
  kind: DownloadJobKind;
  status: DownloadJobStatus;
  createdAt: string;
  startedAt: string | null;
  finishedAt: string | null;
  options: DownloadOptions;
  title: string;
  taskCount: number;
  completedTaskCount: number;
  failedTaskCount: number;
  cancelledTaskCount: number;
  tasks: DownloadTaskSnapshot[];
  error: DownloadErrorInfo | null;
}

export interface DownloadManagerSnapshot {
  jobs: DownloadJobSnapshot[];
  activeJobId: string | null;
  queuedJobIds: string[];
}

export interface DownloadTaskProgressEvent {
  jobId: string;
  taskId: string;
  status: DownloadTaskStatus;
  bytesDone: number;
  bytesTotal: number | null;
  songIndex: number;
  songCount: number;
  speedBytesPerSec: number;
}

export interface CreateDownloadJobRequest {
  kind: DownloadJobKind;
  songCids: string[];
  albumCid: string | null;
  options: DownloadOptions;
}

export interface PlaybackFormatState {
  sourceSampleRate: number;
  sourceChannels: number;
  sourceBitsPerSample: number | null;
  sourceBitrateKbps?: number | null;
  outputSampleRate: number;
  outputChannels: number;
  outputBitsPerSample: number | null;
  outputSampleFormat: string;
  resampling: boolean;
  channelRemix: boolean;
}

export interface PlayerState {
  sessionId: number;
  songCid: string | null;
  songName: string | null;
  artists: string[];
  coverUrl: string | null;
  isPlaying: boolean;
  isPaused: boolean;
  isLoading: boolean;
  hasPrevious: boolean;
  hasNext: boolean;
  progress: number;
  duration: number;
  volume: number;
  playbackFormat: PlaybackFormatState | null;
}

export interface PlaybackStartResult {
  duration: number;
  sessionId: number;
}

export type PlaybackErrorCode =
  | 'superseded'
  | 'noActiveTrack'
  | 'noNextTrack'
  | 'noPreviousTrack'
  | 'loading'
  | 'network'
  | 'audio'
  | 'io'
  | 'internal';

export interface PlaybackErrorPayload {
  code: PlaybackErrorCode;
  message: string;
  retryable: boolean;
  sessionId: number | null;
}

export interface PlaybackEndedEvent {
  sessionId: number;
  songCid: string;
  progress: number;
  duration: number;
}

export type LogLevel = 'debug' | 'info' | 'warn' | 'error';

export interface AppPreferences {
  outputFormat: OutputFormat;
  outputDir: string;
  downloadLyrics: boolean;
  notifyOnDownloadComplete: boolean;
  notifyOnPlaybackChange: boolean;
  logLevel: LogLevel;
  locale: Locale;
  volume: number;
  theme?: ThemePreferences;
}

export type AppErrorLevel = 'warn' | 'error';

export interface AppErrorEvent {
  id: string;
  ts: string;
  level: AppErrorLevel;
  domain: string;
  code: string;
  message: string;
}

export type LogFileKind = 'session' | 'persistent';

export interface LogViewerQuery {
  kind: LogFileKind;
  level?: LogLevel | null;
  domain?: string | null;
  search?: string | null;
  limit?: number | null;
  offset?: number | null;
}

export interface LogViewerRecord {
  id: string;
  ts: string;
  level: LogLevel;
  domain: string;
  code: string;
  message: string;
  details: string | null;
}

export interface LogViewerPage {
  records: LogViewerRecord[];
  total: number;
  kind: LogFileKind;
}

export interface LogFileStatus {
  hasSessionLog: boolean;
  hasPersistentLog: boolean;
}

export interface SeriesGroup {
  series: string;
  albums: Album[];
}

export interface TagGroup {
  dimensionKey: string;
  value: string;
  albums: Album[];
}

export interface HistoryEntry {
  songCid: string;
  songName: string;
  albumCid: string;
  albumName: string;
  coverUrl: string | null;
  artists: string[];
  heat: number;
  playedAt: string;
}

export interface HomepageStatus {
  platformAlbumCount: number;
  platformSongCount: number;
  localDownloadedCount: number;
  localStorageBytes: number;
  activeDownloadCount: number;
  completedDownloadCount: number;
}

// ─── Tag Editor ──────────────────────────────────────────────────────────────

export type TagEditorEntityType = 'album' | 'song';

export interface TagEditorLocalizedValue {
  [locale: string]: string;
}

export interface TagEditorTagSet {
  tags: Record<string, TagEditorLocalizedValue[]>;
}

export interface TagEditorDimension {
  key: string;
  label: Record<string, string>;
}

export interface TagEditorAlbumEntry {
  cid: string;
  type?: string[];
  name: string | null;
  releaseDate: string | null;
  faction: TagEditorLocalizedValue | null;
  character: TagEditorLocalizedValue | null;
  [key: string]:
    | string
    | string[]
    | TagEditorLocalizedValue
    | TagEditorLocalizedValue[]
    | null
    | undefined;
}

export interface TagEditorRegistry {
  schemaVersion: number;
  updatedAt: string;
  tagDimensions: TagEditorDimension[];
  typeDefinitions: Record<string, TagEditorLocalizedValue>;
  albums: TagEditorAlbumEntry[];
  songs: Record<string, TagEditorTagSet>;
}

export interface TagEditorMergeConflict {
  entityType: TagEditorEntityType;
  cid: string;
  dimensionKey: string;
  baseValues: TagEditorLocalizedValue[] | null;
  remoteValues: TagEditorLocalizedValue[] | null;
  localValues: TagEditorLocalizedValue[] | null;
}

export interface TagEditorMergeResult {
  conflicts: TagEditorMergeConflict[];
  autoMergedCount: number;
}

export type ConflictResolution = 'keepLocal' | 'keepRemote';

// ─── Collections ─────────────────────────────────────────────────────────────

export interface CollectionSummary {
  id: string;
  name: string;
  description: string;
  cover: string | null;
  songCount: number;
  isOfficial: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface CollectionSection {
  name?: string;
  songIds: string[];
}

export interface Collection extends CollectionSummary {
  sections: CollectionSection[];
}

export type CollectionErrorCode =
  | 'notFound'
  | 'readOnly'
  | 'database'
  | 'serialization'
  | 'unsupportedVersion';

export type CollectionError =
  | { code: 'notFound'; detail: { id: string } }
  | { code: 'readOnly' }
  | { code: 'database'; detail: string }
  | { code: 'serialization'; detail: string }
  | { code: 'unsupportedVersion'; detail: { version: number } };
