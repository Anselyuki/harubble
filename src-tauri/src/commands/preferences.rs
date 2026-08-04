//! 应用偏好读取、保存与导入导出相关的 Tauri command。
//!
//! 当前暴露的接口覆盖偏好快照读取、设置落盘、导入导出与通知相关辅助能力，
//! 主要用于设置面板初始化、持久化保存和环境能力检查。

use crate::app_state::AppState;
use crate::local_inventory::spawn_inventory_scan;
use crate::preferences::{AppPreferences, CURRENT_PREFERENCES_SCHEMA_VERSION};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_notification::NotificationExt;

/// 偏好 command 层统一错误类型。
///
/// - `Io`：文件读写、序列化/反序列化（toml/serde）等 I/O 相关失败。
/// - `NotFound`：请求的偏好资源不存在。
/// - `Internal`：其他内部错误，如线程调度失败、校验逻辑错误等。
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase", tag = "code", content = "detail")]
pub enum PreferencesError {
    Io(String),
    NotFound,
    RevisionMismatch(PreferencesRevisionMismatchDetail),
    Internal(String),
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesRevisionMismatchDetail {
    pub current_revision: u64,
    pub expected_revision: u64,
    pub message: String,
}

impl std::fmt::Display for PreferencesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreferencesError::Io(m) | PreferencesError::Internal(m) => write!(f, "{m}"),
            PreferencesError::NotFound => write!(f, "偏好设置未找到"),
            PreferencesError::RevisionMismatch(detail) => write!(f, "{}", detail.message),
        }
    }
}

impl std::error::Error for PreferencesError {}

fn apply_settings_update(current: &mut AppPreferences, requested: AppPreferences) {
    let preserved_schema_version = current
        .schema_version
        .max(CURRENT_PREFERENCES_SCHEMA_VERSION);
    let preserved_volume = current.volume;
    let preserved_active_package_id = current.theme.active_package_id.clone();
    let preserved_theme_revision = current.theme.revision;
    *current = requested;
    current.schema_version = preserved_schema_version;
    current.volume = preserved_volume;
    current.theme.active_package_id = preserved_active_package_id;
    // Every successful full snapshot mutation gets a new monotonic token.  The
    // token orders ordinary preference writes as well as theme CAS writes, so a
    // delayed snapshot can never roll back unrelated settings.
    current.theme.revision = preserved_theme_revision.wrapping_add(1);
}

fn apply_import_update(current: &mut AppPreferences, mut imported: AppPreferences) {
    // Theme package installation state is machine-local and is not part of a
    // preferences export. Keep the current CAS-owned reference and revision so
    // an imported file cannot create a dangling package id or rewind CAS.
    imported.schema_version = current
        .schema_version
        .max(CURRENT_PREFERENCES_SCHEMA_VERSION);
    imported.theme.active_package_id = current.theme.active_package_id.clone();
    imported.theme.revision = current.theme.revision.wrapping_add(1);
    *current = imported;
}

pub(crate) fn emit_preferences_snapshot(app: &tauri::AppHandle, snapshot: &AppPreferences) {
    if let Err(error) = app.emit("preferences_snapshot", snapshot) {
        eprintln!("[preferences] failed to emit preferences_snapshot: {error}");
    }
}

/// 获取当前偏好。
///
/// 适用于设置面板初始化、应用启动后恢复用户配置，或在导入/保存后重新同步偏好。
/// 返回值为当前生效的完整偏好快照。
/// 该接口只读取当前内存中的已生效偏好，不会触发磁盘写入或额外副作用。
#[tauri::command]
pub async fn get_preferences(
    state: State<'_, AppState>,
) -> Result<AppPreferences, PreferencesError> {
    Ok(state.preferences())
}

