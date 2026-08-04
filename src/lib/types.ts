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

export interface AlbumCatalogSnapshot {
  albums: Album[];
  revision: number;
  checkedAt: number;
}

export interface AlbumCatalogRefreshedEvent {
  revision: number;
  checkedAt: number;
  changed: boolean;
  albumCount: number;
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
  /** v2：当前激活的主题包 id；null / 缺失表示走 preset 派生路径 */
  activePackageId?: string | null;
  /** v2：主题偏好 CAS 版本号，每次成功写入递增 1；缺失时视为 0 */
  revision?: number;
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
  tint: string;
  tintRgb: string;
  border: string;
  ring: string;
  destructive: string;
  destructiveRgb: string;
  surfaceState: string;
  surfaceBase: string;
  surfaceSidebar: string;
  surfaceOverlay: string;
}

// ---------------------------------------------------------------------------
// Theme package types (mirrors src-tauri/src/theme_packages/types.rs)
// ---------------------------------------------------------------------------

export type ThemePackageStatus = 'staging' | 'committed' | 'pendingDelete';

export interface ThemePackageManifest {
  id: string;
  name: string;
  version: string;
  description?: string;
  author?: string;
  license?: string;
  minAppVersion?: string;
}

export interface ThemePackageVariants {
  light?: Partial<ThemeColorSlots>;
  dark?: Partial<ThemeColorSlots>;
}

/**
 * 主题包声明的 motion 档位覆盖（毫秒）。
 *
 * 与后端 ThemePackageMotion 形状一致；每个字段对应 gsap.ts 中 MOTION 常量的一档。
 * 主题包激活时前端 applyMotionOverride 会同步到 GSAP + CSS 变量。
 */
export interface ThemePackageMotion {
  micro?: number;
  fast?: number;
  base?: number;
  slow?: number;
  page?: number;
  baseOut?: number;
  slowOut?: number;
  pageOut?: number;
  overlayIn?: number;
}

/**
 * 主题包 shape 档位覆盖（像素）。与 --shape-* CSS 变量一一对应。
 */
export interface ThemePackageShape {
  xs?: number;
  sm?: number;
  md?: number;
  lg?: number;
  xl?: number;
  '2xl'?: number;
  pill?: number;
}

/**
 * 主题包 density 档位覆盖（像素）。与 --density-* CSS 变量一一对应。
 */
export interface ThemePackageDensity {
  xs?: number;
  sm?: number;
  md?: number;
  lg?: number;
  xl?: number;
}

/**
 * 主题包 elevation 档位覆盖（完整 box-shadow 字符串）。
 * 与 --elevation-* CSS 变量一一对应。sanitizer 会拒绝含 url/expression 等的值。
 */
export interface ThemePackageElevation {
  none?: string;
  xs?: string;
  sm?: string;
  md?: string;
  lg?: string;
  xl?: string;
}

/**
 * 主题包 blur 档位覆盖（backdrop-filter 半径，像素）。
 */
export interface ThemePackageBlur {
  sm?: number;
  md?: number;
  lg?: number;
  xl?: number;
}

/**
 * 主题包字体族声明（Phase 4 JSON 最小安全子集）。
 *
 * 声明 body / display / mono 三个语义角色的字体名，前端覆盖
 * `--font-body` / `--font-display` / `--font-mono` CSS 变量。
 * sanitizer 强制值不含 CSS 注入关键字与非法字符。
 */
export interface ThemePackageFontFamily {
  body?: string;
  display?: string;
  mono?: string;
}

/**
 * 主题包自定义 CSS 变量的昼夜覆盖。
 *
 * 每个模式仅需声明相对顶层 `cssVariables` 变化的变量；运行时会先应用基础变量，
 * 再合并当前 effective scheme 的覆盖。缺失该字段的旧主题包保持原有行为。
 */
export interface ThemePackageCssVariableVariants {
  light?: Record<string, string>;
  dark?: Record<string, string>;
}

/**
 * 主题包 visualContract 声明（Phase 3）。
 *
 * `family`：视觉语言族。当前 app 版本支持集在 SUPPORTED_THEME_FAMILIES 常量里；
 * 不在支持集内的值会 fallback 到 `glass`。
 * `depth`：视觉深度。同上；同时兼容 legacy 三档与 Ark UI 四档，未知值
 * fallback 到 `balanced`。
 */
export interface ThemePackageVisualContract {
  family?: string;
  depth?: string;
}

