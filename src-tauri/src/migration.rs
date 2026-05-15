use std::fs;
use std::path::Path;

const OLD_CACHE_DIR_NAME: &str = "siren-music-download";
const OLD_DB_NAME: &str = "siren_local.db";
const NEW_DB_NAME: &str = "harubble_local.db";

/// 从旧版本数据路径迁移到新路径（一次性，幂等）。
pub fn migrate_legacy_data(app_data_dir: &Path) {
    migrate_legacy_app_data(app_data_dir);
    migrate_legacy_audio_cache();
}

fn migrate_legacy_app_data(app_data_dir: &Path) {
    let old_app_data_dir = app_data_dir
        .parent()
        .map(|p| p.join("com.siren.music-download"));

    if let Some(old_dir) = old_app_data_dir {
        if old_dir.is_dir() && !app_data_dir.exists() && fs::rename(&old_dir, app_data_dir).is_ok()
        {
            rename_db_in_dir(app_data_dir);
            return;
        }
    }

    rename_db_in_dir(app_data_dir);
}

fn rename_db_in_dir(dir: &Path) {
    let old_db = dir.join(OLD_DB_NAME);
    let new_db = dir.join(NEW_DB_NAME);
    if old_db.exists() && !new_db.exists() {
        let _ = fs::rename(&old_db, &new_db);
    }
}

fn migrate_legacy_audio_cache() {
    let cache_base = match dirs::cache_dir() {
        Some(d) => d,
        None => return,
    };
    let old_cache = cache_base.join(OLD_CACHE_DIR_NAME);
    let new_cache = cache_base.join("harubble");
    if old_cache.is_dir() && !new_cache.exists() {
        let _ = fs::rename(&old_cache, &new_cache);
    }
}