/// 设置偏好（验证后落盘）。
///
/// 适用于用户在设置面板保存配置后的正式提交。
/// 入参 `preferences` 为完整偏好对象；返回值为已经通过校验并写入后的最终偏好。
///
/// **字段所有权语义**：`schema_version` 由后端迁移层维护，`volume` 由播放器子系统单独持久化；
/// `theme.active_package_id` / `theme.revision` 由主题包 CAS 命令持久化。
/// 设置面板 UI 不编辑音量。为避免设置保存与音量拖动之间的写-写竞态覆盖当前音量，
/// 本命令会在校验完成后保留后端当前的上述字段。为了保证与并发写入之间不出现
/// TOCTOU 竞态，本命令通过
/// `try_update_preferences` 在偏好写锁内一次性完成 revision CAS、读取当前
/// volume 与覆盖其它字段，避免锁外快照又被并发写入覆盖。调用方必须传入
/// 上次权威快照的 `expected_revision`；若下载目录发生变化，该接口会自动触发一次
/// 本地库存重新扫描，调用方不需要再额外手动发起扫描。
#[tauri::command]
pub async fn set_preferences(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    preferences: AppPreferences,
    expected_revision: u64,
) -> Result<AppPreferences, PreferencesError> {
    let locale = preferences.locale;
    preferences
        .validate(locale)
        .map_err(PreferencesError::Internal)?;
    // CAS 校验、字段所有权合并与落盘必须处于同一写锁。否则两个窗口
    // 携带的旧全量快照依然可以按锁顺序相互覆盖。
    let result = state
        .try_update_preferences(move |current| {
            let current_revision = current.theme.revision;
            if current_revision != expected_revision {
                return Err(PreferencesError::RevisionMismatch(
                    PreferencesRevisionMismatchDetail {
                        current_revision,
                        expected_revision,
                        message: format!(
                            "preferences revision drift: expected {expected_revision}, got {current_revision}"
                        ),
                    },
                ));
            }
            let previous = (current.output_dir.clone(), current.locale);
            apply_settings_update(current, preferences);
            Ok(previous)
        })
        .await
        .map_err(PreferencesError::Io)?;
    let (persisted, (previous_output_dir, previous_locale)) = result?;
    if previous_output_dir != persisted.output_dir {
        spawn_inventory_scan(
            app.clone(),
            state.inner().clone(),
            persisted.output_dir.clone(),
            None,
        );
    }
    if previous_locale != persisted.locale {
        if let Err(error) = crate::install_menu(&app) {
            state.record_log(
                crate::LogPayload::new(
                    crate::LogLevel::Warn,
                    "menu",
                    "menu.install_failed",
                    "Failed to rebuild app menu after locale change",
                )
                .details(error.to_string()),
            );
        }
    }
    emit_preferences_snapshot(&app, &persisted);
    Ok(persisted)
}

/// 导出偏好到指定路径。
///
/// 适用于备份当前配置、跨设备迁移设置，或在重装前导出用户偏好。
/// 入参 `output_path` 为导出文件目标路径；返回值为本次导出的偏好内容。
/// 该接口不会改变当前运行中的偏好状态，只会把现有配置写出到指定文件。
#[tauri::command]
pub async fn export_preferences(
    state: State<'_, AppState>,
    output_path: String,
) -> Result<AppPreferences, PreferencesError> {
    let prefs = state.preferences();
    let locale = prefs.locale;
    let store = state.preferences_store();
    let path = PathBuf::from(output_path);
    let prefs_to_export = prefs.clone();
    tokio::task::spawn_blocking(move || {
        store.export_to(&prefs_to_export, Path::new(&path), locale)
    })
    .await
    .map_err(|e| PreferencesError::Internal(e.to_string()))?
    .map_err(|e| PreferencesError::Io(e.to_string()))?;
    Ok(prefs)
}

