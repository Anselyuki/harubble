use crate::audio_cache;
use crate::audio_cache::AudioCacheLease;
use crate::i18n;
use crate::logging::{LogLevel, LogPayload};
use crate::player::backend::OutputFormat;
use crate::player::stream::{AudioFormat, GrowingFileHandle, PlaybackInput, SampleBuffer};
use crate::player::PlaybackContext;
use crate::player::PlaybackQueueEntry;
use crate::player::{PlaybackError, PlaybackErrorCode, PlaybackStartResult};
use anyhow::{Context, Result};
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncWriteExt;

use super::{normalize_seek_position, AppState, PreparedPlaybackInput};

impl AppState {
    pub fn toggle_playback_from_lifecycle(&self) -> Result<(), String> {
        self.player
            .toggle_playback()
            .map_err(|error| format!("{error:#}"))
    }

    pub(crate) async fn play_song_for_request(
        &self,
        request_id: u64,
        song_cid: String,
        cover_url: Option<String>,
        playback_context: Option<PlaybackContext>,
    ) -> Result<PlaybackStartResult, PlaybackError> {
        self.play_song_with_start_intent(
            request_id,
            song_cid,
            cover_url,
            playback_context,
            PlaybackStartIntent::NewSelection,
        )
        .await
    }

    async fn play_song_with_start_intent(
        &self,
        request_id: u64,
        song_cid: String,
        cover_url: Option<String>,
        playback_context: Option<PlaybackContext>,
        start_intent: PlaybackStartIntent,
    ) -> Result<PlaybackStartResult, PlaybackError> {
        self.ensure_playback_request_active(request_id, None)?;
        let song_detail = self.get_playback_song_detail(&song_cid, None).await?;
        self.ensure_playback_request_active(request_id, None)?;

        self.player.prepare_playback_context(
            playback_context,
            PlaybackQueueEntry {
                cid: song_cid.clone(),
                name: song_detail.name.clone(),
                artists: song_detail.artists.clone(),
                cover_url: cover_url.clone(),
            },
        );

        let loading_started_at = Instant::now();
        let session_id = self
            .player
            .begin_loading_session(
                song_cid.clone(),
                song_detail.name.clone(),
                song_detail.artists.clone(),
                cover_url.clone(),
                0.0,
                None,
            )
            .map_err(|error| classify_playback_error(error, None))?;

        let result: Result<f64, PlaybackStartFailure> = async {
            self.ensure_playback_request_active(request_id, Some(session_id))?;
            self.start_playback_session(
                request_id,
                session_id,
                &song_cid,
                &song_detail.source_url,
                0.0,
                start_intent,
            )
            .await
        }
        .await;

        match result {
            Ok(duration) => {
                self.spawn_playback_startup_metric(
                    request_id,
                    session_id,
                    loading_started_at,
                    "playing",
                    false,
                );
                Ok(PlaybackStartResult::new(duration, session_id))
            }
            Err(error) => {
                let playback_error = classify_playback_error(error.error, Some(session_id));
                let ticket_superseded = playback_error.code == PlaybackErrorCode::Superseded;
                self.player.fail_session(session_id);
                self.spawn_playback_startup_metric(
                    request_id,
                    session_id,
                    loading_started_at,
                    if ticket_superseded {
                        "superseded"
                    } else {
                        "failed"
                    },
                    ticket_superseded,
                );
                Err(playback_error)
            }
        }
    }

