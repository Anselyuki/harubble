//! 主题包子系统组合根。
//!
//! # 模块职责
//!
//! `ThemePackageService` 是主题包子系统对外暴露的唯一入口，聚合编译期内置注册表、
//! `PackageStore`（磁盘状态机）与内部 sanitize / hash 逻辑。Tauri command 层只应
//! 通过本类型访问主题包能力，不允许直接持有 `PackageStore` 或原始文件路径。
//!
//! # 生命周期
//!
//! 由 `AppState::new` 在启动时初始化一次并 `Arc` 包装后长期持有；跨命令共享。
//! 内部 `PackageStore` 已在 `new` 时完成 pending-delete 清扫和三态目录创建。
//!
//! # 能力边界
//!
//! 当前方法覆盖 list / inspect / install-file / install-url / uninstall / preview /
//! dismiss / export。激活主题包的 preferences CAS 与跨窗口事件广播由 command 层
//! 协调，本模块只承担主题包持久化、校验和进程内预览状态。

use crate::theme_packages::builtin::{builtin_theme_package_source, load_builtin_theme_packages};
use crate::theme_packages::sanitizer::{
    sanitize_import_document, sanitize_stored_document, sanitize_untrusted_stored_document,
    validate_stored_package_id,
};
use crate::theme_packages::store::PackageStore;
use crate::theme_packages::types::{ThemePackageDocument, ThemePackageStatus, ThemePackageSummary};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

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
    builtins: Arc<BTreeMap<String, ThemePackageDocument>>,
    mutation_lock: Arc<Mutex<()>>,
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
        let builtins = Arc::new(load_builtin_theme_packages()?);
        Ok(Self {
            store,
            builtins,
            mutation_lock: Arc::new(Mutex::new(())),
            preview_state: Arc::new(RwLock::new(PreviewState::default())),
        })
    }

    /// 设置进程内预览态；返回 `Err` 表示指定 id 未安装。
    ///
    /// 该操作不写 preferences，也不改变 committed 目录内容；只登记内存中
    /// "当前正在预览的 id"，供前端 UI 状态查询。
    pub(crate) fn set_preview(&self, id: &str) -> Result<ThemePackageDocument, String> {
        // Keep inspection and preview registration under the same mutation lock
        // used by uninstall, so a removed package cannot become the new preview.
        let _mutation_guard = self
            .mutation_lock
            .lock()
            .map_err(|_| "theme package mutation lock poisoned".to_string())?;
        let doc = self
            .inspect_locked(id)?
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

    /// 将主题包 JSON 导出到指定路径。
    ///
    /// 内置包导出编译期嵌入的规范源文件，用户包导出 committed 目录中经 sanitizer
    /// 规范化后的 JSON。
    /// 该操作不改变主题包状态。目标路径需要是绝对路径；上层命令层负责路径校验。
    pub(crate) fn export_to(&self, id: &str, dst: &std::path::Path) -> Result<(), String> {
        validate_stored_package_id(id)?;
        // A user package with the same id as a newly shipped builtin remains
        // authoritative until the user explicitly uninstalls it.
        if self.store.read_committed_raw(id)?.is_none() {
            if let Some(source) = builtin_theme_package_source(id) {
                return std::fs::write(dst, source.as_bytes())
                    .map_err(|e| format!("failed to write export: {e}"));
            }
        }
        let raw = self
            .store
            .read_committed_raw(id)?
            .ok_or_else(|| format!("theme package not found: {id}"))?;
        std::fs::write(dst, &raw).map_err(|e| format!("failed to write export: {e}"))
    }

    /// 列出所有可用的内置包与已安装包摘要。
    ///
    /// 顺序按 id 字典序；返回值包含 manifest 精简字段、status、builtin、sha256
    /// 与 warnings，slots 仍需通过 `inspect` 按需读取。无法加载或校验的用户包会被
    /// 跳过；完整文档加载成功后，摘要阶段再次读取 sidecar 失败时 sha256 返回 None。
    pub(crate) fn list(&self) -> Result<Vec<ThemePackageSummary>, String> {
        let ids = self.store.list_committed_ids()?;
        let mut summaries = Vec::with_capacity(self.builtins.len() + ids.len());
        for doc in self.builtins.values() {
            if ids.iter().any(|id| id == &doc.manifest.id) {
                continue;
            }
            let source = builtin_theme_package_source(&doc.manifest.id)
                .ok_or_else(|| format!("missing built-in source: {}", doc.manifest.id))?;
            summaries.push(ThemePackageSummary {
                id: doc.manifest.id.clone(),
                name: doc.manifest.name.clone(),
                version: doc.manifest.version.clone(),
                status: ThemePackageStatus::Committed,
                builtin: true,
                sha256: Some(sha256_hex(source.as_bytes())),
                warnings: doc.warnings.clone(),
            });
        }
        for id in ids {
            // Older releases accepted a broader but still path-safe file stem.
            // Keep those packages visible so users can inspect/export/uninstall them.
            if validate_stored_package_id(&id).is_err() {
                continue;
            }
            match self.load_document(&id) {
                Ok(Some(doc)) => {
                    let sha256 = self.store.read_sidecar(&id).ok().flatten();
                    summaries.push(ThemePackageSummary {
                        id: doc.manifest.id.clone(),
                        name: doc.manifest.name.clone(),
                        version: doc.manifest.version.clone(),
                        status: ThemePackageStatus::Committed,
                        builtin: false,
                        sha256,
                        warnings: doc.warnings.clone(),
                    });
                }
                Ok(None) => continue,
                Err(error) => {
                    eprintln!("[theme-packages] skipped unreadable package {id}: {error}");
                    continue;
                }
            }
        }
        summaries.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(summaries)
    }

    /// 读取指定内置包或已安装包的完整文档。
    ///
    /// 返回 `None` 表示 id 不存在。JSON 反序列化失败会返回 Err（sanitizer
    /// 层的 warnings 已包含在文档内）。
    pub(crate) fn inspect(&self, id: &str) -> Result<Option<ThemePackageDocument>, String> {
        let _mutation_guard = self
            .mutation_lock
            .lock()
            .map_err(|_| "theme package mutation lock poisoned".to_string())?;
        self.inspect_locked(id)
    }

    /// Inspect while the caller owns `mutation_lock`.
    fn inspect_locked(&self, id: &str) -> Result<Option<ThemePackageDocument>, String> {
        validate_stored_package_id(id)?;
        // User files shadow builtins with the same id. This is important when a
        // builtin is added after an older user installation already exists.
        if self.store.read_committed_raw(id)?.is_none() {
            if let Some(document) = self.builtins.get(id) {
                return Ok(Some(document.clone()));
            }
        }
        self.load_document_locked(id)
    }

    /// 从文件路径安装主题包。
    ///
    /// 步骤：
    /// 1. 读取原始字节，校验大小 <= `MAX_PACKAGE_JSON_BYTES`
    /// 2. serde 反序列化，丢弃外部自带 warnings，再由 `sanitize_import_document`
    ///    做字段级清洗（warn-而非-reject）
    /// 3. 将清洗后的文档重新序列化为规范 JSON（去除未知字段 + 补齐 warnings）
    /// 4. 计算真实 SHA-256（用于 sidecar 完整性校验）
    /// 5. 通过 `PackageStore::commit` 分别以 tempfile + atomic rename 写入 committed
    ///    JSON 与 sidecar；两次落盘不是一笔跨文件原子事务
    ///
    /// 返回值：安装后的主题包摘要。若同 id 用户包已存在则覆盖。内置 id 通常拒绝覆盖；
    /// 但升级前已存在并正在遮蔽同 id 内置包的用户包仍可替换，卸载后才恢复内置包。
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
        let _mutation_guard = self
            .mutation_lock
            .lock()
            .map_err(|_| "theme package mutation lock poisoned".to_string())?;

        let mut doc: ThemePackageDocument =
            serde_json::from_slice(&raw).map_err(|e| format!("invalid theme package JSON: {e}"))?;
        sanitize_import_document(&mut doc)?;
        if self.builtins.contains_key(&doc.manifest.id)
            && self.store.read_committed_raw(&doc.manifest.id)?.is_none()
        {
            return Err(format!(
                "built-in theme package id cannot be overwritten: {}",
                doc.manifest.id
            ));
        }

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
            warnings: doc.warnings.clone(),
        })
    }

    /// 卸载指定主题包（JSON 原子搬到 pending-delete，sidecar 尽力删除）。
    ///
    /// 对不存在的用户包 id 幂等成功；没有同 id committed 用户包时，内置包拒绝
    /// 卸载。若历史用户包正在遮蔽同 id 内置包，则允许卸载该用户包并露出内置包。
    /// 调用方需在卸载前确认用户包不是当前 active_package_id，或已在 preferences
    /// 层完成 rollback。
    pub(crate) fn validate_uninstall_target(&self, id: &str) -> Result<(), String> {
        validate_stored_package_id(id)?;
        if self.builtins.contains_key(id) && self.store.read_committed_raw(id)?.is_none() {
            return Err(format!(
                "built-in theme package cannot be uninstalled: {id}"
            ));
        }
        Ok(())
    }

    pub(crate) fn uninstall(&self, id: &str) -> Result<(), String> {
        let _mutation_guard = self
            .mutation_lock
            .lock()
            .map_err(|_| "theme package mutation lock poisoned".to_string())?;
        self.validate_uninstall_target(id)?;
        self.store.uninstall(id)?;
        let mut state = self
            .preview_state
            .write()
            .map_err(|_| "preview state lock poisoned".to_string())?;
        if state.previewing_id.as_deref() == Some(id) {
            state.previewing_id = None;
        }
        Ok(())
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
        let _mutation_guard = self
            .mutation_lock
            .lock()
            .map_err(|_| "theme package mutation lock poisoned".to_string())?;
        self.load_document_locked(id)
    }

    /// Load and normalize a user package while the caller owns `mutation_lock`.
    fn load_document_locked(&self, id: &str) -> Result<Option<ThemePackageDocument>, String> {
        validate_stored_package_id(id)?;
        let raw = match self.store.read_committed_raw(id)? {
            Some(r) => r,
            None => return Ok(None),
        };
        let computed = sha256_hex(&raw);
        let stored_hash = self.store.read_sidecar(id)?;
        if let Some(stored) = stored_hash.as_deref() {
            if stored != computed {
                return Err(format!(
                    "theme package {id} hash mismatch: sidecar={stored}, computed={computed}"
                ));
            }
        }
        // Integrity is established before parsing. A corrupt payload paired with
        // a stale sidecar therefore reports hash mismatch instead of a parser error.
        let mut document = serde_json::from_slice::<ThemePackageDocument>(&raw)
            .map_err(|e| format!("failed to deserialize theme package {id}: {e}"))?;
        if document.manifest.id != id {
            return Err(format!(
                "theme package identity mismatch: requested {id}, manifest contains {}",
                document.manifest.id
            ));
        }
        // A missing sidecar has no integrity proof. Treat the document as an
        // externally supplied legacy file and discard package-authored warnings
        // before generating the first trusted normalized sidecar.
        if stored_hash.is_none() {
            document.warnings.clear();
        }
        if stored_hash.is_some() {
            sanitize_stored_document(&mut document)?;
        } else {
            sanitize_untrusted_stored_document(&mut document)?;
        }
        let normalized = serde_json::to_vec_pretty(&document)
            .map_err(|e| format!("failed to serialize sanitized document: {e}"))?;
        if stored_hash.is_none() || normalized != raw {
            let normalized_hash = sha256_hex(&normalized);
            self.store
                .commit(id, &normalized, &normalized_hash)
                .map_err(|e| format!("failed to normalize theme package {id}: {e}"))?;
        }
        Ok(Some(document))
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
    use crate::theme_packages::builtin::BUILTIN_THEME_PACKAGE_IDS;

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
        assert!(!summary.builtin);
        assert!(summary.sha256.is_some());
        assert!(summary.warnings.is_empty());
    }

    #[test]
    fn install_and_list_summaries_expose_sanitizer_warnings() {
        let (_dir, svc) = tempservice();
        let raw = br##"{
            "schemaVersion": 1,
            "manifest": {"id":"warning-theme","name":"Warnings","version":"1"},
            "slots": {"accent":"#7c3aed","unknownSlot":"#ffffff"}
        }"##
        .to_vec();

        let installed = svc.install_from_bytes(raw).expect("install");
        assert!(installed
            .warnings
            .iter()
            .any(|warning| warning.contains("unknownSlot")));

        let listed = svc
            .list()
            .unwrap()
            .into_iter()
            .find(|summary| summary.id == "warning-theme")
            .expect("warning summary");
        assert_eq!(listed.warnings, installed.warnings);
    }

    #[test]
    fn install_discards_package_authored_warnings() {
        let (_dir, svc) = tempservice();
        let raw = br##"{
            "schemaVersion": 1,
            "manifest": {"id":"warning-origin","name":"Warnings","version":"1"},
            "slots": {"accent":"#7c3aed","unknownSlot":"#ffffff"},
            "warnings": ["package-authored warning"]
        }"##
        .to_vec();

        let installed = svc.install_from_bytes(raw).expect("install");

        assert!(!installed
            .warnings
            .iter()
            .any(|warning| warning == "package-authored warning"));
        assert!(installed
            .warnings
            .iter()
            .any(|warning| warning.contains("unknownSlot")));
    }

    #[test]
    fn list_returns_all_builtin_packages_without_disk_state() {
        let (_dir, svc) = tempservice();

        let summaries = svc.list().unwrap();
        let builtin_summaries = summaries
            .iter()
            .filter(|summary| summary.builtin)
            .collect::<Vec<_>>();
        let ids = builtin_summaries
            .iter()
            .map(|summary| summary.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ids, BUILTIN_THEME_PACKAGE_IDS);
        assert!(builtin_summaries
            .iter()
            .all(|summary| summary.status == ThemePackageStatus::Committed));
        assert!(builtin_summaries
            .iter()
            .all(|summary| summary.sha256.as_ref().is_some_and(|hash| hash.len() == 64)));
    }

    #[test]
    fn list_returns_installed_packages() {
        let (_dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();

        let summaries = svc.list().unwrap();
        let installed = summaries
            .iter()
            .find(|summary| summary.id == "acme-glass")
            .expect("installed package summary");
        assert!(!installed.builtin);
    }

    #[test]
    fn list_keeps_legacy_safe_file_stems() {
        let (_dir, svc) = tempservice();
        let mut legacy: ThemePackageDocument =
            serde_json::from_slice(&sample_package_json()).unwrap();
        legacy.manifest.id = "legacy_theme".to_string();
        let legacy_raw = serde_json::to_vec_pretty(&legacy).unwrap();
        svc.store
            .commit("legacy_theme", &legacy_raw, &sha256_hex(&legacy_raw))
            .unwrap();

        let summaries = svc.list().expect("legacy entry must not break listing");

        assert_eq!(
            summaries.iter().filter(|summary| summary.builtin).count(),
            BUILTIN_THEME_PACKAGE_IDS.len()
        );
        assert!(summaries.iter().any(|summary| summary.id == "legacy_theme"));
    }

    #[test]
    fn list_keeps_builtins_visible_when_one_user_package_is_corrupted() {
        let (_dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        std::fs::write(svc.store.sidecar_for("acme-glass"), "deadbeef").unwrap();

        let summaries = svc.list().expect("corrupt user package must be isolated");

        assert_eq!(
            summaries.iter().filter(|summary| summary.builtin).count(),
            BUILTIN_THEME_PACKAGE_IDS.len()
        );
        assert!(summaries.iter().all(|summary| summary.id != "acme-glass"));
        assert!(svc.inspect("acme-glass").is_err());
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
    fn id_entrypoints_reject_path_traversal_but_allow_safe_legacy_ids() {
        let (dir, svc) = tempservice();
        for invalid in ["../../escape", "/tmp/x", "a/b", "a\\b"] {
            assert!(svc.inspect(invalid).is_err());
            assert!(svc.set_preview(invalid).is_err());
            assert!(svc.uninstall(invalid).is_err());
            assert!(svc
                .export_to(invalid, &dir.path().join("out.json"))
                .is_err());
        }
        assert!(svc.inspect("Ark-UI").unwrap().is_none());
    }

    #[test]
    fn install_rejects_path_traversal_manifest_id_without_writing_outside_store() {
        let (dir, svc) = tempservice();
        let mut document: ThemePackageDocument =
            serde_json::from_slice(&sample_package_json()).unwrap();
        document.manifest.id = "../../escape".to_string();
        let raw = serde_json::to_vec(&document).unwrap();

        assert!(svc.install_from_bytes(raw).is_err());
        assert!(!dir.path().join("escape.json").exists());
    }

    #[test]
    fn uninstall_removes_from_committed() {
        let (_dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        svc.uninstall("acme-glass").unwrap();
        assert!(svc.inspect("acme-glass").unwrap().is_none());
    }

    #[test]
    fn uninstall_clears_preview_state_for_removed_package() {
        let (_dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        svc.set_preview("acme-glass").unwrap();
        svc.uninstall("acme-glass").unwrap();
        assert_eq!(svc.current_preview_id(), None);
    }

    #[test]
    fn inspect_and_preview_resolve_builtin_packages() {
        let (_dir, svc) = tempservice();

        let doc = svc.inspect("ark-ui-endfield").unwrap().unwrap();
        assert_eq!(doc.manifest.id, "ark-ui-endfield");
        assert_eq!(
            doc.visual_contract.unwrap().family.as_deref(),
            Some("endfield")
        );

        let preview = svc.set_preview("ark-ui-endfield").unwrap();
        assert_eq!(preview.manifest.id, "ark-ui-endfield");
        assert_eq!(
            svc.current_preview_id(),
            Some("ark-ui-endfield".to_string())
        );
    }

    #[test]
    fn install_rejects_builtin_id_collision() {
        let (_dir, svc) = tempservice();
        let source = builtin_theme_package_source("ark-ui-ark").unwrap();

        let error = svc
            .install_from_bytes(source.as_bytes().to_vec())
            .expect_err("built-in id must be reserved");

        assert!(error.contains("cannot be overwritten"));
        assert!(svc.inspect("ark-ui-ark").unwrap().is_some());
    }

    #[test]
    fn user_package_shadows_builtin_until_explicitly_uninstalled() {
        let (_dir, svc) = tempservice();
        let mut user: ThemePackageDocument =
            serde_json::from_str(builtin_theme_package_source("ark-ui-ark").unwrap()).unwrap();
        user.manifest.name = "User Override".to_string();
        let raw = serde_json::to_vec(&user).unwrap();
        // A pre-existing user file represents an installation from before the
        // id became reserved by the current build.
        svc.store
            .commit(
                "ark-ui-ark",
                &serde_json::to_vec_pretty(&user).unwrap(),
                &sha256_hex(&serde_json::to_vec_pretty(&user).unwrap()),
            )
            .unwrap();

        let listed = svc.list().unwrap();
        let summary = listed.iter().find(|item| item.id == "ark-ui-ark").unwrap();
        assert!(!summary.builtin);
        assert_eq!(
            svc.inspect("ark-ui-ark").unwrap().unwrap().manifest.name,
            "User Override"
        );

        let replacement = svc
            .install_from_bytes(raw)
            .expect("existing user package may be replaced");
        assert!(!replacement.builtin);
        svc.uninstall("ark-ui-ark").unwrap();
        assert!(svc.inspect("ark-ui-ark").unwrap().unwrap().manifest.name != "User Override");
    }

    #[test]
    fn uninstall_rejects_builtin_package() {
        let (_dir, svc) = tempservice();

        let error = svc
            .uninstall("ark-ui-ark")
            .expect_err("built-in package must be immutable");

        assert!(error.contains("cannot be uninstalled"));
        assert!(svc.inspect("ark-ui-ark").unwrap().is_some());
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
    fn export_to_writes_committed_normalized_bytes() {
        let (dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        let dst = dir.path().join("exported.json");
        svc.export_to("acme-glass", &dst).unwrap();
        let content = std::fs::read(&dst).unwrap();
        // 用户包导出 committed 中经过 sanitizer 规范化的字节。
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
    fn export_to_writes_builtin_source() {
        let (dir, svc) = tempservice();
        let dst = dir.path().join("ark-ui-exa.json");

        svc.export_to("ark-ui-exa", &dst).unwrap();

        let content = std::fs::read(&dst).unwrap();
        let doc: ThemePackageDocument = serde_json::from_slice(&content).unwrap();
        assert_eq!(doc.manifest.id, "ark-ui-exa");
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
        let installed = listed
            .iter()
            .find(|summary| summary.id == "acme-glass")
            .expect("installed package summary");
        assert_eq!(installed.sha256.as_deref(), Some(disk_hash));
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
    fn missing_sidecar_recovery_persists_only_sanitized_content() {
        let (dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        let committed = dir.path().join("theme-packages/committed/acme-glass.json");
        let sidecar = dir
            .path()
            .join("theme-packages/committed/acme-glass.sha256");
        let mut document: ThemePackageDocument =
            serde_json::from_slice(&std::fs::read(&committed).unwrap()).unwrap();
        document.css_variables.insert(
            "--theme-custom-image".to_string(),
            "url(https://example.invalid/tracker.png)".to_string(),
        );
        std::fs::write(&committed, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
        std::fs::remove_file(&sidecar).unwrap();

        let inspected = svc.inspect("acme-glass").unwrap().unwrap();
        let persisted = std::fs::read(&committed).unwrap();
        let persisted_document: ThemePackageDocument = serde_json::from_slice(&persisted).unwrap();

        assert!(!inspected.css_variables.contains_key("--theme-custom-image"));
        assert!(!persisted_document
            .css_variables
            .contains_key("--theme-custom-image"));
        assert_eq!(
            std::fs::read_to_string(sidecar).unwrap(),
            sha256_hex(&persisted)
        );
    }

    #[test]
    fn missing_sidecar_recovery_discards_package_authored_warnings() {
        let (dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        let committed = dir.path().join("theme-packages/committed/acme-glass.json");
        let sidecar = dir
            .path()
            .join("theme-packages/committed/acme-glass.sha256");
        let mut document: ThemePackageDocument =
            serde_json::from_slice(&std::fs::read(&committed).unwrap()).unwrap();
        document
            .warnings
            .push("package-authored warning".to_string());
        std::fs::write(&committed, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
        std::fs::remove_file(&sidecar).unwrap();

        let inspected = svc.inspect("acme-glass").unwrap().unwrap();

        assert!(!inspected
            .warnings
            .iter()
            .any(|warning| warning == "package-authored warning"));
    }

    #[test]
    fn inspect_rejects_identity_mismatch_without_restoring_missing_sidecar() {
        let (dir, svc) = tempservice();
        svc.install_from_bytes(sample_package_json()).unwrap();
        let committed = dir.path().join("theme-packages/committed/acme-glass.json");
        let sidecar = dir
            .path()
            .join("theme-packages/committed/acme-glass.sha256");
        let mut document: ThemePackageDocument =
            serde_json::from_slice(&std::fs::read(&committed).unwrap()).unwrap();
        document.manifest.id = "different-package".to_string();
        std::fs::write(&committed, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
        std::fs::remove_file(&sidecar).unwrap();

        let error = svc.inspect("acme-glass").unwrap_err();

        assert!(error.contains("identity mismatch"));
        assert!(!sidecar.exists(), "invalid package must remain untouched");
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

    #[test]
    fn hash_mismatch_is_reported_before_json_parse() {
        let (dir, svc) = tempservice();
        let committed = dir.path().join("theme-packages/committed/acme-glass.json");
        let sidecar = dir
            .path()
            .join("theme-packages/committed/acme-glass.sha256");
        std::fs::write(&committed, b"{not valid json").unwrap();
        std::fs::write(&sidecar, "deadbeef").unwrap();
        let error = svc.inspect("acme-glass").unwrap_err();
        assert!(error.contains("hash mismatch"));
        assert!(!error.contains("deserialize"));
    }
}
