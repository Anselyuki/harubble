//! 主题包子系统组合根。
//!
//! # 模块职责
//!
//! `ThemePackageService` 是主题包子系统对外暴露的唯一入口，聚合 `PackageStore`（磁盘状态机）
//! 与内部 sanitize / hash 逻辑。Tauri command 层只应通过本类型访问主题包能力，
//! 不允许直接持有 `PackageStore` 或原始文件路径。
//!
//! # 生命周期
//!
//! 由 `AppState::new` 在启动时初始化一次并 `Arc` 包装后长期持有；跨命令共享。
//! 内部 `PackageStore` 已在 `new` 时完成 pending-delete 清扫和三态目录创建。
//!
//! # Phase 1 MVP 边界
//!
//! 当前提供的公开方法覆盖 list / install / uninstall / read 主链路。
//! URL 抓取（`install_from_url`）、preview / dismiss、CAS 与事件广播由后续
//! Step 1.c-1.e 依次接入，本模块仅承担持久化与校验的核心职责。

use crate::theme_packages::sanitizer::sanitize_document;
use crate::theme_packages::store::PackageStore;
use crate::theme_packages::types::{ThemePackageDocument, ThemePackageStatus, ThemePackageSummary};
use serde_json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// 主题包 JSON 单文件最大字节数（512 KiB）。
///
/// 超过该阈值的导入直接拒绝，防止恶意大文件耗尽内存或磁盘。
/// 与主方案 §5.2.0 的 URL 下载大小限制保持一致。
pub(crate) const MAX_PACKAGE_JSON_BYTES: usize = 512 * 1024;

/// Preview 态（进程内内存态，非持久化）。
///
/// 用户点击"预览"某个主题包时，前端应用其派生 token 但不写 preferences；
/// 该态记录当前正在预览的主题包 id，方便前端查询"是否处于预览中"。
/// 应用重启后清空。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct PreviewState {
    pub(crate) previewing_id: Option<String>,
}

/// 主题包子系统组合根。
///
/// 内部 `preview_state` 使用 `RwLock`，因为大多数读取（is_preview_active）
/// 不修改状态；写入仅在 `preview_theme_package` / `dismiss_theme_preview` 命令中发生。
#[derive(Clone)]
pub(crate) struct ThemePackageService {
    store: Arc<PackageStore>,
    preview_state: Arc<RwLock<PreviewState>>,
}

impl ThemePackageService {
    /// 从应用数据目录初始化服务。
    ///
    /// 会在 `<app_data>/theme-packages/` 下创建三态子目录并清扫 pending-delete。
    /// 该方法应在 `AppState::new` 中调用一次；后续命令通过 `AppState::theme_packages()`
    /// accessor 获取克隆。
    pub(crate) fn new(app_data_dir: PathBuf) -> Result<Self, String> {
        let store = Arc::new(PackageStore::new(app_data_dir)?);
        Ok(Self {
            store,
            preview_state: Arc::new(RwLock::new(PreviewState::default())),
        })
    }

    /// 设置进程内预览态；返回 `Err` 表示指定 id 未安装。
    ///
    /// 该操作不写 preferences，也不改变 committed 目录内容；只登记内存中
    /// "当前正在预览的 id"，供前端 UI 状态查询。
    pub(crate) fn set_preview(&self, id: &str) -> Result<ThemePackageDocument, String> {
        let doc = self
            .inspect(id)?
            .ok_or_else(|| format!("theme package not found: {id}"))?;
        let mut state = self
            .preview_state
            .write()
            .map_err(|_| "preview state lock poisoned".to_string())?;
        state.previewing_id = Some(id.to_string());
        Ok(doc)
    }

    /// 清空预览态。幂等，无 preview 时也返回 Ok。
    pub(crate) fn dismiss_preview(&self) -> Result<(), String> {
        let mut state = self
            .preview_state
            .write()
            .map_err(|_| "preview state lock poisoned".to_string())?;
        state.previewing_id = None;
        Ok(())
    }

    /// 查询当前预览态的 id（若无返回 None）。
    #[allow(dead_code)]
    pub(crate) fn current_preview_id(&self) -> Option<String> {
        self.preview_state
            .read()
            .ok()
            .and_then(|state| state.previewing_id.clone())
    }

    /// 将 committed 目录下的主题包原始 JSON 导出到指定路径。
    ///
    /// 该操作只读取磁盘，不改变主题包状态。目标路径需要是绝对路径；
    /// 上层命令层负责路径校验（非软链、可写入）。
    pub(crate) fn export_to(&self, id: &str, dst: &std::path::Path) -> Result<(), String> {
        let raw = self
            .store
            .read_committed_raw(id)?
            .ok_or_else(|| format!("theme package not found: {id}"))?;
        std::fs::write(dst, &raw).map_err(|e| format!("failed to write export: {e}"))
    }

