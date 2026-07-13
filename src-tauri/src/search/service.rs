use crate::app_state::AppState;
use crate::logging::{LogLevel, LogPayload};
use crate::preferences::Locale;
use crate::search::index::{sanitize_search_request, LibrarySearchIndex};
use crate::search::snapshot::{
    build_library_search_snapshot, load_library_search_snapshot, save_library_search_snapshot,
    LibrarySearchAlbumRecord, LibrarySearchSnapshot, LibrarySearchSongRecord,
};
use anyhow::Result;
use harubble_core::{
    LibraryIndexState, LocalInventorySnapshot, LocalInventoryStatus, SearchLibraryRequest,
    SearchLibraryResponse, SEARCH_LIBRARY_MAX_LIMIT, SEARCH_LIBRARY_MAX_OFFSET,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Clone)]
pub(crate) struct LibrarySearchService {
    base_dir: PathBuf,
    state: Arc<Mutex<LibrarySearchState>>,
}

struct LibrarySearchState {
    index_state: LibraryIndexState,
    current_output_dir: String,
    current_inventory_version: Option<String>,
    last_ready_output_dir: Option<String>,
    last_ready_inventory_version: Option<String>,
    active_index: Option<Arc<LibrarySearchIndex>>,
    build_generation: u64,
}

