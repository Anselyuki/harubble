//! 本地库存文件系统扫描逻辑。
//!
//! 负责递归遍历输出目录、识别音频文件、计算校验和并构建证据记录。
//! 由 [`super::local_inventory`] 模块在扫描任务中调用。

use crate::local_inventory_provenance::LocalInventoryProvenanceRecord;
use crate::preferences::Locale;
use harubble_core::{
    LocalAudioFileEvidence, LocalAudioFileVerificationState, LocalInventoryScanProgressEvent,
};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::UNIX_EPOCH;

pub(crate) struct ScanCollectionResult {
    pub(crate) audio_files: Vec<LocalAudioFileEvidence>,
    pub(crate) files_scanned: usize,
    pub(crate) matched_track_count: usize,
    pub(crate) verified_track_count: usize,
}

pub(crate) enum ScanCollectionOutcome {
    Completed(ScanCollectionResult),
    Cancelled,
}

pub(crate) fn collect_local_audio_evidence(
    root_output_dir: &Path,
    root_output_dir_text: &str,
    inventory_version: &str,
    provenance_records: &[LocalInventoryProvenanceRecord],
    cancel_flag: &AtomicBool,
    locale: Locale,
    mut on_progress: impl FnMut(LocalInventoryScanProgressEvent),
) -> Result<ScanCollectionOutcome, String> {
    if root_output_dir.as_os_str().is_empty() || !root_output_dir.exists() {
        return Ok(ScanCollectionOutcome::Completed(ScanCollectionResult {
            audio_files: Vec::new(),
            files_scanned: 0,
            matched_track_count: 0,
            verified_track_count: 0,
        }));
    }

    if !root_output_dir.is_dir() {
        return Err(crate::i18n::tr(
            locale,
            "inventory-output-dir-not-directory",
        ));
    }

    let mut audio_files = Vec::new();
    let mut files_scanned = 0_usize;
    let mut verified_track_count = 0_usize;
    let visit_result = visit_directory(
        root_output_dir,
        root_output_dir,
        cancel_flag,
        locale,
        &mut |path| {
            files_scanned += 1;
            let relative_path = path
                .strip_prefix(root_output_dir)
                .ok()
                .map(to_normalized_relative_path);

            if is_audio_file(path) {
                if let Some(relative_path) = relative_path.clone() {
                    let evidence = build_audio_file_evidence(
                        root_output_dir,
                        path,
                        relative_path,
                        provenance_records,
                        locale,
                    )?;
                    if evidence.verification_state == LocalAudioFileVerificationState::Verified {
                        verified_track_count += 1;
                    }
                    audio_files.push(evidence);
                }
            }

            on_progress(LocalInventoryScanProgressEvent {
                root_output_dir: root_output_dir_text.to_string(),
                inventory_version: inventory_version.to_string(),
                files_scanned,
                matched_track_count: audio_files.len(),
                verified_track_count,
                current_path: relative_path,
            });

            Ok(())
        },
    )?;

    if visit_result {
        Ok(ScanCollectionOutcome::Completed(ScanCollectionResult {
            matched_track_count: audio_files.len(),
            verified_track_count,
            audio_files,
            files_scanned,
        }))
    } else {
        Ok(ScanCollectionOutcome::Cancelled)
    }
}

fn visit_directory(
    root_output_dir: &Path,
    current_path: &Path,
    cancel_flag: &AtomicBool,
    locale: Locale,
    on_file: &mut impl FnMut(&Path) -> Result<(), String>,
) -> Result<bool, String> {
    let mut entries = std::fs::read_dir(current_path)
        .map_err(|_| crate::i18n::tr(locale, "inventory-read-dir-failed"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| crate::i18n::tr(locale, "inventory-enumerate-dir-failed"))?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        if cancel_flag.load(Ordering::SeqCst) {
            return Ok(false);
        }

        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|_| crate::i18n::tr(locale, "inventory-read-metadata-failed"))?;

        if metadata.file_type().is_symlink() {
            continue;
        }

        if metadata.is_dir() {
            if !visit_directory(root_output_dir, &path, cancel_flag, locale, on_file)? {
                return Ok(false);
            }
        } else if metadata.is_file() {
            let _ = root_output_dir;
            on_file(&path)?;
        }
    }

    Ok(true)
}

fn build_audio_file_evidence(
    root_output_dir: &Path,
    path: &Path,
    relative_path: String,
    provenance_records: &[LocalInventoryProvenanceRecord],
    locale: Locale,
) -> Result<LocalAudioFileEvidence, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|_| crate::i18n::tr(locale, "inventory-read-metadata-failed"))?;
    let parent = path
        .parent()
        .and_then(|dir| dir.strip_prefix(root_output_dir).ok());
    let is_in_album_directory = parent
        .map(|dir| !dir.as_os_str().is_empty())
        .unwrap_or(false);
    let modified_at_ms = metadata
        .modified()
        .ok()
        .and_then(|ts| ts.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64);
    let candidate_checksum = checksum_path(path, locale)?;
    let verification_state =
        resolve_verification_state(&relative_path, &candidate_checksum, provenance_records);

    Ok(LocalAudioFileEvidence {
        relative_path,
        file_size: metadata.len(),
        modified_at_ms,
        candidate_checksum: Some(candidate_checksum),
        is_in_album_directory,
        verification_state,
    })
}

fn resolve_verification_state(
    relative_path: &str,
    final_artifact_checksum: &str,
    provenance_records: &[LocalInventoryProvenanceRecord],
) -> LocalAudioFileVerificationState {
    if let Some(record) = provenance_records
        .iter()
        .find(|record| record.relative_path == relative_path)
    {
        return if record.final_artifact_checksum == final_artifact_checksum {
            LocalAudioFileVerificationState::Verified
        } else {
            LocalAudioFileVerificationState::Mismatch
        };
    }

    if provenance_records
        .iter()
        .any(|record| record.final_artifact_checksum == final_artifact_checksum)
    {
        return LocalAudioFileVerificationState::Verified;
    }

    LocalAudioFileVerificationState::Unchecked
}

fn checksum_path(path: &Path, locale: Locale) -> Result<String, String> {
    use std::io::Read;
    let file = std::fs::File::open(path)
        .map_err(|_| crate::i18n::tr(locale, "inventory-read-audio-failed"))?;
    let mut reader = std::io::BufReader::with_capacity(8192, file);
    let mut context = md5::Context::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|_| crate::i18n::tr(locale, "inventory-read-audio-failed"))?;
        if n == 0 {
            break;
        }
        context.consume(&buf[..n]);
    }
    Ok(format!("{:x}", context.compute()))
}

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "flac" | "wav" | "mp3"
            )
        })
        .unwrap_or(false)
}

fn to_normalized_relative_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
