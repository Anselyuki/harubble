//! 应用偏好读取、保存与导入导出相关的 Tauri command。
//!
//! 当前暴露的接口覆盖偏好快照读取、设置落盘、导入导出与通知相关辅助能力，
//! 主要用于设置面板初始化、持久化保存和环境能力检查。

use crate::app_state::AppState;
use crate::local_inventory::spawn_inventory_scan;
use crate::preferences::AppPreferences;
use std::path::{Path, PathBuf};
use tauri::State;
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
    Internal(String),
}

impl std::fmt::Display for PreferencesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreferencesError::Io(m) | PreferencesError::Internal(m) => write!(f, "{m}"),
            PreferencesError::NotFound => write!(f, "偏好设置未找到"),
        }
    }
}

impl std::error::Error for PreferencesError {}

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
/// **volume 语义**：该字段由播放器子系统通过 `set_playback_volume` 单独持久化；
/// 设置面板 UI 不编辑音量。为避免设置保存与音量拖动之间的写-写竞态覆盖当前音量，
/// 本命令会在校验完成后忽略入参中的 `volume`，改用后端当前 `AppPreferences::volume`
/// 落盘。为了保证与并发的 `set_playback_volume` 之间不出现 TOCTOU 竞态，本命令通过
/// `update_preferences` 在偏好写锁内一次性完成"读取当前 volume + 覆盖其它字段"，
/// 避免锁外快照 volume 后又被并发写入覆盖。若下载目录发生变化，该接口会自动触发
/// 一次本地库存重新扫描；调用方不需要再额外手动发起扫描。
#[tauri::command]
pub async fn set_preferences(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    preferences: AppPreferences,
) -> Result<AppPreferences, PreferencesError> {
    let locale = preferences.locale;
    preferences
        .validate(locale)
        .map_err(PreferencesError::Internal)?;
    let previous_output_dir = state.preferences().output_dir.clone();
    let previous_locale = state.preferences().locale;
    // 通过 update_preferences 让 "读取当前 volume + 应用新字段 + 落盘" 都发生在
    // preferences_write_lock 内，避免与 set_playback_volume 之间产生 TOCTOU。
    let persisted = state
        .update_preferences(move |current| {
            let preserved_volume = current.volume;
            *current = preferences;
            current.volume = preserved_volume;
        })
        .await
        .map_err(PreferencesError::Io)?;
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
/// 该接口会覆盖当前偏好并写回本地存储；若导入后的下载目录发生变化，也会自动触发本地库存重新扫描。
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
    let imported_to_save = imported.clone();
    state
        .persist_preferences(imported_to_save)
        .await
        .map_err(PreferencesError::Io)?;
    if previous.output_dir != imported.output_dir {
        spawn_inventory_scan(
            app.clone(),
            state.inner().clone(),
            imported.output_dir.clone(),
            None,
        );
    }
    if previous.locale != imported.locale {
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
    Ok(imported)
}

/// 获取通知权限状态字符串。
///
/// 适用于设置面板展示当前系统通知授权状态，或在发送测试通知前决定是否提示用户授权。
/// 返回值为标准化后的权限状态字符串，如 `granted`、`denied` 或 `prompt`。
/// 该接口反映的是当前系统权限快照；若用户刚在系统设置中修改权限，调用方应重新调用以获取最新状态。
#[tauri::command]
pub fn get_notification_permission_state(
    state: State<'_, AppState>,
) -> Result<String, PreferencesError> {
    let app = state.player().app_handle();
    let permission = app
        .notification()
        .permission_state()
        .map_err(|e| PreferencesError::Internal(format!("{e}")))?;
    Ok(match permission {
        tauri::plugin::PermissionState::Granted => "granted",
        tauri::plugin::PermissionState::Denied => "denied",
        tauri::plugin::PermissionState::Prompt => "prompt",
        tauri::plugin::PermissionState::PromptWithRationale => "prompt-with-rationale",
    }
    .to_string())
}

/// 发送一条测试通知，用于验证系统通知链路。
///
/// 适用于用户在设置面板主动验证通知是否可达的场景。
/// 成功时返回空值。
/// 该接口会向系统真正发送一条可见通知，调用方应只在用户明确触发时调用，避免把测试通知当成静默探测手段。
#[tauri::command]
pub fn send_test_notification(state: State<'_, AppState>) -> Result<(), PreferencesError> {
    let app = state.player().app_handle();
    crate::notification::notify_test(app).map_err(PreferencesError::Internal)
}
