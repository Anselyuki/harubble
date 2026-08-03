//! 下载任务执行器。
//!
//! 该模块负责执行单个下载任务，包括网络下载、格式转换、标签写入与歌词侧车
//! 文件写入。
//!
//! ## 流水线支持
//!
//! 除了串行执行的 [`InternalDownloadTask::execute`] 之外，还提供两段式接口：
//!
//! - [`InternalDownloadTask::execute_download_phase`]：仅执行网络下载，返回 [`WritePayload`]。
//! - [`InternalDownloadTask::execute_write_phase`]：仅执行本地磁盘写入，消费 [`WritePayload`]。

use crate::api::ApiClient;
use crate::download::model::{
    DownloadErrorCode, DownloadErrorInfo, DownloadTaskProgressEvent, InternalDownloadTask,
};
use crate::downloader::{
    download_song_payload, write_payload_to_disk, DownloadCancelledError, DownloadProvenanceSeed,
    MetaOverride, WritePayload,
};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn is_cancellation_error(e: &anyhow::Error) -> bool {
    e.downcast_ref::<DownloadCancelledError>().is_some()
}

/// 已完成任务产出的关键工件信息。
///
/// 适用于任务执行成功后把最终写盘结果与来源证明一并交还给上层持久化或事件广播。
#[derive(Debug, Clone)]
pub struct CompletedTaskArtifacts {
    /// 最终写出的文件路径。
    pub output_path: String,
    /// 记录本次下载与写入链路的来源信息。
    pub provenance_seed: DownloadProvenanceSeed,
}

/// 下载任务执行结果。
///
/// 用于统一表达成功、用户取消与失败三类终态；失败时会附带已分类的
/// [`DownloadErrorInfo`]，成功时会返回可继续持久化的工件信息。
#[derive(Debug)]
pub enum TaskExecutionResult {
    /// 任务执行成功。
    Completed(CompletedTaskArtifacts),
    /// 任务被用户取消。
    Cancelled,
    /// 任务执行失败。
    Failed(DownloadErrorInfo),
}