export interface ThemePackageDocument {
  schemaVersion: number;
  manifest: ThemePackageManifest;
  slots: Partial<ThemeColorSlots>;
  variants?: ThemePackageVariants;
  motion?: ThemePackageMotion;
  shape?: ThemePackageShape;
  density?: ThemePackageDensity;
  elevation?: ThemePackageElevation;
  blur?: ThemePackageBlur;
  visualContract?: ThemePackageVisualContract;
  /** Phase 4 JSON 最小安全子集：字体族声明，覆盖 --font-body / --font-display / --font-mono。 */
  fontFamily?: ThemePackageFontFamily;
  /** Phase 4 JSON 最小安全子集：自定义 CSS 变量，key 必须以 `--theme-custom-` 开头。 */
  cssVariables?: Record<string, string>;
  /** 自定义 CSS 变量的稀疏昼夜覆盖。 */
  cssVariableVariants?: ThemePackageCssVariableVariants;
  warnings?: string[];
}

export interface ThemePackageSummary {
  id: string;
  name: string;
  version: string;
  status: ThemePackageStatus;
  builtin?: boolean;
  sha256?: string | null;
  warnings?: string[];
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

export type DownloadTaskErrorCode =
  | 'network'
  | 'api'
  | 'io'
  | 'decode'
  | 'tagging'
  | 'lyrics'
  | 'cancelled'
  | 'invalidRequest'
  | 'internal';

export interface DownloadTaskErrorInfo {
  code: DownloadTaskErrorCode;
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
  error: DownloadTaskErrorInfo | null;
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
  error: DownloadTaskErrorInfo | null;
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

export type RepeatMode = 'off' | 'all' | 'one';

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
  schemaVersion: number;
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

// ── Search ───────────────────────────────────────────────────────────────────

export type SearchErrorCode = 'notReady' | 'internal';

export type SearchError =
  | { code: 'notReady' }
  | { code: 'internal'; detail: string };

// ── Library ──────────────────────────────────────────────────────────────────

export type LibraryErrorCode = 'network' | 'notFound' | 'internal';

export type LibraryError =
  | { code: 'network'; detail: string }
  | { code: 'notFound'; detail: { cid: string } }
  | { code: 'internal'; detail: string };

// ── Download ─────────────────────────────────────────────────────────────────

export type DownloadErrorCode =
  | 'notFound'
  | 'network'
  | 'io'
  | 'invalidState'
  | 'internal';

export type DownloadError =
  | { code: 'notFound'; detail: { id: string } }
  | { code: 'network'; detail: string }
  | { code: 'io'; detail: string }
  | { code: 'invalidState'; detail: { reason: string } }
  | { code: 'internal'; detail: string };

// ── TagEditor ────────────────────────────────────────────────────────────────

export type TagEditorErrorCode =
  | 'io'
  | 'serialization'
  | 'unsupportedVersion'
  | 'internal';

export type TagEditorError =
  | { code: 'io'; detail: string }
  | { code: 'serialization'; detail: string }
  | { code: 'unsupportedVersion'; detail: { version: number } }
  | { code: 'internal'; detail: string };

// ── Preferences ──────────────────────────────────────────────────────────────

export type PreferencesErrorCode =
  | 'notFound'
  | 'io'
  | 'revisionMismatch'
  | 'internal';

export type PreferencesError =
  | { code: 'notFound' }
  | { code: 'io'; detail: string }
  | {
      code: 'revisionMismatch';
      detail: {
        currentRevision: number;
        expectedRevision: number;
        message: string;
      };
    }
  | { code: 'internal'; detail: string };

// ── Logging ───────────────────────────────────────────────────────────────────

export type LoggingErrorCode = 'io' | 'internal';

export type LoggingError =
  | { code: 'io'; detail: string }
  | { code: 'internal'; detail: string };

// ── LocalInventory ────────────────────────────────────────────────────────────

export type LocalInventoryErrorCode = 'io' | 'internal';

export type LocalInventoryError =
  | { code: 'io'; detail: string }
  | { code: 'internal'; detail: string };

// ── Homepage ──────────────────────────────────────────────────────────────────

export type HomepageErrorCode = 'network' | 'internal';

export type HomepageError =
  | { code: 'network'; detail: string }
  | { code: 'internal'; detail: string };

// ── TagRegistry ───────────────────────────────────────────────────────────────

export type TagRegistryErrorCode = 'network' | 'internal';

export type TagRegistryError =
  | { code: 'network'; detail: string }
  | { code: 'internal'; detail: string };

// ── Window ────────────────────────────────────────────────────────────────────

export type WindowErrorCode = 'internal';

export type WindowError = { code: 'internal'; detail: string };
