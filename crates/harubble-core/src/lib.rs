//! Harubble 共享核心库。
//!
//! `harubble_core` 负责承载桌面端后端中与平台无关、可脱离 Tauri runtime 复用的核心能力，
//! 包括上游数据访问、音频格式处理、下载任务模型与本地库存扫描等。
//! 对于希望在 CLI、测试、后台服务或未来的其它宿主中复用同一套下载语义的调用方，
//! 这里是最稳定的公共 Rust API 入口。
//!
//! # 适用场景
//!
//! - 需要通过 [`ApiClient`] 访问专辑、歌曲详情与远端资源下载时。
//! - 需要使用 [`DownloadService`] 维护下载批次、任务快照与取消/重试状态时。
//! - 需要调用 [`download_song`]、[`write_payload_to_disk`] 等高层接口完成单曲下载与写盘时。
//! - 需要复用 [`save_audio`]、[`tag_flac`] 等音频处理能力，或复用本地库存扫描 / 库内搜索模型时。
//!
//! # 模块导览
//!
//! - [`api`]：对上游公开 HTTP 接口的强类型访问与响应缓存。
//! - [`audio`]：音频格式识别、文件写入、封面处理与 FLAC 标签工具。
//! - [`download`]：下载领域模型、错误类型、状态机与服务层实现。
//! - [`downloader`]：面向单首歌曲 / 封面的高层下载与写盘编排。
//! - [`local_inventory`]：本地库存扫描、证据建模与专辑 / 曲目徽标聚合。
//! - [`search`]：库内搜索请求、结果模型与约束常量。
//!
//! # 调用起点建议
//!
//! 若你从“页面或 command 发起一批下载”进入，通常应先看 [`CreateDownloadJobRequest`]、
//! [`DownloadOptions`] 与 [`DownloadService`]；若你从“拿到某首歌后直接保存文件”进入，
//! 则优先查看 [`download_song`]、[`WritePayload`]、[`write_payload_to_disk`] 与 [`OutputFormat`]。
//! 如果你的目标是理解前端为什么会收到某个下载状态或搜索结果，也可以直接从这里重导出的
//! 快照类型和事件载荷往下追踪，而不必先进入具体模块内部。
//!
//! # 边界说明
//!
//! 这个 crate 不负责 Tauri command 注册、窗口事件广播或平台媒体会话等宿主相关逻辑；
//! 这些能力由 `src-tauri` crate 负责封装。`harubble_core` 更偏向“可复用的领域层与库层契约”，
//! 上层应把它当作稳定核心，而不是依赖某个具体桌面运行时的薄包装。
//!
//! # 生成 rustdoc
//!
//! ```bash
//! cargo doc -p harubble_core --no-deps
//! ```

pub mod api;
pub mod audio;
pub mod download;
pub mod downloader;
pub mod homepage;
pub mod local_inventory;
pub mod search;
pub mod url_validator;

// 重新导出公共 API，便于上层直接使用
/// 上游数据访问相关的公共类型与客户端入口。
pub use api::{Album, AlbumDetail, ApiClient, SongDetail, SongEntry, TagEntry};
/// 音频格式识别、落盘与 FLAC 标签写入相关的公共工具。
pub use audio::{
    ensure_available_space, save_audio, tag_flac, write_file_atomically, AudioFormat, OutputFormat,
};
/// 下载领域模型、快照与事件载荷。
pub use download::model::{
    CreateDownloadJobRequest, DownloadErrorCode, DownloadErrorInfo, DownloadJobKind,
    DownloadJobSnapshot, DownloadJobStatus, DownloadManagerSnapshot, DownloadOptions,
    DownloadTaskProgressEvent, DownloadTaskSnapshot, DownloadTaskStatus,
};
/// 下载服务高层入口。
pub use download::service::{prepare_job, DownloadService, PreparedJob};
/// 高层下载编排、写盘与进度上报相关公共接口。
pub use downloader::{
    album_cover_exists, album_output_dir, download_album_cover, download_song,
    write_album_cover_bytes, write_payload_to_disk, DownloadCancelledError, DownloadProgress,
    DownloadProvenanceSeed, MetaOverride, OwnedFlacMetadata, WritePayload,
};
/// 首页数据结构：系列分组、收听历史、收听事件与状态仪表盘。
pub use homepage::{HistoryEntry, HomepageStatus, ListeningEvent, SeriesGroup, TagGroup};
/// 本地库存扫描、徽标聚合与证据建模相关公共类型与工具。
pub use local_inventory::{
    aggregate_album_download_badge, album_badge_for_status, album_badge_from_evidence,
    badge_for_detected_file, badge_for_status, candidate_relative_paths, has_detected_track,
    matched_track_evidence, missing_album_badge, missing_track_badge, track_badge_from_matches,
    AlbumDownloadBadge, LocalAudioFileEvidence, LocalAudioFileVerificationState,
    LocalInventoryScanProgressEvent, LocalInventorySnapshot, LocalInventoryStatus,
    LocalTrackDownloadStatus, LocalTrackEvidenceMatchRule, MatchedTrackEvidence,
    TrackDownloadBadge, VerificationMode,
};
/// 本地库内搜索请求、响应、命中模型与常量约束。
pub use search::{
    LibraryIndexState, LibrarySearchHitField, LibrarySearchScope, SearchLibraryRequest,
    SearchLibraryResponse, SearchLibraryResultItem, SearchLibraryResultKind,
    SEARCH_LIBRARY_DEFAULT_LIMIT, SEARCH_LIBRARY_DEFAULT_OFFSET, SEARCH_LIBRARY_MAX_LIMIT,
    SEARCH_LIBRARY_MAX_OFFSET, SEARCH_LIBRARY_QUERY_MAX_LENGTH,
};
/// 下载 URL 域名白名单校验。
pub use url_validator::validate_download_url;
