use crate::audio_cache;
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

        let result: Result<f64> = async {
            self.ensure_playback_request_active(request_id, Some(session_id))?;
            self.start_playback_session(
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
                let playback_error = classify_playback_error(error, Some(session_id));
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

        let loading_started_at = Instant::now();
        let session_id = self
            .player
            .begin_loading_session(
                song_cid.clone(),
                song_detail.name.clone(),
                song_detail.artists.clone(),
                current_state.cover_url.clone(),
                target_position,
                (current_state.duration > 0.0).then_some(current_state.duration),
            )
            .map_err(|error| classify_playback_error(error, Some(current_state.session_id)))?;

        let should_pause_after_seek = current_state.is_paused;
        let result: Result<f64> = async {
            self.ensure_playback_request_active(request_id, Some(session_id))?;
            let duration = self
                .start_playback_session(
                    session_id,
                    &song_cid,
                    &song_detail.source_url,
                    target_position,
                    PlaybackStartIntent::InteractiveRestart,
                )
                .await?;

            if should_pause_after_seek {
                self.player.pause()?;
            }

            Ok(duration)
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
                let playback_error = classify_playback_error(error, Some(session_id));
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

    async fn start_playback_session(
        &self,
        session_id: u64,
        song_cid: &str,
        source_url: &str,
        start_position_secs: f64,
        start_intent: PlaybackStartIntent,
    ) -> Result<f64> {
        let stop_flag = self.player.stop_signal();
        let pause_flag = self.player.pause_signal();
        let prepared_input = self
            .prepare_playback_input(song_cid.to_string(), source_url.to_string(), &stop_flag)
            .await?;
        let cache_path_for_failure = Some(prepared_input.cache_path.clone());
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
                self.cleanup_failed_playback_cache(
                    cache_path_for_failure.clone(),
                    session_id,
                    &stop_flag,
                );
                return Err(error);
            }
        };

        anyhow::ensure!(
            self.player.is_session_active(session_id),
            "Playback stopped"
        );

        let output_format = self.player.negotiate_output_format(source_format)?;
        self.player
            .set_playback_format(session_id, source_format, &output_format);
        self.record_playback_format_selection(session_id, source_format, &output_format);
        let start_position_secs =
            normalize_seek_position(start_position_secs, source_format.duration_secs);
        let sample_buffer = SampleBuffer::new();

        let log_center = Arc::clone(&self.log_center);
        let player = Arc::clone(&self.player);
        let stop_flag_for_error = Arc::clone(&stop_flag);
        let cache_path_for_error = cache_path_for_failure.clone();
        let app_for_error = self.clone();
        let error_handler: crate::player::stream::PlaybackErrorHandler = Arc::new(move |message| {
            log_center.record(
                crate::logging::LogPayload::new(
                    crate::logging::LogLevel::Error,
                    "player",
                    "player.decode_worker_failed",
                    "Audio decode worker failed",
                )
                .details(message),
            );
            app_for_error.cleanup_failed_playback_cache(
                cache_path_for_error.clone(),
                session_id,
                &stop_flag_for_error,
            );
            player.fail_session(session_id);
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
                self.cleanup_failed_playback_cache(
                    cache_path_for_failure.clone(),
                    session_id,
                    &stop_flag,
                );
                return Err(error);
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
            self.cleanup_failed_playback_cache(cache_path_for_failure, session_id, &stop_flag);
            return Err(error);
        }

        self.start_prepared_playback(
            session_id,
            output_format,
            sample_buffer,
            start_position_secs,
        )
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
        song_cid: String,
        source_url: String,
        stop_flag: &Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<PreparedPlaybackInput> {
        let (cache_path, pending_download, prepared_input) = {
            let song_cid = song_cid.clone();
            let source_url = source_url.clone();
            self.playback_runtime
                .handle()
                .spawn_blocking(move || prepare_cached_or_streaming_input(&song_cid, &source_url))
                .await
                .map_err(|error| anyhow::anyhow!(error.to_string()))??
        };

        let input = match prepared_input {
            PlaybackInput::GrowingFile(handle) => {
                let Some((cache_path, pending_marker)) = pending_download else {
                    anyhow::bail!("Streaming playback cache paths were not prepared");
                };
                self.spawn_stream_download(
                    source_url,
                    Arc::clone(stop_flag),
                    handle.clone(),
                    cache_path.clone(),
                    pending_marker,
                );
                PlaybackInput::growing_file(handle)
            }
            input => input,
        };

        Ok(PreparedPlaybackInput { input, cache_path })
    }

    fn cleanup_failed_playback_cache(
        &self,
        cache_path: Option<PathBuf>,
        session_id: u64,
        stop_flag: &Arc<std::sync::atomic::AtomicBool>,
    ) {
        if stop_flag.load(Ordering::SeqCst) || !self.player.is_session_active(session_id) {
            return;
        }
        stop_flag.store(true, Ordering::SeqCst);

        let Some(cache_path) = cache_path else {
            return;
        };
        let pending_marker = audio_cache::pending_marker_path(&cache_path);
        self.playback_runtime.handle().spawn_blocking(move || {
            let _ = std::fs::remove_file(&pending_marker);
            let _ = std::fs::remove_file(&cache_path);
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
        cache_path: PathBuf,
        pending_marker: PathBuf,
    ) {
        let api = Arc::clone(&self.api_clients.playback_api);
        let log_center = Arc::clone(&self.log_center);
        let download_runtime = Arc::clone(&self.playback_runtime);
        let cleanup_runtime = Arc::clone(&download_runtime);

        let _download_task = download_runtime.spawn(async move {
            let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(2);
            let writer_handle = handle.clone();
            let writer_cache_path = cache_path.clone();
            let write_task = tokio::spawn(async move {
                let mut writer = tokio::fs::OpenOptions::new()
                    .create(false)
                    .write(true)
                    .open(&writer_cache_path)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to open streaming cache file {}",
                            writer_cache_path.display()
                        )
                    })?;
                let mut position = 0_u64;

                while let Some(chunk) = chunk_rx.recv().await {
                    writer
                        .write_all(&chunk)
                        .await
                        .context("Failed to append audio chunk to cache file")?;
                    position += chunk.len() as u64;
                    writer_handle.publish_available_len(position);
                }

                writer
                    .flush()
                    .await
                    .context("Failed to flush streaming cache file")?;
                Ok::<_, anyhow::Error>(())
            });

            let total_len_set = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let handle_for_download = handle.clone();
            let stop_flag_for_download = Arc::clone(&stop_flag);
            let chunk_tx_for_download = chunk_tx.clone();
            let download_result = api
                .download_stream_owned(&source_url, move |chunk, _, total| {
                    let chunk_tx = chunk_tx_for_download.clone();
                    let handle = handle_for_download.clone();
                    let stop_flag = Arc::clone(&stop_flag_for_download);
                    let total_len_set = Arc::clone(&total_len_set);
                    async move {
                        if stop_flag.load(Ordering::SeqCst) {
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

            match (download_result, write_result) {
                (Ok(()), Ok(())) if !stop_flag.load(Ordering::SeqCst) => {
                    handle.mark_complete();
                    cleanup_runtime.handle().spawn_blocking(move || {
                        let _ = std::fs::remove_file(&pending_marker);
                        audio_cache::spawn_cleanup_if_needed();
                    });
                }
                (Ok(()), Ok(())) => {
                    handle.mark_error("Playback stopped");
                    cleanup_runtime.handle().spawn_blocking(move || {
                        let _ = std::fs::remove_file(&pending_marker);
                        let _ = std::fs::remove_file(&cache_path);
                    });
                }
                (Err(error), _) | (_, Err(error)) => {
                    log_center.record(
                        LogPayload::new(
                            LogLevel::Error,
                            "player",
                            "player.stream_download_failed",
                            "Streaming download failed during playback",
                        )
                        .details(format!("{error:#}")),
                    );
                    handle.mark_error(error.to_string());
                    cleanup_runtime.handle().spawn_blocking(move || {
                        let _ = std::fs::remove_file(&pending_marker);
                        let _ = std::fs::remove_file(&cache_path);
                    });
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

fn prepare_cached_or_streaming_input(
    song_cid: &str,
    source_url: &str,
) -> Result<(PathBuf, Option<(PathBuf, PathBuf)>, PlaybackInput)> {
    let cache_path = audio_cache::cached_song_path(song_cid, source_url)?;
    prepare_playback_input_from_cache_path(cache_path)
}

fn prepare_playback_input_from_cache_path(
    cache_path: PathBuf,
) -> Result<(PathBuf, Option<(PathBuf, PathBuf)>, PlaybackInput)> {
    let pending_marker = audio_cache::pending_marker_path(&cache_path);
    if audio_cache::is_song_cached(&cache_path) {
        return Ok((
            cache_path.clone(),
            None,
            PlaybackInput::cached_file(cache_path),
        ));
    }

    let _ = std::fs::remove_file(&cache_path);
    let _ = std::fs::remove_file(&pending_marker);
    std::fs::write(&pending_marker, b"pending")
        .with_context(|| format!("Failed to create cache marker {}", pending_marker.display()))?;

    let (handle, writer) = GrowingFileHandle::new(cache_path.clone())?;
    drop(writer);
    Ok((
        cache_path.clone(),
        Some((cache_path, pending_marker)),
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
    let message = format!("{error:#}");
    let lowered = message.to_ascii_lowercase();
    let code = if lowered.contains("playback request was superseded")
        || lowered.contains("playback stopped")
        || lowered.contains("playback session expired")
    {
        PlaybackErrorCode::Superseded
    } else if lowered.contains("failed to open")
        || lowered.contains("failed to create cache")
        || lowered.contains("failed to append")
        || lowered.contains("failed to flush")
        || lowered.contains("audio cache")
    {
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
        initial_buffer_samples, prepare_playback_input_from_cache_path, PlaybackStartIntent,
    };
    use crate::audio_cache;
    use crate::player::stream::{AudioFormat, PlaybackInput, SampleBuffer};
    use tempfile::tempdir;

    #[test]
    fn cached_playback_input_keeps_cache_path_for_failure_cleanup() {
        let temp_dir = tempdir().expect("tempdir");
        let cache_path = temp_dir.path().join("cached-song.wav");
        let pending_marker = audio_cache::pending_marker_path(&cache_path);
        std::fs::write(&cache_path, b"not a real wav").expect("cached file");

        let (failure_cache_path, pending_download, input) =
            prepare_playback_input_from_cache_path(cache_path.clone())
                .expect("prepared cached input");

        assert_eq!(failure_cache_path, cache_path);
        assert!(pending_download.is_none());
        assert!(matches!(input, PlaybackInput::CachedFile(path) if path == failure_cache_path));
        assert!(!pending_marker.exists());
    }

    #[test]
    fn streaming_playback_input_keeps_cache_path_for_failure_cleanup() {
        let temp_dir = tempdir().expect("tempdir");
        let cache_path = temp_dir.path().join("stream-song.wav");
        let pending_marker = audio_cache::pending_marker_path(&cache_path);

        let (failure_cache_path, pending_download, input) =
            prepare_playback_input_from_cache_path(cache_path.clone())
                .expect("prepared streaming input");

        assert_eq!(failure_cache_path, cache_path);
        assert_eq!(
            pending_download,
            Some((failure_cache_path.clone(), pending_marker.clone()))
        );
        assert!(matches!(input, PlaybackInput::GrowingFile(_)));
        assert!(pending_marker.exists());
        assert!(failure_cache_path.exists());
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
