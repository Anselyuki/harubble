//! 共享专辑目录读取与远端刷新相关的 Tauri command。

use crate::album_catalog::AlbumCatalogSnapshot;
use crate::app_state::AppState;
use crate::commands::library::LibraryError;
use tauri::State;

/// 读取共享目录；尚无快照时触发一次首次加载。
#[tauri::command]
pub async fn get_album_catalog(
    state: State<'_, AppState>,
) -> Result<AlbumCatalogSnapshot, LibraryError> {
    let snapshot = state
        .album_catalog()
        .get()
        .await
        .map_err(LibraryError::Network)?;
    let albums = state.attach_album_enrichment(snapshot.albums).await;
    Ok(AlbumCatalogSnapshot {
        albums,
        revision: snapshot.revision,
        checked_at: snapshot.checked_at,
    })
}

/// 定向失效专辑列表缓存，强制检查远端并返回最新共享快照。
#[tauri::command]
pub async fn refresh_album_catalog(
    state: State<'_, AppState>,
) -> Result<AlbumCatalogSnapshot, LibraryError> {
    let snapshot = state
        .album_catalog()
        .refresh()
        .await
        .map_err(LibraryError::Network)?;
    let albums = state.attach_album_enrichment(snapshot.albums).await;
    Ok(AlbumCatalogSnapshot {
        albums,
        revision: snapshot.revision,
        checked_at: snapshot.checked_at,
    })
}
