use crate::logging::{LogCenter, LogLevel};
use crate::startup_recovery::{
    backup_path_if_exists, record_recovery_log, recovered_backup_suffix,
};
use anyhow::{Context, Result};
use harubble_core::download::model::InternalDownloadTask;
use harubble_core::download::worker::CompletedTaskArtifacts;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalInventoryProvenanceRecord {
    pub song_cid: String,
    pub album_cid: String,
    pub relative_path: String,
    pub source_url: String,
    pub source_audio_checksum: String,
    pub processing_fingerprint: String,
    pub final_artifact_checksum: String,
    pub final_artifact_size: u64,
    pub recorded_at: String,
}

#[derive(Clone)]
pub(crate) struct LocalInventoryProvenanceStore {
    path: PathBuf,
    records: Arc<Mutex<Vec<LocalInventoryProvenanceRecord>>>,
}

impl LocalInventoryProvenanceStore {
    #[cfg(test)]
    pub(crate) fn new(app_data_dir: PathBuf) -> Result<Self> {
        Self::new_with_logger(app_data_dir, None)
    }

    pub(crate) fn new_with_logger(
        app_data_dir: PathBuf,
        log_center: Option<&LogCenter>,
    ) -> Result<Self> {
        std::fs::create_dir_all(&app_data_dir)
            .with_context(|| format!("failed to create {}", app_data_dir.display()))?;
        let path = app_data_dir.join("local_inventory_provenance.json");
        let records = load_records(&path, log_center)?;
        Ok(Self {
            path,
            records: Arc::new(Mutex::new(records)),
        })
    }

    pub(crate) async fn record_completed_download(
        &self,
        root_output_dir: &Path,
        task: &InternalDownloadTask,
        completed: &CompletedTaskArtifacts,
    ) -> Result<()> {
        let output_path = Path::new(&completed.output_path);
        let relative_path = normalize_relative_path(root_output_dir, output_path)?;
        let metadata = std::fs::metadata(output_path)
            .with_context(|| format!("failed to stat {}", output_path.display()))?;
        let final_artifact_checksum = checksum_path(output_path)?;
        let record = LocalInventoryProvenanceRecord {
            song_cid: task.song_cid.clone(),
            album_cid: task.album_cid.clone(),
            relative_path: relative_path.clone(),
            source_url: completed.provenance_seed.source_url.clone(),
            source_audio_checksum: completed.provenance_seed.source_audio_checksum.clone(),
            processing_fingerprint: completed.provenance_seed.processing_fingerprint.clone(),
            final_artifact_checksum,
            final_artifact_size: metadata.len(),
            recorded_at: OffsetDateTime::now_utc()
                .format(&Iso8601::DEFAULT)
                .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string()),
        };

        let mut records = self.records.lock().await;
        records.retain(|existing| {
            existing.relative_path != relative_path
                && !(existing.song_cid == record.song_cid
                    && existing.album_cid == record.album_cid
                    && existing.processing_fingerprint == record.processing_fingerprint)
        });
        records.push(record);
        save_records(&self.path, &records)
    }

    pub(crate) async fn snapshot_records(&self) -> Vec<LocalInventoryProvenanceRecord> {
        self.records.lock().await.clone()
    }
}

fn load_records(
    path: &Path,
    log_center: Option<&LogCenter>,
) -> Result<Vec<LocalInventoryProvenanceRecord>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    match serde_json::from_str(&content) {
        Ok(records) => Ok(records),
        Err(error) => {
            let suffix = recovered_backup_suffix();
            let backup = backup_path_if_exists(path, &suffix, log_center);
            let backup_details = backup
                .as_ref()
                .map(|backup| format!(" backupPath={}", backup.display()))
                .unwrap_or_default();
            record_recovery_log(
                log_center,
                LogLevel::Warn,
                "startup-recovery",
                "startup-recovery.provenance_recreated_from_backup",
                "Recreated provenance cache from backup",
                format!("path={}{} error={error}", path.display(), backup_details),
            );
            Ok(Vec::new())
        }
    }
}

fn save_records(path: &Path, records: &[LocalInventoryProvenanceRecord]) -> Result<()> {
    let content = serde_json::to_string_pretty(records)?;
    std::fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
}

fn normalize_relative_path(root_output_dir: &Path, output_path: &Path) -> Result<String> {
    let relative = output_path.strip_prefix(root_output_dir).with_context(|| {
        format!(
            "{} is not under {}",
            output_path.display(),
            root_output_dir.display()
        )
    })?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn checksum_path(path: &Path) -> Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(format!("{:x}", md5::compute(bytes)))
}

#[cfg(test)]
mod tests {
    use super::LocalInventoryProvenanceStore;
    use tempfile::tempdir;

    #[test]
    fn backs_up_invalid_json_and_starts_with_empty_records() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("local_inventory_provenance.json");
        std::fs::write(&path, b"not json").expect("write invalid json");

        let store = LocalInventoryProvenanceStore::new(dir.path().to_path_buf())
            .expect("store should load invalid json with empty records");

        let backup_exists = std::fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("local_inventory_provenance.json.recovered-backup-")
            });
        assert!(backup_exists);

        let records = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(store.snapshot_records());
        assert!(records.is_empty());
    }

    #[test]
    fn missing_file_starts_empty_without_backup() {
        let dir = tempdir().expect("temp dir");

        let store = LocalInventoryProvenanceStore::new(dir.path().to_path_buf())
            .expect("store should load empty records");

        let backup_exists = std::fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("local_inventory_provenance.json.recovered-backup-")
            });
        assert!(!backup_exists);

        let records = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(store.snapshot_records());
        assert!(records.is_empty());
    }
}