    pub(crate) async fn seek_current_for_request(
        &self,
        request_id: u64,
        position_secs: f64,
    ) -> Result<PlaybackStartResult, PlaybackError> {
        self.ensure_playback_request_active(request_id, None)?;
        let current_state = self.player.get_state();
        let song_cid = current_state.song_cid.clone().ok_or_else(|| {
            playback_error(
                PlaybackErrorCode::NoActiveTrack,
                i18n::tr(self.preferences().locale, "player-no-active-track"),
                false,
                None,
            )
        })?;

        if current_state.is_loading {
            return Err(playback_error(
                PlaybackErrorCode::Loading,
                i18n::tr(self.preferences().locale, "player-still-loading"),
                false,
                Some(current_state.session_id),
            ));
        }

        let target_position = normalize_seek_position(position_secs, current_state.duration);
        if (current_state.progress - target_position).abs() < 0.05 {
            return Ok(PlaybackStartResult::new(
                current_state.duration,
                current_state.session_id,
            ));
        }

        let song_detail = self
            .get_playback_song_detail(&song_cid, Some(current_state.session_id))
            .await?;
        self.ensure_playback_request_active(request_id, Some(current_state.session_id))?;

        let cover_url = current_state.cover_url.clone();
        let known_duration = (current_state.duration > 0.0).then_some(current_state.duration);
        let should_pause_after_seek = current_state.is_paused;
        let mut cache_recovery_attempted = false;

        loop {
            let attempt_started_at = Instant::now();
            let previous_session_id = self.player.get_state().session_id;
            let session_id = self
                .player
                .begin_loading_session(
                    song_cid.clone(),
                    song_detail.name.clone(),
                    song_detail.artists.clone(),
                    cover_url.clone(),
                    target_position,
                    known_duration,
                )
                .map_err(|error| classify_playback_error(error, Some(previous_session_id)))?;

            let attempt_result: Result<f64, PlaybackStartFailure> = async {
                self.ensure_playback_request_active(request_id, Some(session_id))?;
                let duration = self
                    .start_playback_session(
                        request_id,
                        session_id,
                        &song_cid,
                        &song_detail.source_url,
                        target_position,
                        PlaybackStartIntent::InteractiveRestart,
                    )
                    .await?;

                if should_pause_after_seek {
                    self.player.pause().map_err(PlaybackStartFailure::from)?;
                }

                Ok(duration)
            }
            .await;

            match attempt_result {
                Ok(duration) => {
                    self.spawn_playback_startup_metric(
                        request_id,
                        session_id,
                        attempt_started_at,
                        "playing",
                        false,
                    );
                    return Ok(PlaybackStartResult::new(duration, session_id));
                }
                Err(error) if should_recover_seek_cache(cache_recovery_attempted, &error) => {
                    cache_recovery_attempted = true;
                    let cache_lease = error.cache_lease.clone().expect("checked by policy");
                    let original_error = error.error;
                    if let Err(clear_error) = self.clear_cache_for_seek_recovery(cache_lease).await
                    {
                        if self
                            .ensure_playback_request_active(request_id, None)
                            .is_err()
                        {
                            return Err(PlaybackError::superseded(session_id));
                        }
                        self.player.fail_session(session_id);
                        self.log_seek_cache_recovery_failure(request_id, session_id, &clear_error);
                        return Err(classify_playback_error(original_error, Some(session_id)));
                    }
                    self.log_seek_cache_recovery(request_id, session_id);
                    self.ensure_playback_request_active(request_id, None)?;
                    self.spawn_playback_startup_metric(
                        request_id,
                        session_id,
                        attempt_started_at,
                        "cache-retry",
                        false,
                    );
                }
                Err(error) => {
                    let playback_error = classify_playback_error(error.error, Some(session_id));
                    let ticket_superseded = playback_error.code == PlaybackErrorCode::Superseded;
                    self.player.fail_session(session_id);
                    self.spawn_playback_startup_metric(
                        request_id,
                        session_id,
                        attempt_started_at,
                        if ticket_superseded {
                            "superseded"
                        } else {
                            "failed"
                        },
                        ticket_superseded,
                    );
                    return Err(playback_error);
                }
            }
        }
    }

