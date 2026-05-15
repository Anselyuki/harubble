//! 下载服务与 Tauri 事件系统之间的桥接层。
//!
//! 该模块负责把 `harubble-core` 的下载执行循环接入 Tauri 事件体系，使前端能够通过
//! 事件订阅下载状态、批次变更与任务进度。
//!
//! ## 流水线架构
//!
//! 单个批次内的任务按“两阶段流水线”执行：
//!
//! ```text
//! 歌曲 N:    [download (network)] ──► channel ──► [write (disk I/O)]
//! 歌曲 N+1:                        [download]  ──► channel ──► [write]
//! ```
//!
//! 下载阶段（HTTP 拉取）与写入阶段（磁盘 I/O、FLAC 标签写入、歌词侧车写入）会
//! 交叠执行，因此当 Song N 仍在写盘时，Song N+1 的下载已经可以开始。这里使用
//! 有界 `tokio::sync::mpsc` channel 提供背压：缓冲区里最多只会额外等待一个已完成的
//! [`WritePayload`]，从而把峰值内存控制在大约两首歌的音频数据量。

use crate::app_state::AppState;
use crate::downloads::events::{
    emit_download_job_updated, emit_download_manager_state_changed, DOWNLOAD_TASK_PROGRESS,
};
use crate::local_inventory::spawn_inventory_scan;
use harubble_core::download::model::{DownloadJobKind, DownloadTaskStatus, InternalDownloadTask};
use harubble_core::download::worker::{CompletedTaskArtifacts, TaskExecutionResult};
use harubble_core::WritePayload;
use harubble_core::{album_cover_exists, album_output_dir, download_album_cover};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// 初始化下载桥接执行循环。
///
/// 适用于 `main.rs` 启动阶段调用一次；该函数会在后台启动轮询循环，持续消费排队中
/// 的下载批次并驱动其完成。
pub fn initialize(app: &AppHandle, state: &AppState) {
    let app = app.clone();
    let state = state.clone();

    tauri::async_runtime::spawn(async move {
        execution_loop(&app, state).await;
    });
}

// ─── Write worker types ──────────────────────────────────────────────────────

/// 发送到写入 worker 的任务载荷。
struct WriteJob {
    /// 任务元数据（供 `execute_write_phase` 使用）。
    task: InternalDownloadTask,
    /// 下载阶段产出的写入载荷。
    payload: WritePayload,
    /// 用于把写入结果回传给流水线驱动侧的通道。
    result_tx: tokio::sync::oneshot::Sender<WriteResult>,
    /// 发出写入进度事件所需的上下文。
    progress_ctx: WriteProgressCtx,
}

/// 写入 worker 发出进度事件所需的全部上下文。
#[derive(Clone)]
struct WriteProgressCtx {
    service: Arc<tokio::sync::Mutex<harubble_core::DownloadService>>,
    app: AppHandle,
}

/// 一次写入操作的结果。
struct WriteResult {
    task: InternalDownloadTask,
    outcome: TaskExecutionResult,
}

struct StartedJob {
    job_id: String,
    write_tx: tokio::sync::mpsc::Sender<WriteJob>,
    write_worker_handle: tokio::task::JoinHandle<()>,
}

// ─── Main execution loop ─────────────────────────────────────────────────────