impl InternalDownloadTask {
    /// 串行执行当前任务的下载与写入阶段。
    ///
    /// 该方法会完成歌曲详情拉取、资源下载、格式转换、标签写入以及歌词侧车
    /// 文件写入。
    pub async fn execute<F>(
        &self,
        api: &ApiClient,
        output_dir: &Path,
        cancellation_flag: Option<Arc<AtomicBool>>,
        on_progress: F,
    ) -> TaskExecutionResult
    where
        F: Fn(DownloadTaskProgressEvent) + Send + Sync + Clone + 'static,
    {
        if is_cancelled(cancellation_flag.as_ref()) {
            return TaskExecutionResult::Cancelled;
        }

        // Fetch song and album details first
        let song = match api.get_song_detail(&self.song_cid).await {
            Ok(s) => s,
            Err(e) => {
                return TaskExecutionResult::Failed(make_error(
                    DownloadErrorCode::Api,
                    "Failed to fetch song detail",
                    e,
                    true,
                ));
            }
        };

        let album = match api.get_album_detail(&self.album_cid).await {
            Ok(a) => a,
            Err(e) => {
                return TaskExecutionResult::Failed(make_error(
                    DownloadErrorCode::Api,
                    "Failed to fetch album detail",
                    e,
                    true,
                ));
            }
        };

        if is_cancelled(cancellation_flag.as_ref()) {
            return TaskExecutionResult::Cancelled;
        }

        // Build progress callback that maps to our event format
        let task_id = self.id.clone();
        let job_id = self.job_id.clone();
        let song_index = self.song_index;
        let song_count = self.song_count;

        // Speed calculation state (using Arc<Mutex> for thread-safe interior mutability)
        let start_time = Instant::now();
        let last_bytes = Arc::new(AtomicU64::new(0));
        let last_time = Arc::new(Mutex::new(start_time));

        let payload_result = download_song_payload(
            api,
            &song,
            &album,
            output_dir,
            self.format,
            self.download_lyrics,
            &MetaOverride {
                album_name: String::new(),
                artists: Vec::new(),
                album_artists: Vec::new(),
            },
            cancellation_flag.clone(),
            {
                let on_progress = on_progress.clone();
                let last_bytes = Arc::clone(&last_bytes);
                let last_time = Arc::clone(&last_time);
                move |progress| {
                    let now = Instant::now();
                    let prev_time = {
                        let mut time_guard = last_time.lock().unwrap_or_else(|e| e.into_inner());
                        let prev = *time_guard;
                        *time_guard = now;
                        prev
                    };
                    let elapsed = now.duration_since(prev_time).as_secs_f64();

                    let prev_bytes = last_bytes.swap(progress.bytes_done, Ordering::Relaxed);
                    let speed_bytes_per_sec = if elapsed > 0.0 {
                        let bytes_delta = progress.bytes_done.saturating_sub(prev_bytes);
                        bytes_delta as f64 / elapsed
                    } else {
                        0.0
                    };

                    on_progress(DownloadTaskProgressEvent {
                        job_id: job_id.clone(),
                        task_id: task_id.clone(),
                        status: progress.status,
                        bytes_done: progress.bytes_done,
                        bytes_total: progress.bytes_total,
                        song_index,
                        song_count,
                        speed_bytes_per_sec,
                    });
                }
            },
        )
        .await;

        match payload_result {
            Ok(payload) => match write_payload_to_disk(&payload, None) {
                Ok(path) => TaskExecutionResult::Completed(CompletedTaskArtifacts {
                    output_path: path.to_string_lossy().to_string(),
                    provenance_seed: payload.provenance_seed,
                }),
                Err(e) => {
                    if is_cancellation_error(&e) {
                        TaskExecutionResult::Cancelled
                    } else {
                        TaskExecutionResult::Failed(classify_error(e))
                    }
                }
            },
            Err(e) => {
                if is_cancellation_error(&e) {
                    TaskExecutionResult::Cancelled
                } else {
                    TaskExecutionResult::Failed(classify_error(e))
                }
            }
        }
    }

    /// 仅执行当前任务的网络下载阶段。
    ///
    /// 成功时返回可交给写入线程继续处理的 [`WritePayload`]。
    pub async fn execute_download_phase<F>(
        &self,
        api: &ApiClient,
        output_dir: &Path,
        cancellation_flag: Option<Arc<AtomicBool>>,
        on_progress: F,
    ) -> Result<WritePayload, TaskExecutionResult>
    where
        F: Fn(DownloadTaskProgressEvent) + Send + Sync + Clone + 'static,
    {
        if is_cancelled(cancellation_flag.as_ref()) {
            return Err(TaskExecutionResult::Cancelled);
        }

        let song = match api.get_song_detail(&self.song_cid).await {
            Ok(s) => s,
            Err(e) => {
                return Err(TaskExecutionResult::Failed(make_error(
                    DownloadErrorCode::Api,
                    "Failed to fetch song detail",
                    e,
                    true,
                )));
            }
        };

        let album = match api.get_album_detail(&self.album_cid).await {
            Ok(a) => a,
            Err(e) => {
                return Err(TaskExecutionResult::Failed(make_error(
                    DownloadErrorCode::Api,
                    "Failed to fetch album detail",
                    e,
                    true,
                )));
            }
        };

        if is_cancelled(cancellation_flag.as_ref()) {
            return Err(TaskExecutionResult::Cancelled);
        }

        let task_id = self.id.clone();
        let job_id = self.job_id.clone();
        let song_index = self.song_index;
        let song_count = self.song_count;

        let start_time = Instant::now();
        let last_bytes = Arc::new(AtomicU64::new(0));
        let last_time = Arc::new(Mutex::new(start_time));

        let payload_result = download_song_payload(
            api,
            &song,
            &album,
            output_dir,
            self.format,
            self.download_lyrics,
            &MetaOverride {
                album_name: String::new(),
                artists: Vec::new(),
                album_artists: Vec::new(),
            },
            cancellation_flag,
            {
                let on_progress = on_progress.clone();
                let last_bytes = Arc::clone(&last_bytes);
                let last_time = Arc::clone(&last_time);
                move |progress| {
                    let now = Instant::now();
                    let prev_time = {
                        let mut time_guard = last_time.lock().unwrap_or_else(|e| e.into_inner());
                        let prev = *time_guard;
                        *time_guard = now;
                        prev
                    };
                    let elapsed = now.duration_since(prev_time).as_secs_f64();

                    let prev_bytes = last_bytes.swap(progress.bytes_done, Ordering::Relaxed);
                    let speed_bytes_per_sec = if elapsed > 0.0 {
                        let bytes_delta = progress.bytes_done.saturating_sub(prev_bytes);
                        bytes_delta as f64 / elapsed
                    } else {
                        0.0
                    };

                    on_progress(DownloadTaskProgressEvent {
                        job_id: job_id.clone(),
                        task_id: task_id.clone(),
                        status: progress.status,
                        bytes_done: progress.bytes_done,
                        bytes_total: progress.bytes_total,
                        song_index,
                        song_count,
                        speed_bytes_per_sec,
                    });
                }
            },
        )
        .await;

        match payload_result {
            Ok(payload) => Ok(payload),
            Err(e) => {
                if is_cancellation_error(&e) {
                    Err(TaskExecutionResult::Cancelled)
                } else {
                    Err(TaskExecutionResult::Failed(classify_error(e)))
                }
            }
        }
    }

