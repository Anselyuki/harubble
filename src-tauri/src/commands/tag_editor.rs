//! Tag 编辑器相关的 Tauri command。
//!
//! 提供双层存储（remote + local overlay）的 CRUD、合并计算、三路合并与冲突解决能力，
//! 以及注册表的导入导出。

use crate::app_state::AppState;
use crate::tag_editor::{ConflictResolution, EntityType, MergeResult};
use crate::tag_registry::{LocalizedValue, TagRegistry, CURRENT_SCHEMA_VERSION};
use std::path::{Path, PathBuf};
use tauri::State;

/// 返回合并后的完整 tag 注册表（remote + local overlay 合并计算结果）。
#[tauri::command]
pub fn get_tag_editor_merged(state: State<'_, AppState>) -> Result<TagRegistry, String> {
    Ok(state.tag_editor.compute_merged())
}

/// 返回本地 overlay 层的原始内容。
#[tauri::command]
pub fn get_tag_editor_local_overlay(state: State<'_, AppState>) -> Result<TagRegistry, String> {
    Ok(state.tag_editor.local_registry())
}

/// 设置指定实体在指定维度上的 tag 值（写入本地 overlay）。
#[tauri::command]
pub async fn set_tag_editor_entity_tag(
    state: State<'_, AppState>,
    entity_type: EntityType,
    cid: String,
    dimension_key: String,
    values: Vec<LocalizedValue>,
) -> Result<(), String> {
    let editor = state.tag_editor.clone();
    tokio::task::spawn_blocking(move || {
        editor.set_entity_tag(entity_type, &cid, &dimension_key, values)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// 删除指定实体在指定维度上的本地 overlay tag。
#[tauri::command]
pub async fn remove_tag_editor_entity_tag(
    state: State<'_, AppState>,
    entity_type: EntityType,
    cid: String,
    dimension_key: String,
) -> Result<(), String> {
    let editor = state.tag_editor.clone();
    tokio::task::spawn_blocking(move || editor.remove_entity_tag(entity_type, &cid, &dimension_key))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// 新增本地维度定义。
#[tauri::command]
pub async fn add_tag_editor_dimension(
    state: State<'_, AppState>,
    key: String,
    label_zh: String,
    label_en: String,
) -> Result<(), String> {
    let editor = state.tag_editor.clone();
    tokio::task::spawn_blocking(move || editor.add_local_dimension(&key, &label_zh, &label_en))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// 删除本地维度定义及其关联的所有 tag 数据。
#[tauri::command]
pub async fn remove_tag_editor_dimension(
    state: State<'_, AppState>,
    key: String,
) -> Result<(), String> {
    let editor = state.tag_editor.clone();
    tokio::task::spawn_blocking(move || editor.remove_local_dimension(&key))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// 接收新的远端快照并执行三路合并，返回合并结果（含冲突列表）。
#[tauri::command]
pub async fn apply_tag_editor_remote_update(
    state: State<'_, AppState>,
    new_remote: TagRegistry,
) -> Result<MergeResult, String> {
    let editor = state.tag_editor.clone();
    tokio::task::spawn_blocking(move || editor.apply_remote_update(new_remote))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// 解决单个三路合并冲突。
#[tauri::command]
pub async fn resolve_tag_editor_conflict(
    state: State<'_, AppState>,
    entity_type: EntityType,
    cid: String,
    dimension_key: String,
    keep: ConflictResolution,
) -> Result<(), String> {
    let editor = state.tag_editor.clone();
    tokio::task::spawn_blocking(move || {
        editor.resolve_conflict(entity_type, &cid, &dimension_key, keep)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// 将合并后的完整 tag 注册表导出到用户指定路径。
///
/// 输出格式与线上 `tag_registry.json` 一致，可直接用于发布。
#[tauri::command]
pub async fn export_tag_editor_registry(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let editor = state.tag_editor.clone();
    let path = PathBuf::from(path);
    tokio::task::spawn_blocking(move || editor.export_merged(Path::new(&path)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// 从用户指定路径导入 tag 注册表文件，作为新 remote 触发三路合并。
///
/// 校验 schema 版本后调用 `apply_remote_update`，返回合并结果（含冲突列表）。
#[tauri::command]
pub async fn import_tag_editor_registry(
    state: State<'_, AppState>,
    path: String,
) -> Result<MergeResult, String> {
    let editor = state.tag_editor.clone();
    let path = PathBuf::from(path);
    tokio::task::spawn_blocking(move || {
        let content =
            std::fs::read_to_string(Path::new(&path)).map_err(|e| format!("读取文件失败: {e}"))?;
        let registry: TagRegistry =
            serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {e}"))?;
        if registry.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(format!(
                "schema 版本不匹配: 文件为 {}，当前支持 {}",
                registry.schema_version, CURRENT_SCHEMA_VERSION
            ));
        }
        editor
            .apply_remote_update(registry)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
