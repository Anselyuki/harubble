use crate::audio_cache;
use crate::i18n;
use crate::logging::{LogLevel, LogPayload};
use crate::player::stream::{GrowingFileHandle, PlaybackInput, SampleBuffer};
use crate::player::PlaybackContext;
use crate::player::PlaybackQueueEntry;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::{normalize_seek_position, AppState, PreparedPlaybackInput};

impl AppState {
    pub(crate) async fn play_song_internal(
        &self,
        song_cid: String,
        cover_url: Option<String>,
        playback_context: Option<PlaybackContext>,
    ) -> Result<f64, String> {
        let song_detail = self
            .api
            .get_song_detail(&song_cid)
            .await
            .map_err(|e| e.to_string())?;

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
            .map_err(|e| e.to_string())?;

        let result: Result<f64> = async {
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
                if let Err(e) = self.listening_history.record(&listening_event) {
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
                Ok(duration)
            }
            Err(error) => {
                self.player.fail_session(session_id);
                Err(error.to_string())
            }
        }
    }

    pub(crate) async fn seek_current_internal(&self, position_secs: f64) -> Result<f64, String> {
        let current_state = self.player.get_state();
        let song_cid = current_state
            .song_cid
            .clone()
            .ok_or_else(|| i18n::tr(self.preferences().locale, "player-no-active-track"))?;

        if current_state.is_loading {
            return Err(i18n::tr(self.preferences().locale, "player-still-loading"));
        }

        let target_position = normalize_seek_position(position_secs, current_state.duration);
        if (current_state.progress - target_position).abs() < 0.05 {
            return Ok(current_state.duration);
        }

        let song_detail = self
            .api
            .get_song_detail(&song_cid)
            .await
            .map_err(|e| e.to_string())?;

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
            .map_err(|e| e.to_string())?;

        let should_pause_after_seek = current_state.is_paused;
        let result: Result<f64> = async {
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
            Ok(duration) => Ok(duration),
            Err(error) => {
                self.player.fail_session(session_id);
                Err(error.to_string())
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
        let prepared_input = self.prepare_playback_input(song_cid, source_url, &stop_flag)?;
        let input = prepared_input.input.clone();

        let inspect_input = input.clone();
        let source_format = tokio::task::spawn_blocking(move || inspect_input.inspect_format())
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))??;

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

        let _decode_worker = input.spawn_decode_worker(
            source_format,
            output_format,
            sample_buffer.clone(),
            Arc::clone(&stop_flag),
            Arc::clone(&pause_flag),
            start_position_secs,
            error_handler,
        )?;

        self.wait_for_initial_buffer(&sample_buffer, output_format, &stop_flag)
            .await?;

        self.start_prepared_playback(
            session_id,
            output_format,
            sample_buffer,
            start_position_secs,
        )
    }

    pub(crate) async fn play_next_internal(&self) -> Result<f64, String> {
        let target = self
            .player
            .select_next_entry()
            .ok_or_else(|| i18n::tr(self.preferences().locale, "player-no-next-track"))?;
        self.play_song_internal(target.cid, target.cover_url, None)
            .await
    }

    pub(crate) async fn play_previous_internal(&self) -> Result<f64, String> {
        let target = self
            .player
            .select_previous_entry()
            .ok_or_else(|| i18n::tr(self.preferences().locale, "player-no-previous-track"))?;
        self.play_song_internal(target.cid, target.cover_url, None)
            .await
    }

    fn prepare_playback_input(
        &self,
        song_cid: &str,
        source_url: &str,
        stop_flag: &Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<PreparedPlaybackInput> {
        let cache_path = audio_cache::cached_song_path(song_cid, source_url)?;
        let pending_marker = audio_cache::pending_marker_path(&cache_path);

        let input = if audio_cache::is_song_cached(&cache_path) {
            PlaybackInput::cached_file(cache_path.clone())
        } else {
            let _ = std::fs::remove_file(&cache_path);
            let _ = std::fs::remove_file(&pending_marker);
            std::fs::write(&pending_marker, b"pending").with_context(|| {
                format!("Failed to create cache marker {}", pending_marker.display())
            })?;

            let (handle, writer) = GrowingFileHandle::new(cache_path.clone())?;
            self.spawn_stream_download(
                source_url.to_string(),
                Arc::clone(stop_flag),
                handle.clone(),
                writer,
                cache_path.clone(),
                pending_marker.clone(),
            );
            PlaybackInput::growing_file(handle)
        };

        Ok(PreparedPlaybackInput { input })
    }

    fn spawn_stream_download(
        &self,
        source_url: String,
        stop_flag: Arc<std::sync::atomic::AtomicBool>,
        handle: GrowingFileHandle,
        mut writer: std::fs::File,
        cache_path: PathBuf,
        pending_marker: PathBuf,
    ) {
        let api = Arc::clone(&self.api);
        let log_center = Arc::clone(&self.log_center);

        tokio::spawn(async move {
            let total_len_set = std::sync::atomic::AtomicBool::new(false);
            let download_result = api
                .download_stream(&source_url, |chunk, _, total| {
                    if stop_flag.load(Ordering::SeqCst) {
                        return Ok(false);
                    }
                    if !total_len_set.load(Ordering::Relaxed) {
                        if let Some(total) = total {
                            handle.set_expected_total_len(total);
                            total_len_set.store(true, Ordering::Relaxed);
                        }
                    }
                    handle.append_chunk(&mut writer, chunk)?;
                    Ok(true)
                })
                .await;

            match download_result {
                Ok(()) if !stop_flag.load(Ordering::SeqCst) => {
                    handle.mark_complete();
                    let _ = std::fs::remove_file(&pending_marker);
                    audio_cache::spawn_cleanup_if_needed();
                }
                Ok(()) => {
                    handle.mark_error("Playback stopped");
                    let _ = std::fs::remove_file(&pending_marker);
                    let _ = std::fs::remove_file(&cache_path);
                }
                Err(error) => {
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
                    let _ = std::fs::remove_file(&pending_marker);
                    let _ = std::fs::remove_file(&cache_path);
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