    /// 列出所有已安装（committed 状态）的主题包摘要。
    ///
    /// 顺序按 id 字典序；返回值仅包含 manifest 精简字段，slots 需通过
    /// `inspect` 按需读取。sidecar 读取失败时对应条目的 sha256 字段返回 None。
    pub(crate) fn list(&self) -> Result<Vec<ThemePackageSummary>, String> {
        let ids = self.store.list_committed_ids()?;
        let mut summaries = Vec::with_capacity(ids.len());
        for id in ids {
            match self.load_document(&id)? {
                Some(doc) => {
                    let sha256 = self.store.read_sidecar(&id).ok().flatten();
                    summaries.push(ThemePackageSummary {
                        id: doc.manifest.id.clone(),
                        name: doc.manifest.name.clone(),
                        version: doc.manifest.version.clone(),
                        status: ThemePackageStatus::Committed,
                        builtin: false,
                        sha256,
                    });
                }
                None => continue,
            }
        }
        Ok(summaries)
    }

    /// 读取指定主题包完整文档。
    ///
    /// 返回 `None` 表示 id 不存在。JSON 反序列化失败会返回 Err（sanitizer
    /// 层的 warnings 已包含在文档内）。
    pub(crate) fn inspect(&self, id: &str) -> Result<Option<ThemePackageDocument>, String> {
        self.load_document(id)
    }

    /// 从文件路径安装主题包。
    ///
    /// 步骤：
    /// 1. 读取原始字节，校验大小 <= `MAX_PACKAGE_JSON_BYTES`
    /// 2. serde 反序列化，`sanitize_document` 做字段级清洗（warn-而非-reject）
    /// 3. 将清洗后的文档重新序列化为规范 JSON（去除未知字段 + 补齐 warnings）
    /// 4. 计算真实 SHA-256（用于 sidecar 完整性校验）
    /// 5. 通过 `PackageStore::commit` 原子写入 committed + sidecar
    ///
    /// 返回值：安装后的主题包摘要。若同 id 已存在则覆盖（下一版考虑冲突提示）。
    /// **落盘的是清洗后的字节**（含 warnings），而不是用户提供的原始 raw；
    /// 因此下次 inspect 读到的哈希是清洗后 JSON 的哈希，与 sidecar 一致。
    pub(crate) fn install_from_bytes(&self, raw: Vec<u8>) -> Result<ThemePackageSummary, String> {
        if raw.len() > MAX_PACKAGE_JSON_BYTES {
            return Err(format!(
                "theme package exceeds size limit ({} > {} bytes)",
                raw.len(),
                MAX_PACKAGE_JSON_BYTES
            ));
        }

        let mut doc: ThemePackageDocument =
            serde_json::from_slice(&raw).map_err(|e| format!("invalid theme package JSON: {e}"))?;
        sanitize_document(&mut doc)?;

        // 清洗后的文档重新序列化为规范字节；这是最终落盘的内容
        let normalized = serde_json::to_vec_pretty(&doc)
            .map_err(|e| format!("failed to serialize sanitized document: {e}"))?;
        let sha256 = sha256_hex(&normalized);
        self.store.commit(&doc.manifest.id, &normalized, &sha256)?;

        Ok(ThemePackageSummary {
            id: doc.manifest.id.clone(),
            name: doc.manifest.name.clone(),
            version: doc.manifest.version.clone(),
            status: ThemePackageStatus::Committed,
            builtin: false,
            sha256: Some(sha256),
        })
    }

    /// 卸载指定主题包（原子搬到 pending-delete）。
    ///
    /// 对不存在的 id 幂等成功。调用方需在卸载前确认该 id 不是当前 active_package_id
    /// 或已在 preferences 层完成 rollback。
    pub(crate) fn uninstall(&self, id: &str) -> Result<(), String> {
        self.store.uninstall(id)
    }

    /// 从 https URL 下载并安装主题包（异步）。
    ///
    /// 严格 SSRF 防护，见 [`crate::theme_packages::downloader`] 模块说明。
    /// 下载得到原始字节后走 `install_from_bytes` 相同的 sanitize + commit 流水线。
    pub(crate) async fn install_from_url(&self, url: &str) -> Result<ThemePackageSummary, String> {
        let raw = crate::theme_packages::downloader::download_theme_package(url).await?;
        self.install_from_bytes(raw)
    }

