use crate::app_state::AppState;
use crate::logging::{LogLevel, LogPayload};
use crate::preferences::Locale;
use crate::search::index::{sanitize_search_request, LibrarySearchIndex};
use crate::search::snapshot::{
    build_library_search_snapshot, load_library_search_snapshot, save_library_search_snapshot,
    LibrarySearchSnapshot,
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

        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            state
                .wait_for_background_io_gate("library_search_rebuild", Duration::from_millis(250))
                .await;

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
