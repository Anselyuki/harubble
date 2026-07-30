//! 主题包（Theme Package）相关的 Tauri command。
//!
//! 本模块承担 Phase 1 主题包 MVP 的对外 IPC 边界。命令行为通过 `AppState::theme_packages()`
//! accessor 委托给 [`crate::theme_packages::ThemePackageService`]，不直接持有 store 或
//! 原始文件路径，遵循 `command_scheduling.rs` 的私有字段守卫约束。
//!
//! # 命令清单（Phase 1 MVP）
//!
//! - [`list_theme_packages`]：列出所有已安装主题包摘要
//! - [`install_theme_package_from_file`]：从本地文件路径安装主题包
//! - [`inspect_theme_package`]：读取指定主题包完整文档
//! - [`uninstall_theme_package`]：卸载主题包（搬到 pending-delete）
//!
//! 后续步骤（Phase 1 Step 1.c-1.e）将补齐 URL 抓取、CAS-based 激活、preview/dismiss
//! 与导出命令。

use crate::app_state::AppState;
use crate::preferences::AppPreferences;
use crate::theme_packages::{ThemePackageDocument, ThemePackageSummary};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::State;

/// 主题包命令的结构化错误类型。
///
/// 通过 `#[serde(tag = "code", content = "detail")]` 序列化为
/// `{ code: "invalidPackage", detail: "..." }` 的标签联合，与其他 command 错误
/// 结构保持一致。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "code", content = "detail")]
pub enum ThemePackageError {
    /// 主题包内容不合法（JSON 解析失败、schema 不支持、字段缺失等）。
    InvalidPackage(String),
    /// 文件系统或 IO 错误（读取失败、写入失败等）。
    Io(String),
    /// 主题包未找到（id 不存在于 committed 目录）。
    NotFound(String),
    /// CAS 冲突：客户端携带的 expected_revision 与后端当前不一致；
    /// `detail` 附带最新的 revision 供前端 rebase 后重试。
    RevisionMismatch(RevisionMismatchDetail),
    /// 服务内部错误。
    Internal(String),
}

/// CAS 冲突的详情载荷。
///
/// 前端在收到此错误后应重新 `getPreferences` 拉取最新 revision，展示给用户
/// "刚才别处也在改主题"的提示，让用户决定是否覆盖。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevisionMismatchDetail {
    pub current_revision: u64,
    pub expected_revision: u64,
    pub message: String,
}

impl std::fmt::Display for ThemePackageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemePackageError::InvalidPackage(m)
            | ThemePackageError::Io(m)
            | ThemePackageError::NotFound(m)
            | ThemePackageError::Internal(m) => write!(f, "{m}"),
            ThemePackageError::RevisionMismatch(d) => write!(f, "{}", d.message),
        }
    }
}

impl std::error::Error for ThemePackageError {}

/// 列出所有已安装（committed 状态）的主题包摘要。
///
/// 返回值按 id 字典序，仅包含 manifest 精简字段（id/name/version/sha256）。
/// slots 与 variants 需通过 [`inspect_theme_package`] 按需读取。
///
/// # 稳定性
///
/// 返回体积随主题包数量线性增长；用户量级下通常 < 10 项，无需分页。
#[tauri::command]
pub fn list_theme_packages(
    state: State<'_, AppState>,
) -> Result<Vec<ThemePackageSummary>, ThemePackageError> {
    state
        .theme_packages()
        .list()
        .map_err(ThemePackageError::Internal)
}

