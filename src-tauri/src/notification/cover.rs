//! 通知封面图的共享磁盘缓存接入。

use crate::app_state::AppState;
use crate::{image_cache, storage_paths};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub async fn get_cached_path(app: &AppHandle, cover_url: &str) -> Option<PathBuf> {
    let cache_dir = storage_paths::image_cache_root(app).ok()?;
    let api = {
        let state = app.state::<AppState>();
        state.image_api_client().clone()
    };
    image_cache::get_or_download(cache_dir, cover_url.to_string(), api)
        .await
        .ok()
}
