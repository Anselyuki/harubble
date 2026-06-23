//! 媒体库与详情读取相关的 Tauri command。
//!
//! 当前暴露的接口覆盖专辑列表、专辑详情、单曲详情、歌词文本、远程封面主题提取，
//! 以及远程封面 data URL 转换与默认下载目录建议值读取，主要服务于前端的浏览、播放前预取与展示增强场景。

use crate::app_state::AppState;
use crate::theme;
use base64::Engine;
use tauri::State;

/// 获取专辑列表，并附带本地库存增强后的下载徽标与 tag 信息。
///
/// 适用于首页或专辑浏览视图的初始加载与刷新场景。
/// 入参 `state` 提供共享后端状态与 API 客户端；返回值为已经过本地库存增强与 tag 注入的专辑列表。
/// 调用方应把该结果视为展示快照：远端数据或本地库存状态变化后，需要重新调用以获取最新结果。
#[tauri::command]
pub async fn get_albums(
    state: State<'_, AppState>,
) -> Result<Vec<harubble_core::api::Album>, String> {
    let albums = state.api.get_albums().await.map_err(|e| e.to_string())?;
    let mut enriched = state.local_inventory_service.enrich_albums(albums).await;
    let locale = state.preferences().locale;
    for album in &mut enriched {
        album.tags = state.tag_registry.get_album_tags(&album.cid, locale);
    }
    Ok(enriched)
}

/// 根据专辑 CID 获取专辑详情，并补充本地库存相关信息。
///
/// 适用于进入专辑详情页、刷新当前专辑信息或在下载后重新拉取专辑展示数据。
/// 入参 `album_cid` 为上游专辑唯一标识；返回值为已补齐本地库存状态的专辑详情。
/// 调用方应确保 `album_cid` 来自有效列表项；若 CID 无效或上游请求失败，将返回错误字符串。
/// 该接口会在成功获取详情后顺带更新 belong 缓存（尽力而为，失败不影响主流程返回值）。
#[tauri::command]
pub async fn get_album_detail(
    state: State<'_, AppState>,
    album_cid: String,
) -> Result<harubble_core::api::AlbumDetail, String> {
    let album = state
        .api
        .get_album_detail(&album_cid)
        .await
        .map_err(|e| e.to_string())?;
    let cache = state.album_metadata_cache.clone();
    let album_cid_for_cache = album.cid.clone();
    let album_belong_for_cache = album.belong.clone();
    let _ = tokio::task::spawn_blocking(move || {
        cache.upsert_belong(&album_cid_for_cache, &album_belong_for_cache)
    })
    .await;
    let mut enriched = state
        .local_inventory_service
        .enrich_album_detail(album)
        .await;
    let locale = state.preferences().locale;
    enriched.tags = state.tag_registry.get_album_tags(&enriched.cid, locale);
    for song in &mut enriched.songs {
        song.tags = state
            .tag_registry
            .get_song_tags(&song.cid, &enriched.cid, locale);
    }
    Ok(enriched)
}

/// 根据歌曲 CID 获取单曲详情，并联动所属专辑补齐库存徽标。
///
/// 适用于播放前拉取单曲信息、展示单曲弹层或刷新当前曲目的本地下载状态。
/// 入参 `cid` 为歌曲唯一标识；返回值为结合所属专辑目录信息增强后的单曲详情。
/// 该接口会额外请求一次所属专辑详情以推导库存信息，因此不适合在高频轮询场景中反复调用。
#[tauri::command]
pub async fn get_song_detail(
    state: State<'_, AppState>,
    cid: String,
) -> Result<harubble_core::api::SongDetail, String> {
    let song = state
        .api
        .get_song_detail(&cid)
        .await
        .map_err(|e| e.to_string())?;
    let album = state
        .api
        .get_album_detail(&song.album_cid)
        .await
        .map_err(|e| e.to_string())?;
    let mut enriched = state
        .local_inventory_service
        .enrich_song_detail(song, &album.name)
        .await;
    let locale = state.preferences().locale;
    enriched.tags = state
        .tag_registry
        .get_song_tags(&enriched.cid, &enriched.album_cid, locale);
    Ok(enriched)
}

/// 获取歌曲歌词文本；若上游未提供歌词地址则返回 `None`。
///
/// 适用于歌词面板首次展开或切歌后按需加载歌词内容。
/// 入参 `cid` 为歌曲唯一标识；返回值在成功时要么是歌词文本，要么是显式的 `None`，表示该歌曲没有可下载歌词。
/// 调用方应区分“无歌词”和“请求失败”两类结果：前者返回 `Ok(None)`，后者返回错误字符串。
#[tauri::command]
pub async fn get_song_lyrics(
    state: State<'_, AppState>,
    cid: String,
) -> Result<Option<String>, String> {
    let song_detail = state
        .api
        .get_song_detail(&cid)
        .await
        .map_err(|e| e.to_string())?;

    let Some(lyric_url) = song_detail.lyric_url else {
        return Ok(None);
    };

    state
        .api
        .download_text(&lyric_url)
        .await
        .map(Some)
        .map_err(|e| e.to_string())
}

/// 下载图片并提取主题色调板。
///
/// 适用于封面主题取色、界面动态配色等需要从远端图片推导视觉主题的场景。
/// 入参 `image_url` 为可访问的远端图片地址；返回值为提取后的主题色调板。
/// 该接口会发起网络请求并在阻塞线程中执行图片分析，调用方应避免把它作为高频实时操作。
#[tauri::command]
pub async fn extract_image_theme(
    state: State<'_, AppState>,
    image_url: String,
) -> Result<theme::ThemePalette, String> {
    harubble_core::validate_download_url(&image_url).map_err(|e| e.to_string())?;

    let bytes = state
        .api
        .download_bytes(&image_url, |_, _| {})
        .await
        .map_err(|e| e.to_string())?;

    tokio::task::spawn_blocking(move || theme::extract_theme_palette(&bytes))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

fn encode_image_data_url(mime: &str, bytes: &[u8]) -> String {
    format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

/// 下载图片并返回 data URL，供前端直接渲染。
///
/// 适用于需要把远端封面转换为可直接绑定到 `<img>` 或 CSS 背景的内联资源场景。
/// 入参 `image_url` 为远端图片地址；返回值为带 MIME 前缀的完整 data URL 字符串。
/// 该接口会把整张图片载入内存并进行 Base64 编码，不适合对大图或高频批量列表长时间重复调用。
#[tauri::command]
pub async fn get_image_data_url(
    state: State<'_, AppState>,
    image_url: String,
) -> Result<String, String> {
    harubble_core::validate_download_url(&image_url).map_err(|e| e.to_string())?;

    let bytes = state
        .api
        .download_bytes(&image_url, |_, _| {})
        .await
        .map_err(|e| e.to_string())?;

    let mime =
        harubble_core::audio::detect_image_mime(&bytes).unwrap_or("application/octet-stream");
    Ok(encode_image_data_url(mime, &bytes))
}

/// 返回默认下载输出目录。
///
/// 适用于首次启动或重置偏好时为下载目录提供默认值。
/// 返回值始终为字符串路径：优先使用系统下载目录，其次回退到当前工作目录，再统一追加 `Harubble` 子目录。
/// 该接口只提供默认建议值，不保证目录已经存在，也不会自动创建目录。
#[tauri::command]
pub fn get_default_output_dir() -> String {
    dirs::download_dir()
        .unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"))
        })
        .join("Harubble")
        .to_string_lossy()
        .to_string()
}
