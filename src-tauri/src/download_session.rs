use crate::logging::{LogCenter, LogLevel, LogPayload};
use crate::preferences::Locale;
use harubble_core::download::model::{
    DownloadErrorCode, DownloadErrorInfo, DownloadJobSnapshot, DownloadJobStatus,
    DownloadManagerSnapshot, DownloadTaskSnapshot, DownloadTaskStatus,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

const DOWNLOAD_SESSION_SCHEMA_VERSION: u32 = 1;
const DOWNLOAD_SESSION_FILE_NAME: &str = "download_session.json";
const MAX_TERMINAL_JOB_HISTORY: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedDownloadSession {
    schema_version: u32,
    saved_at: String,
    manager: DownloadManagerSnapshot,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedDownloadSession {
    pub(crate) snapshot: DownloadManagerSnapshot,
    pub(crate) should_persist: bool,
}

#[derive(Clone)]
pub(crate) struct DownloadSessionStore {
    path: PathBuf,
    save_lock: Arc<Mutex<()>>,
}

impl DownloadSessionStore {
    pub(crate) fn new(app_data_dir: PathBuf) -> Self {
        Self {
            path: app_data_dir.join(DOWNLOAD_SESSION_FILE_NAME),
            save_lock: Arc::new(Mutex::new(())),
        }
    }

    pub(crate) fn load(
        &self,
        log_center: Option<&LogCenter>,
        locale: Locale,
    ) -> LoadedDownloadSession {
        if !self.path.exists() {
            return LoadedDownloadSession {
                snapshot: DownloadManagerSnapshot::default(),
                should_persist: false,
            };
        }

        let content = match fs::read_to_string(&self.path) {
            Ok(content) => content,
            Err(error) => {
                log_persistence_error(
                    log_center,
                    "download_session.read_failed",
                    "Failed to read persisted download session",
                    &crate::i18n::tr(locale, "download-session-read-failed"),
                    error.to_string(),
                );
                return LoadedDownloadSession {
                    snapshot: DownloadManagerSnapshot::default(),
                    should_persist: false,
                };
            }
        };

        if content.trim().is_empty() {
            return LoadedDownloadSession {
                snapshot: DownloadManagerSnapshot::default(),
                should_persist: false,
            };
        }

        let persisted = match serde_json::from_str::<PersistedDownloadSession>(&content) {
            Ok(persisted) => persisted,
            Err(error) => {
                log_persistence_error(
                    log_center,
                    "download_session.parse_failed",
                    "Failed to parse persisted download session",
                    &crate::i18n::tr(locale, "download-session-parse-failed"),
                    error.to_string(),
                );
                return LoadedDownloadSession {
                    snapshot: DownloadManagerSnapshot::default(),
                    should_persist: false,
                };
            }
        };

        if persisted.schema_version != DOWNLOAD_SESSION_SCHEMA_VERSION {
            log_persistence_error(
                log_center,
                "download_session.unsupported_schema",
                "Unsupported persisted download session schema version",
                &crate::i18n::tr(locale, "download-session-schema-incompatible"),
                format!(
                    "expected schema {}, got {}",
                    DOWNLOAD_SESSION_SCHEMA_VERSION, persisted.schema_version
                ),
            );
            return LoadedDownloadSession {
                snapshot: DownloadManagerSnapshot::default(),
                should_persist: false,
            };
        }

        let (normalized_snapshot, normalized_changed) =
            normalize_restored_snapshot(persisted.manager, locale);
        let (retained_snapshot, retention_changed) = apply_retention(normalized_snapshot);

        LoadedDownloadSession {
            snapshot: retained_snapshot,
            should_persist: normalized_changed || retention_changed,
        }
    }

    pub(crate) fn save(
        &self,
        snapshot: &DownloadManagerSnapshot,
        locale: Locale,
    ) -> Result<(), String> {
        let _guard = self.save_lock.lock().map_err(|error| error.to_string())?;

        let parent = self
            .path
            .parent()
            .ok_or_else(|| crate::i18n::tr(locale, "download-session-dir-invalid"))?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;

        let persisted = PersistedDownloadSession {
            schema_version: DOWNLOAD_SESSION_SCHEMA_VERSION,
            saved_at: iso_timestamp_now(),
            manager: apply_retention(snapshot.clone()).0,
        };
        let content = serde_json::to_vec_pretty(&persisted).map_err(|error| error.to_string())?;

        let mut temp_file = NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
        temp_file
            .write_all(&content)
            .and_then(|_| temp_file.as_file().sync_all())
            .map_err(|error| error.to_string())?;

        temp_file.persist(&self.path).map_err(|error| {
            let tempfile::PersistError { error, .. } = error;
            error.to_string()
        })?;

        Ok(())
    }
}

const INTERRUPTED_SENTINEL: &str = "interrupted-by-restart";

fn normalize_restored_snapshot(
    snapshot: DownloadManagerSnapshot,
    locale: Locale,
) -> (DownloadManagerSnapshot, bool) {
    let recovery_finished_at = iso_timestamp_now();
    let mut changed = snapshot.active_job_id.is_some() || !snapshot.queued_job_ids.is_empty();
    let jobs = snapshot
        .jobs
        .into_iter()
        .map(|job| {
            let (job, job_changed) = normalize_restored_job(job, &recovery_finished_at, locale);
            changed |= job_changed;
            job
        })
        .collect();

    (
        DownloadManagerSnapshot {
            jobs,
            active_job_id: None,
            queued_job_ids: Vec::new(),
        },
        changed,
    )
}

fn normalize_restored_job(
    mut job: DownloadJobSnapshot,
    recovery_finished_at: &str,
    locale: Locale,
) -> (DownloadJobSnapshot, bool) {
    job.tasks = job
        .tasks
        .into_iter()
        .map(|task| normalize_restored_task(task, locale))
        .collect();

    let mut changed = job.tasks.iter().any(|task| {
        task.error
            .as_ref()
            .is_some_and(|error| error.details.as_deref() == Some(INTERRUPTED_SENTINEL))
    });
    let previous_status = job.status;
    let previous_finished_at = job.finished_at.clone();
    let completed_task_count = job
        .tasks
        .iter()
        .filter(|task| task.status == DownloadTaskStatus::Completed)
        .count();
    let failed_task_count = job
        .tasks
        .iter()
        .filter(|task| task.status == DownloadTaskStatus::Failed)
        .count();
    let cancelled_task_count = job
        .tasks
        .iter()
        .filter(|task| task.status == DownloadTaskStatus::Cancelled)
        .count();

    job.task_count = job.tasks.len();
    job.completed_task_count = completed_task_count;
    job.failed_task_count = failed_task_count;
    job.cancelled_task_count = cancelled_task_count;
    job.status = derive_job_status(&job.tasks, job.status);
    if is_terminal_job_status(job.status) && job.finished_at.is_none() {
        job.finished_at = Some(recovery_finished_at.to_string());
    }
    changed |= job.status != previous_status || job.finished_at != previous_finished_at;

    (job, changed)
}

fn normalize_restored_task(mut task: DownloadTaskSnapshot, locale: Locale) -> DownloadTaskSnapshot {
    match task.status {
        DownloadTaskStatus::Queued | DownloadTaskStatus::Preparing => {
            task.status = DownloadTaskStatus::Cancelled;
            if task.error.is_none() {
                task.error = Some(interrupted_cancelled_error(locale));
            }
        }
        DownloadTaskStatus::Downloading | DownloadTaskStatus::Writing => {
            task.status = DownloadTaskStatus::Failed;
            if task.error.is_none() {
                task.error = Some(interrupted_failed_error(locale));
            }
        }
        DownloadTaskStatus::Completed
        | DownloadTaskStatus::Failed
        | DownloadTaskStatus::Cancelled => {}
    }

    task
}

fn derive_job_status(
    tasks: &[DownloadTaskSnapshot],
    fallback: DownloadJobStatus,
) -> DownloadJobStatus {
    if tasks.is_empty() {
        return fallback;
    }

    let completed_count = tasks
        .iter()
        .filter(|task| task.status == DownloadTaskStatus::Completed)
        .count();
    let failed_count = tasks
        .iter()
        .filter(|task| task.status == DownloadTaskStatus::Failed)
        .count();
    let cancelled_count = tasks
        .iter()
        .filter(|task| task.status == DownloadTaskStatus::Cancelled)
        .count();

    if completed_count == tasks.len() {
        return DownloadJobStatus::Completed;
    }

    if completed_count > 0 && (failed_count > 0 || cancelled_count > 0) {
        return DownloadJobStatus::PartiallyFailed;
    }

    if completed_count == 0 && failed_count > 0 {
        return DownloadJobStatus::Failed;
    }

    if completed_count == 0 && cancelled_count > 0 {
        return DownloadJobStatus::Cancelled;
    }

    fallback
}

fn apply_retention(mut snapshot: DownloadManagerSnapshot) -> (DownloadManagerSnapshot, bool) {
    let original_jobs = snapshot.jobs.clone();
    let mut active_jobs = Vec::new();
    let mut terminal_jobs = Vec::new();

    for job in snapshot.jobs.drain(..) {
        if is_terminal_job_status(job.status) {
            terminal_jobs.push(job);
        } else {
            active_jobs.push(job);
        }
    }

    terminal_jobs.sort_by(|left, right| job_sort_key(right).cmp(&job_sort_key(left)));
    terminal_jobs.truncate(MAX_TERMINAL_JOB_HISTORY);

    active_jobs.extend(terminal_jobs);
    snapshot.jobs = active_jobs;
    snapshot.active_job_id = None;
    snapshot.queued_job_ids = snapshot
        .jobs
        .iter()
        .filter(|job| job.status == DownloadJobStatus::Queued)
        .map(|job| job.id.clone())
        .collect();
    let changed = snapshot.jobs != original_jobs;
    (snapshot, changed)
}

fn job_sort_key(job: &DownloadJobSnapshot) -> (&str, &str) {
    let finished_at = job.finished_at.as_deref().unwrap_or("");
    (finished_at, job.created_at.as_str())
}

fn is_terminal_job_status(status: DownloadJobStatus) -> bool {
    matches!(
        status,
        DownloadJobStatus::Completed
            | DownloadJobStatus::Failed
            | DownloadJobStatus::Cancelled
            | DownloadJobStatus::PartiallyFailed
    )
}

fn interrupted_cancelled_error(locale: Locale) -> DownloadErrorInfo {
    DownloadErrorInfo {
        code: DownloadErrorCode::Cancelled,
        message: crate::i18n::tr(locale, "download-session-interrupted-cancelled"),
        retryable: true,
        details: Some(INTERRUPTED_SENTINEL.to_string()),
    }
}

fn interrupted_failed_error(locale: Locale) -> DownloadErrorInfo {
    DownloadErrorInfo {
        code: DownloadErrorCode::Internal,
        message: crate::i18n::tr(locale, "download-session-interrupted-failed"),
        retryable: true,
        details: Some(INTERRUPTED_SENTINEL.to_string()),
    }
}

fn log_persistence_error(
    log_center: Option<&LogCenter>,
    code: &str,
    message: &str,
    user_message: &str,
    details: String,
) {
    if let Some(log_center) = log_center {
        log_center.record(
            LogPayload::new(LogLevel::Error, "download-session", code, message)
                .user_message(user_message)
                .details(details),
        );
    }
}

fn iso_timestamp_now() -> String {
    OffsetDateTime::now_utc()
        .format(&Iso8601::DEFAULT)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use super::{apply_retention, normalize_restored_snapshot, DownloadSessionStore};
    use crate::preferences::Locale;
    use harubble_core::audio::OutputFormat;
    use harubble_core::download::model::{
        DownloadJobKind, DownloadJobSnapshot, DownloadJobStatus, DownloadManagerSnapshot,
        DownloadOptions, DownloadTaskSnapshot, DownloadTaskStatus,
    };
    use std::sync::{Arc, Barrier};
    use std::thread;
    use tempfile::tempdir;

    fn make_task(id: &str, status: DownloadTaskStatus) -> DownloadTaskSnapshot {
        DownloadTaskSnapshot {
            id: id.to_string(),
            job_id: "job-1".to_string(),
            song_cid: "song-1".to_string(),
            song_name: "Song".to_string(),
            artists: vec!["Artist".to_string()],
            album_cid: "album-1".to_string(),
            album_name: "Album".to_string(),
            status,
            bytes_done: 128,
            bytes_total: Some(512),
            output_path: None,
            error: None,
            attempt: 0,
            song_index: 0,
            song_count: 1,
        }
    }

    fn make_job(
        id: &str,
        status: DownloadJobStatus,
        task_status: DownloadTaskStatus,
    ) -> DownloadJobSnapshot {
        let index = id.trim_start_matches("job-").parse::<u32>().unwrap_or(1);
        DownloadJobSnapshot {
            id: id.to_string(),
            kind: DownloadJobKind::Album,
            status,
            created_at: format!("2026-04-15T00:00:{index:04}Z"),
            started_at: Some("2026-04-15T00:00:0000Z".to_string()),
            finished_at: Some(format!("2026-04-15T00:01:{index:04}Z")),
            options: DownloadOptions {
                output_dir: "/tmp".to_string(),
                format: OutputFormat::Flac,
                download_lyrics: true,
            },
            title: id.to_string(),
            task_count: 1,
            completed_task_count: usize::from(task_status == DownloadTaskStatus::Completed),
            failed_task_count: usize::from(task_status == DownloadTaskStatus::Failed),
            cancelled_task_count: usize::from(task_status == DownloadTaskStatus::Cancelled),
            tasks: vec![make_task("task-1", task_status)],
            error: None,
        }
    }

    #[test]
    fn returns_empty_state_when_file_is_missing() {
        let dir = tempdir().expect("temp dir should exist");
        let store = DownloadSessionStore::new(dir.path().to_path_buf());

        let snapshot = store.load(None, Locale::default()).snapshot;

        assert!(snapshot.jobs.is_empty());
        assert!(snapshot.queued_job_ids.is_empty());
        assert!(snapshot.active_job_id.is_none());
    }

    #[test]
    fn returns_empty_state_when_json_is_corrupted() {
        let dir = tempdir().expect("temp dir should exist");
        let store = DownloadSessionStore::new(dir.path().to_path_buf());
        std::fs::write(dir.path().join("download_session.json"), b"not json")
            .expect("fixture should be written");

        let snapshot = store.load(None, Locale::default()).snapshot;

        assert!(snapshot.jobs.is_empty());
    }

    #[test]
    fn roundtrips_persisted_snapshot() {
        let dir = tempdir().expect("temp dir should exist");
        let store = DownloadSessionStore::new(dir.path().to_path_buf());
        let snapshot = DownloadManagerSnapshot {
            jobs: vec![make_job(
                "job-1",
                DownloadJobStatus::Completed,
                DownloadTaskStatus::Completed,
            )],
            active_job_id: None,
            queued_job_ids: Vec::new(),
        };

        store
            .save(&snapshot, Locale::default())
            .expect("snapshot should save");
        let loaded = store.load(None, Locale::default()).snapshot;

        assert_eq!(loaded.jobs.len(), 1);
        assert_eq!(loaded.jobs[0].id, "job-1");
        assert!(matches!(
            loaded.jobs[0].status,
            DownloadJobStatus::Completed
        ));
    }

    #[test]
    fn concurrent_saves_do_not_race_on_temp_file() {
        let dir = tempdir().expect("temp dir should exist");
        let store = Arc::new(DownloadSessionStore::new(dir.path().to_path_buf()));
        let barrier = Arc::new(Barrier::new(5));
        let handles: Vec<_> = (1..=4)
            .map(|index| {
                let store = store.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    let snapshot = DownloadManagerSnapshot {
                        jobs: vec![make_job(
                            &format!("job-{index}"),
                            DownloadJobStatus::Completed,
                            DownloadTaskStatus::Completed,
                        )],
                        active_job_id: None,
                        queued_job_ids: Vec::new(),
                    };
                    barrier.wait();
                    store.save(&snapshot, Locale::default())
                })
            })
            .collect();

        barrier.wait();
        for handle in handles {
            handle
                .join()
                .expect("save thread should complete")
                .expect("concurrent save should succeed");
        }

        let loaded = store.load(None, Locale::default()).snapshot;
        assert_eq!(loaded.jobs.len(), 1);
        assert!(matches!(
            loaded.jobs[0].status,
            DownloadJobStatus::Completed
        ));
        assert!(matches!(
            loaded.jobs[0].id.as_str(),
            "job-1" | "job-2" | "job-3" | "job-4"
        ));
    }

    #[test]
    fn normalizes_interrupted_tasks_on_restore() {
        let snapshot = DownloadManagerSnapshot {
            jobs: vec![DownloadJobSnapshot {
                id: "job-1".to_string(),
                kind: DownloadJobKind::Album,
                status: DownloadJobStatus::Running,
                created_at: "2026-04-15T00:00:00Z".to_string(),
                started_at: Some("2026-04-15T00:00:10Z".to_string()),
                finished_at: None,
                options: DownloadOptions {
                    output_dir: "/tmp".to_string(),
                    format: OutputFormat::Flac,
                    download_lyrics: true,
                },
                title: "job-1".to_string(),
                task_count: 2,
                completed_task_count: 0,
                failed_task_count: 0,
                cancelled_task_count: 0,
                tasks: vec![
                    make_task("task-1", DownloadTaskStatus::Queued),
                    make_task("task-2", DownloadTaskStatus::Downloading),
                ],
                error: None,
            }],
            active_job_id: Some("job-1".to_string()),
            queued_job_ids: vec!["job-1".to_string()],
        };

        let normalized = normalize_restored_snapshot(snapshot, Locale::default()).0;

        assert!(normalized.active_job_id.is_none());
        assert!(normalized.queued_job_ids.is_empty());
        assert!(matches!(
            normalized.jobs[0].tasks[0].status,
            DownloadTaskStatus::Cancelled
        ));
        assert!(matches!(
            normalized.jobs[0].tasks[1].status,
            DownloadTaskStatus::Failed
        ));
        assert!(matches!(
            normalized.jobs[0].status,
            DownloadJobStatus::Failed
        ));
        assert!(normalized.jobs[0].finished_at.is_some());
        assert!(normalized.jobs[0].tasks[0]
            .error
            .as_ref()
            .is_some_and(|error| error.retryable));
        assert!(normalized.jobs[0].tasks[1]
            .error
            .as_ref()
            .is_some_and(|error| error.retryable));
    }

    #[test]
    fn retention_keeps_most_recent_terminal_jobs() {
        let jobs = (1..=205)
            .map(|index| {
                make_job(
                    &format!("job-{index}"),
                    DownloadJobStatus::Completed,
                    DownloadTaskStatus::Completed,
                )
            })
            .collect();
        let snapshot = DownloadManagerSnapshot {
            jobs,
            active_job_id: None,
            queued_job_ids: Vec::new(),
        };

        let retained = apply_retention(snapshot).0;

        assert_eq!(retained.jobs.len(), 200);
        assert_eq!(
            retained.jobs.first().map(|job| job.id.as_str()),
            Some("job-205")
        );
        assert_eq!(
            retained.jobs.last().map(|job| job.id.as_str()),
            Some("job-6")
        );
    }
}
