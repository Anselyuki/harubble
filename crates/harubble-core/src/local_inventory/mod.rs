//! 本地库存扫描、证据建模与下载徽标聚合。
//!
//! 该模块定义本地库存扫描状态、音频文件证据、校验模式与专辑/曲目下载徽标相关模型，
//! 供库存扫描服务、下载结果增强与前端展示层复用。

use crate::audio::sanitize_filename;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const AUDIO_EXTENSIONS: [&str; 3] = ["flac", "wav", "mp3"];

/// 单曲在本地库存中的下载状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalTrackDownloadStatus {
    /// 未发现任何候选文件。
    Missing,
    /// 发现了候选文件，但未完成严格校验。
    Detected,
    /// 发现文件且校验通过。
    Verified,
    /// 发现文件，但校验结果与记录不一致。
    Mismatch,
    /// 命中了多个候选文件或部分命中。
    Partial,
    /// 在严格模式下发现文件，但无法完成可信校验。
    Unverifiable,
    /// 状态未知，通常用于聚合结果中的保守兜底。
    Unknown,
}

/// 本地库存扫描任务的整体状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalInventoryStatus {
    Idle,
    Scanning,
    Completed,
    Failed,
}

/// 本地库存校验模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VerificationMode {
    /// 不做校验，只做存在性检测。
    None,
    /// 条件允许时执行校验，否则退化为检测。
    WhenAvailable,
    /// 无法完成校验时按不可验证处理。
    Strict,
}

/// 单曲证据命中的路径规则。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalTrackEvidenceMatchRule {
    /// 命中了根目录下的相对路径。
    RootRelativePath,
    /// 命中了专辑子目录下的相对路径。
    AlbumRelativePath,
}

/// 本地音频文件的校验状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LocalAudioFileVerificationState {
    /// 尚未执行或无法执行校验。
    Unchecked,
    /// 校验通过。
    Verified,
    /// 校验失败。
    Mismatch,
}

/// 扫描阶段采集到的本地音频文件证据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalAudioFileEvidence {
    /// 相对于当前 active outputDir 的规范化相对路径。
    pub relative_path: String,
    /// 扫描时读取到的文件大小。
    pub file_size: u64,
    /// 扫描时读取到的 mtime（Unix ms），用于后续校验链扩展。
    pub modified_at_ms: Option<u64>,
    /// 预留给 checksum / provenance 链的候选摘要字段。
    pub candidate_checksum: Option<String>,
    /// 该文件是否位于专辑子目录下。
    pub is_in_album_directory: bool,
    /// 基于直接 checksum 或 provenance 解析出的校验结论。
    pub verification_state: LocalAudioFileVerificationState,
}

/// 命中当前歌曲后的归一化证据结构。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchedTrackEvidence {
    /// 相对于根输出目录的规范化路径。
    pub relative_path: String,
    /// 文件大小。
    pub file_size: u64,
    /// 文件修改时间。
    pub modified_at_ms: Option<u64>,
    /// 供后续校验链复用的候选摘要。
    pub candidate_checksum: Option<String>,
    /// 文件是否位于专辑子目录内。
    pub is_in_album_directory: bool,
    /// 当前命中的路径规则。
    pub match_rule: LocalTrackEvidenceMatchRule,
    /// 当前证据的校验状态。
    pub verification_state: LocalAudioFileVerificationState,
}

/// 单曲级下载徽标。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackDownloadBadge {
    /// 当前单曲是否可视为本地已存在。
    pub is_downloaded: bool,
    /// 单曲级下载状态。
    pub download_status: LocalTrackDownloadStatus,
    /// 用于前端缓存失效的库存版本号。
    pub inventory_version: String,
}

