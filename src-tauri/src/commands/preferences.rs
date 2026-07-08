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

/// 获取当前偏好。
///
/// 适用于设置面板初始化、应用启动后恢复用户配置，或在导入/保存后重新同步偏好。
/// 返回值为当前生效的完整偏好快照。
/// 该接口只读取当前内存中的已生效偏好，不会触发磁盘写入或额外副作用。
#[tauri::command]
pub async fn get_preferences(state: State<'_, AppState>) -> Result<AppPreferences, String> {
    Ok(state.preferences())
}

/// 设置偏好（验证后落盘）。
///
/// 适用于用户在设置面板保存配置后的正式提交。
/// 入参 `preferences` 为完整偏好对象；返回值为已经通过校验并写入后的最终偏好。
/// 若下载目录发生变化，该接口会自动触发一次本地库存重新扫描；调用方不需要再额外手动发起扫描。
#[tauri::command]
pub async fn set_preferences(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    preferences: AppPreferences,
) -> Result<AppPreferences, String> {
    let locale = preferences.locale;
    preferences.validate(locale)?;
    let previous = state.preferences();
    state.persist_preferences(preferences.clone()).await?;
    if previous.output_dir != preferences.output_dir {
        spawn_inventory_scan(
            app,
            state.inner().clone(),
            preferences.output_dir.clone(),
            None,
        );
    }
    Ok(preferences)
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
) -> Result<AppPreferences, String> {
    let prefs = state.preferences();
    let locale = prefs.locale;
    let store = state.preferences_store();
    let path = PathBuf::from(output_path);
    let prefs_to_export = prefs.clone();
    tokio::task::spawn_blocking(move || {
        store.export_to(&prefs_to_export, Path::new(&path), locale)
    })
    .await
    .map_err(|e| e.to_string())??;
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
) -> Result<AppPreferences, String> {
    let previous = state.preferences();
    let locale = previous.locale;
    let store = state.preferences_store();
    let input_path = PathBuf::from(input_path);
    let imported = {
        let store = store.clone();
        tokio::task::spawn_blocking(move || store.import_from(Path::new(&input_path), locale))
            .await
            .map_err(|e| e.to_string())??
    };
    let imported_to_save = imported.clone();
    state.persist_preferences(imported_to_save).await?;
    if previous.output_dir != imported.output_dir {
        spawn_inventory_scan(
            app,
            state.inner().clone(),
            imported.output_dir.clone(),
            None,
        );
    }
    Ok(imported)
}

/// 获取通知权限状态字符串。
///
/// 适用于设置面板展示当前系统通知授权状态，或在发送测试通知前决定是否提示用户授权。
/// 返回值为标准化后的权限状态字符串，如 `granted`、`denied` 或 `prompt`。
/// 该接口反映的是当前系统权限快照；若用户刚在系统设置中修改权限，调用方应重新调用以获取最新状态。
#[tauri::command]
pub fn get_notification_permission_state(state: State<'_, AppState>) -> Result<String, String> {
    let app = state.player.app_handle();
    let permission = app
        .notification()
        .permission_state()
        .map_err(|e| format!("{e}"))?;
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
pub fn send_test_notification(state: State<'_, AppState>) -> Result<(), String> {
    let app = state.player.app_handle();
    crate::notification::notify_test(app)
}