impl LibrarySearchService {
    pub(crate) fn new(base_dir: PathBuf, current_output_dir: String) -> Self {
        let mut state = LibrarySearchState {
            index_state: LibraryIndexState::NotReady,
            current_output_dir: current_output_dir.clone(),
            current_inventory_version: None,
            last_ready_output_dir: None,
            last_ready_inventory_version: None,
            active_index: None,
            build_generation: 0,
        };

        if let Ok(Some(snapshot)) = load_library_search_snapshot(&base_dir) {
            if let Ok(index) = LibrarySearchIndex::open(&base_dir, &snapshot.inventory_version) {
                state.last_ready_output_dir = Some(snapshot.root_output_dir.clone());
                state.last_ready_inventory_version = Some(snapshot.inventory_version.clone());
                state.active_index = Some(Arc::new(index));
                state.index_state = if snapshot.root_output_dir == current_output_dir {
                    LibraryIndexState::Stale
                } else {
                    LibraryIndexState::NotReady
                };
            }
        }

        Self {
            base_dir,
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub(crate) async fn prepare_for_inventory_scan(&self, root_output_dir: String) {
        let mut state = self.state.lock().await;
        state.current_output_dir = root_output_dir.clone();
        state.current_inventory_version = None;
        state.index_state = if state.active_index.is_some()
            && state.last_ready_output_dir.as_deref() == Some(root_output_dir.as_str())
        {
            LibraryIndexState::Stale
        } else {
            LibraryIndexState::NotReady
        };
    }

    pub(crate) async fn start_rebuild(&self, inventory: &LocalInventorySnapshot) -> u64 {
        let mut state = self.state.lock().await;
        state.build_generation = state.build_generation.saturating_add(1);
        state.current_output_dir = inventory.root_output_dir.clone();
        state.current_inventory_version = Some(inventory.inventory_version.clone());
        state.index_state = LibraryIndexState::Building;
        state.build_generation
    }

    pub(crate) async fn publish_rebuild(
        &self,
        generation: u64,
        snapshot: &LibrarySearchSnapshot,
        index: LibrarySearchIndex,
    ) -> bool {
        let mut state = self.state.lock().await;
        if state.build_generation != generation
            || state.current_inventory_version.as_deref()
                != Some(snapshot.inventory_version.as_str())
            || state.current_output_dir != snapshot.root_output_dir
        {
            return false;
        }

        state.last_ready_output_dir = Some(snapshot.root_output_dir.clone());
        state.last_ready_inventory_version = Some(snapshot.inventory_version.clone());
        state.active_index = Some(Arc::new(index));
        state.index_state = LibraryIndexState::Ready;
        true
    }

    pub(crate) async fn fail_rebuild(
        &self,
        generation: u64,
        root_output_dir: &str,
        inventory_version: &str,
    ) {
        let mut state = self.state.lock().await;
        if state.build_generation != generation
            || state.current_inventory_version.as_deref() != Some(inventory_version)
            || state.current_output_dir != root_output_dir
        {
            return;
        }

        state.index_state = if state.active_index.is_some()
            && state.last_ready_output_dir.as_deref() == Some(root_output_dir)
        {
            LibraryIndexState::Stale
        } else {
            LibraryIndexState::NotReady
        };
    }

    /// 从当前磁盘快照读取"歌曲 CID → 专辑 CID"映射。
    ///
    /// 适用于需要将歌曲级 tag 变更回溯到所属专辑的场景（例如远端 tag registry
    /// 增量同步）。由于 tag registry 本身不存储歌曲与专辑的父子关系，本方法
    /// 通过读取搜索快照来重建这一映射。
    ///
    /// # 返回值
    /// - `Some(map)` 当前存在可读的搜索快照，返回快照中所有歌曲的 CID 到专辑
    ///   CID 的映射。
    /// - `None` 快照缺失或读取失败，调用方应回退为全量重建。
    ///
    /// # 注意
    /// - 该方法不会阻塞主状态锁；读取操作在 `spawn_blocking` 中执行，避免同步
    ///   IO 影响 tokio 调度。
    pub(crate) async fn current_song_album_map(
        &self,
    ) -> Option<std::collections::HashMap<String, String>> {
        let base_dir = self.base_dir.clone();
        tokio::task::spawn_blocking(move || {
            let snapshot = load_library_search_snapshot(&base_dir).ok().flatten()?;
            let mut map = std::collections::HashMap::with_capacity(snapshot.songs.len());
            for song in &snapshot.songs {
                map.insert(song.song_cid.clone(), song.album_cid.clone());
            }
            Some(map)
        })
        .await
        .ok()
        .flatten()
    }

    /// 应用一批增量 tag 变更到活跃索引与磁盘快照。
    ///
    /// 用于远端 tag registry 同步或本地 tag 编辑仅影响 tag_values 字段的场景：
    /// 只重建受影响专辑的 pinyin 派生字段与 tag_values，然后调用 Tantivy 的
    /// `upsert_albums` 原子替换文档；成功后原地更新磁盘快照对应记录，避免下次
    /// 启动时读取到过期的 tag 内容。
    ///
    /// # 参数
    /// - `album_updates`：受影响专辑的新版记录及其所属歌曲的新版记录。
    ///
    /// # 返回值
    /// - `Ok(true)` 增量成功。
    /// - `Ok(false)` 当前无活跃索引 / 快照，调用方应触发全量重建。
    /// - `Err(...)` 增量过程中发生错误（Tantivy 写入失败 / 快照持久化失败等），
    ///   调用方应记录日志并回退到全量重建。
    ///
    /// # 注意
    /// - 不改动 `build_generation`，增量搭乘当前活跃版本；
    /// - 若在写入过程中 `inventory_version` 发生漂移（例如全量重建刚刚完成），
    ///   仅跳过快照持久化，Tantivy 侧仍会应用变更，下次全量重建将修复快照。
    pub(crate) async fn apply_incremental_tag_update(
        &self,
        album_updates: Vec<(LibrarySearchAlbumRecord, Vec<LibrarySearchSongRecord>)>,
    ) -> Result<bool> {
        let (active_index, snapshot_meta) = {
            let state = self.state.lock().await;
            let Some(index) = state.active_index.clone() else {
                return Ok(false);
            };
            let Some(inventory_version) = state.last_ready_inventory_version.clone() else {
                return Ok(false);
            };
            let Some(root_output_dir) = state.last_ready_output_dir.clone() else {
                return Ok(false);
            };
            (index, (root_output_dir, inventory_version))
        };

        let base_dir = self.base_dir.clone();
        let updates_for_index = album_updates.clone();
        let (_root_output_dir, inventory_version_for_snapshot) = snapshot_meta;

        let write_result = tokio::task::spawn_blocking(move || -> Result<()> {
            active_index.upsert_albums(&updates_for_index)?;
            if let Some(mut snapshot) = load_library_search_snapshot(&base_dir)? {
                if snapshot.inventory_version == inventory_version_for_snapshot {
                    for (album, songs) in &album_updates {
                        if let Some(idx) = snapshot
                            .albums
                            .iter()
                            .position(|a| a.album_cid == album.album_cid)
                        {
                            snapshot.albums[idx] = album.clone();
                        } else {
                            snapshot.albums.push(album.clone());
                        }
                        snapshot.songs.retain(|s| s.album_cid != album.album_cid);
                        snapshot.songs.extend(songs.iter().cloned());
                    }
                    save_library_search_snapshot(&base_dir, &snapshot)?;
                }
            }
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("spawn_blocking join failed: {e}"))?;

        write_result?;
        Ok(true)
    }

    pub(crate) async fn search(
        &self,
        request: SearchLibraryRequest,
        locale: Locale,
    ) -> Result<SearchLibraryResponse, String> {
        let sanitized = sanitize_search_request(
            request,
            SEARCH_LIBRARY_MAX_LIMIT,
            SEARCH_LIBRARY_MAX_OFFSET,
            locale,
        )
        .map_err(|error| error.to_string())?;

        let (index_state, active_index) = {
            let state = self.state.lock().await;
            (state.index_state, state.active_index.clone())
        };

        if index_state != LibraryIndexState::Ready {
            return Ok(SearchLibraryResponse::empty(
                sanitized.query,
                sanitized.scope,
                index_state,
            ));
        }

        let Some(active_index) = active_index else {
            return Ok(SearchLibraryResponse::empty(
                sanitized.query,
                sanitized.scope,
                LibraryIndexState::NotReady,
            ));
        };

        let sanitized_for_search = sanitized.clone();
        let search_result =
            tokio::task::spawn_blocking(move || active_index.search(&sanitized_for_search))
                .await
                .map_err(|error| error.to_string())?
                .map_err(|error| error.to_string())?;
        let (items, total) = search_result;

        Ok(SearchLibraryResponse {
            items,
            total,
            query: sanitized.query,
            scope: sanitized.scope,
            index_state: LibraryIndexState::Ready,
        })
    }

    pub(crate) fn schedule_rebuild(&self, state: AppState, inventory: LocalInventorySnapshot) {
        if inventory.status != LocalInventoryStatus::Completed {
            return;
        }

        let directory = state.task_directory.clone();
        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            let task_id = directory.next_task_id("library_search", "rebuild").await;
            crate::background_tasks::spawn_tracked(
                directory,
                task_id,
                move |cancel_token| async move {
                    tokio::select! {
                        _ = cancel_token.cancelled() => { return; }
                        _ = state.wait_for_background_io_gate(
                            "library_search_rebuild",
                            Duration::from_millis(250),
                        ) => {}
                    }
                    if cancel_token.is_cancelled() {
                        return;
                    }

                    let generation = service.start_rebuild(&inventory).await;
                    let snapshot_result = build_library_search_snapshot(
                        state.api_clients.api.clone(),
                        state.tag_registry.clone(),
                        inventory.root_output_dir.clone(),
                        inventory.inventory_version.clone(),
                    )
                    .await;

                    let snapshot = match snapshot_result {
                        Ok(snapshot) => snapshot,
                        Err(error) => {
                            state.record_log(
                                LogPayload::new(
                                    LogLevel::Warn,
                                    "library-search",
                                    "library_search.snapshot_build_failed",
                                    "Failed to build search snapshot",
                                )
                                .user_message(crate::i18n::tr(
                                    state.preferences().locale,
                                    "search-index-build-failed",
                                ))
                                .details(error.to_string()),
                            );
                            service
                                .fail_rebuild(
                                    generation,
                                    &inventory.root_output_dir,
                                    &inventory.inventory_version,
                                )
                                .await;
                            return;
                        }
                    };

                    let base_dir = service.base_dir.clone();
                    let snapshot_for_build = snapshot.clone();
                    let build_result = tokio::task::spawn_blocking(move || -> Result<_> {
                        save_library_search_snapshot(&base_dir, &snapshot_for_build)?;
                        LibrarySearchIndex::build(&base_dir, &snapshot_for_build)
                    })
                    .await;

                    let index = match build_result {
                        Ok(Ok(index)) => index,
                        Ok(Err(error)) => {
                            state.record_log(
                                LogPayload::new(
                                    LogLevel::Warn,
                                    "library-search",
                                    "library_search.index_build_failed",
                                    "Failed to build search index",
                                )
                                .user_message(crate::i18n::tr(
                                    state.preferences().locale,
                                    "search-index-build-failed",
                                ))
                                .details(error.to_string()),
                            );
                            service
                                .fail_rebuild(
                                    generation,
                                    &inventory.root_output_dir,
                                    &inventory.inventory_version,
                                )
                                .await;
                            return;
                        }
                        Err(error) => {
                            state.record_log(
                                LogPayload::new(
                                    LogLevel::Warn,
                                    "library-search",
                                    "library_search.index_build_join_failed",
                                    "Search index build worker failed",
                                )
                                .user_message(crate::i18n::tr(
                                    state.preferences().locale,
                                    "search-index-build-failed",
                                ))
                                .details(error.to_string()),
                            );
                            service
                                .fail_rebuild(
                                    generation,
                                    &inventory.root_output_dir,
                                    &inventory.inventory_version,
                                )
                                .await;
                            return;
                        }
                    };

                    if !service.publish_rebuild(generation, &snapshot, index).await {
                        state.record_log(
                            LogPayload::new(
                                LogLevel::Info,
                                "library-search",
                                "library_search.rebuild_discarded",
                                "Discarded stale search rebuild result",
                            )
                            .details(format!(
                                "inventoryVersion={} rootOutputDir={}",
                                inventory.inventory_version, inventory.root_output_dir
                            )),
                        );
                    }
                },
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::LibrarySearchService;
    use crate::preferences::Locale;
    use crate::search::index::LibrarySearchIndex;
    use crate::search::snapshot::LibrarySearchSnapshot;
    use crate::search::snapshot::{LibrarySearchAlbumRecord, LibrarySearchSongRecord};
    use harubble_core::{LibraryIndexState, LocalInventorySnapshot, LocalInventoryStatus};
    use tempfile::tempdir;

    fn inventory_snapshot(version: &str) -> LocalInventorySnapshot {
        LocalInventorySnapshot {
            root_output_dir: "/tmp/music".to_string(),
            status: LocalInventoryStatus::Completed,
            inventory_version: version.to_string(),
            started_at: None,
            finished_at: None,
            scanned_file_count: 0,
            matched_track_count: 0,
            verified_track_count: 0,
            last_error: None,
        }
    }

    fn search_snapshot(version: &str) -> LibrarySearchSnapshot {
        LibrarySearchSnapshot {
            root_output_dir: "/tmp/music".to_string(),
            inventory_version: version.to_string(),
            built_at: "2026-01-01T00:00:00Z".to_string(),
            albums: vec![LibrarySearchAlbumRecord {
                album_cid: "album-a".to_string(),
                album_title: "Alpha".to_string(),
                artist_line: Some("Artist".to_string()),
                intro: None,
                belong: None,
                album_title_pinyin_full: None,
                album_title_pinyin_initials: None,
                artist_line_pinyin_full: None,
                artist_line_pinyin_initials: None,
                belong_pinyin_full: None,
                belong_pinyin_initials: None,
                tag_values: None,
                tag_values_pinyin_full: None,
                tag_values_pinyin_initials: None,
            }],
            songs: vec![LibrarySearchSongRecord {
                album_cid: "album-a".to_string(),
                song_cid: "song-a1".to_string(),
                album_title: "Alpha".to_string(),
                song_title: "Beacon".to_string(),
                artist_line: Some("Artist".to_string()),
                song_title_pinyin_full: None,
                song_title_pinyin_initials: None,
                artist_line_pinyin_full: None,
                artist_line_pinyin_initials: None,
                tag_values: None,
                tag_values_pinyin_full: None,
                tag_values_pinyin_initials: None,
            }],
        }
    }

    #[tokio::test]
    async fn transitions_from_not_ready_to_building() {
        let temp_dir = tempdir().expect("temp dir");
        let service =
            LibrarySearchService::new(temp_dir.path().to_path_buf(), "/tmp/music".to_string());
        service
            .prepare_for_inventory_scan("/tmp/music".to_string())
            .await;
        let generation = service.start_rebuild(&inventory_snapshot("inv-1")).await;
        assert_eq!(generation, 1);
        let response = service
            .search(
                harubble_core::SearchLibraryRequest {
                    query: "alpha".to_string(),
                    scope: harubble_core::LibrarySearchScope::All,
                    limit: None,
                    offset: None,
                },
                Locale::default(),
            )
            .await
            .expect("response");
        assert_eq!(response.index_state, LibraryIndexState::Building);
    }

    #[tokio::test]
    async fn returns_ready_results_after_publish() {
        let temp_dir = tempdir().expect("temp dir");
        let service =
            LibrarySearchService::new(temp_dir.path().to_path_buf(), "/tmp/music".to_string());
        let inventory = inventory_snapshot("inv-1");
        let generation = service.start_rebuild(&inventory).await;
        let snapshot = search_snapshot("inv-1");
        let index = LibrarySearchIndex::build(temp_dir.path(), &snapshot).expect("index");
        assert!(service.publish_rebuild(generation, &snapshot, index).await);

        let response = service
            .search(
                harubble_core::SearchLibraryRequest {
                    query: "alpha".to_string(),
                    scope: harubble_core::LibrarySearchScope::All,
                    limit: None,
                    offset: None,
                },
                Locale::default(),
            )
            .await
            .expect("response");
        assert_eq!(response.index_state, LibraryIndexState::Ready);
        assert_eq!(response.total, 1);
    }

    #[tokio::test]
    async fn falls_back_to_stale_when_rebuild_fails_with_previous_index() {
        let temp_dir = tempdir().expect("temp dir");
        let service =
            LibrarySearchService::new(temp_dir.path().to_path_buf(), "/tmp/music".to_string());
        let ready_generation = service.start_rebuild(&inventory_snapshot("inv-1")).await;
        let ready_snapshot = search_snapshot("inv-1");
        let ready_index =
            LibrarySearchIndex::build(temp_dir.path(), &ready_snapshot).expect("index");
        assert!(
            service
                .publish_rebuild(ready_generation, &ready_snapshot, ready_index)
                .await
        );

        let rebuild_generation = service.start_rebuild(&inventory_snapshot("inv-2")).await;
        service
            .fail_rebuild(rebuild_generation, "/tmp/music", "inv-2")
            .await;

        let response = service
            .search(
                harubble_core::SearchLibraryRequest {
                    query: "alpha".to_string(),
                    scope: harubble_core::LibrarySearchScope::All,
                    limit: None,
                    offset: None,
                },
                Locale::default(),
            )
            .await
            .expect("response");
        assert_eq!(response.index_state, LibraryIndexState::Stale);
    }
}