/// 从指定路径导入偏好。
///
/// 适用于恢复先前备份、迁移其他设备配置，或批量恢复用户设置。
/// 入参 `input_path` 为待导入文件路径；返回值为导入后已经生效的偏好。
/// 该接口会恢复普通偏好并写回本地存储；主题包激活引用与 CAS revision 属于当前
/// 机器的主题包子系统，不从导入文件覆盖。若导入后的下载目录发生变化，也会自动
/// 触发本地库存重新扫描。
#[tauri::command]
pub async fn import_preferences(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    input_path: String,
) -> Result<AppPreferences, PreferencesError> {
    let previous = state.preferences();
    let locale = previous.locale;
    let store = state.preferences_store();
    let input_path = PathBuf::from(input_path);
    let imported = {
        let store = store.clone();
        tokio::task::spawn_blocking(move || store.import_from(Path::new(&input_path), locale))
            .await
            .map_err(|e| PreferencesError::Internal(e.to_string()))?
            .map_err(|e| PreferencesError::Io(e.to_string()))?
    };
    let persisted = state
        .update_preferences(move |current| {
            apply_import_update(current, imported);
        })
        .await
        .map_err(PreferencesError::Io)?;
    if previous.output_dir != persisted.output_dir {
        spawn_inventory_scan(
            app.clone(),
            state.inner().clone(),
            persisted.output_dir.clone(),
            None,
        );
    }
    if previous.locale != persisted.locale {
        if let Err(error) = crate::install_menu(&app) {
            state.record_log(
                crate::LogPayload::new(
                    crate::LogLevel::Warn,
                    "menu",
                    "menu.install_failed",
                    "Failed to rebuild app menu after locale change",
                )
                .details(error.to_string()),
            );
        }
    }
    emit_preferences_snapshot(&app, &persisted);
    Ok(persisted)
}

/// 获取通知权限状态字符串。
///
/// 适用于设置面板展示当前系统通知授权状态，或在发送测试通知前决定是否提示用户授权。
/// 返回值为标准化后的权限状态字符串，如 `granted`、`denied` 或 `prompt`。
/// 该接口反映的是当前系统权限快照；若用户刚在系统设置中修改权限，调用方应重新调用以获取最新状态。
#[tauri::command]
pub fn get_notification_permission_state(app: AppHandle) -> Result<String, PreferencesError> {
    let permission = app
        .notification()
        .permission_state()
        .map_err(|e| PreferencesError::Internal(format!("{e}")))?;
    Ok(notification_permission_label(permission).to_string())
}

/// 在用户明确操作后请求系统通知权限，并返回请求后的权限状态。
#[tauri::command]
pub fn request_notification_permission(app: AppHandle) -> Result<String, PreferencesError> {
    let permission = app
        .notification()
        .request_permission()
        .map_err(|e| PreferencesError::Internal(format!("{e}")))?;
    Ok(notification_permission_label(permission).to_string())
}

fn notification_permission_label(permission: tauri::plugin::PermissionState) -> &'static str {
    match permission {
        tauri::plugin::PermissionState::Granted => "granted",
        tauri::plugin::PermissionState::Denied => "denied",
        tauri::plugin::PermissionState::Prompt => "prompt",
        tauri::plugin::PermissionState::PromptWithRationale => "prompt-with-rationale",
    }
}

