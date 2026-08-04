//! 合集相关的 Tauri command。
//!
//! 提供合集的增删改查、歌曲管理（添加、移除、排序）以及导入导出能力。
//! 官方合集只读，用户合集支持完整写操作。

use crate::app_state::AppState;
use crate::collection::{Collection, CollectionError, CollectionSummary};
use crate::preferences::Locale;
use tauri::State;

/// 将 `Locale` 枚举转换为 BCP 47 字符串。
fn locale_str(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCN => "zh-CN",
        Locale::EnUS => "en-US",
    }
}

/// 列出所有合集（官方 + 用户）。
///
/// 按当前偏好语言本地化官方合集名称与描述。
/// 返回值为合集摘要列表，官方合集在前，用户合集按更新时间倒序排列。
#[tauri::command]
pub fn list_collections(
    state: State<'_, AppState>,
) -> Result<Vec<CollectionSummary>, CollectionError> {
    let locale = locale_str(state.preferences().locale);
    state.collection().list_all(locale)
}

/// 查询单个合集详情，包含完整歌曲 ID 列表。
///
/// 入参 `id` 为合集 ID（官方合集以 `"official:"` 为前缀）。
/// 返回值为合集详情，歌曲 ID 按 position 升序排列。
/// 合集不存在时返回错误。
#[tauri::command]
pub fn get_collection(
    state: State<'_, AppState>,
    id: String,
) -> Result<Collection, CollectionError> {
    let locale = locale_str(state.preferences().locale);
    state.collection().get(&id, locale)
}

/// 创建用户合集。
///
/// 入参 `name` 为合集名称，`description` 为描述，`cover_path` 为封面路径（可为 `null`）。
/// 返回值为新建的合集详情（歌曲列表为空）。
#[tauri::command]
pub fn create_collection(
    state: State<'_, AppState>,
    name: String,
    description: String,
    cover_path: Option<String>,
) -> Result<Collection, CollectionError> {
    state
        .collection()
        .create(&name, &description, cover_path.as_deref())
}

/// 更新用户合集的名称、描述或封面（部分更新）。
///
/// 入参 `id` 为合集 ID，不可为官方合集。
/// `name`、`description`、`cover_path` 均为可选，`null` 表示不修改该字段；
/// `cover_path` 为 `Some(null)` 时表示清除封面。
/// 返回值为更新后的合集详情。
#[tauri::command]
pub fn update_collection(
    state: State<'_, AppState>,
    id: String,
    name: Option<String>,
    description: Option<String>,
    cover_path: Option<Option<String>>,
) -> Result<Collection, CollectionError> {
    // cover_path: None = 不修改, Some(None) = 清除封面, Some(Some(s)) = 设置新封面
    // 需要先将 Option<Option<String>> 转换为 Option<Option<&str>>，借用生命周期需要临时变量
    let cover_ref: Option<Option<&str>> = match &cover_path {
        None => None,
        Some(None) => Some(None),
        Some(Some(s)) => Some(Some(s.as_str())),
    };
    state
        .collection()
        .update(&id, name.as_deref(), description.as_deref(), cover_ref)
}

/// 删除用户合集（级联删除关联歌曲记录）。
///
/// 入参 `id` 为合集 ID，不可为官方合集。
/// 合集不存在时返回错误。
#[tauri::command]
pub fn delete_collection(state: State<'_, AppState>, id: String) -> Result<(), CollectionError> {
    state.collection().delete(&id)
}

/// 向合集添加歌曲（已存在的歌曲忽略，不报错）。
///
/// 入参 `id` 为合集 ID，不可为官方合集；`song_ids` 为要添加的歌曲 ID 列表。
/// 成功时更新合集的 `updated_at` 时间戳。
#[tauri::command]
pub fn add_songs_to_collection(
    state: State<'_, AppState>,
    id: String,
    song_ids: Vec<String>,
) -> Result<(), CollectionError> {
    state.collection().add_songs(&id, &song_ids)
}

/// 从合集移除歌曲。
///
/// 入参 `id` 为合集 ID，不可为官方合集；`song_ids` 为要移除的歌曲 ID 列表。
/// 成功时更新合集的 `updated_at` 时间戳。
#[tauri::command]
pub fn remove_songs_from_collection(
    state: State<'_, AppState>,
    id: String,
    song_ids: Vec<String>,
) -> Result<(), CollectionError> {
    state.collection().remove_songs(&id, &song_ids)
}

/// 对合集中的歌曲重新排序。
///
/// 入参 `id` 为合集 ID，不可为官方合集；`song_ids` 为按新顺序排列的歌曲 ID 列表。
/// 列表中的 index 即为新 position，在事务中批量更新。
#[tauri::command]
pub fn reorder_collection_songs(
    state: State<'_, AppState>,
    id: String,
    song_ids: Vec<String>,
) -> Result<(), CollectionError> {
    state.collection().reorder_songs(&id, &song_ids)
}

/// 将合集导出为 JSON 字符串。
///
/// 入参 `id` 为合集 ID（官方或用户均可导出）。
/// 返回值为格式化的 JSON 字符串（pretty-printed），可用于 `import_collection` 导入。
#[tauri::command]
pub fn export_collection(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, CollectionError> {
    let locale = locale_str(state.preferences().locale);
    state.collection().export(&id, locale)
}

/// 从 JSON 字符串导入合集，创建新的用户合集。
///
/// 入参 `json` 为由 `export_collection` 生成的 JSON 字符串。
/// 返回值为新建的合集详情（ID 为新生成的 UUID v4）。
/// JSON 格式不合法或版本不兼容时返回错误。
#[tauri::command]
pub fn import_collection(
    state: State<'_, AppState>,
    json: String,
) -> Result<Collection, CollectionError> {
    state.collection().import(&json)
}