/// 读取指定主题包的完整文档（含 slots / variants / warnings）。
///
/// 返回 `None` 表示 id 不存在。前端通常在用户展开主题详情或点击 preview
/// 时调用；列表页面无需调用该命令。
#[tauri::command]
pub fn inspect_theme_package(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<ThemePackageDocument>, ThemePackageError> {
    state
        .theme_packages()
        .inspect(&id)
        .map_err(ThemePackageError::Internal)
}

/// 从本地文件路径安装主题包。
///
/// 入参 `path` 必须是绝对路径，指向可读的 `.json` 文件。
/// 步骤：
/// 1. 校验路径合法性（存在、非目录、非软链、大小 <= 512 KiB）
/// 2. 读取文件字节
/// 3. 通过 `ThemePackageService::install_from_bytes` 走 sanitize + hash + commit
///
/// 返回值：安装后的主题包摘要（含 sha256）。若 id 冲突则覆盖已有版本。
///
/// # 安全
///
/// 只读取本地文件系统，不发起网络请求；URL 场景由未来的
/// `install_theme_package_from_url` 单独承担并做 SSRF 白名单。
#[tauri::command]
pub fn install_theme_package_from_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<ThemePackageSummary, ThemePackageError> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.is_absolute() {
        return Err(ThemePackageError::InvalidPackage(
            "path must be absolute".to_string(),
        ));
    }
    if !path_buf.exists() {
        return Err(ThemePackageError::NotFound(format!(
            "file not found: {path}"
        )));
    }
    let metadata =
        fs::symlink_metadata(&path_buf).map_err(|e| ThemePackageError::Io(e.to_string()))?;
    if metadata.file_type().is_symlink() {
        return Err(ThemePackageError::InvalidPackage(
            "symlinks are not allowed".to_string(),
        ));
    }
    if !metadata.is_file() {
        return Err(ThemePackageError::InvalidPackage(
            "path must point to a regular file".to_string(),
        ));
    }
    const MAX_PACKAGE_BYTES: u64 = crate::theme_packages::service::MAX_PACKAGE_JSON_BYTES as u64;
    if metadata.len() > MAX_PACKAGE_BYTES {
        return Err(ThemePackageError::InvalidPackage(format!(
            "theme package exceeds size limit ({} > {} bytes)",
            metadata.len(),
            MAX_PACKAGE_BYTES
        )));
    }
    // Bound the read as well as the metadata check. A file can grow between
    // stat and open (or report a stale size), so never let the command buffer
    // more than one byte past the service limit.
    let file = fs::File::open(&path_buf).map_err(|e| ThemePackageError::Io(e.to_string()))?;
    let mut raw = Vec::with_capacity(metadata.len().min(MAX_PACKAGE_BYTES) as usize);
    file.take(MAX_PACKAGE_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|e| ThemePackageError::Io(e.to_string()))?;
    if raw.len() as u64 > MAX_PACKAGE_BYTES {
        return Err(ThemePackageError::InvalidPackage(format!(
            "theme package exceeds size limit ({} > {} bytes)",
            raw.len(),
            MAX_PACKAGE_BYTES
        )));
    }
    state
        .theme_packages()
        .install_from_bytes(raw)
        .map_err(ThemePackageError::InvalidPackage)
}

/// 从远程 https URL 安装主题包（SSRF 防护 + sanitize + 原子提交）。
///
/// # 安全边界
///
/// - 协议白名单：仅接受 `https://`，端口白名单：443
/// - DNS 解析后校验所有 IP 都不在 loopback / 私有段 / CGNAT / multicast / 保留段内
/// - 重定向禁用（防 DNS rebinding 变种）
/// - 响应大小 ≤ 512 KiB，超时 15s，`Content-Type` 必须是 JSON / text/plain / octet-stream
///
/// 下载成功后与 `install_theme_package_from_file` 走同一 sanitize 流水线。
#[tauri::command]
pub async fn install_theme_package_from_url(
    state: State<'_, AppState>,
    url: String,
) -> Result<ThemePackageSummary, ThemePackageError> {
    state
        .theme_packages()
        .install_from_url(&url)
        .await
        .map_err(|e| {
            // 简单分类：网络 / 校验类归 Io，格式类归 InvalidPackage
            let lower = e.to_lowercase();
            if lower.contains("invalid")
                || lower.contains("unsupported")
                || lower.contains("schema")
                || lower.contains("credentials")
            {
                ThemePackageError::InvalidPackage(e)
            } else {
                ThemePackageError::Io(e)
            }
        })
}

/// 卸载主题包（原子搬到 pending-delete，启动扫描时清理）。
///
/// 对不存在的 id 幂等成功。若被卸载的 id 是当前 active_package_id，会同步清空
/// 偏好中的激活状态（回退到 preset + customColors 路径），避免遗留悬挂引用。
#[tauri::command]
pub async fn uninstall_theme_package(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), ThemePackageError> {
    let packages = state.theme_packages().clone();
    packages
        .validate_uninstall_target(&id)
        .map_err(ThemePackageError::Internal)?;
    let id_for_update = id.clone();
    let (snapshot, was_active, uninstall_result) = state
        .update_preferences_then(
            move |prefs| {
                let was_active =
                    prefs.theme.active_package_id.as_deref() == Some(id_for_update.as_str());
                if was_active {
                    prefs.theme.active_package_id = None;
                    prefs.theme.revision = prefs.theme.revision.wrapping_add(1);
                }
                was_active
            },
            move || packages.uninstall(&id),
        )
        .await
        .map_err(ThemePackageError::Io)?;
    if was_active {
        crate::commands::preferences::emit_preferences_snapshot(&app, &snapshot);
    }
    uninstall_result.map_err(ThemePackageError::Internal)?;
    Ok(())
}