    /// 内部：读取 committed JSON 并反序列化为 Document。
    ///
    /// P0-5 卡点：sidecar 缺失时不允许静默重生成，必须走完整 schema + hash 校验后补写。
    /// sidecar 存在但 hash 不匹配时直接返回 Err，拒绝加载被篡改的主题包。
    fn load_document(&self, id: &str) -> Result<Option<ThemePackageDocument>, String> {
        let raw = match self.store.read_committed_raw(id)? {
            Some(r) => r,
            None => return Ok(None),
        };
        let computed = sha256_hex(&raw);
        match self.store.read_sidecar(id)? {
            Some(stored) if stored != computed => {
                return Err(format!(
                    "theme package {id} hash mismatch: sidecar={stored}, computed={computed}"
                ));
            }
            None => {
                // sidecar 缺失：先跑完整 sanitize + 校验，通过后按规范化字节补写 sidecar（P0-5）
                let mut doc: ThemePackageDocument = serde_json::from_slice(&raw)
                    .map_err(|e| format!("theme package {id} corrupted (no sidecar): {e}"))?;
                sanitize_document(&mut doc)?;
                self.store
                    .commit(id, &raw, &computed)
                    .map_err(|e| format!("failed to restore sidecar for {id}: {e}"))?;
            }
            _ => {}
        }
        serde_json::from_slice::<ThemePackageDocument>(&raw)
            .map(Some)
            .map_err(|e| format!("failed to deserialize theme package {id}: {e}"))
    }
}

