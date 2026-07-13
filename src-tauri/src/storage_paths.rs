//! 应用存储路径边界 —— 持久数据 vs 可重建缓存的清晰分离。
//!
//! P1-7 目标：明确所有磁盘落地文件属于以下两类之一，避免混淆：
//!
//! ## 持久数据（app_data_dir）
//!
//! 内容损坏或删除 → 用户数据丢失；启动失败视为致命错误。
//! 需要版本、迁移、原子写入、损坏检测与恢复。
//!
//! - `preferences.toml` — 用户偏好
//! - `download_session.json` — 下载会话恢复
//! - `local_inventory_provenance.json` — 库存匹配证据
//! - `tag_registry.json` — 用户 tag overlay
//! - SQLite `harubble.db` — 收听历史、专辑元数据、合集
//! - `logs/persistent.jsonl` — 持久化日志
//!
//! ## 可重建缓存（app_cache_dir）
//!
//! 内容损坏或删除 → 只是丢失性能，下次启动会自动重建。
//! 允许在启动时主动清理；不需要迁移；损坏应静默恢复。
//!
//! - `library-search/` — Tantivy 搜索索引 + snapshot（可从 API + tag_registry 重建）
//! - `logs/session.jsonl` — 会话日志
//! - 音频缓存目录（由 audio_cache 模块管理）
//!
//! # 使用约束
//!
//! - 新增磁盘持久化路径时，必须显式决定属于哪一类
//! - 不要把可重建数据放到 app_data_dir，否则会误增加"用户数据损坏"的风险面
//! - 不要把不可重建数据放到 app_cache_dir，否则 OS 清理缓存时会丢失用户数据

use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// 返回持久数据根目录。
///
/// 存放此目录下的文件在应用未主动清理时应始终存在。
/// 目录不存在时自动创建。
pub fn app_data_root(app: &AppHandle) -> Result<PathBuf, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app_data_dir: {e}"))?;
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("failed to create {}: {e}", path.display()))?;
    Ok(path)
}

/// 返回可重建缓存根目录。
///
/// 系统或用户可能在任何时刻清理此目录；调用方必须能从磁盘外部数据源
/// （API、其他持久文件、内存状态）重建缓存内容。
/// 目录不存在时自动创建。
pub fn app_cache_root(app: &AppHandle) -> Result<PathBuf, String> {
    let path = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("failed to resolve app_cache_dir: {e}"))?;
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("failed to create {}: {e}", path.display()))?;
    Ok(path)
}

/// 若缓存目录初始化失败，尝试用备用位置（app_data_dir 下的 cache 子目录）。
///
/// 用于 app_cache_dir 出现权限或路径错误时的降级，避免 fatal 启动。
/// 备用位置在下次可用时不会自动清理，需要用户手动删除。
pub fn app_cache_root_with_fallback(app: &AppHandle) -> Result<PathBuf, String> {
    match app_cache_root(app) {
        Ok(path) => Ok(path),
        Err(_) => {
            let fallback = app_data_root(app)?.join("_cache_fallback");
            std::fs::create_dir_all(&fallback)
                .map_err(|e| format!("failed to create fallback cache dir: {e}"))?;
            Ok(fallback)
        }
    }
}

/// 返回搜索索引根目录（属于可重建缓存）。
pub fn search_index_root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_cache_root_with_fallback(app)?.join("library-search"))
}

/// 检查一个路径是否是相对于给定根目录内的（防目录遍历攻击场景使用）。
///
/// 简单地对比 canonicalize 前缀；根目录必须存在且可访问。
pub fn is_within(root: &Path, candidate: &Path) -> bool {
    match (root.canonicalize(), candidate.canonicalize()) {
        (Ok(r), Ok(c)) => c.starts_with(&r),
        _ => false,
    }
}
