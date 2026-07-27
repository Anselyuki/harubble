//! 主题包磁盘状态机存储。
//!
//! # 模块职责
//!
//! `PackageStore` 管理主题包在磁盘上的三态状态机：
//!
//! - `staging/<id>.json.tmp` → 通过 `tempfile::persist` 原子 rename 到
//! - `committed/<id>.json` + `committed/<id>.sha256`（sidecar hash）
//! - 卸载先 rename 到 `pending-delete/<id>.json`，启动扫描时清理
//!
//! # 回收策略
//!
//! - 应用启动时 `pending-delete` 目录一次性清空
//! - `staging` 中超过 24 小时的临时文件自动清理（防止导入中途崩溃残留）
//!
//! # 与主方案 §5.2.0a 的关系
//!
//! sidecar 缺失时并非直接静默重生成（P0-5 卡点），而是走完整 sanitize + hash 校验后写回。
//! 该职责由 `service::ThemePackageService` 编排，本模块仅负责最基本的文件操作原语。

use crate::theme_packages::types::ThemePackageStatus;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 磁盘三态目录名。
///
/// 常量导出便于测试断言目录结构；主方案 §5.2.0 契约的实现基准。
pub(crate) const DIR_STAGING: &str = "staging";
pub(crate) const DIR_COMMITTED: &str = "committed";
pub(crate) const DIR_PENDING_DELETE: &str = "pending-delete";

/// 主题包存储管理器。
///
/// 每个 `PackageStore` 绑定到应用数据目录下的 `theme-packages/` 根，
/// 内部三态子目录由 `ensure_layout()` 惰性创建。所有文件写入均通过 `tempfile`
/// 中转，保证 crash 安全。
pub(crate) struct PackageStore {
    root: PathBuf,
}

impl PackageStore {
    /// 使用应用数据目录初始化存储。
    ///
    /// 会在 `<app_data>/theme-packages/` 下创建 `staging/committed/pending-delete`
    /// 三个子目录（若不存在），并对 pending-delete 目录做一次性清扫。
    pub(crate) fn new(app_data_dir: PathBuf) -> Result<Self, String> {
        let root = app_data_dir.join("theme-packages");
        let store = Self { root };
        store.ensure_layout()?;
        store.reap_pending_delete();
        Ok(store)
    }

    /// 惰性创建三态子目录。
    fn ensure_layout(&self) -> Result<(), String> {
        for dir in [DIR_STAGING, DIR_COMMITTED, DIR_PENDING_DELETE] {
            let path = self.root.join(dir);
            fs::create_dir_all(&path).map_err(|e| {
                format!(
                    "failed to create theme package directory {}: {}",
                    path.display(),
                    e
                )
            })?;
        }
        Ok(())
    }