    /// 仅执行当前任务的本地写入阶段。
    ///
    /// 该方法与 [`execute_download_phase`](Self::execute_download_phase) 配对使用。
    pub fn execute_write_phase<F>(
        &self,
        payload: &WritePayload,
        on_progress: F,
    ) -> TaskExecutionResult
    where
        F: Fn(DownloadTaskProgressEvent) + Send + Sync + Clone + 'static,
    {
        let task_id = self.id.clone();
        let job_id = self.job_id.clone();
        let song_index = self.song_index;
        let song_count = self.song_count;

        let progress_adapter = |progress: crate::downloader::DownloadProgress| {
            on_progress(DownloadTaskProgressEvent {
                job_id: job_id.clone(),
                task_id: task_id.clone(),
                status: progress.status,
                bytes_done: progress.bytes_done,
                bytes_total: progress.bytes_total,
                song_index,
                song_count,
                speed_bytes_per_sec: 0.0,
            });
        };

        match write_payload_to_disk(payload, Some(&progress_adapter)) {
            Ok(path) => TaskExecutionResult::Completed(CompletedTaskArtifacts {
                output_path: path.to_string_lossy().to_string(),
                provenance_seed: payload.provenance_seed.clone(),
            }),
            Err(e) => {
                if is_cancellation_error(&e) {
                    TaskExecutionResult::Cancelled
                } else {
                    TaskExecutionResult::Failed(classify_error(e))
                }
            }
        }
    }
}

fn sanitized_error_details(error: &anyhow::Error) -> Option<String> {
    let message = error
        .chain()
        .last()
        .map(ToString::to_string)
        .unwrap_or_else(|| error.to_string());
    if message.is_empty() {
        return None;
    }

    let normalized = message.replace('\\', "/");
    let looks_like_path = normalized.starts_with('/') || normalized.contains(":/");
    if looks_like_path {
        return Path::new(&normalized)
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .filter(|value| !value.is_empty());
    }

    if normalized.contains('/') {
        return None;
    }

    Some(normalized)
}

fn make_error(
    code: DownloadErrorCode,
    message: &str,
    e: anyhow::Error,
    retryable: bool,
) -> DownloadErrorInfo {
    DownloadErrorInfo {
        code,
        message: message.to_string(),
        retryable,
        details: sanitized_error_details(&e),
    }
}

fn is_cancelled(cancellation_flag: Option<&Arc<AtomicBool>>) -> bool {
    cancellation_flag
        .map(|flag| flag.load(Ordering::SeqCst))
        .unwrap_or(false)
}