/// 计算字节数组的 SHA-256 哈希并返回小写十六进制字符串（64 位）。
///
/// 用于主题包 sidecar 完整性校验。sha2 crate 已在 workspace 依赖树中（由 tauri 引入），
/// 此处通过 `sha2 = "0.10"` 直接依赖。
fn sha256_hex(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    hash.iter().fold(String::with_capacity(64), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempservice() -> (tempfile::TempDir, ThemePackageService) {
        let dir = tempfile::tempdir().unwrap();
        let svc = ThemePackageService::new(dir.path().to_path_buf()).unwrap();
        (dir, svc)
    }

    fn sample_package_json() -> Vec<u8> {
        br##"{
            "schemaVersion": 1,
            "manifest": {
                "id": "acme-glass",
                "name": "Acme Glass",
                "version": "1.0.0",
                "author": "acme"
            },
            "slots": {
                "accent": "#7c3aed",
                "surface": "#ffffff",
                "textPrimary": "#1d1d1f",
                "textSecondary": "#6e6e73",
                "tint": "#82829B",
                "danger": "#ef4444"
            }
        }"##
        .to_vec()
    }

    #[test]
    fn install_from_bytes_creates_committed_entry() {
        let (_dir, svc) = tempservice();
        let raw = sample_package_json();
        let summary = svc.install_from_bytes(raw).expect("install");
        assert_eq!(summary.id, "acme-glass");
        assert_eq!(summary.name, "Acme Glass");
        assert_eq!(summary.status, ThemePackageStatus::Committed);
        assert!(summary.sha256.is_some());
    }

    #[test]
    fn list_returns_installed_packages() {
        let (_dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();

        let summaries = svc.list().unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, "acme-glass");
    }

    #[test]
    fn inspect_returns_full_document() {
        let (_dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();

        let doc = svc.inspect("acme-glass").unwrap().unwrap();
        assert_eq!(doc.manifest.id, "acme-glass");
        assert_eq!(doc.slots.get("accent").unwrap(), "#7c3aed");
    }

    #[test]
    fn uninstall_removes_from_committed() {
        let (_dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        svc.uninstall("acme-glass").unwrap();
        assert_eq!(svc.list().unwrap().len(), 0);
    }

    #[test]
    fn install_rejects_oversized_payload() {
        let (_dir, svc) = tempservice();
        let oversized = vec![0u8; MAX_PACKAGE_JSON_BYTES + 1];
        assert!(svc.install_from_bytes(oversized).is_err());
    }

    #[test]
    fn install_rejects_invalid_json() {
        let (_dir, svc) = tempservice();
        assert!(svc.install_from_bytes(b"not json".to_vec()).is_err());
    }

    #[test]
    fn install_rejects_unsupported_schema_version() {
        let (_dir, svc) = tempservice();
        let raw = br#"{
            "schemaVersion": 99,
            "manifest": {"id":"x","name":"x","version":"1"},
            "slots": {}
        }"#
        .to_vec();
        assert!(svc.install_from_bytes(raw).is_err());
    }

    #[test]
    fn install_rejects_empty_id() {
        let (_dir, svc) = tempservice();
        let raw = br#"{
            "schemaVersion": 1,
            "manifest": {"id":"","name":"x","version":"1"},
            "slots": {}
        }"#
        .to_vec();
        assert!(svc.install_from_bytes(raw).is_err());
    }

    #[test]
    fn inspect_missing_id_returns_none() {
        let (_dir, svc) = tempservice();
        assert!(svc.inspect("nonexistent").unwrap().is_none());
    }

    #[test]
    fn uninstall_is_idempotent() {
        let (_dir, svc) = tempservice();
        svc.uninstall("nonexistent").expect("idempotent");
    }

    #[test]
    fn set_preview_returns_document_and_updates_state() {
        let (_dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        let doc = svc.set_preview("acme-glass").unwrap();
        assert_eq!(doc.manifest.id, "acme-glass");
        assert_eq!(svc.current_preview_id(), Some("acme-glass".to_string()));
    }

    #[test]
    fn set_preview_rejects_missing_id() {
        let (_dir, svc) = tempservice();
        assert!(svc.set_preview("nonexistent").is_err());
        assert_eq!(svc.current_preview_id(), None);
    }

    #[test]
    fn dismiss_preview_clears_state_and_is_idempotent() {
        let (_dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        svc.set_preview("acme-glass").unwrap();
        assert!(svc.current_preview_id().is_some());
        svc.dismiss_preview().unwrap();
        assert!(svc.current_preview_id().is_none());
        // Idempotent
        svc.dismiss_preview().unwrap();
    }

    #[test]
    fn export_to_writes_committed_raw_bytes() {
        let (dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        let dst = dir.path().join("exported.json");
        svc.export_to("acme-glass", &dst).unwrap();
        let content = std::fs::read(&dst).unwrap();
        // 导出内容与原始安装字节一致（除格式化外可能有差异，
        // 但由于我们保存原始 raw，导出后应完全相同）
        assert!(!content.is_empty());
        let doc: ThemePackageDocument = serde_json::from_slice(&content).unwrap();
        assert_eq!(doc.manifest.id, "acme-glass");
    }

    #[test]
    fn export_to_rejects_missing_id() {
        let (dir, svc) = tempservice();
        let dst = dir.path().join("exported.json");
        assert!(svc.export_to("nonexistent", &dst).is_err());
    }

    #[test]
    fn sha256_hex_returns_deterministic_64char_lowercase() {
        // 空输入的 SHA-256 是 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let empty = sha256_hex(b"");
        assert_eq!(empty.len(), 64);
        assert!(empty
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
        assert_eq!(
            empty,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        // 相同输入生成相同哈希
        assert_eq!(sha256_hex(b"abc"), sha256_hex(b"abc"));
        // 不同输入生成不同哈希
        assert_ne!(sha256_hex(b"abc"), sha256_hex(b"abd"));
    }

    #[test]
    fn install_persists_real_sha256_of_sanitized_bytes_in_sidecar() {
        let (_dir, svc) = tempservice();
        let raw = sample_package_json();
        let summary = svc.install_from_bytes(raw).unwrap();
        // 落盘的哈希应为 64 位十六进制 SHA-256（非空、稳定长度）
        let disk_hash = summary.sha256.as_deref().unwrap();
        assert_eq!(disk_hash.len(), 64);
        assert!(disk_hash.chars().all(|c| c.is_ascii_hexdigit()));
        // list 返回的 sha256 与安装时一致
        let listed = svc.list().unwrap();
        assert_eq!(listed[0].sha256.as_deref(), Some(disk_hash));
        // 再 inspect 一次不会报 hash mismatch（sidecar 与落盘规范化字节匹配）
        assert!(svc.inspect("acme-glass").unwrap().is_some());
    }

    #[test]
    fn inspect_regenerates_sidecar_when_missing_after_full_validation() {
        let (dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        // 手动删除 sidecar 模拟 P0-5 场景
        let sidecar = dir
            .path()
            .join("theme-packages/committed/acme-glass.sha256");
        std::fs::remove_file(&sidecar).unwrap();
        assert!(!sidecar.exists());
        // inspect 触发 load_document → sidecar 缺失走完整校验重生成
        let doc = svc.inspect("acme-glass").unwrap().unwrap();
        assert_eq!(doc.manifest.id, "acme-glass");
        assert!(sidecar.exists(), "sidecar should have been restored");
    }

    #[test]
    fn inspect_rejects_when_sidecar_hash_mismatches_raw() {
        let (dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        // 手动篡改 sidecar 为错误的 hash
        let sidecar = dir
            .path()
            .join("theme-packages/committed/acme-glass.sha256");
        std::fs::write(&sidecar, "deadbeef".repeat(8)).unwrap();
        let result = svc.inspect("acme-glass");
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("hash mismatch"),
            "expected hash mismatch error"
        );
    }
}