/// 通过 CAS 激活指定主题包（或传 `None` 清空激活状态）。
///
/// # CAS 语义
///
/// 客户端携带 `expected_revision`（上次读取到的 `theme.revision`），后端在锁内
/// 比较：
/// - 匹配 → 更新 `active_package_id` 并 `revision += 1`，返回新快照
/// - 不匹配 → 返回 `RevisionMismatch { current, expected, message }`
///   前端应重新 `getPreferences` 拉取后再决定是否覆盖
///
/// 成功路径会通过 `preferences_snapshot` 事件广播给所有窗口（含 Mini Player），
/// 消除跨窗口写-写竞态导致的 `customColors` 倒退（P1-1）。
///
/// # 入参
///
/// - `id`：目标主题包 id；`None` 表示清空激活状态，回退到 preset 路径
/// - `expected_revision`：CAS 版本比对基线
#[tauri::command]
pub async fn set_active_theme_package(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: Option<String>,
    expected_revision: u64,
) -> Result<AppPreferences, ThemePackageError> {
    let packages = state.theme_packages().clone();
    let result = state
        .try_update_preferences(move |prefs| {
            if let Some(ref target_id) = id {
                if packages
                    .inspect(target_id)
                    .map_err(ThemePackageError::Internal)?
                    .is_none()
                {
                    return Err(ThemePackageError::NotFound(format!(
                        "theme package not found: {target_id}"
                    )));
                }
            }

            let current_revision = prefs.theme.revision;
            if current_revision != expected_revision {
                return Err(ThemePackageError::RevisionMismatch(
                    RevisionMismatchDetail {
                        current_revision,
                        expected_revision,
                        message: format!(
                            "theme revision drift: expected {expected_revision}, got {current_revision}"
                        ),
                    },
                ));
            }

            prefs.theme.active_package_id = id;
            prefs.theme.revision = prefs.theme.revision.wrapping_add(1);
            Ok(())
        })
        .await
        .map_err(ThemePackageError::Io)?;
    let (snapshot, ()) = result?;
    crate::commands::preferences::emit_preferences_snapshot(&app, &snapshot);
    Ok(snapshot)
}

/// 进入指定主题包的预览态（内存中）。
///
/// 该命令不写 preferences，只登记进程内 "current preview id"。前端应用派生 token
/// 到 DOM，但用户关闭 UI 或调用 `dismiss_theme_preview` 后即恢复到 committed 态。
#[tauri::command]
pub fn preview_theme_package(
    state: State<'_, AppState>,
    id: String,
) -> Result<ThemePackageDocument, ThemePackageError> {
    state.theme_packages().set_preview(&id).map_err(|e| {
        // set_preview 内部区分不存在 / 内部错误
        if e.contains("not found") {
            ThemePackageError::NotFound(e)
        } else {
            ThemePackageError::Internal(e)
        }
    })
}

/// 关闭预览态，恢复到 committed / preference 派生。
///
/// 对未处于预览态的调用幂等成功。
#[tauri::command]
pub fn dismiss_theme_preview(state: State<'_, AppState>) -> Result<(), ThemePackageError> {
    state
        .theme_packages()
        .dismiss_preview()
        .map_err(ThemePackageError::Internal)
}

/// 将指定主题包的原始 JSON 导出到本地路径。
///
/// 入参 `output_path` 必须为绝对路径；父目录必须存在。目标已存在时被覆盖。
#[tauri::command]
pub fn export_theme_package(
    state: State<'_, AppState>,
    id: String,
    output_path: String,
) -> Result<(), ThemePackageError> {
    let path_buf = PathBuf::from(&output_path);
    if !path_buf.is_absolute() {
        return Err(ThemePackageError::InvalidPackage(
            "output_path must be absolute".to_string(),
        ));
    }
    if let Some(parent) = path_buf.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(ThemePackageError::Io(format!(
                "parent directory does not exist: {}",
                parent.display()
            )));
        }
    }
    state
        .theme_packages()
        .export_to(&id, Path::new(&path_buf))
        .map_err(|e| {
            if e.contains("not found") {
                ThemePackageError::NotFound(e)
            } else {
                ThemePackageError::Io(e)
            }
        })
}