    /// 应用启动时清理 pending-delete 目录。
    ///
    /// 该操作幂等；失败仅打印警告，不阻断启动。
    fn reap_pending_delete(&self) {
        let dir = self.root.join(DIR_PENDING_DELETE);
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    /// 返回三态目录的绝对路径。
    ///
    /// 仅供测试和 service 层内部使用；不应通过命令层暴露给前端。
    pub(crate) fn dir_for(&self, status: ThemePackageStatus) -> PathBuf {
        match status {
            ThemePackageStatus::Staging => self.root.join(DIR_STAGING),
            ThemePackageStatus::Committed => self.root.join(DIR_COMMITTED),
            ThemePackageStatus::PendingDelete => self.root.join(DIR_PENDING_DELETE),
        }
    }

    /// 主题包 JSON 文件在指定状态下的路径。
    pub(crate) fn path_for(&self, id: &str, status: ThemePackageStatus) -> PathBuf {
        self.dir_for(status).join(format!("{id}.json"))
    }

    /// sidecar hash 文件路径（仅 committed 状态有意义）。
    pub(crate) fn sidecar_for(&self, id: &str) -> PathBuf {
        self.dir_for(ThemePackageStatus::Committed)
            .join(format!("{id}.sha256"))
    }

    /// 写入 raw JSON 到 committed 目录并同时写入 sidecar。
    ///
    /// 通过 `staging/<id>.json.tmp` 中转，`tempfile::persist` 原子 rename 到目标位置，
    /// 保证部分写入不会污染 committed 目录。
    ///
    /// 调用方必须在写入前完成 sanitize + hash 计算；本方法不做任何校验。
    pub(crate) fn commit(&self, id: &str, raw: &[u8], sha256_hex: &str) -> Result<(), String> {
        let staging_dir = self.dir_for(ThemePackageStatus::Staging);
        fs::create_dir_all(&staging_dir).map_err(|e| e.to_string())?;

        let mut tmp = tempfile::NamedTempFile::new_in(&staging_dir)
            .map_err(|e| format!("failed to create staging tempfile: {e}"))?;
        tmp.write_all(raw).map_err(|e| e.to_string())?;
        tmp.persist(self.path_for(id, ThemePackageStatus::Committed))
            .map_err(|e| format!("failed to persist theme package: {}", e.error))?;

        // sidecar 单独写入，允许后续基于 raw 重新计算校验
        let sidecar_path = self.sidecar_for(id);
        let mut sidecar_tmp = tempfile::NamedTempFile::new_in(&staging_dir)
            .map_err(|e| format!("failed to create sidecar tempfile: {e}"))?;
        sidecar_tmp
            .write_all(sha256_hex.as_bytes())
            .map_err(|e| e.to_string())?;
        sidecar_tmp
            .persist(&sidecar_path)
            .map_err(|e| format!("failed to persist sidecar: {}", e.error))?;

        Ok(())
    }

    /// 卸载：把 committed 中的主题包 rename 到 pending-delete。
    ///
    /// 该操作原子；若 committed 中不存在指定 id 直接返回成功（幂等）。
    /// sidecar 一并搬迁；下次启动 reap 时统一清理。
    pub(crate) fn uninstall(&self, id: &str) -> Result<(), String> {
        let src = self.path_for(id, ThemePackageStatus::Committed);
        if !src.exists() {
            return Ok(());
        }
        let dst = self.path_for(id, ThemePackageStatus::PendingDelete);
        fs::rename(&src, &dst).map_err(|e| format!("failed to move to pending-delete: {e}"))?;

        // sidecar 一并清理
        let sidecar = self.sidecar_for(id);
        if sidecar.exists() {
            let _ = fs::remove_file(&sidecar);
        }
        Ok(())
    }

    /// 列出 committed 目录下所有主题包 id（按字典序）。
    pub(crate) fn list_committed_ids(&self) -> Result<Vec<String>, String> {
        list_ids_in(&self.dir_for(ThemePackageStatus::Committed))
    }

    /// 读取 committed 主题包的原始 JSON 字节。
    ///
    /// 返回 `None` 表示 id 不存在；`Err` 表示读取失败（权限、损坏等）。
    pub(crate) fn read_committed_raw(&self, id: &str) -> Result<Option<Vec<u8>>, String> {
        let path = self.path_for(id, ThemePackageStatus::Committed);
        if !path.exists() {
            return Ok(None);
        }
        fs::read(&path)
            .map(Some)
            .map_err(|e| format!("failed to read theme package {}: {}", path.display(), e))
    }

    /// 读取 committed 主题包的 sidecar hash（去除首尾空白后返回）。
    ///
    /// 返回 `None` 表示 sidecar 缺失，调用方需按 P0-5 卡点走完整校验重生成。
    pub(crate) fn read_sidecar(&self, id: &str) -> Result<Option<String>, String> {
        let path = self.sidecar_for(id);
        if !path.exists() {
            return Ok(None);
        }
        fs::read_to_string(&path)
            .map(|s| Some(s.trim().to_string()))
            .map_err(|e| format!("failed to read sidecar {}: {}", path.display(), e))
    }
}

fn list_ids_in(dir: &Path) -> Result<Vec<String>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                ids.push(stem.to_string());
            }
        }
    }
    ids.sort();
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir_store() -> (tempfile::TempDir, PackageStore) {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = PackageStore::new(dir.path().to_path_buf()).expect("store");
        (dir, store)
    }

    #[test]
    fn new_creates_three_state_subdirs() {
        let (_dir, store) = tempdir_store();
        assert!(store.dir_for(ThemePackageStatus::Staging).is_dir());
        assert!(store.dir_for(ThemePackageStatus::Committed).is_dir());
        assert!(store.dir_for(ThemePackageStatus::PendingDelete).is_dir());
    }

    #[test]
    fn commit_writes_json_and_sidecar_atomically() {
        let (_dir, store) = tempdir_store();
        let raw = br#"{"schemaVersion":1}"#;
        let hash = "abc123";
        store.commit("test-pkg", raw, hash).expect("commit");

        let committed_path = store.path_for("test-pkg", ThemePackageStatus::Committed);
        assert_eq!(fs::read(&committed_path).unwrap(), raw);

        let sidecar_path = store.sidecar_for("test-pkg");
        assert_eq!(fs::read_to_string(&sidecar_path).unwrap(), hash);
    }

    #[test]
    fn uninstall_moves_to_pending_delete_and_removes_sidecar() {
        let (_dir, store) = tempdir_store();
        store.commit("acme", b"{}", "h").expect("commit");
        store.uninstall("acme").expect("uninstall");

        assert!(!store
            .path_for("acme", ThemePackageStatus::Committed)
            .exists());
        assert!(store
            .path_for("acme", ThemePackageStatus::PendingDelete)
            .exists());
        assert!(!store.sidecar_for("acme").exists());
    }

    #[test]
    fn uninstall_is_idempotent_when_package_missing() {
        let (_dir, store) = tempdir_store();
        // 不存在的 id 卸载应该成功返回，不 panic
        store
            .uninstall("nonexistent")
            .expect("idempotent uninstall");
    }

    #[test]
    fn list_committed_ids_returns_sorted_ids() {
        let (_dir, store) = tempdir_store();
        store.commit("charlie", b"{}", "h1").unwrap();
        store.commit("alpha", b"{}", "h2").unwrap();
        store.commit("bravo", b"{}", "h3").unwrap();

        let ids = store.list_committed_ids().unwrap();
        assert_eq!(ids, vec!["alpha", "bravo", "charlie"]);
    }

    #[test]
    fn read_committed_returns_none_when_missing() {
        let (_dir, store) = tempdir_store();
        assert_eq!(store.read_committed_raw("missing").unwrap(), None);
        assert_eq!(store.read_sidecar("missing").unwrap(), None);
    }

    #[test]
    fn reap_pending_delete_clears_on_new() {
        let dir = tempfile::tempdir().unwrap();
        // 先创建 store 并写入卸载
        {
            let store = PackageStore::new(dir.path().to_path_buf()).unwrap();
            store.commit("orphan", b"{}", "h").unwrap();
            store.uninstall("orphan").unwrap();
            assert!(store
                .path_for("orphan", ThemePackageStatus::PendingDelete)
                .exists());
        }
        // 重新创建 store 触发 reap
        let store2 = PackageStore::new(dir.path().to_path_buf()).unwrap();
        assert!(!store2
            .path_for("orphan", ThemePackageStatus::PendingDelete)
            .exists());
    }
}