fn classify_error(e: anyhow::Error) -> DownloadErrorInfo {
    let msg = format!("{e:#}");
    let lower = msg.to_ascii_lowercase();

    if lower.contains("cancelled") || lower.contains("canceled") {
        return make_error(DownloadErrorCode::Cancelled, "Cancelled by user", e, false);
    }

    if lower.contains("lyric") || lower.contains("lrc") {
        return make_error(DownloadErrorCode::Lyrics, "Lyrics download failed", e, true);
    }

    if lower.contains("metadata") || lower.contains("metaflac") || lower.contains("tag") {
        return make_error(
            DownloadErrorCode::Tagging,
            "Failed to write metadata",
            e,
            false,
        );
    }

    if lower.contains("network")
        || lower.contains("connection")
        || lower.contains("timeout")
        || lower.contains("dns")
        || lower.contains("tls")
    {
        return make_error(DownloadErrorCode::Network, "Network error", e, true);
    }

    if lower.contains("api") || lower.contains("404") || lower.contains("500") {
        return make_error(DownloadErrorCode::Api, "API error", e, true);
    }

    // Typed IO causes take precedence over codec keywords in surrounding context such as
    // "Failed to write FLAC file". Metadata and lyric failures keep their explicit categories.
    if e.downcast_ref::<std::io::Error>().is_some() {
        return make_error(DownloadErrorCode::Io, "IO error", e, false);
    }

    if lower.contains("decode")
        || lower.contains("encode")
        || lower.contains("flac")
        || lower.contains("wav sample")
        || lower.contains("sample format")
    {
        return make_error(DownloadErrorCode::Decode, "Decode/encode error", e, false);
    }

    if looks_like_io_error(&lower) {
        return make_error(DownloadErrorCode::Io, "IO error", e, false);
    }

    make_error(DownloadErrorCode::Internal, "Internal error", e, false)
}

fn looks_like_io_error(lower_message: &str) -> bool {
    lower_message.contains("write")
        || lower_message.contains("save audio")
        || lower_message.contains("sidecar")
        || lower_message.contains("failed to save")
        || lower_message.contains("failed to write")
        || lower_message.contains("disk")
        || lower_message.contains("permission denied")
        || lower_message.contains("no space left")
}

#[cfg(test)]
mod tests {
    use super::classify_error;
    use crate::download::model::DownloadErrorCode;
    use anyhow::anyhow;

    #[test]
    fn classifies_lyric_errors_explicitly() {
        let error = classify_error(anyhow!("Failed to save lyric sidecar for song"));

        assert!(matches!(error.code, DownloadErrorCode::Lyrics));
        assert!(error.retryable);
    }

    #[test]
    fn classifies_metadata_errors_before_io_errors() {
        let error = classify_error(anyhow!("Failed to write FLAC metadata for song"));

        assert!(matches!(error.code, DownloadErrorCode::Tagging));
        assert!(!error.retryable);
    }

    #[test]
    fn classifies_io_errors_from_underlying_io_type() {
        let io_error =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let error = classify_error(io_error.into());

        assert!(matches!(error.code, DownloadErrorCode::Io));
        assert!(!error.retryable);
    }

    #[test]
    fn classifies_flac_write_failures_from_underlying_io_type() {
        let io_error =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let error = anyhow!(io_error)
            .context("Failed to write FLAC file")
            .context("Failed to save audio file for song");
        let error = classify_error(error);

        assert!(matches!(error.code, DownloadErrorCode::Io));
        assert_eq!(error.details.as_deref(), Some("permission denied"));
        assert!(!error.retryable);
    }

    #[test]
    fn classifies_nested_wav_sample_errors_as_decode_errors() {
        let error = anyhow!("The sample format differs from the destination format")
            .context("Failed to read float WAV sample")
            .context("Failed to save audio file for song");
        let error = classify_error(error);

        assert!(matches!(error.code, DownloadErrorCode::Decode));
        assert_eq!(
            error.details.as_deref(),
            Some("The sample format differs from the destination format")
        );
        assert!(!error.retryable);
    }
}
