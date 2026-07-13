//! 首页数据聚合相关的 Tauri command。

use crate::app_state::AppState;
use harubble_core::api::Album;
use harubble_core::homepage::{derive_series_tags, HistoryEntry, HomepageStatus, SeriesGroup};
use tauri::State;

/// 获取最新专辑列表。
///
/// 从上游 API 获取全量专辑并取前 N 条，附带本地库存增强。
/// 入参 `limit` 为最多返回的专辑数量；返回值为已经过本地库存增强的专辑列表。
/// 调用方应把该结果视为展示快照：远端数据或本地库存状态变化后，需要重新调用以获取最新结果。
#[tauri::command]
pub async fn get_latest_albums(
    state: State<'_, AppState>,
    limit: u32,
) -> Result<Vec<Album>, String> {
    let albums = state
        .api_client()
        .get_albums()
        .await
        .map_err(|e| e.to_string())?;
    let enriched = state.attach_album_enrichment(albums).await;
    Ok(enriched.into_iter().take(limit as usize).collect())
}

/// 按系列分组获取专辑列表。
///
/// 从 SQLite 读取 belong 映射，与全量专辑做内存 join 分组。
/// 除 belong 分组外，还会从专辑名称中派生额外的系列标签（如 OST、EP），
/// 同一专辑可同时出现在 belong 分组与名称派生分组中。
/// 无 belong 记录且无名称派生标签的专辑不参与分组。
/// 返回值为按系列分组的专辑集合列表，按每组专辑数量降序排列。
/// 调用方应注意：belong 映射来自本地缓存，若缓存尚未写入则分组结果可能为空。
#[tauri::command]
pub async fn get_albums_by_series(state: State<'_, AppState>) -> Result<Vec<SeriesGroup>, String> {
    let albums = state
        .api_client()
        .get_albums()
        .await
        .map_err(|e| e.to_string())?;
    let enriched = state.attach_album_enrichment(albums).await;
    let cache = state.album_metadata_cache().clone();
    let belongs = tokio::task::spawn_blocking(move || cache.get_all_belongs())
        .await
        .map_err(|e| e.to_string())??;

    let belong_map: std::collections::HashMap<&str, &str> = belongs
        .iter()
        .map(|r| (r.album_cid.as_str(), r.belong.as_str()))
        .collect();

    let mut groups: std::collections::HashMap<String, Vec<Album>> =
        std::collections::HashMap::new();

    for album in enriched {
        let belong = belong_map.get(album.cid.as_str()).copied().unwrap_or("");
        let derived = derive_series_tags(&album.name);

        if belong.is_empty() && derived.is_empty() {
            continue;
        }

        if !belong.is_empty() {
            groups
                .entry(belong.to_string())
                .or_default()
                .push(album.clone());
        }
        for tag in derived {
            groups
                .entry(tag.to_string())
                .or_default()
                .push(album.clone());
        }
    }

    let mut result: Vec<SeriesGroup> = groups
        .into_iter()
        .map(|(series, albums)| SeriesGroup { series, albums })
        .collect();
    result.sort_by(|a, b| b.albums.len().cmp(&a.albums.len()));
    Ok(result)
}

/// 获取最近收听历史。
///
/// 从 SQLite 按播放时间倒序返回最近 `limit` 条收听记录。
/// 入参 `limit` 为最多返回的条目数量；返回值为收听历史条目列表。
/// 该接口只读取已持久化的历史，不会触发任何写入或状态变更。
#[tauri::command]
pub async fn get_recent_history(
    state: State<'_, AppState>,
    limit: u32,
) -> Result<Vec<HistoryEntry>, String> {
    let history = state.listening_history().clone();
    tokio::task::spawn_blocking(move || history.get_recent(limit))
        .await
        .map_err(|e| e.to_string())?
}

/// 记录歌曲热度（当播放进度达到阈值时由前端调用）。
///
/// 入参 `song_cid` 为歌曲 CID，`cover_url` 为可选封面 URL。
/// 内部通过 UPSERT 增加该歌曲的热度计数并更新最近播放时间；
/// 若歌曲不存在则先从 API 获取元数据后插入。
/// 该接口应只在前端确认播放进度达到阈值后调用，不应高频轮询。
#[tauri::command]
pub async fn record_song_heat(
    state: State<'_, AppState>,
    song_cid: String,
    cover_url: Option<String>,
) -> Result<(), String> {
    let song_detail = state
        .api_client()
        .get_song_detail(&song_cid)
        .await
        .map_err(|e| e.to_string())?;
    let event = harubble_core::ListeningEvent {
        song_cid,
        song_name: song_detail.name,
        album_cid: song_detail.album_cid,
        album_name: String::new(),
        cover_url,
        artists: song_detail.artists,
    };
    let history = state.listening_history().clone();
    tokio::task::spawn_blocking(move || history.record(&event))
        .await
        .map_err(|e| e.to_string())?
}

/// 清空所有收听历史记录。
///
/// 适用于用户手动清空收听历史面板的场景。
/// 返回值为本次实际删除的记录条数。
/// 该接口会删除所有历史记录，操作不可逆；调用方应在执行前向用户确认。
#[tauri::command]
pub async fn clear_listening_history(state: State<'_, AppState>) -> Result<u32, String> {
    state
        .dispatch_playback_side_effect("clear_listening_history", |state| async move {
            let history = state.listening_history().clone();
            tokio::task::spawn_blocking(move || history.clear())
                .await
                .map_err(|e| e.to_string())?
        })
        .await
}

/// 获取首页状态仪表盘聚合数据。
///
/// 聚合平台专辑总数、本地已下载曲目数、活跃下载数与已完成下载数，供首页仪表盘展示。
/// 返回值为 `HomepageStatus` 快照；`local_storage_bytes` 当前固定返回 `0`，后续版本将补充磁盘用量计算。
/// 该接口会发起一次上游 API 请求与多次本地状态读取，不适合高频轮询。
#[tauri::command]
pub async fn get_homepage_status(state: State<'_, AppState>) -> Result<HomepageStatus, String> {
    state.homepage_status().await
}
