//! 通知封面图的下载与临时缓存逻辑。

use crate::app_state::AppState;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Manager};

const MAX_CACHE_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const MAX_CACHED_FILES: usize = 128;

pub fn temp_dir() -> PathBuf {
    std::env::temp_dir().join("harubble-covers")
}

fn file_extension(cover_url: &str) -> &'static str {
    let path = cover_url.split('?').next().unwrap_or(cover_url);
    if path.ends_with(".png") {
        "png"
    } else if path.ends_with(".webp") {
        "webp"
    } else if path.ends_with(".gif") {
        "gif"
    } else {
        "jpg"
    }
}

fn cleanup_cache(dir: &std::path::Path) {
    let now = SystemTime::now();
    let mut retained = Vec::new();

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }

        let Ok(modified_at) = metadata.modified() else {
            continue;
        };

        let is_expired = now
            .duration_since(modified_at)
            .map(|age| age > MAX_CACHE_AGE)
            .unwrap_or(false);
        if is_expired {
            let _ = std::fs::remove_file(&path);
            continue;
        }

        retained.push((path, modified_at));
    }

    if retained.len() <= MAX_CACHED_FILES {
        return;
    }

    retained.sort_by_key(|(_, modified_at)| *modified_at);
    let files_to_remove = retained.len() - MAX_CACHED_FILES;
    for (path, _) in retained.into_iter().take(files_to_remove) {
        let _ = std::fs::remove_file(path);
    }
}

pub async fn download_to_temp(app: &AppHandle, cover_url: &str) -> Option<PathBuf> {
    let temp_dir = temp_dir();
    tokio::fs::create_dir_all(&temp_dir).await.ok()?;

    let cleanup_dir = temp_dir.clone();
    tauri::async_runtime::spawn(async move {
        let _ = tokio::task::spawn_blocking(move || cleanup_cache(&cleanup_dir)).await;
    });

    let url_hash = format!("{:x}", md5::compute(cover_url.as_bytes()));
    let temp_path = temp_dir.join(format!("{}.{}", url_hash, file_extension(cover_url)));

    if tokio::fs::try_exists(&temp_path).await.ok()? {
        return Some(temp_path);
    }

    let api = {
        let state = app.state::<AppState>();
        state.image_api_client().clone()
    };

    let bytes = api.download_bytes(cover_url, |_, _| {}).await.ok()?;
    tokio::fs::write(&temp_path, &bytes).await.ok()?;
    Some(temp_path)
}
