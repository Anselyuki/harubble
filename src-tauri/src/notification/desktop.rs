//! 基于 Tauri 插件的跨平台系统通知实现。

use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

pub fn show_playback(
    app: &AppHandle,
    title: &str,
    body: &str,
    cover_path: Option<&PathBuf>,
) -> Result<(), String> {
    let mut builder = app.notification().builder().title(title).body(body);

    if let Some(path) = cover_path.and_then(|path| path.to_str()) {
        builder = builder.icon(path);
    }

    builder.show().map_err(|error| error.to_string())?;

    Ok(())
}

pub fn show_download(app: &AppHandle, title: &str, body: &str) -> Result<(), String> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|error| error.to_string())?;

    Ok(())
}

pub fn show_test(app: &AppHandle, title: &str, body: &str) -> Result<(), String> {
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|error| error.to_string())?;

    Ok(())
}
