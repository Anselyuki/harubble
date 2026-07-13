//! Window lifecycle commands shared by secondary UI surfaces.

use crate::desktop_lifecycle;
use tauri::AppHandle;

/// 窗口操作错误类型。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "code", content = "detail")]
pub enum WindowError {
    Internal(String),
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowError::Internal(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for WindowError {}

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), WindowError> {
    desktop_lifecycle::restore_main_window(&app).map_err(|e| WindowError::Internal(e.to_string()))
}