/// 发送一条测试通知，用于验证系统通知链路。
///
/// 适用于用户在设置面板主动验证通知是否可达的场景。
/// 成功时返回空值。
/// 该接口会向系统真正发送一条可见通知，调用方应只在用户明确触发时调用，避免把测试通知当成静默探测手段。
#[tauri::command]
pub fn send_test_notification(app: AppHandle) -> Result<(), PreferencesError> {
    crate::notification::notify_test(app).map_err(PreferencesError::Internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_update_preserves_fields_owned_by_other_subsystems() {
        let mut current = AppPreferences {
            volume: 0.37,
            ..AppPreferences::default()
        };
        current.theme.active_package_id = Some("ark-ui-endfield".to_string());
        current.theme.revision = 12;

        let mut requested = AppPreferences {
            schema_version: 0,
            volume: 0.9,
            download_lyrics: false,
            ..AppPreferences::default()
        };
        requested.theme.active_package_id = None;
        requested.theme.revision = 0;

        apply_settings_update(&mut current, requested);

        assert_eq!(current.schema_version, CURRENT_PREFERENCES_SCHEMA_VERSION);
        assert_eq!(current.volume, 0.37);
        assert!(!current.download_lyrics);
        assert_eq!(
            current.theme.active_package_id.as_deref(),
            Some("ark-ui-endfield")
        );
        assert_eq!(current.theme.revision, 13);
    }

    #[test]
    fn settings_update_increments_revision_for_every_snapshot_mutation() {
        let mut current = AppPreferences::default();
        current.theme.revision = 7;

        let mut non_theme_update = current.clone();
        non_theme_update.download_lyrics = !current.download_lyrics;
        apply_settings_update(&mut current, non_theme_update);
        assert_eq!(current.theme.revision, 8);

        let mut theme_update = current.clone();
        theme_update.theme.dynamic_album_accent = !current.theme.dynamic_album_accent;
        apply_settings_update(&mut current, theme_update);
        assert_eq!(current.theme.revision, 9);
    }

    #[test]
    fn revision_mismatch_error_serializes_the_authoritative_revision() {
        let error = PreferencesError::RevisionMismatch(PreferencesRevisionMismatchDetail {
            current_revision: 9,
            expected_revision: 7,
            message: "preferences revision drift: expected 7, got 9".to_string(),
        });

        let payload = serde_json::to_value(error).expect("serialize revision mismatch");
        assert_eq!(payload["code"], "revisionMismatch");
        assert_eq!(payload["detail"]["currentRevision"], 9);
        assert_eq!(payload["detail"]["expectedRevision"], 7);
    }

    #[test]
    fn import_update_preserves_machine_local_theme_package_identity() {
        let mut current = AppPreferences {
            volume: 0.37,
            ..AppPreferences::default()
        };
        current.theme.active_package_id = Some("ark-ui-endfield".to_string());
        current.theme.revision = 12;

        let mut imported = AppPreferences {
            schema_version: 1,
            volume: 0.9,
            download_lyrics: false,
            ..AppPreferences::default()
        };
        imported.theme.preset_id = "clear-aqua".to_string();
        imported.theme.active_package_id = Some("missing-package".to_string());
        imported.theme.revision = 2;

        apply_import_update(&mut current, imported);

        assert_eq!(current.schema_version, CURRENT_PREFERENCES_SCHEMA_VERSION);
        assert_eq!(current.volume, 0.9);
        assert!(!current.download_lyrics);
        assert_eq!(current.theme.preset_id, "clear-aqua");
        assert_eq!(
            current.theme.active_package_id.as_deref(),
            Some("ark-ui-endfield")
        );
        assert_eq!(current.theme.revision, 13);
    }

    #[test]
    fn import_without_theme_changes_still_advances_snapshot_revision() {
        let mut current = AppPreferences::default();
        current.theme.active_package_id = Some("ark-ui-endfield".to_string());
        current.theme.revision = 12;

        let mut imported = current.clone();
        imported.theme.active_package_id = Some("missing-package".to_string());
        imported.theme.revision = 1;
        imported.download_lyrics = !current.download_lyrics;
        apply_import_update(&mut current, imported);

        assert_eq!(
            current.theme.active_package_id.as_deref(),
            Some("ark-ui-endfield")
        );
        assert_eq!(current.theme.revision, 13);
    }

    #[test]
    fn color_scheme_save_reload_keeps_active_theme_package() {
        let app_data_dir = tempfile::tempdir().expect("create app data dir");
        let output_dir = app_data_dir.path().join("output");
        std::fs::create_dir_all(&output_dir).expect("create output dir");
        let store = crate::preferences::PreferencesStore::new(app_data_dir.path().to_path_buf());

        let mut current = AppPreferences {
            output_dir: output_dir.to_string_lossy().to_string(),
            ..AppPreferences::default()
        };
        current.theme.active_package_id = Some("ark-ui-endfield".to_string());
        current.theme.revision = 12;

        // The frontend DTO historically omitted schemaVersion and sent zero.
        let mut requested = current.clone();
        requested.schema_version = 0;
        requested.theme.color_scheme = crate::preferences::ColorScheme::Dark;
        requested.theme.active_package_id = None;
        requested.theme.revision = 0;

        apply_settings_update(&mut current, requested);
        store
            .save(&current, current.locale)
            .expect("persist updated preferences");
        let reloaded = store.load(None);

        assert_eq!(reloaded.schema_version, CURRENT_PREFERENCES_SCHEMA_VERSION);
        assert_eq!(
            reloaded.theme.active_package_id.as_deref(),
            Some("ark-ui-endfield")
        );
        assert_eq!(reloaded.theme.revision, 13);
        assert_eq!(
            reloaded.theme.color_scheme,
            crate::preferences::ColorScheme::Dark
        );
    }
}
