//! 下载完成与播放状态切换的系统通知集成。

mod cover;
mod desktop;

use crate::app_state::AppState;
use crate::i18n::{fluent_args, tr, tr_args};
use crate::logging::{LogLevel, LogPayload};
use crate::player::state::PlayerState;
use harubble_core::download::model::{DownloadJobKind, DownloadJobSnapshot, DownloadJobStatus};
use std::collections::HashSet;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

/// 记录最近一次已发送播放通知的歌曲 CID，用于避免同一首歌重复通知。
static LAST_NOTIFIED_SONG: Mutex<Option<String>> = Mutex::new(None);

fn selection_notification_title(
    job: &DownloadJobSnapshot,
    locale: crate::preferences::Locale,
) -> String {
    let album_count = job
        .tasks
        .iter()
        .map(|t| t.album_cid.as_str())
        .collect::<HashSet<_>>()
        .len();
    let count = job.task_count;
    if album_count > 1 {
        let args = fluent_args!(
            "count" => count as i64,
            "albumCount" => album_count as i64
        );
        tr_args(locale, "notification-selection-title-cross-albums", &args)
    } else {
        let args = fluent_args!("count" => count as i64);
        tr_args(locale, "notification-selection-title", &args)
    }
}

/// 当下载批次进入终态时触发系统通知。
///
/// 只有在用户偏好开启下载完成通知，且批次状态为 `Completed` 或
/// `PartiallyFailed` 时才会真正发送；通知文案会根据批次类型与完成结果自动调整。
pub fn notify_download_completed(app: &AppHandle, job: &DownloadJobSnapshot) {
    let state = app.state::<AppState>();
    let prefs = state.preferences();
    if !prefs.notify_on_download_complete {
        return;
    }

    if !matches!(
        job.status,
        DownloadJobStatus::Completed | DownloadJobStatus::PartiallyFailed
    ) {
        return;
    }

    let locale = prefs.locale;

    let (title, body) = match job.kind {
        DownloadJobKind::Song => (
            job.title.clone(),
            tr(locale, "notification-download-completed"),
        ),
        DownloadJobKind::Album | DownloadJobKind::Selection => {
            let title = if job.kind == DownloadJobKind::Selection {
                selection_notification_title(job, locale)
            } else {
                job.title.clone()
            };
            let body = if job.status == DownloadJobStatus::PartiallyFailed {
                let args = fluent_args!(
                    "completed" => job.completed_task_count as i64,
                    "failed" => job.failed_task_count as i64
                );
                tr_args(locale, "notification-album-partial", &args)
            } else {
                let args = fluent_args!("count" => job.completed_task_count as i64);
                tr_args(locale, "notification-album-completed", &args)
            };
            (title, body)
        }
    };

    if let Err(error) = desktop::show_download(app, &title, &body) {
        state.log_center().record(
            LogPayload::new(
                LogLevel::Warn,
                "notification",
                "notification.download_delivery_failed",
                "Failed to show download notification",
            )
            .details(error.clone()),
        );
    }
}

/// 当播放切换到新歌曲时触发系统通知。
///
/// 通知标题为歌曲名、正文为艺术家列表，并会按"最近一次已通知的歌曲 CID"做去重，
/// 以避免同一首歌在连续播放状态更新中被重复通知。
pub fn notify_playback_changed(app: &AppHandle, player_state: &PlayerState) {
    let app_state = app.state::<AppState>();
    let prefs = app_state.preferences();
    if !prefs.notify_on_playback_change {
        return;
    }

    if !player_state.is_playing {
        return;
    }

    let Some(ref song_cid) = player_state.song_cid else {
        return;
    };

    let last_notified = LAST_NOTIFIED_SONG.lock().unwrap();
    if last_notified.as_ref() == Some(song_cid) {
        return;
    }
    drop(last_notified);

    let title = player_state.song_name.clone().unwrap_or_default();
    let body = if player_state.artists.is_empty() {
        String::new()
    } else {
        player_state.artists.join(", ")
    };
    let cover_url = player_state.cover_url.clone();

    let app_for_task = app.clone();
    let song_cid_for_task = song_cid.clone();

    tauri::async_runtime::spawn(async move {
        let cover_path = if let Some(ref url) = cover_url {
            cover::download_to_temp(&app_for_task, url).await
        } else {
            None
        };

        let Some(state) = app_for_task.try_state::<AppState>() else {
            return;
        };
        let current_state = state.player().get_state();
        if current_state.song_cid.as_deref() != Some(song_cid_for_task.as_str())
            || !current_state.is_playing
        {
            return;
        }

        if let Err(error) =
            desktop::show_playback(&app_for_task, &title, &body, cover_path.as_ref())
        {
            state.log_center().record(
                LogPayload::new(
                    LogLevel::Warn,
                    "notification",
                    "notification.playback_delivery_failed",
                    "Failed to show playback notification",
                )
                .details(error.clone()),
            );
        }

        if let Ok(mut last) = LAST_NOTIFIED_SONG.lock() {
            *last = Some(song_cid_for_task);
        }
    });
}

/// 发送一条测试通知，用于验证通知链路是否可用。
pub fn notify_test(app: AppHandle) -> Result<(), String> {
    let locale = app
        .try_state::<AppState>()
        .map(|s| s.preferences().locale)
        .unwrap_or_default();
    let title = tr(locale, "notification-test-title");
    let body = tr(locale, "notification-test-body");

    let result = desktop::show_test(&app, &title, &body);
    if let Err(error) = &result {
        if let Some(state) = app.try_state::<AppState>() {
            state.log_center().record(
                LogPayload::new(
                    LogLevel::Warn,
                    "notification",
                    "notification.test_delivery_failed",
                    "Failed to show test notification",
                )
                .details(error.clone()),
            );
        }
    }
    result
}
