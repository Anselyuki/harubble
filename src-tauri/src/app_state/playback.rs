use crate::audio_cache;
use crate::i18n;
use crate::logging::{LogLevel, LogPayload};
use crate::player::stream::{GrowingFileHandle, PlaybackInput, SampleBuffer};
use crate::player::PlaybackContext;
use crate::player::PlaybackQueueEntry;
use crate::player::{PlaybackError, PlaybackErrorCode, PlaybackStartResult};
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

use super::{normalize_seek_position, AppState, PreparedPlaybackInput};

impl AppState {
    pub fn toggle_playback_from_lifecycle(&self) -> Result<(), String> {
        self.player
            .toggle_playback()
            .map_err(|error| format!("{error:#}"))
    }

    pub(crate) async fn play_song_internal(
        &self,
        song_cid: String,
        cover_url: Option<String>,
        playback_context: Option<PlaybackContext>,
    ) -> Result<PlaybackStartResult, PlaybackError> {
        let request_id = self.player.begin_playback_request();
        let song_detail = self.api.get_song_detail(&song_cid).await.map_err(|error| {
            playback_error(PlaybackErrorCode::Network, error.to_string(), true, None)
        })?;
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
            self.start_playback_session(session_id, &song_cid, &song_detail.source_url, 0.0)
                .await
        }
        .await;

        match result {
            Ok(duration) => {
                let listening_event = harubble_core::ListeningEvent {
                    song_cid: song_cid.clone(),
                    song_name: song_detail.name.clone(),
                    album_cid: song_detail.album_cid.clone(),
                    album_name: String::new(),
                    cover_url: cover_url.clone(),
                    artists: song_detail.artists.clone(),
                };
                let listening_history = self.listening_history.clone();
                let record_result =
                    tokio::task::spawn_blocking(move || listening_history.record(&listening_event))
                        .await
                        .map_err(|error| error.to_string())
                        .and_then(|result| result);
                if let Err(e) = record_result {
                    self.log_center.record(
                        LogPayload::new(
                            LogLevel::Warn,
                            "listening-history",
                            "listening_history.record_failed",
                            "Failed to record listening history",
                        )
                        .details(e),
                    );
                }
                Ok(PlaybackStartResult::new(duration, session_id))
            }
            Err(error) => {
                self.player.fail_session(session_id);
                Err(classify_playback_error(error, Some(session_id)))
            }
        }
    }

    pub(crate) async fn seek_current_internal(
        &self,
        position_secs: f64,
    ) -> Result<PlaybackStartResult, PlaybackError> {
        let request_id = self.player.begin_playback_request();
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

        let song_detail = self.api.get_song_detail(&song_cid).await.map_err(|error| {
            playback_error(
                PlaybackErrorCode::Network,
                error.to_string(),
                true,
                Some(current_state.session_id),
            )
        })?;
        self.ensure_playback_request_active(request_id, Some(current_state.session_id))?;

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
                )
                .await?;

            if should_pause_after_seek {
                self.player.pause()?;
            }

            Ok(duration)
        }
        .await;

        match result {
            Ok(duration) => Ok(PlaybackStartResult::new(duration, session_id)),
            Err(error) => {
                self.player.fail_session(session_id);
                Err(classify_playback_error(error, Some(session_id)))
            }
        }
    }

    async fn start_playback_session(
        &self,
        session_id: u64,
        song_cid: &str,
        source_url: &str,
        start_position_secs: f64,
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
        let source_format = match tokio::task::spawn_blocking(move || {
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
        let start_position_secs =
            normalize_seek_position(start_position_secs, source_format.duration_secs);
        let sample_buffer = SampleBuffer::new();

        let log_center = Arc::clone(&self.log_center);
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
        });

        let _decode_worker = match input.spawn_decode_worker(
            source_format,
            output_format,
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

        if let Err(error) = self
            .wait_for_initial_buffer(&sample_buffer, output_format, &stop_flag)
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

    pub(crate) async fn play_next_internal(&self) -> Result<PlaybackStartResult, PlaybackError> {
        let target = self.player.select_next_entry().ok_or_else(|| {
            playback_error(
                PlaybackErrorCode::NoNextTrack,
                i18n::tr(self.preferences().locale, "player-no-next-track"),
                false,
                Some(self.player.get_state().session_id),
            )
        })?;
        self.play_song_internal(target.cid, target.cover_url, None)
            .await
    }

    pub(crate) async fn play_previous_internal(
        &self,
    ) -> Result<PlaybackStartResult, PlaybackError> {
        let target = self.player.select_previous_entry().ok_or_else(|| {
            playback_error(
                PlaybackErrorCode::NoPreviousTrack,
                i18n::tr(self.preferences().locale, "player-no-previous-track"),
                false,
                Some(self.player.get_state().session_id),
            )
        })?;
        self.play_song_internal(target.cid, target.cover_url, None)
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
            tokio::task::spawn_blocking(move || {
                prepare_cached_or_streaming_input(&song_cid, &source_url)
            })
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
        tokio::task::spawn_blocking(move || {
            let _ = std::fs::remove_file(&pending_marker);
            let _ = std::fs::remove_file(&cache_path);
        });
    }

    fn spawn_stream_download(
        &self,
        source_url: String,
        stop_flag: Arc<std::sync::atomic::AtomicBool>,
        handle: GrowingFileHandle,
        cache_path: PathBuf,
        pending_marker: PathBuf,
    ) {
        let api = Arc::clone(&self.api);
        let log_center = Arc::clone(&self.log_center);

        tokio::spawn(async move {
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
                    tokio::task::spawn_blocking(move || {
                        let _ = std::fs::remove_file(&pending_marker);
                        audio_cache::spawn_cleanup_if_needed();
                    });
                }
                (Ok(()), Ok(())) => {
                    handle.mark_error("Playback stopped");
                    tokio::task::spawn_blocking(move || {
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
                    tokio::task::spawn_blocking(move || {
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
        output_format: crate::player::stream::AudioFormat,
        stop_flag: &Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<()> {
        let minimum_samples =
            ((output_format.sample_rate as usize * output_format.channels as usize) / 3)
                .max(output_format.channels as usize * 4096)
                .min(output_format.channels as usize * 32_768);

        let wait_buffer = sample_buffer.clone();
        let wait_stop = Arc::clone(stop_flag);
        tokio::task::spawn_blocking(move || {
            wait_buffer.wait_for_samples(minimum_samples, &wait_stop)
        })
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))??;
        Ok(())
    }

    fn start_prepared_playback(
        &self,
        session_id: u64,
        output_format: crate::player::stream::AudioFormat,
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
    use super::prepare_playback_input_from_cache_path;
    use crate::audio_cache;
    use crate::player::stream::PlaybackInput;
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
}
