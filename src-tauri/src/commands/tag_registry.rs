//! 自定义元数据 Tag Registry 相关的 Tauri command。

use crate::app_state::AppState;
use harubble_core::api::Album;
use harubble_core::homepage::TagGroup;
use tauri::State;

/// 获取适用于专辑粒度的 tag 维度列表。
///
/// 过滤掉 `scope = "song"` 的维度（如 "event"），仅返回可用于专辑分组浏览的维度。
/// 若 tag registry 为空或未加载，返回空数组。
#[tauri::command]
pub fn get_tag_dimensions(
    state: State<'_, AppState>,
) -> Result<Vec<crate::tag_registry::TagDimensionResolved>, String> {
    let locale = state.preferences().locale;
    Ok(state.tag_registry().get_album_dimensions(locale))
}

/// 按指定 tag 维度聚合专辑，用于分组浏览。
///
/// 入参 `dimension_key` 为维度程序标识符（如 "faction"、"genre"）。
/// 返回值为按 tag 值分组的专辑列表，每组内的 `value` 已本地化。
#[tauri::command]
pub async fn get_albums_by_tag_dimension(
    state: State<'_, AppState>,
    dimension_key: String,
) -> Result<Vec<TagGroup>, String> {
    let locale = state.preferences().locale;
    let value_to_cids = state
        .tag_registry()
        .get_album_cids_by_dimension(&dimension_key, locale);

    if value_to_cids.is_empty() {
        return Ok(Vec::new());
    }

    let albums = state
        .api_client()
        .get_albums()
        .await
        .map_err(|e| e.to_string())?;
    let enriched = state.attach_album_enrichment(albums).await;

    let album_map: std::collections::HashMap<&str, &Album> =
        enriched.iter().map(|a| (a.cid.as_str(), a)).collect();

    let mut groups: Vec<TagGroup> = value_to_cids
        .into_iter()
        .filter_map(|(value, cids)| {
            let albums: Vec<Album> = cids
                .iter()
                .filter_map(|cid| album_map.get(cid.as_str()).copied().cloned())
                .collect();
            if albums.is_empty() {
                return None;
            }
            Some(TagGroup {
                dimension_key: dimension_key.clone(),
                value,
                albums,
            })
        })
        .collect();

    groups.sort_by(|a, b| b.albums.len().cmp(&a.albums.len()));
    Ok(groups)
}
