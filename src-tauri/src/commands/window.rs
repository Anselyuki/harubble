//! Window lifecycle commands shared by secondary UI surfaces.

use crate::desktop_lifecycle;
use tauri::AppHandle;

#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    desktop_lifecycle::restore_main_window(&app).map_err(|error| error.to_string())
}
