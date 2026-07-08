use crate::app_state::AppState;
use crate::local_inventory_provenance::{
    LocalInventoryProvenanceRecord, LocalInventoryProvenanceStore,
};
use crate::local_inventory_scan::{
    collect_local_audio_evidence, ScanCollectionOutcome, ScanCollectionResult,
};
use harubble_core::{
    aggregate_album_download_badge, album_badge_from_evidence, matched_track_evidence,
    track_badge_from_matches, Album, AlbumDetail, LocalAudioFileEvidence,
    LocalInventoryScanProgressEvent, LocalInventorySnapshot, LocalInventoryStatus, SongDetail,
    TrackDownloadBadge, VerificationMode,
};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;
use tokio::sync::Mutex;

pub(crate) const LOCAL_INVENTORY_STATE_CHANGED: &str = "local-inventory-state-changed";
pub(crate) const LOCAL_INVENTORY_SCAN_PROGRESS: &str = "local-inventory-scan-progress";

#[derive(Debug, Clone)]
pub(crate) struct LocalInventoryState {
    pub(crate) snapshot: LocalInventorySnapshot,
    pub(crate) verification_mode: VerificationMode,
    pub(crate) audio_files: Vec<LocalAudioFileEvidence>,
}

impl Default for LocalInventoryState {
    fn default() -> Self {
        Self {
            snapshot: LocalInventorySnapshot::default(),
            verification_mode: VerificationMode::WhenAvailable,
            audio_files: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct LocalInventoryService {
    state: Arc<Mutex<LocalInventoryState>>,
    provenance_store: Arc<LocalInventoryProvenanceStore>,
    pub(crate) cancel_flag: Arc<AtomicBool>,
}

impl LocalInventoryService {
    pub(crate) fn new(provenance_store: Arc<LocalInventoryProvenanceStore>) -> Self {
        Self {
            state: Arc::new(Mutex::new(LocalInventoryState::default())),
            provenance_store,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) async fn snapshot(&self) -> LocalInventorySnapshot {
        self.state.lock().await.snapshot.clone()
    }

    pub(crate) async fn verification_mode(&self) -> VerificationMode {
        self.state.lock().await.verification_mode
    }

    pub(crate) async fn provenance_records(&self) -> Vec<LocalInventoryProvenanceRecord> {
        self.provenance_store.snapshot_records().await
    }

    pub(crate) async fn begin_scan(
        &self,
        root_output_dir: String,
        verification_mode: VerificationMode,
    ) -> LocalInventorySnapshot {
        self.cancel_flag.store(false, Ordering::SeqCst);
        let inventory_version = next_inventory_version();
        let mut state = self.state.lock().await;
        state.verification_mode = verification_mode;
        state.audio_files.clear();
        state.snapshot = LocalInventorySnapshot {
            root_output_dir,
            status: LocalInventoryStatus::Scanning,
            inventory_version,
            started_at: Some(iso_timestamp_now()),
            finished_at: None,
            scanned_file_count: 0,
            matched_track_count: 0,
            verified_track_count: 0,
            last_error: None,
        };
        state.snapshot.clone()
    }

    async fn complete_scan(
        &self,
        inventory_version: &str,
        result: ScanCollectionResult,
    ) -> LocalInventorySnapshot {
        let mut state = self.state.lock().await;
        if state.snapshot.inventory_version != inventory_version
            || state.snapshot.status != LocalInventoryStatus::Scanning
        {
            return state.snapshot.clone();
        }
        state.audio_files = result.audio_files;
        state.snapshot.status = LocalInventoryStatus::Completed;
        state.snapshot.scanned_file_count = result.files_scanned;
        state.snapshot.matched_track_count = result.matched_track_count;
        state.snapshot.verified_track_count = result.verified_track_count;
        state.snapshot.finished_at = Some(iso_timestamp_now());
        state.snapshot.last_error = None;
        state.snapshot.clone()
    }

    pub(crate) async fn fail_scan(
        &self,
        inventory_version: &str,
        error: String,
    ) -> LocalInventorySnapshot {
        let mut state = self.state.lock().await;
        if state.snapshot.inventory_version != inventory_version
            || state.snapshot.status != LocalInventoryStatus::Scanning
        {
            return state.snapshot.clone();
        }
        state.audio_files.clear();
        state.snapshot.status = LocalInventoryStatus::Failed;
        state.snapshot.scanned_file_count = 0;
        state.snapshot.matched_track_count = 0;
        state.snapshot.verified_track_count = 0;
        state.snapshot.finished_at = Some(iso_timestamp_now());
        state.snapshot.last_error = Some(error);
        state.snapshot.clone()
    }

    pub(crate) async fn cancel_scan(&self) -> LocalInventorySnapshot {
        self.cancel_flag.store(true, Ordering::SeqCst);
        let mut state = self.state.lock().await;
        state.audio_files.clear();
        state.snapshot.status = LocalInventoryStatus::Idle;
        state.snapshot.finished_at = Some(iso_timestamp_now());
        state.snapshot.clone()
    }

    pub(crate) async fn enrich_albums(&self, mut albums: Vec<Album>) -> Vec<Album> {
        let state = self.state.lock().await;
        let inventory_version = state.snapshot.inventory_version.clone();
        for album in &mut albums {
            album.download = album_badge_from_evidence(
                &state.audio_files,
                &album.name,
                inventory_version.clone(),
            );
        }
        albums
    }

    pub(crate) async fn enrich_album_detail(&self, mut album: AlbumDetail) -> AlbumDetail {
        let state = self.state.lock().await;
        let inventory_version = state.snapshot.inventory_version.clone();
        let track_badges = album
            .songs
            .iter_mut()
            .map(|song| {
                let badge = track_badge_for_song(
                    &state.audio_files,
                    &album.name,
                    &song.name,
                    state.verification_mode,
                    &inventory_version,
                );
                song.download = badge.clone();
                badge
            })
            .collect::<Vec<_>>();
        album.download = aggregate_album_download_badge(&track_badges, inventory_version);
        album
    }

    pub(crate) async fn enrich_song_detail(
        &self,
        mut song: SongDetail,
        album_name: &str,
    ) -> SongDetail {
        let state = self.state.lock().await;
        song.download = track_badge_for_song(
            &state.audio_files,
            album_name,
            &song.name,
            state.verification_mode,
            &state.snapshot.inventory_version,
        );
        song
    }
}

pub(crate) fn emit_local_inventory_state_changed(
    app: &AppHandle,
    snapshot: &LocalInventorySnapshot,
) {
    let _ = app.emit(LOCAL_INVENTORY_STATE_CHANGED, snapshot);
}

pub(crate) fn emit_local_inventory_scan_progress(
    app: &AppHandle,
    event: &LocalInventoryScanProgressEvent,
) {
    let _ = app.emit(LOCAL_INVENTORY_SCAN_PROGRESS, event);
}

/// 启动一次本地库存扫描任务，并通过事件持续回传进度与最终状态。
pub fn spawn_inventory_scan(
    app: AppHandle,
    state: AppState,
    root_output_dir: String,
    verification_mode: Option<VerificationMode>,
) {
    tauri::async_runtime::spawn(async move {
        state
            .wait_for_background_io_gate("local_inventory_scan", Duration::from_millis(250))
            .await;

        let mode = match verification_mode {
            Some(mode) => mode,
            None => state.local_inventory_service.verification_mode().await,
        };
        let provenance_records = state.local_inventory_service.provenance_records().await;
        let started = state
            .local_inventory_service
            .begin_scan(root_output_dir.clone(), mode)
            .await;
        state
            .library_search_service
            .prepare_for_inventory_scan(root_output_dir.clone())
            .await;
        emit_local_inventory_state_changed(&app, &started);

        let inventory_version = started.inventory_version.clone();
        let locale = state.preferences().locale;
        let app_for_scan = app.clone();
        let cancel_flag = state.local_inventory_service.cancel_flag.clone();
        let root_output_dir_for_scan = root_output_dir.clone();
        let inventory_version_for_scan = inventory_version.clone();
        let scan_result = tokio::task::spawn_blocking(move || {
            collect_local_audio_evidence(
                Path::new(&root_output_dir_for_scan),
                &root_output_dir_for_scan,
                &inventory_version_for_scan,
                &provenance_records,
                &cancel_flag,
                locale,
                |event| emit_local_inventory_scan_progress(&app_for_scan, &event),
            )
        })
        .await
        .map_err(|error| format!("inventory scan worker failed: {error}"))
        .and_then(|result| result);

        let finished = match scan_result {
            Ok(ScanCollectionOutcome::Completed(result)) => {
                state
                    .local_inventory_service
                    .complete_scan(&inventory_version, result)
                    .await
            }
            Ok(ScanCollectionOutcome::Cancelled) => state.local_inventory_service.snapshot().await,
            Err(error) => {
                state
                    .local_inventory_service
                    .fail_scan(&inventory_version, error)
                    .await
            }
        };
        if finished.status == LocalInventoryStatus::Completed {
            state
                .library_search_service
                .schedule_rebuild(state.clone(), finished.clone());
        }
        emit_local_inventory_state_changed(&app, &finished);
    });
}

fn track_badge_for_song(
    audio_files: &[LocalAudioFileEvidence],
    album_name: &str,
    song_name: &str,
    verification_mode: VerificationMode,
    inventory_version: &str,
) -> TrackDownloadBadge {
    let matches = matched_track_evidence(audio_files, album_name, song_name);
    track_badge_from_matches(&matches, verification_mode, inventory_version.to_string())
}

fn iso_timestamp_now() -> String {
    OffsetDateTime::now_utc()
        .format(&Iso8601::DEFAULT)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn next_inventory_version() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("inventory-{millis}")
}

#[cfg(test)]
mod tests {
    use super::{
        collect_local_audio_evidence, track_badge_for_song, LocalInventoryService,
        ScanCollectionOutcome,
    };
    use crate::local_inventory_provenance::{
        LocalInventoryProvenanceRecord, LocalInventoryProvenanceStore,
    };
    use crate::preferences::Locale;
    use harubble_core::{
        Album, AlbumDetail, LocalAudioFileEvidence, LocalAudioFileVerificationState,
        LocalInventoryStatus, LocalTrackDownloadStatus, SongEntry, VerificationMode,
    };
    use std::path::Path;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn make_service(temp_root: &Path) -> LocalInventoryService {
        let store = Arc::new(
            LocalInventoryProvenanceStore::new(temp_root.to_path_buf()).expect("provenance store"),
        );
        LocalInventoryService::new(store)
    }

    mod enrich {
        use super::*;

        #[tokio::test]
        async fn enriches_album_detail_song_downloads_from_output_dir() {
            let temp_dir = tempdir().expect("temp dir");
            std::fs::create_dir_all(temp_dir.path().join("Album")).expect("album dir");
            std::fs::write(temp_dir.path().join("Album/Track.flac"), b"audio").expect("audio file");

            let service = make_service(temp_dir.path());
            let started = service
                .begin_scan(
                    temp_dir.path().to_string_lossy().to_string(),
                    VerificationMode::WhenAvailable,
                )
                .await;
            let files = collect_local_audio_evidence(
                temp_dir.path(),
                &started.root_output_dir,
                &started.inventory_version,
                &[],
                &AtomicBool::new(false),
                Locale::default(),
                |_| {},
            )
            .expect("scan files");
            let ScanCollectionOutcome::Completed(files) = files else {
                panic!("scan should complete");
            };
            let snapshot = service
                .complete_scan(&started.inventory_version, files)
                .await;

            assert_eq!(snapshot.status, LocalInventoryStatus::Completed);
            assert_eq!(snapshot.scanned_file_count, 1);
            assert_eq!(snapshot.matched_track_count, 1);

            let album_detail = AlbumDetail {
                cid: "album-1".to_string(),
                name: "Album".to_string(),
                intro: None,
                belong: "test".to_string(),
                cover_url: "cover".to_string(),
                cover_de_url: None,
                artists: Some(vec!["Artist".to_string()]),
                download: Default::default(),
                tags: Vec::new(),
                songs: vec![SongEntry {
                    cid: "song-1".to_string(),
                    name: "Track".to_string(),
                    artists: vec!["Artist".to_string()],
                    download: Default::default(),
                    tags: Vec::new(),
                }],
            };

            let enriched_detail = service.enrich_album_detail(album_detail).await;

            assert!(enriched_detail.songs[0].download.is_downloaded);
            assert_eq!(
                enriched_detail.songs[0].download.download_status,
                LocalTrackDownloadStatus::Detected
            );
            assert_eq!(
                enriched_detail.download.download_status,
                LocalTrackDownloadStatus::Detected
            );
            assert!(enriched_detail.download.is_downloaded);
        }

        #[tokio::test]
        async fn album_detail_aggregate_is_partial_when_only_some_songs_are_downloaded() {
            let temp_dir = tempdir().expect("temp dir");
            std::fs::create_dir_all(temp_dir.path().join("Album")).expect("album dir");
            std::fs::write(temp_dir.path().join("Album/Track A.flac"), b"audio")
                .expect("audio file");

            let service = make_service(temp_dir.path());
            let started = service
                .begin_scan(
                    temp_dir.path().to_string_lossy().to_string(),
                    VerificationMode::WhenAvailable,
                )
                .await;
            let files = collect_local_audio_evidence(
                temp_dir.path(),
                &started.root_output_dir,
                &started.inventory_version,
                &[],
                &AtomicBool::new(false),
                Locale::default(),
                |_| {},
            )
            .expect("scan files");
            let ScanCollectionOutcome::Completed(files) = files else {
                panic!("scan should complete");
            };
            let _snapshot = service
                .complete_scan(&started.inventory_version, files)
                .await;

            let album_detail = AlbumDetail {
                cid: "album-1".to_string(),
                name: "Album".to_string(),
                intro: None,
                belong: "test".to_string(),
                cover_url: "cover".to_string(),
                cover_de_url: None,
                artists: Some(vec!["Artist".to_string()]),
                download: Default::default(),
                tags: Vec::new(),
                songs: vec![
                    SongEntry {
                        cid: "song-1".to_string(),
                        name: "Track A".to_string(),
                        artists: vec!["Artist".to_string()],
                        download: Default::default(),
                        tags: Vec::new(),
                    },
                    SongEntry {
                        cid: "song-2".to_string(),
                        name: "Track B".to_string(),
                        artists: vec!["Artist".to_string()],
                        download: Default::default(),
                        tags: Vec::new(),
                    },
                ],
            };

            let enriched_detail = service.enrich_album_detail(album_detail).await;

            assert_eq!(
                enriched_detail.download.download_status,
                LocalTrackDownloadStatus::Partial
            );
            assert!(enriched_detail.download.is_downloaded);
            assert_eq!(
                enriched_detail.songs[0].download.download_status,
                LocalTrackDownloadStatus::Detected
            );
            assert_eq!(
                enriched_detail.songs[1].download.download_status,
                LocalTrackDownloadStatus::Missing
            );
        }

        #[tokio::test]
        async fn enriches_album_list_with_partial_badge_when_album_directory_has_audio() {
            let temp_dir = tempdir().expect("temp dir");
            std::fs::create_dir_all(temp_dir.path().join("Album")).expect("album dir");
            std::fs::write(temp_dir.path().join("Album/Track.flac"), b"audio").expect("audio file");

            let service = make_service(temp_dir.path());
            let started = service
                .begin_scan(
                    temp_dir.path().to_string_lossy().to_string(),
                    VerificationMode::WhenAvailable,
                )
                .await;
            let files = collect_local_audio_evidence(
                temp_dir.path(),
                &started.root_output_dir,
                &started.inventory_version,
                &[],
                &AtomicBool::new(false),
                Locale::default(),
                |_| {},
            )
            .expect("scan files");
            let ScanCollectionOutcome::Completed(files) = files else {
                panic!("scan should complete");
            };
            let snapshot = service
                .complete_scan(&started.inventory_version, files)
                .await;

            let albums = service
                .enrich_albums(vec![Album {
                    cid: "album-1".to_string(),
                    name: "Album".to_string(),
                    cover_url: "cover".to_string(),
                    artists: vec!["Artist".to_string()],
                    download: Default::default(),
                    tags: Vec::new(),
                }])
                .await;

            assert_eq!(
                albums[0].download.download_status,
                LocalTrackDownloadStatus::Partial
            );
            assert!(albums[0].download.is_downloaded);
            assert_eq!(
                albums[0].download.inventory_version,
                snapshot.inventory_version
            );
        }
    }

    mod track_badge {
        use super::*;

        #[test]
        fn root_level_single_track_layout_marks_song_as_downloaded() {
            let badge = track_badge_for_song(
                &[LocalAudioFileEvidence {
                    relative_path: "Track.flac".to_string(),
                    file_size: 42,
                    modified_at_ms: Some(1),
                    candidate_checksum: Some("abc".to_string()),
                    is_in_album_directory: false,
                    verification_state: LocalAudioFileVerificationState::Unchecked,
                }],
                "Album",
                "Track",
                VerificationMode::WhenAvailable,
                "v1",
            );
            assert!(badge.is_downloaded);
            assert_eq!(badge.download_status, LocalTrackDownloadStatus::Detected);
        }

        #[test]
        fn strict_mode_marks_detected_track_as_unverifiable() {
            let badge = track_badge_for_song(
                &[LocalAudioFileEvidence {
                    relative_path: "Album/Track.flac".to_string(),
                    file_size: 42,
                    modified_at_ms: Some(1),
                    candidate_checksum: Some("abc".to_string()),
                    is_in_album_directory: true,
                    verification_state: LocalAudioFileVerificationState::Unchecked,
                }],
                "Album",
                "Track",
                VerificationMode::Strict,
                "v1",
            );
            assert_eq!(
                badge.download_status,
                LocalTrackDownloadStatus::Unverifiable
            );
            assert!(badge.is_downloaded);
        }

        #[test]
        fn multiple_matching_candidates_mark_track_as_partial() {
            let badge = track_badge_for_song(
                &[
                    LocalAudioFileEvidence {
                        relative_path: "Track.flac".to_string(),
                        file_size: 42,
                        modified_at_ms: Some(1),
                        candidate_checksum: Some("abc".to_string()),
                        is_in_album_directory: false,
                        verification_state: LocalAudioFileVerificationState::Unchecked,
                    },
                    LocalAudioFileEvidence {
                        relative_path: "Album/Track.wav".to_string(),
                        file_size: 43,
                        modified_at_ms: Some(2),
                        candidate_checksum: Some("def".to_string()),
                        is_in_album_directory: true,
                        verification_state: LocalAudioFileVerificationState::Unchecked,
                    },
                ],
                "Album",
                "Track",
                VerificationMode::WhenAvailable,
                "v1",
            );
            assert_eq!(badge.download_status, LocalTrackDownloadStatus::Partial);
            assert!(badge.is_downloaded);
        }

        #[test]
        fn verified_match_promotes_track_to_verified() {
            let badge = track_badge_for_song(
                &[LocalAudioFileEvidence {
                    relative_path: "Album/Track.flac".to_string(),
                    file_size: 42,
                    modified_at_ms: Some(1),
                    candidate_checksum: Some("abc".to_string()),
                    is_in_album_directory: true,
                    verification_state: LocalAudioFileVerificationState::Verified,
                }],
                "Album",
                "Track",
                VerificationMode::WhenAvailable,
                "v1",
            );
            assert_eq!(badge.download_status, LocalTrackDownloadStatus::Verified);
            assert!(badge.is_downloaded);
        }

        #[test]
        fn mismatch_match_promotes_track_to_mismatch() {
            let badge = track_badge_for_song(
                &[LocalAudioFileEvidence {
                    relative_path: "Album/Track.flac".to_string(),
                    file_size: 42,
                    modified_at_ms: Some(1),
                    candidate_checksum: Some("abc".to_string()),
                    is_in_album_directory: true,
                    verification_state: LocalAudioFileVerificationState::Mismatch,
                }],
                "Album",
                "Track",
                VerificationMode::WhenAvailable,
                "v1",
            );
            assert_eq!(badge.download_status, LocalTrackDownloadStatus::Mismatch);
            assert!(!badge.is_downloaded);
        }
    }

    mod scan {
        use super::*;

        #[tokio::test]
        async fn missing_output_dir_yields_empty_snapshot() {
            let temp_dir = tempdir().expect("temp dir");
            let service = make_service(temp_dir.path());
            let started = service
                .begin_scan(
                    "/path/that/does/not/exist".to_string(),
                    VerificationMode::WhenAvailable,
                )
                .await;
            let files = collect_local_audio_evidence(
                Path::new("/path/that/does/not/exist"),
                &started.root_output_dir,
                &started.inventory_version,
                &[],
                &AtomicBool::new(false),
                Locale::default(),
                |_| {},
            )
            .expect("empty scan");
            let ScanCollectionOutcome::Completed(files) = files else {
                panic!("missing path scan should complete");
            };
            let snapshot = service
                .complete_scan(&started.inventory_version, files)
                .await;

            assert_eq!(snapshot.status, LocalInventoryStatus::Completed);
            assert_eq!(snapshot.scanned_file_count, 0);
            assert_eq!(snapshot.matched_track_count, 0);
        }

        #[tokio::test]
        async fn non_directory_output_dir_uses_requested_locale() {
            let temp_dir = tempdir().expect("temp dir");
            let file_path = temp_dir.path().join("not-a-directory");
            std::fs::write(&file_path, b"not a directory").expect("fixture file");

            let error = match collect_local_audio_evidence(
                &file_path,
                &file_path.to_string_lossy(),
                "v1",
                &[],
                &AtomicBool::new(false),
                Locale::EnUS,
                |_| {},
            ) {
                Ok(_) => panic!("file path should fail as output directory"),
                Err(error) => error,
            };

            assert_eq!(error, "outputDir is not a directory");
        }
    }

    mod provenance {
        use super::*;

        #[tokio::test]
        async fn scan_marks_file_as_verified_when_provenance_checksum_matches() {
            let temp_dir = tempdir().expect("temp dir");
            std::fs::create_dir_all(temp_dir.path().join("Album")).expect("album dir");
            let file_path = temp_dir.path().join("Album/Track.flac");
            std::fs::write(&file_path, b"audio").expect("audio file");
            let checksum = format!("{:x}", md5::compute(b"audio"));
            let records = vec![LocalInventoryProvenanceRecord {
                song_cid: "song-1".to_string(),
                album_cid: "album-1".to_string(),
                relative_path: "Album/Track.flac".to_string(),
                source_url: "https://example.test/source".to_string(),
                source_audio_checksum: "source-md5".to_string(),
                processing_fingerprint: "fp".to_string(),
                final_artifact_checksum: checksum.clone(),
                final_artifact_size: 5,
                recorded_at: "2026-04-21T00:00:00Z".to_string(),
            }];

            let result = collect_local_audio_evidence(
                temp_dir.path(),
                &temp_dir.path().to_string_lossy(),
                "v1",
                &records,
                &AtomicBool::new(false),
                Locale::default(),
                |_| {},
            )
            .expect("scan files");
            let ScanCollectionOutcome::Completed(result) = result else {
                panic!("scan should complete");
            };

            assert_eq!(result.verified_track_count, 1);
            assert_eq!(
                result.audio_files[0].verification_state,
                LocalAudioFileVerificationState::Verified
            );
            assert_eq!(
                result.audio_files[0].candidate_checksum.as_deref(),
                Some(checksum.as_str())
            );
        }

        #[tokio::test]
        async fn scan_marks_file_as_mismatch_when_provenance_checksum_drifts() {
            let temp_dir = tempdir().expect("temp dir");
            std::fs::create_dir_all(temp_dir.path().join("Album")).expect("album dir");
            let file_path = temp_dir.path().join("Album/Track.flac");
            std::fs::write(&file_path, b"tampered").expect("audio file");
            let records = vec![LocalInventoryProvenanceRecord {
                song_cid: "song-1".to_string(),
                album_cid: "album-1".to_string(),
                relative_path: "Album/Track.flac".to_string(),
                source_url: "https://example.test/source".to_string(),
                source_audio_checksum: "source-md5".to_string(),
                processing_fingerprint: "fp".to_string(),
                final_artifact_checksum: format!("{:x}", md5::compute(b"audio")),
                final_artifact_size: 5,
                recorded_at: "2026-04-21T00:00:00Z".to_string(),
            }];

            let result = collect_local_audio_evidence(
                temp_dir.path(),
                &temp_dir.path().to_string_lossy(),
                "v1",
                &records,
                &AtomicBool::new(false),
                Locale::default(),
                |_| {},
            )
            .expect("scan files");
            let ScanCollectionOutcome::Completed(result) = result else {
                panic!("scan should complete");
            };

            assert_eq!(result.verified_track_count, 0);
            assert_eq!(
                result.audio_files[0].verification_state,
                LocalAudioFileVerificationState::Mismatch
            );
        }
    }
}