impl Default for TrackDownloadBadge {
    fn default() -> Self {
        Self {
            is_downloaded: false,
            download_status: LocalTrackDownloadStatus::Missing,
            inventory_version: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDownloadBadge {
    /// 是否可将当前专辑视为“本地已有内容”。
    pub is_downloaded: bool,
    /// 当前列表级专辑提示状态；现阶段以保守提示语义为主。
    pub download_status: LocalTrackDownloadStatus,
    /// 用于前端缓存失效的盘点版本。
    pub inventory_version: String,
}

impl Default for AlbumDownloadBadge {
    fn default() -> Self {
        Self {
            is_downloaded: false,
            download_status: LocalTrackDownloadStatus::Missing,
            inventory_version: String::new(),
        }
    }
}

/// 当前库存的整体快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalInventorySnapshot {
    /// 当前根输出目录。
    pub root_output_dir: String,
    /// 当前扫描状态。
    pub status: LocalInventoryStatus,
    /// 库存版本号。
    pub inventory_version: String,
    /// 扫描开始时间。
    pub started_at: Option<String>,
    /// 扫描结束时间。
    pub finished_at: Option<String>,
    /// 已扫描文件数量。
    pub scanned_file_count: usize,
    /// 命中的歌曲数量。
    pub matched_track_count: usize,
    /// 校验通过的歌曲数量。
    pub verified_track_count: usize,
    /// 最近一次错误信息。
    pub last_error: Option<String>,
}

impl Default for LocalInventorySnapshot {
    fn default() -> Self {
        Self {
            root_output_dir: String::new(),
            status: LocalInventoryStatus::Idle,
            inventory_version: String::new(),
            started_at: None,
            finished_at: None,
            scanned_file_count: 0,
            matched_track_count: 0,
            verified_track_count: 0,
            last_error: None,
        }
    }
}

/// 扫描过程中发往前端的进度事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalInventoryScanProgressEvent {
    /// 当前扫描的根输出目录。
    pub root_output_dir: String,
    /// 当前扫描对应的库存版本号。
    pub inventory_version: String,
    /// 已扫描文件数。
    pub files_scanned: usize,
    /// 已命中的歌曲数。
    pub matched_track_count: usize,
    /// 已校验通过的歌曲数。
    pub verified_track_count: usize,
    /// 当前处理到的相对路径。
    pub current_path: Option<String>,
}

/// 判断某个状态是否应被视为“本地已下载”。
///
/// # 示例
///
/// ```
/// use harubble_core::local_inventory::is_downloaded_status;
/// use harubble_core::LocalTrackDownloadStatus;
///
/// assert!(is_downloaded_status(LocalTrackDownloadStatus::Verified));
/// assert!(!is_downloaded_status(LocalTrackDownloadStatus::Missing));
/// ```
pub fn is_downloaded_status(status: LocalTrackDownloadStatus) -> bool {
    matches!(
        status,
        LocalTrackDownloadStatus::Detected
            | LocalTrackDownloadStatus::Verified
            | LocalTrackDownloadStatus::Partial
            | LocalTrackDownloadStatus::Unverifiable
    )
}

/// 返回一个表示“未下载”的单曲徽标。
pub fn missing_track_badge(inventory_version: impl Into<String>) -> TrackDownloadBadge {
    badge_for_status(LocalTrackDownloadStatus::Missing, inventory_version)
}

/// 返回一个表示“未下载”的专辑徽标。
pub fn missing_album_badge(inventory_version: impl Into<String>) -> AlbumDownloadBadge {
    album_badge_for_status(LocalTrackDownloadStatus::Missing, inventory_version)
}

/// 根据校验模式为已发现文件构造单曲徽标。
pub fn badge_for_detected_file(
    verification_mode: VerificationMode,
    inventory_version: impl Into<String>,
) -> TrackDownloadBadge {
    let status = match verification_mode {
        VerificationMode::Strict => LocalTrackDownloadStatus::Unverifiable,
        VerificationMode::None | VerificationMode::WhenAvailable => {
            LocalTrackDownloadStatus::Detected
        }
    };
    badge_for_status(status, inventory_version)
}

/// 根据状态构造单曲徽标。
pub fn badge_for_status(
    status: LocalTrackDownloadStatus,
    inventory_version: impl Into<String>,
) -> TrackDownloadBadge {
    TrackDownloadBadge {
        is_downloaded: is_downloaded_status(status),
        download_status: status,
        inventory_version: inventory_version.into(),
    }
}

/// 根据状态构造专辑徽标。
pub fn album_badge_for_status(
    status: LocalTrackDownloadStatus,
    inventory_version: impl Into<String>,
) -> AlbumDownloadBadge {
    AlbumDownloadBadge {
        is_downloaded: is_downloaded_status(status),
        download_status: status,
        inventory_version: inventory_version.into(),
    }
}

/// 将多首歌曲的徽标聚合为专辑级徽标。
pub fn aggregate_album_download_badge(
    track_badges: &[TrackDownloadBadge],
    inventory_version: impl Into<String>,
) -> AlbumDownloadBadge {
    let inventory_version = inventory_version.into();

    if track_badges.is_empty() {
        return missing_album_badge(inventory_version);
    }

    let statuses = track_badges
        .iter()
        .map(|badge| badge.download_status)
        .collect::<Vec<_>>();

    if statuses
        .iter()
        .all(|status| *status == LocalTrackDownloadStatus::Missing)
    {
        return missing_album_badge(inventory_version);
    }

    if statuses.contains(&LocalTrackDownloadStatus::Mismatch) {
        return album_badge_for_status(LocalTrackDownloadStatus::Mismatch, inventory_version);
    }

    if statuses.contains(&LocalTrackDownloadStatus::Partial) {
        return album_badge_for_status(LocalTrackDownloadStatus::Partial, inventory_version);
    }

    let downloaded_count = statuses
        .iter()
        .filter(|status| is_downloaded_status(**status))
        .count();

    if downloaded_count == 0 {
        if statuses.contains(&LocalTrackDownloadStatus::Unknown) {
            return album_badge_for_status(LocalTrackDownloadStatus::Unknown, inventory_version);
        }
        return missing_album_badge(inventory_version);
    }

    if downloaded_count < statuses.len() {
        return album_badge_for_status(LocalTrackDownloadStatus::Partial, inventory_version);
    }

    if statuses
        .iter()
        .all(|status| *status == LocalTrackDownloadStatus::Verified)
    {
        return album_badge_for_status(LocalTrackDownloadStatus::Verified, inventory_version);
    }

    if statuses.contains(&LocalTrackDownloadStatus::Unverifiable) {
        return album_badge_for_status(LocalTrackDownloadStatus::Unverifiable, inventory_version);
    }

    if statuses.contains(&LocalTrackDownloadStatus::Detected) {
        return album_badge_for_status(LocalTrackDownloadStatus::Detected, inventory_version);
    }

    album_badge_for_status(LocalTrackDownloadStatus::Unknown, inventory_version)
}

/// 根据专辑目录下的文件证据推导专辑徽标。
pub fn album_badge_from_evidence(
    audio_files: &[LocalAudioFileEvidence],
    album_name: &str,
    inventory_version: impl Into<String>,
) -> AlbumDownloadBadge {
    let safe_album_name = sanitize_filename(album_name);
    let album_prefix = format!("{safe_album_name}/");

    if audio_files
        .iter()
        .any(|evidence| evidence.relative_path.starts_with(&album_prefix))
    {
        return album_badge_for_status(LocalTrackDownloadStatus::Partial, inventory_version);
    }

    missing_album_badge(inventory_version)
}

/// 根据命中的候选证据推导单曲徽标。
pub fn track_badge_from_matches(
    matches: &[MatchedTrackEvidence],
    verification_mode: VerificationMode,
    inventory_version: impl Into<String>,
) -> TrackDownloadBadge {
    if matches.is_empty() {
        return missing_track_badge(inventory_version);
    }

    if matches.len() > 1 {
        return badge_for_status(LocalTrackDownloadStatus::Partial, inventory_version);
    }

    match matches[0].verification_state {
        LocalAudioFileVerificationState::Verified => {
            badge_for_status(LocalTrackDownloadStatus::Verified, inventory_version)
        }
        LocalAudioFileVerificationState::Mismatch => {
            badge_for_status(LocalTrackDownloadStatus::Mismatch, inventory_version)
        }
        LocalAudioFileVerificationState::Unchecked => {
            badge_for_detected_file(verification_mode, inventory_version)
        }
    }
}

/// 根据专辑名与歌曲名筛出命中的本地音频证据。
pub fn matched_track_evidence(
    audio_files: &[LocalAudioFileEvidence],
    album_name: &str,
    song_name: &str,
) -> Vec<MatchedTrackEvidence> {
    let safe_song_name = sanitize_filename(song_name);
    let safe_album_name = sanitize_filename(album_name);
    let root_candidates = AUDIO_EXTENSIONS
        .iter()
        .map(|extension| format!("{safe_song_name}.{extension}"))
        .collect::<HashSet<_>>();
    let album_candidates = AUDIO_EXTENSIONS
        .iter()
        .map(|extension| format!("{safe_album_name}/{safe_song_name}.{extension}"))
        .collect::<HashSet<_>>();

    audio_files
        .iter()
        .filter_map(|evidence| {
            let match_rule = if root_candidates.contains(&evidence.relative_path) {
                Some(LocalTrackEvidenceMatchRule::RootRelativePath)
            } else if album_candidates.contains(&evidence.relative_path) {
                Some(LocalTrackEvidenceMatchRule::AlbumRelativePath)
            } else {
                None
            }?;

            Some(MatchedTrackEvidence {
                relative_path: evidence.relative_path.clone(),
                file_size: evidence.file_size,
                modified_at_ms: evidence.modified_at_ms,
                candidate_checksum: evidence.candidate_checksum.clone(),
                is_in_album_directory: evidence.is_in_album_directory,
                match_rule,
                verification_state: evidence.verification_state,
            })
        })
        .collect()
}

/// 生成歌曲在根目录与专辑子目录下的候选相对路径。
///
/// # 示例
///
/// ```
/// use harubble_core::candidate_relative_paths;
///
/// let paths = candidate_relative_paths("My Album", "Track/01");
///
/// assert!(paths.contains(&"Track_01.flac".to_string()));
/// assert!(paths.contains(&"My Album/Track_01.mp3".to_string()));
/// ```
pub fn candidate_relative_paths(album_name: &str, song_name: &str) -> Vec<String> {
    let safe_song_name = sanitize_filename(song_name);
    let safe_album_name = sanitize_filename(album_name);
    let mut candidates = Vec::with_capacity(AUDIO_EXTENSIONS.len() * 2);

    for extension in AUDIO_EXTENSIONS {
        candidates.push(format!("{safe_song_name}.{extension}"));
        candidates.push(format!("{safe_album_name}/{safe_song_name}.{extension}"));
    }

    candidates
}

/// 判断某首歌是否在候选路径集合中存在已下载文件。
pub fn has_detected_track(
    relative_audio_paths: &HashSet<String>,
    album_name: &str,
    song_name: &str,
) -> bool {
    candidate_relative_paths(album_name, song_name)
        .into_iter()
        .any(|candidate| relative_audio_paths.contains(&candidate))
}