/// 下载桥接主执行循环。
///
/// 该循环会常驻运行，轮询排队中的批次并以下载/写入流水线策略驱动其完成，同时在
/// 关键状态迁移时持续发出 Tauri 事件。
async fn execution_loop(app: &AppHandle, state: AppState) {
    let service = Arc::clone(&state.download_service);
    let api = Arc::clone(&state.api);

    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;

        let Some(started_job) = start_job(app, &service).await else {
            continue;
        };

        let mut pending_write: Option<tokio::sync::oneshot::Receiver<WriteResult>> = None;

        loop {
            let task = {
                let mut svc = service.lock().await;
                svc.pop_next_task(&started_job.job_id)
            };

            let Some((task, preparing_snapshot)) = task else {
                break;
            };

            state.persist_download_snapshot(&service.lock().await.manager_snapshot());
            emit_download_job_updated(app, &preparing_snapshot);
            let manager_snapshot = service.lock().await.manager_snapshot();
            emit_download_manager_state_changed(app, &manager_snapshot);

            let task_id = task.id.clone();
            let job_id_clone = started_job.job_id.clone();
            let cancellation_flag = {
                let svc = service.lock().await;
                svc.active_task_cancel_flag(&started_job.job_id, &task_id)
            };
            let out_dir = prepare_task_output_dir(
                &service,
                &api,
                &started_job.job_id,
                &task,
                cancellation_flag.as_ref(),
            )
            .await;
            let download_result =
                run_download_phase(app, &service, &api, &task, &out_dir, cancellation_flag).await;

            match download_result {
                Ok(payload) => {
                    let (result_tx, result_rx) = tokio::sync::oneshot::channel();

                    flush_pending_write(&mut pending_write, &state, app).await;

                    let _ = started_job
                        .write_tx
                        .send(WriteJob {
                            task: task.clone(),
                            payload,
                            result_tx,
                            progress_ctx: WriteProgressCtx {
                                service: Arc::clone(&service),
                                app: app.clone(),
                            },
                        })
                        .await;

                    pending_write = Some(result_rx);
                }
                Err(failed_result) => {
                    let (final_status, output_path_str, error, _) =
                        unpack_task_result(failed_result);

                    let update = {
                        let mut svc = service.lock().await;
                        svc.update_task_state(
                            &job_id_clone,
                            &task_id,
                            final_status,
                            None,
                            None,
                            output_path_str.as_deref(),
                            error,
                        )
                    };

                    if let Some(update) = update {
                        let manager_snapshot = service.lock().await.manager_snapshot();
                        state.persist_download_snapshot(&manager_snapshot);
                        emit_download_job_updated(app, &update.snapshot);
                        emit_download_manager_state_changed(app, &manager_snapshot);
                    }
                }
            }
        }

        flush_pending_write(&mut pending_write, &state, app).await;
        finalize_job(app, &service, started_job).await;
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

async fn start_job(
    app: &AppHandle,
    service: &Arc<tokio::sync::Mutex<harubble_core::DownloadService>>,
) -> Option<StartedJob> {
    let job_snapshot = {
        let mut svc = service.lock().await;
        svc.start_next_queued_job()
    }?;

    let manager_snapshot = service.lock().await.manager_snapshot();
    if let Some(state) = app.try_state::<AppState>() {
        state.persist_download_snapshot(&manager_snapshot);
    }
    emit_download_job_updated(app, &job_snapshot);
    emit_download_manager_state_changed(app, &manager_snapshot);

    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<WriteJob>(1);
    let write_worker_handle = tokio::spawn(async move {
        while let Some(job) = write_rx.recv().await {
            let progress_ctx = job.progress_ctx.clone();
            let task_for_write = job.task.clone();
            let task_for_result = job.task;

            let outcome = tokio::task::spawn_blocking(move || {
                task_for_write.execute_write_phase(&job.payload, {
                    let service = progress_ctx.service;
                    let app = progress_ctx.app;
                    move |progress| {
                        let service = Arc::clone(&service);
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let update = {
                                let mut svc = service.lock().await;
                                svc.update_task_state(
                                    &progress.job_id,
                                    &progress.task_id,
                                    progress.status,
                                    Some(progress.bytes_done),
                                    progress.bytes_total,
                                    None,
                                    None,
                                )
                            };
                            if let Some(update) = update {
                                let _ = app.emit(DOWNLOAD_TASK_PROGRESS, &progress);
                                if update.should_persist {
                                    let manager_snapshot = service.lock().await.manager_snapshot();
                                    if let Some(state) = app.try_state::<AppState>() {
                                        state.persist_download_snapshot(&manager_snapshot);
                                    }
                                }
                                emit_download_job_updated(&app, &update.snapshot);
                            }
                        });
                    }
                })
            })
            .await
            .unwrap_or_else(|_| {
                TaskExecutionResult::Failed(harubble_core::download::model::DownloadErrorInfo {
                    code: harubble_core::download::model::DownloadErrorCode::Internal,
                    message: "Write worker panicked".to_string(),
                    retryable: false,
                    details: None,
                })
            });

            let _ = job.result_tx.send(WriteResult {
                task: task_for_result,
                outcome,
            });
        }
    });

    Some(StartedJob {
        job_id: job_snapshot.id.clone(),
        write_tx,
        write_worker_handle,
    })
}

fn resolve_task_output_dir(
    job_kind: DownloadJobKind,
    base_output_dir: &Path,
    task: &InternalDownloadTask,
) -> PathBuf {
    match job_kind {
        DownloadJobKind::Song | DownloadJobKind::Album | DownloadJobKind::Selection => {
            album_output_dir(base_output_dir, &task.album_name)
        }
    }
}

