use crate::logging::{LogLevel, LogPayload};
use souvlaki::{MediaControlEvent, SeekDirection};

use super::AppState;

impl AppState {
    /// 绑定系统媒体控制事件。
    ///
    /// 适用于应用启动后启用系统级播放/暂停/切歌控制的场景。
    /// 成功时返回空值。
    /// 该接口通常只需要在启动阶段绑定一次；重复绑定可能导致媒体事件处理关系变得难以推断。
    pub fn bind_media_controls(&self) -> Result<(), String> {
        let media_state = self.clone();
        self.player
            .bind_media_controls(move |event| media_state.handle_media_control(event))
            .map_err(|error| error.to_string())
    }

    fn handle_media_control(&self, event: MediaControlEvent) {
        match event {
            MediaControlEvent::Play => self.handle_media_play(),
            MediaControlEvent::Pause => self.handle_media_pause(),
            MediaControlEvent::Toggle => self.handle_media_toggle(),
            MediaControlEvent::Stop | MediaControlEvent::Quit => self.handle_media_stop_or_quit(),
            MediaControlEvent::Next => self.handle_media_next(),
            MediaControlEvent::Previous => self.handle_media_previous(),
            MediaControlEvent::SetPosition(position) => {
                self.handle_media_set_position(position.0.as_secs_f64())
            }
            MediaControlEvent::SeekBy(direction, delta) => {
                self.handle_media_seek_by(direction, delta.as_secs_f64())
            }
            MediaControlEvent::Seek(direction) => self.handle_media_seek(direction),
            _ => {}
        }
    }

    fn handle_media_play(&self) {
        if let Err(error) = self.player.resume() {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Warn,
                    "media-session",
                    "media_session.resume_failed",
                    "Failed to resume playback",
                )
                .details(format!("{error:#}")),
            );
        }
    }

    fn handle_media_pause(&self) {
        if let Err(error) = self.player.pause() {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Warn,
                    "media-session",
                    "media_session.pause_failed",
                    "Failed to pause playback",
                )
                .details(format!("{error:#}")),
            );
        }
    }

    fn handle_media_toggle(&self) {
        if let Err(error) = self.player.toggle_playback() {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Warn,
                    "media-session",
                    "media_session.toggle_failed",
                    "Failed to toggle playback",
                )
                .details(format!("{error:#}")),
            );
        }
    }

    fn handle_media_stop_or_quit(&self) {
        if let Err(error) = self.player.stop() {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Warn,
                    "media-session",
                    "media_session.stop_failed",
                    "Failed to stop playback",
                )
                .details(format!("{error:#}")),
            );
        }
    }

    fn handle_media_next(&self) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = state.play_next_internal().await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.next_track_failed",
                        "Failed to play next track",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn handle_media_previous(&self) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = state.play_previous_internal().await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.previous_track_failed",
                        "Failed to play previous track",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn handle_media_set_position(&self, position_secs: f64) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = state.seek_current_internal(position_secs).await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.seek_failed",
                        "Failed to seek playback",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn handle_media_seek_by(&self, direction: SeekDirection, delta_secs: f64) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            let current = state.player.get_state();
            let target = match direction {
                SeekDirection::Forward => current.progress + delta_secs,
                SeekDirection::Backward => current.progress - delta_secs,
            };
            if let Err(error) = state.seek_current_internal(target).await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.seek_by_delta_failed",
                        "Failed to seek by delta",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn handle_media_seek(&self, direction: SeekDirection) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            let current = state.player.get_state();
            let target = match direction {
                SeekDirection::Forward => current.progress + 10.0,
                SeekDirection::Backward => current.progress - 10.0,
            };
            if let Err(error) = state.seek_current_internal(target).await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.seek_forward_failed",
                        "Failed to seek forward/backward",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }
}