    async fn start_playback_session(
        &self,
        request_id: u64,
        session_id: u64,
        song_cid: &str,
        source_url: &str,
        start_position_secs: f64,
        start_intent: PlaybackStartIntent,
    ) -> Result<f64, PlaybackStartFailure> {
        let stop_flag = self.player.stop_signal();
        let pause_flag = self.player.pause_signal();
        let prepared_input = match self
            .prepare_playback_input(
                request_id,
                session_id,
                song_cid.to_string(),
                source_url.to_string(),
                &stop_flag,
            )
            .await
        {
            Ok(input) => input,
            Err(error) => {
                let remove_canonical_cache = error.local_cache_failure;
                self.cleanup_failed_playback_cache(
                    error.cache_lease.clone(),
                    session_id,
                    &stop_flag,
                    remove_canonical_cache,
                );
                return Err(error);
            }
        };
        let cache_lease_for_failure = Some(prepared_input.cache_lease.clone());
        let input = prepared_input.input.clone();

        let inspect_input = input.clone();
        let inspect_stop_flag = Arc::clone(&stop_flag);
        let source_format = match self
            .playback_runtime
            .handle()
            .spawn_blocking(move || {
                inspect_input.inspect_format_with_retry(Some(inspect_stop_flag))
            })
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))
            .and_then(|result| result)
        {
            Ok(source_format) => source_format,
            Err(error) => {
                let remove_canonical_cache = is_local_audio_cache_failure(&error);
                self.cleanup_failed_playback_cache(
                    cache_lease_for_failure.clone(),
                    session_id,
                    &stop_flag,
                    remove_canonical_cache,
                );
                return Err(PlaybackStartFailure::with_cache_context(
                    error,
                    cache_lease_for_failure,
                ));
            }
        };

        if !self.player.is_session_active(session_id) {
            self.cleanup_failed_playback_cache(
                cache_lease_for_failure.clone(),
                session_id,
                &stop_flag,
                false,
            );
            return Err(PlaybackStartFailure::from(anyhow::anyhow!(
                "Playback stopped"
            )));
        }

        let output_format = match self.player.negotiate_output_format(source_format) {
            Ok(format) => format,
            Err(error) => {
                self.cleanup_failed_playback_cache(
                    cache_lease_for_failure.clone(),
                    session_id,
                    &stop_flag,
                    false,
                );
                return Err(PlaybackStartFailure::from(error));
            }
        };
        self.player
            .set_playback_format(session_id, source_format, &output_format);
        self.record_playback_format_selection(session_id, source_format, &output_format);
        let start_position_secs =
            normalize_seek_position(start_position_secs, source_format.duration_secs);
        let sample_buffer = SampleBuffer::new();

        let log_center = Arc::clone(&self.log_center);
        let player = Arc::clone(&self.player);
        let stop_flag_for_error = Arc::clone(&stop_flag);
        let cache_lease_for_error = cache_lease_for_failure.clone();
        let app_for_error = self.clone();
        let startup_pending = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let startup_pending_for_error = Arc::clone(&startup_pending);
        let error_handler: crate::player::stream::PlaybackErrorHandler = Arc::new(move |message| {
            let is_cache_failure = is_local_audio_cache_failure(&anyhow::anyhow!(message.clone()));
            let startup_failure = startup_pending_for_error.load(Ordering::SeqCst);
            log_center.record(
                crate::logging::LogPayload::new(
                    if is_cache_failure && startup_failure {
                        crate::logging::LogLevel::Debug
                    } else if is_cache_failure {
                        crate::logging::LogLevel::Warn
                    } else {
                        crate::logging::LogLevel::Error
                    },
                    "player",
                    if is_cache_failure && startup_failure {
                        "player.decode_worker_cache_recovery"
                    } else if is_cache_failure {
                        "player.decode_worker_cache_failure"
                    } else {
                        "player.decode_worker_failed"
                    },
                    if is_cache_failure && startup_failure {
                        "Audio cache read failed; retrying playback startup"
                    } else if is_cache_failure {
                        "Audio cache read failed; playback recovery may retry"
                    } else {
                        "Audio decode worker failed"
                    },
                )
                .details(message),
            );
            app_for_error.cleanup_failed_playback_cache(
                cache_lease_for_error.clone(),
                session_id,
                &stop_flag_for_error,
                is_cache_failure,
            );
            if !startup_failure {
                player.fail_session(session_id);
            }
        });

        let _decode_worker = match input.spawn_decode_worker(
            source_format,
            output_format.audio_format,
            sample_buffer.clone(),
            Arc::clone(&stop_flag),
            Arc::clone(&pause_flag),
            start_position_secs,
            error_handler,
        ) {
            Ok(worker) => worker,
            Err(error) => {
                let remove_canonical_cache = is_local_audio_cache_failure(&error);
                self.cleanup_failed_playback_cache(
                    cache_lease_for_failure.clone(),
                    session_id,
                    &stop_flag,
                    remove_canonical_cache,
                );
                return Err(PlaybackStartFailure::with_cache_context(
                    error,
                    cache_lease_for_failure,
                ));
            }
        };

        let is_streaming_input = matches!(input, PlaybackInput::GrowingFile(_));
        if let Err(error) = self
            .wait_for_initial_buffer(
                &sample_buffer,
                output_format.audio_format,
                is_streaming_input,
                start_intent,
                &stop_flag,
            )
            .await
        {
            let remove_canonical_cache = is_local_audio_cache_failure(&error);
            self.cleanup_failed_playback_cache(
                cache_lease_for_failure.clone(),
                session_id,
                &stop_flag,
                remove_canonical_cache,
            );
            return Err(PlaybackStartFailure::with_cache_context(
                error,
                cache_lease_for_failure,
            ));
        }

        startup_pending.store(false, Ordering::SeqCst);
        match self.start_prepared_playback(
            session_id,
            output_format,
            sample_buffer,
            start_position_secs,
        ) {
            Ok(duration) => Ok(duration),
            Err(error) => {
                self.cleanup_failed_playback_cache(
                    cache_lease_for_failure.clone(),
                    session_id,
                    &stop_flag,
                    false,
                );
                Err(PlaybackStartFailure::from(error))
            }
        }
    }

    pub(crate) async fn play_next_for_request(
        &self,
        request_id: u64,
    ) -> Result<PlaybackStartResult, PlaybackError> {
        self.ensure_playback_request_active(request_id, None)?;
        let target = self.player.peek_next_entry().ok_or_else(|| {
            playback_error(
                PlaybackErrorCode::NoNextTrack,
                i18n::tr(self.preferences().locale, "player-no-next-track"),
                false,
                Some(self.player.get_state().session_id),
            )
        })?;
        self.play_song_with_start_intent(
            request_id,
            target.cid,
            target.cover_url,
            None,
            PlaybackStartIntent::InteractiveRestart,
        )
        .await
    }

    pub(crate) async fn play_previous_for_request(
        &self,
        request_id: u64,
    ) -> Result<PlaybackStartResult, PlaybackError> {
        self.ensure_playback_request_active(request_id, None)?;
        let target = self.player.peek_previous_entry().ok_or_else(|| {
            playback_error(
                PlaybackErrorCode::NoPreviousTrack,
                i18n::tr(self.preferences().locale, "player-no-previous-track"),
                false,
                Some(self.player.get_state().session_id),
            )
        })?;
        self.play_song_with_start_intent(
            request_id,
            target.cid,
            target.cover_url,
            None,
            PlaybackStartIntent::InteractiveRestart,
        )
        .await
    }

    fn ensure_playback_request_active(
        &self,
        request_id: u64,
        session_id: Option<u64>,
    ) -> Result<(), PlaybackError> {
        if self.player.is_playback_request_active(request_id) {
            return Ok(());
        }
        Err(PlaybackError::superseded(
            session_id.unwrap_or_else(|| self.player.get_state().session_id),
        ))
    }

    async fn prepare_playback_input(
        &self,
        request_id: u64,
        session_id: u64,
        song_cid: String,
        source_url: String,
        stop_flag: &Arc<std::sync::atomic::AtomicBool>,
    ) -> std::result::Result<PreparedPlaybackInput, PlaybackStartFailure> {
        let cache_io_guard = audio_cache::io_lock().lock().await;
        if !self.player.is_playback_request_active(request_id) {
            return Err(PlaybackStartFailure::from(PlaybackError::superseded(
                session_id,
            )));
        }
        let cache_path = audio_cache::cached_song_path(&song_cid, &source_url)
            .map_err(PlaybackStartFailure::from)?;
        let cache_lease = acquire_active_cache_lease(&cache_path, stop_flag)
            .map_err(PlaybackStartFailure::from)?;
        let preparation_lease = cache_lease.clone();
        let (prepared_cache_lease, pending_marker, prepared_input) = self
            .playback_runtime
            .handle()
            .spawn_blocking(move || {
                prepare_playback_input_from_cache_path_with_lease(cache_path, preparation_lease)
            })
            .await
            .map_err(|error| {
                PlaybackStartFailure::with_cache_context(
                    anyhow::anyhow!(error.to_string()),
                    Some(cache_lease.clone()),
                )
            })?
            .map_err(|error| {
                PlaybackStartFailure::with_cache_context(error, Some(cache_lease.clone()))
            })?;
        drop(cache_io_guard);

        let input = match prepared_input {
            PlaybackInput::GrowingFile(handle) => {
                let Some(_) = pending_marker else {
                    return Err(PlaybackStartFailure::from(anyhow::anyhow!(
                        "Streaming playback cache marker was not prepared"
                    )));
                };
                self.spawn_stream_download(
                    source_url,
                    Arc::clone(stop_flag),
                    handle.clone(),
                    prepared_cache_lease.clone(),
                );
                PlaybackInput::growing_file(handle)
            }
            input => input,
        };

        Ok(PreparedPlaybackInput {
            input,
            cache_lease: prepared_cache_lease,
        })
    }

    fn cleanup_failed_playback_cache(
        &self,
        cache_lease: Option<AudioCacheLease>,
        session_id: u64,
        stop_flag: &Arc<std::sync::atomic::AtomicBool>,
        remove_canonical_cache: bool,
    ) {
        if self.player.is_session_active(session_id) {
            stop_flag.store(true, Ordering::SeqCst);
        }

        let Some(cache_lease) = cache_lease else {
            return;
        };
        let log_center = Arc::clone(&self.log_center);
        self.playback_runtime.spawn(async move {
            let _cache_io_guard = audio_cache::io_lock().lock().await;
            let cleanup_result = if remove_canonical_cache {
                audio_cache::remove_song_cache_if_current(&cache_lease).map(|_| ())
            } else {
                audio_cache::remove_staging_cache(&cache_lease)
            };
            if let Err(error) = cleanup_result {
                log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "player",
                        "player.cache_cleanup_failed",
                        "Failed to remove audio cache after playback failure",
                    )
                    .details(format!("{error:#}")),
                );
            }
        });
    }

    fn spawn_playback_startup_metric(
        &self,
        request_id: u64,
        session_id: u64,
        loading_started_at: Instant,
        outcome: &'static str,
        ticket_superseded: bool,
    ) {
        let log_center = self.log_center.clone();
        let loading_ms = loading_started_at.elapsed().as_millis();
        tauri::async_runtime::spawn(async move {
            log_center.record(
                LogPayload::new(
                    LogLevel::Debug,
                    "playback",
                    "playback.startup_completed",
                    "Playback startup completed",
                )
                .context(json!({
                    "playback.request_id": request_id,
                    "playback.session_id": session_id,
                    "playback.loading_ms": loading_ms,
                    "playback.startup_outcome": outcome,
                    "playback.ticket_superseded": ticket_superseded,
                })),
            );
        });
    }

    async fn get_playback_song_detail(
        &self,
        song_cid: &str,
        session_id: Option<u64>,
    ) -> Result<harubble_core::SongDetail, PlaybackError> {
        let api = Arc::clone(&self.api_clients.playback_api);
        let song_cid = song_cid.to_string();

        self.playback_runtime
            .spawn(async move { api.get_song_detail(&song_cid).await })
            .await
            .map_err(|error| {
                playback_error(
                    PlaybackErrorCode::Internal,
                    format!("playback runtime task failed: {error}"),
                    false,
                    session_id,
                )
            })?
            .map_err(|error| {
                playback_error(
                    PlaybackErrorCode::Network,
                    error.to_string(),
                    true,
                    session_id,
                )
            })
    }

    fn spawn_stream_download(
        &self,
        source_url: String,
        stop_flag: Arc<std::sync::atomic::AtomicBool>,
        handle: GrowingFileHandle,
        cache_lease: AudioCacheLease,
    ) {
        let api = Arc::clone(&self.api_clients.playback_api);
        let log_center = Arc::clone(&self.log_center);
        let download_runtime = Arc::clone(&self.playback_runtime);
        let cleanup_runtime = Arc::clone(&download_runtime);

        let _download_task = download_runtime.spawn(async move {
            let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(2);
            let writer_handle = handle.clone();
            let writer_cache_path = cache_lease.staging_path().to_path_buf();
            let writer_cache_lease = cache_lease.clone();
            let writer_stop_flag = Arc::clone(&stop_flag);
            let write_task = tokio::spawn(async move {
                let mut writer = {
                    let _cache_io_guard = audio_cache::io_lock().lock().await;
                    if writer_stop_flag.load(Ordering::SeqCst) || !writer_cache_lease.is_current() {
                        return Ok::<_, anyhow::Error>(false);
                    }
                    tokio::fs::OpenOptions::new()
                        .create(false)
                        .write(true)
                        .open(&writer_cache_path)
                        .await
                        .with_context(|| {
                            format!(
                                "Failed to open streaming cache file {}",
                                writer_cache_path.display()
                            )
                        })?
                };
                let mut position = 0_u64;

                while let Some(chunk) = chunk_rx.recv().await {
                    let _cache_io_guard = audio_cache::io_lock().lock().await;
                    if writer_stop_flag.load(Ordering::SeqCst) || !writer_cache_lease.is_current() {
                        return Ok(false);
                    }
                    writer
                        .write_all(&chunk)
                        .await
                        .context("Failed to append audio chunk to cache file")?;
                    position += chunk.len() as u64;
                    writer_handle.publish_available_len(position);
                }

                let _cache_io_guard = audio_cache::io_lock().lock().await;
                if writer_stop_flag.load(Ordering::SeqCst) || !writer_cache_lease.is_current() {
                    return Ok(false);
                }
                writer
                    .flush()
                    .await
                    .context("Failed to flush streaming cache file")?;
                Ok(true)
            });

            let total_len_set = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let handle_for_download = handle.clone();
            let stop_flag_for_download = Arc::clone(&stop_flag);
            let cache_lease_for_download = cache_lease.clone();
            let chunk_tx_for_download = chunk_tx.clone();
            let download_result = api
                .download_stream_owned(&source_url, move |chunk, _, total| {
                    let chunk_tx = chunk_tx_for_download.clone();
                    let handle = handle_for_download.clone();
                    let stop_flag = Arc::clone(&stop_flag_for_download);
                    let cache_lease = cache_lease_for_download.clone();
                    let total_len_set = Arc::clone(&total_len_set);
                    async move {
                        if stop_flag.load(Ordering::SeqCst) || !cache_lease.is_current() {
                            return Ok(false);
                        }
                        if !total_len_set.load(Ordering::Relaxed) {
                            if let Some(total) = total {
                                if let Some(parent) = handle.path().parent() {
                                    harubble_core::ensure_available_space(parent, total)?;
                                }
                                handle.set_expected_total_len(total);
                                total_len_set.store(true, Ordering::Relaxed);
                            }
                        }
                        if chunk_tx.send(chunk).await.is_err() {
                            return Ok(false);
                        }
                        Ok(true)
                    }
                })
                .await;
            drop(chunk_tx);
            let write_result = write_task
                .await
                .map_err(|error| anyhow::anyhow!(error.to_string()))
                .and_then(|result| result);

            if stop_flag.load(Ordering::SeqCst) || !cache_lease.is_current() {
                handle.mark_error("Playback stopped");
                let _cache_io_guard = audio_cache::io_lock().lock().await;
                let _ = audio_cache::remove_staging_cache(&cache_lease);
                return;
            }

            match (download_result, write_result) {
                (Ok(()), Ok(true)) => {
                    let completion_result = {
                        let _cache_io_guard = audio_cache::io_lock().lock().await;
                        if stop_flag.load(Ordering::SeqCst) {
                            Ok(false)
                        } else {
                            audio_cache::complete_song_cache_if_current(&cache_lease)
                        }
                    };
                    match completion_result {
                        Ok(true) => {
                            handle.mark_complete();
                            cleanup_runtime
                                .handle()
                                .spawn_blocking(audio_cache::spawn_cleanup_if_needed);
                        }
                        Ok(false) => {
                            handle.mark_error("Playback stopped");
                            let _cache_io_guard = audio_cache::io_lock().lock().await;
                            let _ = audio_cache::remove_staging_cache(&cache_lease);
                        }
                        Err(error) => {
                            let is_cache_failure =
                                is_local_audio_cache_failure(&error);
                            log_center.record(
                                LogPayload::new(
                                    if is_cache_failure {
                                        LogLevel::Warn
                                    } else {
                                        LogLevel::Error
                                    },
                                    "player",
                                    if is_cache_failure {
                                        "player.stream_cache_io_failed"
                                    } else {
                                        "player.stream_cache_finalize_failed"
                                    },
                                    if is_cache_failure {
                                        "Audio cache finalization failed; playback recovery may retry"
                                    } else {
                                        "Failed to finalize streaming playback cache"
                                    },
                                )
                                .details(format!("{error:#}")),
                            );
                            handle.mark_error(error.to_string());
                            let _cache_io_guard = audio_cache::io_lock().lock().await;
                            if is_cache_failure {
                                let _ = audio_cache::remove_song_cache_if_current(&cache_lease);
                            } else {
                                let _ = audio_cache::remove_staging_cache(&cache_lease);
                            }
                        }
                    }
                }
                (Ok(()), Ok(false)) => {
                    handle.mark_error("Playback stopped");
                    let _cache_io_guard = audio_cache::io_lock().lock().await;
                    let _ = audio_cache::remove_staging_cache(&cache_lease);
                }
                (Err(error), _) | (_, Err(error)) => {
                    let is_cache_failure =
                        is_local_audio_cache_failure(&anyhow::anyhow!(error.to_string()));
                    log_center.record(
                        LogPayload::new(
                            if is_cache_failure {
                                LogLevel::Warn
                            } else {
                                LogLevel::Error
                            },
                            "player",
                            if is_cache_failure {
                                "player.stream_cache_io_failed"
                            } else {
                                "player.stream_download_failed"
                            },
                            if is_cache_failure {
                                "Audio cache write failed; playback recovery may retry"
                            } else {
                                "Streaming download failed during playback"
                            },
                        )
                        .details(format!("{error:#}")),
                    );
                    handle.mark_error(error.to_string());
                    let _cache_io_guard = audio_cache::io_lock().lock().await;
                    if is_cache_failure {
                        let _ = audio_cache::remove_song_cache_if_current(&cache_lease);
                    } else {
                        let _ = audio_cache::remove_staging_cache(&cache_lease);
                    }
                }
            }
        });
    }

    async fn wait_for_initial_buffer(
        &self,
        sample_buffer: &SampleBuffer,
        output_format: AudioFormat,
        is_streaming_input: bool,
        start_intent: PlaybackStartIntent,
        stop_flag: &Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<()> {
        let minimum_samples =
            initial_buffer_samples(output_format, is_streaming_input, start_intent);

        let wait_buffer = sample_buffer.clone();
        let wait_stop = Arc::clone(stop_flag);
        self.playback_runtime
            .handle()
            .spawn_blocking(move || wait_buffer.wait_for_samples(minimum_samples, &wait_stop))
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))??;
        Ok(())
    }

    fn record_playback_format_selection(
        &self,
        session_id: u64,
        source_format: AudioFormat,
        output_format: &OutputFormat,
    ) {
        self.log_center.record(
            crate::logging::LogPayload::new(
                crate::logging::LogLevel::Debug,
                "player",
                "audio.format_selected",
                "Audio playback format selected",
            )
            .context(json!({
                "playback.session_id": session_id,
                "audio.source.sample_rate": source_format.sample_rate,
                "audio.source.channels": source_format.channels,
                "audio.source.bits_per_sample": source_format.bits_per_sample,
                "audio.source.duration_secs": source_format.duration_secs,
                "audio.output.sample_rate": output_format.audio_format.sample_rate,
                "audio.output.channels": output_format.audio_format.channels,
                "audio.output.bits_per_sample": output_format.audio_format.bits_per_sample,
                "audio.output.sample_format": output_format.sample_format.as_str(),
                "audio.output.device_identity": output_format.device_identity,
                "audio.resampling.enabled": source_format.sample_rate != output_format.audio_format.sample_rate,
                "audio.channel_remix.enabled": source_format.channels != output_format.audio_format.channels,
            })),
        );
    }

    fn start_prepared_playback(
        &self,
        session_id: u64,
        output_format: OutputFormat,
        sample_buffer: SampleBuffer,
        start_position_secs: f64,
    ) -> Result<f64> {
        anyhow::ensure!(
            self.player.is_session_active(session_id),
            "Playback stopped"
        );

        self.player.start_stream_playback(
            session_id,
            output_format,
            sample_buffer,
            start_position_secs,
        )
    }

    async fn clear_cache_for_seek_recovery(&self, cache_lease: AudioCacheLease) -> Result<()> {
        let _cache_io_guard = audio_cache::io_lock().lock().await;
        if audio_cache::remove_song_cache_if_current(&cache_lease)? {
            Ok(())
        } else {
            anyhow::bail!("Audio cache lease was superseded before seek recovery")
        }
    }

    fn log_seek_cache_recovery(&self, request_id: u64, session_id: u64) {
        self.log_center.record(
            LogPayload::new(
                LogLevel::Warn,
                "player",
                "player.seek_cache_recovery",
                "Reloading seek after local audio cache failure",
            )
            .context(json!({
                "playback.request_id": request_id,
                "playback.session_id": session_id,
                "playback.cache_recovery_attempt": 1,
            })),
        );
    }

    fn log_seek_cache_recovery_failure(
        &self,
        request_id: u64,
        session_id: u64,
        error: &anyhow::Error,
    ) {
        self.log_center.record(
            LogPayload::new(
                LogLevel::Warn,
                "player",
                "player.seek_cache_recovery_failed",
                "Failed to clear audio cache before retrying seek",
            )
            .details(format!("{error:#}"))
            .context(json!({
                "playback.request_id": request_id,
                "playback.session_id": session_id,
                "playback.cache_recovery_attempt": 1,
            })),
        );
    }
}