async fn prepare_task_output_dir(
    service: &Arc<tokio::sync::Mutex<harubble_core::DownloadService>>,
    api: &Arc<harubble_core::ApiClient>,
    job_id: &str,
    task: &InternalDownloadTask,
    cancellation_flag: Option<&Arc<std::sync::atomic::AtomicBool>>,
) -> PathBuf {
    let (job_kind, base_output_dir) = {
        let svc = service.lock().await;
        let job_snapshot = svc.get_job(job_id);
        let job_kind = job_snapshot
            .as_ref()
            .map(|job| job.kind)
            .unwrap_or(DownloadJobKind::Song);
        let output_dir = svc
            .job_output_dir(job_id)
            .unwrap_or_else(|| ".".to_string());
        (job_kind, PathBuf::from(output_dir))
    };

    let out_dir = resolve_task_output_dir(job_kind, &base_output_dir, task);

    if matches!(job_kind, DownloadJobKind::Album) && !album_cover_exists(&out_dir) {
        let _ = tokio::fs::create_dir_all(&out_dir).await;
        if let Ok(album) = api.get_album_detail(&task.album_cid).await {
            let _ = download_album_cover(api.as_ref(), &album, &out_dir, cancellation_flag).await;
        }
    }

    out_dir
}

async fn run_download_phase(
    app: &AppHandle,
    service: &Arc<tokio::sync::Mutex<harubble_core::DownloadService>>,
    api: &Arc<harubble_core::ApiClient>,
    task: &InternalDownloadTask,
    out_dir: &Path,
    cancellation_flag: Option<Arc<std::sync::atomic::AtomicBool>>,
) -> Result<WritePayload, TaskExecutionResult> {
    let service_for_progress = Arc::clone(service);
    let app_for_progress = app.clone();

    task.execute_download_phase(api.as_ref(), out_dir, cancellation_flag, {
        let service = Arc::clone(&service_for_progress);
        let app = app_for_progress.clone();
        move |progress| {
            let service = Arc::clone(&service);
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let update = {
                    let mut svc = service.lock().await;
                    svc.update_task_state(
                        &progress.job_id,
                        &progress.task_id,
                        progress.status,
                        Some(progress.bytes_done),
                        progress.bytes_total,
                        None,
                        None,
                    )
                };
                if let Some(update) = update {
                    let _ = app.emit(DOWNLOAD_TASK_PROGRESS, &progress);
                    if update.should_persist {
                        let manager_snapshot = service.lock().await.manager_snapshot();
                        if let Some(app_state) = app.try_state::<AppState>() {
                            app_state.persist_download_snapshot(&manager_snapshot);
                        }
                    }
                    emit_download_job_updated(&app, &update.snapshot);
                }
            });
        }
    })
    .await
}

async fn flush_pending_write(
    pending_write: &mut Option<tokio::sync::oneshot::Receiver<WriteResult>>,
    state: &AppState,
    app: &AppHandle,
) {
    let Some(rx) = pending_write.take() else {
        return;
    };
    collect_write_result(rx, state, app).await;
}

async fn finalize_job(
    app: &AppHandle,
    service: &Arc<tokio::sync::Mutex<harubble_core::DownloadService>>,
    started_job: StartedJob,
) {
    drop(started_job.write_tx);
    let _ = started_job.write_worker_handle.await;

    let snapshot = {
        let mut svc = service.lock().await;
        svc.finish_job(&started_job.job_id)
    };

    if let Some(s) = snapshot {
        let manager_snapshot = service.lock().await.manager_snapshot();
        if let Some(state) = app.try_state::<AppState>() {
            state.persist_download_snapshot(&manager_snapshot);
        }
        emit_download_job_updated(app, &s);
        emit_download_manager_state_changed(app, &manager_snapshot);

        crate::notification::notify_download_completed(app, &s);
    }
}