#[derive(Debug)]
struct PlaybackStartFailure {
    error: anyhow::Error,
    cache_lease: Option<AudioCacheLease>,
    local_cache_failure: bool,
}

impl PlaybackStartFailure {
    fn with_cache_context(error: anyhow::Error, cache_lease: Option<AudioCacheLease>) -> Self {
        let local_cache_failure = is_local_audio_cache_failure(&error);
        Self {
            error,
            cache_lease,
            local_cache_failure,
        }
    }
}

impl From<anyhow::Error> for PlaybackStartFailure {
    fn from(error: anyhow::Error) -> Self {
        Self {
            error,
            cache_lease: None,
            local_cache_failure: false,
        }
    }
}

fn should_recover_seek_cache(recovery_attempted: bool, failure: &PlaybackStartFailure) -> bool {
    !recovery_attempted && failure.local_cache_failure && failure.cache_lease.is_some()
}

impl From<PlaybackError> for PlaybackStartFailure {
    fn from(error: PlaybackError) -> Self {
        Self::from(anyhow::Error::new(error))
    }
}

fn is_local_audio_cache_failure(error: &anyhow::Error) -> bool {
    let message = format!("{error:#}").to_ascii_lowercase();
    message.contains("failed to open cached audio file")
        || message.contains("failed to read cached audio file")
        || message.contains("failed to seek cached audio file")
        || message.contains("failed to inspect cached audio file")
        || message.contains("failed to open streaming cache file")
        || message.contains("failed to read streaming cache file")
        || message.contains("failed to seek streaming cache file")
        || message.contains("failed to inspect streaming cache file")
        || message.contains("failed to append audio chunk to cache file")
        || message.contains("failed to flush streaming cache file")
        || message.contains("failed to create cache marker")
        || message.contains("failed to create cache file")
        || message.contains("failed to promote streaming cache file")
        || message.contains("failed to remove audio cache file")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaybackStartIntent {
    NewSelection,
    InteractiveRestart,
}

impl PlaybackStartIntent {
    fn streaming_buffer_seconds(self) -> usize {
        match self {
            Self::NewSelection => 5,
            Self::InteractiveRestart => 1,
        }
    }
}

fn initial_buffer_samples(
    output_format: AudioFormat,
    is_streaming_input: bool,
    start_intent: PlaybackStartIntent,
) -> usize {
    let sample_rate = output_format.sample_rate.max(1) as usize;
    let channels = output_format.channels.max(1) as usize;
    let seconds = if is_streaming_input {
        start_intent.streaming_buffer_seconds()
    } else {
        1
    };
    let target = sample_rate * channels * seconds;
    let minimum = channels * 4096;
    let maximum = SampleBuffer::max_capacity_samples() / 2;

    target.max(minimum).min(maximum)
}

type PrepareInputResult = (AudioCacheLease, Option<PathBuf>, PlaybackInput);

fn acquire_active_cache_lease(
    cache_path: &std::path::Path,
    stop_flag: &std::sync::atomic::AtomicBool,
) -> Result<AudioCacheLease> {
    anyhow::ensure!(!stop_flag.load(Ordering::SeqCst), "Playback stopped");
    Ok(audio_cache::acquire_song_cache_lease(cache_path))
}

#[cfg(test)]
fn prepare_playback_input_from_cache_path(cache_path: PathBuf) -> Result<PrepareInputResult> {
    let cache_lease = audio_cache::acquire_song_cache_lease(&cache_path);
    prepare_playback_input_from_cache_path_with_lease(cache_path, cache_lease)
}

fn prepare_playback_input_from_cache_path_with_lease(
    cache_path: PathBuf,
    cache_lease: AudioCacheLease,
) -> Result<PrepareInputResult> {
    if audio_cache::is_song_cached(&cache_path) {
        return Ok((cache_lease, None, PlaybackInput::cached_file(cache_path)?));
    }

    audio_cache::remove_song_cache_if_current(&cache_lease)?;
    let pending_marker = audio_cache::pending_marker_path(cache_lease.staging_path());
    std::fs::write(&pending_marker, b"pending")
        .with_context(|| format!("Failed to create cache marker {}", pending_marker.display()))?;

    let (handle, writer) = GrowingFileHandle::new(cache_lease.staging_path().to_path_buf())?;
    drop(writer);
    Ok((
        cache_lease,
        Some(pending_marker),
        PlaybackInput::growing_file(handle),
    ))
}

fn playback_error(
    code: PlaybackErrorCode,
    message: impl Into<String>,
    retryable: bool,
    session_id: Option<u64>,
) -> PlaybackError {
    PlaybackError::new(code, message, retryable, session_id)
}

fn classify_playback_error(error: anyhow::Error, session_id: Option<u64>) -> PlaybackError {
    let local_cache_failure = is_local_audio_cache_failure(&error);
    let message = format!("{error:#}");
    let lowered = message.to_ascii_lowercase();
    let code = if lowered.contains("playback request was superseded")
        || lowered.contains("playback stopped")
        || lowered.contains("playback session expired")
    {
        PlaybackErrorCode::Superseded
    } else if local_cache_failure {
        PlaybackErrorCode::Io
    } else if lowered.contains("download")
        || lowered.contains("network")
        || lowered.contains("request")
        || lowered.contains("http")
    {
        PlaybackErrorCode::Network
    } else if lowered.contains("audio")
        || lowered.contains("decoder")
        || lowered.contains("decode")
        || lowered.contains("probe")
        || lowered.contains("stream")
        || lowered.contains("output")
        || lowered.contains("cpal")
    {
        PlaybackErrorCode::Audio
    } else {
        PlaybackErrorCode::Internal
    };
    let retryable = matches!(
        code,
        PlaybackErrorCode::Network | PlaybackErrorCode::Audio | PlaybackErrorCode::Io
    );
    PlaybackError::new(code, message, retryable, session_id)
}

#[cfg(test)]
mod tests {
    use super::{
        acquire_active_cache_lease, classify_playback_error, initial_buffer_samples,
        prepare_playback_input_from_cache_path, should_recover_seek_cache, PlaybackStartFailure,
        PlaybackStartIntent,
    };
    use crate::audio_cache;
    use crate::player::stream::{AudioFormat, PlaybackInput, SampleBuffer};
    use crate::player::PlaybackErrorCode;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn cached_playback_input_keeps_cache_path_for_failure_cleanup() {
        let temp_dir = tempdir().expect("tempdir");
        let cache_path = temp_dir.path().join("cached-song.wav");
        let pending_marker = audio_cache::pending_marker_path(&cache_path);
        std::fs::write(&cache_path, b"not a real wav").expect("cached file");

        let (cache_lease, pending_download, input) =
            prepare_playback_input_from_cache_path(cache_path.clone())
                .expect("prepared cached input");

        assert_eq!(cache_lease.cache_path(), cache_path);
        assert!(pending_download.is_none());
        assert!(matches!(input, PlaybackInput::CachedFile(handle) if handle.path() == cache_path));
        assert!(!pending_marker.exists());
    }

    #[test]
    fn streaming_playback_input_keeps_cache_path_for_failure_cleanup() {
        let temp_dir = tempdir().expect("tempdir");
        let cache_path = temp_dir.path().join("stream-song.wav");

        let (cache_lease, pending_download, input) =
            prepare_playback_input_from_cache_path(cache_path.clone())
                .expect("prepared streaming input");

        assert_eq!(cache_lease.cache_path(), cache_path);
        let pending_marker = audio_cache::pending_marker_path(cache_lease.staging_path());
        assert_eq!(pending_download, Some(pending_marker.clone()));
        assert!(matches!(input, PlaybackInput::GrowingFile(_)));
        assert!(pending_marker.exists());
        assert!(cache_lease.staging_path().exists());
        assert!(!cache_path.exists());
    }

    #[tokio::test]
    async fn queued_stopped_preparation_does_not_acquire_a_cache_generation() {
        let temp_dir = tempdir().expect("tempdir");
        let cache_path = temp_dir.path().join("stopped-song.wav");
        let stop_flag = Arc::new(AtomicBool::new(false));
        let cache_guard = audio_cache::io_lock().lock().await;
        let queued_stop_flag = Arc::clone(&stop_flag);
        let queued_cache_path = cache_path.clone();
        let queued_prepare = tokio::spawn(async move {
            let _cache_guard = audio_cache::io_lock().lock().await;
            acquire_active_cache_lease(&queued_cache_path, &queued_stop_flag)
        });

        tokio::task::yield_now().await;
        stop_flag.store(true, Ordering::SeqCst);
        drop(cache_guard);

        let error = queued_prepare
            .await
            .expect("queued task")
            .expect_err("stopped preparation");
        assert!(format!("{error:#}").contains("Playback stopped"));
        assert!(!cache_path.exists());
        assert!(!audio_cache::pending_marker_path(&cache_path).exists());
    }

    #[test]
    fn seek_cache_recovery_is_limited_to_one_local_io_retry() {
        let temp_dir = tempdir().expect("tempdir");
        let cache_path = temp_dir.path().join("song.wav");
        let cache_lease = audio_cache::acquire_song_cache_lease(&cache_path);
        let failure = PlaybackStartFailure::with_cache_context(
            anyhow::anyhow!(
                "Failed to open streaming cache file {}: No such file or directory",
                cache_path.display()
            ),
            Some(cache_lease),
        );

        assert!(should_recover_seek_cache(false, &failure));
        assert!(!should_recover_seek_cache(true, &failure));
    }

    #[test]
    fn seek_cache_recovery_does_not_swallow_other_playback_failures() {
        let temp_dir = tempdir().expect("tempdir");
        let cache_path = temp_dir.path().join("song.wav");

        for message in [
            "Failed to probe audio stream",
            "Failed to open output device",
            "HTTP request failed",
            "Audio decoder reset required",
        ] {
            let failure = PlaybackStartFailure::with_cache_context(
                anyhow::anyhow!(message),
                Some(audio_cache::acquire_song_cache_lease(&cache_path)),
            );
            assert!(
                !should_recover_seek_cache(false, &failure),
                "unexpected cache recovery for {message}"
            );
        }
    }

    #[test]
    fn playback_error_classification_distinguishes_cache_and_output_io() {
        let cache_error = classify_playback_error(
            anyhow::anyhow!("Failed to open cached audio file /tmp/song.wav"),
            Some(7),
        );
        let output_error = classify_playback_error(
            anyhow::anyhow!("Failed to open audio output device"),
            Some(8),
        );

        assert_eq!(cache_error.code, PlaybackErrorCode::Io);
        assert_eq!(output_error.code, PlaybackErrorCode::Audio);
    }

    #[test]
    fn streaming_initial_buffer_waits_for_several_seconds_of_audio() {
        let format = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 180.0,
            bits_per_sample: None,
        };

        assert_eq!(
            initial_buffer_samples(format, true, PlaybackStartIntent::NewSelection),
            480_000
        );
        assert_eq!(
            initial_buffer_samples(format, true, PlaybackStartIntent::InteractiveRestart),
            96_000
        );
        assert_eq!(
            initial_buffer_samples(format, false, PlaybackStartIntent::NewSelection),
            96_000
        );
    }

    #[test]
    fn initial_buffer_samples_caps_high_rate_devices_to_keep_headroom() {
        let format = AudioFormat {
            channels: 2,
            sample_rate: 768_000,
            duration_secs: 180.0,
            bits_per_sample: None,
        };

        assert_eq!(
            initial_buffer_samples(format, true, PlaybackStartIntent::NewSelection),
            SampleBuffer::max_capacity_samples() / 2
        );
    }
}