async fn collect_write_result(
    rx: tokio::sync::oneshot::Receiver<WriteResult>,
    state: &AppState,
    app: &AppHandle,
) {
    let write_result = match rx.await {
        Ok(r) => r,
        Err(_) => return,
    };

    let (final_status, output_path_str, error, completed_artifacts) =
        unpack_task_result(write_result.outcome);

    let update = {
        let mut svc = state.download_service.lock().await;
        svc.update_task_state(
            &write_result.task.job_id,
            &write_result.task.id,
            final_status,
            None,
            None,
            output_path_str.as_deref(),
            error,
        )
    };

    if let Some(artifacts) = completed_artifacts {
        let root_output_dir = state.preferences().output_dir;
        let _ = state
            .local_inventory_provenance_store
            .record_completed_download(
                PathBuf::from(&root_output_dir).as_path(),
                &write_result.task,
                &artifacts,
            )
            .await;
        spawn_inventory_scan(app.clone(), state.clone(), root_output_dir, None);
    }

    if let Some(update) = update {
        let manager_snapshot = state.download_service.lock().await.manager_snapshot();
        state.persist_download_snapshot(&manager_snapshot);
        emit_download_job_updated(app, &update.snapshot);
        emit_download_manager_state_changed(app, &manager_snapshot);
    }
}

fn unpack_task_result(
    result: TaskExecutionResult,
) -> (
    DownloadTaskStatus,
    Option<String>,
    Option<harubble_core::download::model::DownloadErrorInfo>,
    Option<CompletedTaskArtifacts>,
) {
    match result {
        TaskExecutionResult::Completed(artifacts) => (
            DownloadTaskStatus::Completed,
            Some(artifacts.output_path.clone()),
            None,
            Some(artifacts),
        ),
        TaskExecutionResult::Cancelled => (
            DownloadTaskStatus::Cancelled,
            None,
            Some(harubble_core::download::model::DownloadErrorInfo {
                code: harubble_core::download::model::DownloadErrorCode::Cancelled,
                message: "Cancelled by user".to_string(),
                retryable: false,
                details: None,
            }),
            None,
        ),
        TaskExecutionResult::Failed(info) => (DownloadTaskStatus::Failed, None, Some(info), None),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_task_output_dir;
    use harubble_core::audio::OutputFormat;
    use harubble_core::download::model::{
        DownloadJobKind, DownloadTaskStatus, InternalDownloadTask,
    };
    use std::path::Path;

    fn make_task(album_name: &str) -> InternalDownloadTask {
        InternalDownloadTask {
            id: "task-1".to_string(),
            job_id: "job-1".to_string(),
            song_cid: "song-1".to_string(),
            song_name: "Song".to_string(),
            artists: vec!["Artist".to_string()],
            album_cid: "album-1".to_string(),
            album_name: album_name.to_string(),
            status: DownloadTaskStatus::Queued,
            bytes_done: 0,
            bytes_total: None,
            output_path: None,
            error: None,
            attempt: 0,
            song_index: 0,
            song_count: 1,
            format: OutputFormat::Flac,
            download_lyrics: true,
        }
    }

    #[test]
    fn song_jobs_use_album_subdirectory() {
        let task = make_task("A/B:C?D");

        let out_dir =
            resolve_task_output_dir(DownloadJobKind::Song, Path::new("/tmp/downloads"), &task);

        assert_eq!(out_dir, Path::new("/tmp/downloads").join("A_B_C_D"));
    }

    #[test]
    fn album_jobs_use_album_subdirectory() {
        let task = make_task("My Album");

        let out_dir =
            resolve_task_output_dir(DownloadJobKind::Album, Path::new("/tmp/downloads"), &task);

        assert_eq!(out_dir, Path::new("/tmp/downloads").join("My Album"));
    }

    #[test]
    fn selection_jobs_use_album_subdirectory() {
        let task = make_task("My Album");

        let out_dir = resolve_task_output_dir(
            DownloadJobKind::Selection,
            Path::new("/tmp/downloads"),
            &task,
        );

        assert_eq!(out_dir, Path::new("/tmp/downloads").join("My Album"));
    }

    #[test]
    fn sanitizes_all_illegal_path_characters() {
        let task = make_task("A\\B*C\"D<E>F|G\0H");

        let out_dir = resolve_task_output_dir(DownloadJobKind::Album, Path::new("/out"), &task);

        let dir_name = out_dir.file_name().unwrap().to_str().unwrap();
        assert!(
            !dir_name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|', '\0']),
            "output dir should not contain illegal characters: {dir_name}"
        );
    }

    #[test]
    fn empty_album_name_produces_valid_path() {
        let task = make_task("");

        let out_dir =
            resolve_task_output_dir(DownloadJobKind::Song, Path::new("/tmp/downloads"), &task);

        assert!(out_dir.starts_with("/tmp/downloads"));
    }
}
