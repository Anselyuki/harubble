//! 应用级专辑目录快照、远端刷新协调与跨窗口通知。

use harubble_core::{Album, ApiClient};
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::sync::{watch, Mutex};

pub(crate) const ALBUM_CATALOG_REFRESHED: &str = "album-catalog-refreshed";

/// 前端消费的共享专辑目录快照。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumCatalogSnapshot {
    pub albums: Vec<Album>,
    pub revision: u64,
    pub checked_at: u64,
}

/// 每次成功强制检查远端目录后广播的轻量通知。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumCatalogRefreshedEvent {
    pub revision: u64,
    pub checked_at: u64,
    pub changed: bool,
    pub album_count: usize,
}

type CatalogFetchResult = Result<AlbumCatalogSnapshot, String>;
type AlbumFetchFuture = Pin<Box<dyn Future<Output = Result<Vec<Album>, String>> + Send>>;
type AlbumFetcher = Arc<dyn Fn() -> AlbumFetchFuture + Send + Sync>;
type CacheInvalidator = Arc<dyn Fn() + Send + Sync>;
type RefreshEventEmitter = Arc<dyn Fn(AlbumCatalogRefreshedEvent) + Send + Sync>;

struct InflightCatalogRequest {
    request_id: u64,
    result: watch::Sender<Option<CatalogFetchResult>>,
}

#[derive(Default)]
struct AlbumCatalogState {
    albums: Option<Vec<Album>>,
    fingerprint: Option<[u8; 16]>,
    revision: u64,
    checked_at: u64,
    next_request_id: u64,
    in_flight: Option<InflightCatalogRequest>,
}

/// 统一持有原始目录快照并合并并发远端请求的应用级服务。
#[derive(Clone)]
pub(crate) struct AlbumCatalogService {
    fetch_albums: AlbumFetcher,
    invalidate_albums_cache: CacheInvalidator,
    emit_refreshed: RefreshEventEmitter,
    state: Arc<Mutex<AlbumCatalogState>>,
}

impl AlbumCatalogService {
    pub(crate) fn new(api: Arc<ApiClient>, app: AppHandle) -> Self {
        let fetch_api = Arc::clone(&api);
        let fetch_albums: AlbumFetcher = Arc::new(move || {
            let api = Arc::clone(&fetch_api);
            Box::pin(async move { api.get_albums().await.map_err(|error| error.to_string()) })
        });
        let invalidate_albums_cache: CacheInvalidator = Arc::new(move || {
            api.invalidate_albums_response_cache();
        });
        let emit_refreshed: RefreshEventEmitter = Arc::new(move |event| {
            let _ = app.emit(ALBUM_CATALOG_REFRESHED, event);
        });
        Self::new_with_dependencies(fetch_albums, invalidate_albums_cache, emit_refreshed)
    }

    fn new_with_dependencies(
        fetch_albums: AlbumFetcher,
        invalidate_albums_cache: CacheInvalidator,
        emit_refreshed: RefreshEventEmitter,
    ) -> Self {
        Self {
            fetch_albums,
            invalidate_albums_cache,
            emit_refreshed,
            state: Arc::new(Mutex::new(AlbumCatalogState::default())),
        }
    }

    /// 返回当前共享目录；尚无快照时执行一次首次加载。
    pub(crate) async fn get(&self) -> CatalogFetchResult {
        self.load(false).await
    }

    /// 定向失效专辑列表缓存并执行一次强制远端检查。
    pub(crate) async fn refresh(&self) -> CatalogFetchResult {
        self.load(true).await
    }

    /// 清除当前目录内容，但保留 revision 与指纹以便下次拉取正确判断内容变化。
    pub(crate) async fn clear_snapshot(&self) {
        let mut state = self.state.lock().await;
        state.albums = None;
        state.checked_at = 0;
        state.in_flight = None;
    }

    async fn load(&self, force_refresh: bool) -> CatalogFetchResult {
        let mut request_to_start = None;
        let receiver = {
            let mut state = self.state.lock().await;

            if let Some(in_flight) = &state.in_flight {
                in_flight.result.subscribe()
            } else if !force_refresh {
                if let Some(snapshot) = snapshot_from_state(&state) {
                    return Ok(snapshot);
                }
                (self.invalidate_albums_cache)();
                start_catalog_request(&mut state, &mut request_to_start)
            } else {
                (self.invalidate_albums_cache)();
                start_catalog_request(&mut state, &mut request_to_start)
            }
        };

        if let Some((request_id, sender)) = request_to_start {
            self.spawn_fetch(request_id, sender);
        }

        wait_for_catalog_result(receiver).await
    }

    fn spawn_fetch(&self, request_id: u64, sender: watch::Sender<Option<CatalogFetchResult>>) {
        let service = self.clone();
        tokio::spawn(async move {
            let fetched = (service.fetch_albums)().await;
            let result = service.finish_fetch(request_id, fetched).await;
            let _ = sender.send(Some(result));
        });
    }

    async fn finish_fetch(
        &self,
        request_id: u64,
        fetched: Result<Vec<Album>, String>,
    ) -> CatalogFetchResult {
        let (result, event) = {
            let mut state = self.state.lock().await;
            let Some(in_flight) = state.in_flight.as_ref() else {
                return Err("album catalog request completed without in-flight state".to_string());
            };
            if in_flight.request_id != request_id {
                return Err("album catalog request was superseded".to_string());
            }
            let (result, event) = match fetched {
                Ok(albums) => {
                    let checked_at = unix_timestamp_ms();
                    let (snapshot, changed) = apply_catalog_update(&mut state, albums, checked_at);
                    let event = Some(AlbumCatalogRefreshedEvent {
                        revision: snapshot.revision,
                        checked_at: snapshot.checked_at,
                        changed,
                        album_count: snapshot.albums.len(),
                    });
                    (Ok(snapshot), event)
                }
                Err(error) => (Err(error), None),
            };
            state.in_flight = None;
            (result, event)
        };

        if let Some(event) = event {
            (self.emit_refreshed)(event);
        }
        result
    }
}

fn start_catalog_request(
    state: &mut AlbumCatalogState,
    request_to_start: &mut Option<(u64, watch::Sender<Option<CatalogFetchResult>>)>,
) -> watch::Receiver<Option<CatalogFetchResult>> {
    state.next_request_id = state.next_request_id.wrapping_add(1);
    let request_id = state.next_request_id;
    let (sender, receiver) = watch::channel(None);
    state.in_flight = Some(InflightCatalogRequest {
        request_id,
        result: sender.clone(),
    });
    *request_to_start = Some((request_id, sender));
    receiver
}

async fn wait_for_catalog_result(
    mut receiver: watch::Receiver<Option<CatalogFetchResult>>,
) -> CatalogFetchResult {
    loop {
        if let Some(result) = receiver.borrow_and_update().clone() {
            return result;
        }
        receiver
            .changed()
            .await
            .map_err(|_| "album catalog request ended before publishing its result".to_string())?;
    }
}

fn snapshot_from_state(state: &AlbumCatalogState) -> Option<AlbumCatalogSnapshot> {
    state.albums.as_ref().map(|albums| AlbumCatalogSnapshot {
        albums: albums.clone(),
        revision: state.revision,
        checked_at: state.checked_at,
    })
}

fn apply_catalog_update(
    state: &mut AlbumCatalogState,
    albums: Vec<Album>,
    checked_at: u64,
) -> (AlbumCatalogSnapshot, bool) {
    let fingerprint = catalog_fingerprint(&albums);
    let changed = state.fingerprint != Some(fingerprint);
    if changed || state.albums.is_none() {
        state.albums = Some(albums);
        state.fingerprint = Some(fingerprint);
    }
    if changed {
        state.revision = state.revision.saturating_add(1);
    }
    state.checked_at = checked_at;

    (
        snapshot_from_state(state).expect("catalog update always creates a snapshot"),
        changed,
    )
}

fn catalog_fingerprint(albums: &[Album]) -> [u8; 16] {
    let upstream_fields = albums
        .iter()
        .map(|album| {
            (
                album.cid.as_str(),
                album.name.as_str(),
                album.cover_url.as_str(),
                album.artists.as_slice(),
            )
        })
        .collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&upstream_fields)
        .expect("album catalog upstream fields are always serializable");
    md5::compute(bytes).0
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_catalog_update, AlbumCatalogRefreshedEvent, AlbumCatalogService, AlbumCatalogState,
        AlbumFetcher, CacheInvalidator, RefreshEventEmitter,
    };
    use harubble_core::{Album, AlbumDownloadBadge};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex as StdMutex};
    use std::time::Duration;

    fn album(cid: &str, name: &str) -> Album {
        Album {
            cid: cid.to_string(),
            name: name.to_string(),
            cover_url: format!("https://example.com/{cid}.jpg"),
            artists: vec!["Test Artist".to_string()],
            download: AlbumDownloadBadge::default(),
            tags: Vec::new(),
        }
    }

    #[test]
    fn revision_only_advances_when_upstream_catalog_changes() {
        let mut state = AlbumCatalogState::default();

        let (initial, initial_changed) =
            apply_catalog_update(&mut state, vec![album("alpha", "Alpha")], 100);
        assert!(initial_changed);
        assert_eq!(initial.revision, 1);
        assert_eq!(initial.checked_at, 100);

        let (unchanged, unchanged_changed) =
            apply_catalog_update(&mut state, vec![album("alpha", "Alpha")], 200);
        assert!(!unchanged_changed);
        assert_eq!(unchanged.revision, 1);
        assert_eq!(unchanged.checked_at, 200);

        let (updated, updated_changed) = apply_catalog_update(
            &mut state,
            vec![album("alpha", "Alpha"), album("beta", "Beta")],
            300,
        );
        assert!(updated_changed);
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.checked_at, 300);
        assert_eq!(updated.albums.len(), 2);
    }

    #[test]
    fn snapshot_and_event_use_camel_case_wire_fields() {
        let mut state = AlbumCatalogState::default();
        let (snapshot, changed) =
            apply_catalog_update(&mut state, vec![album("alpha", "Alpha")], 123);
        let event = AlbumCatalogRefreshedEvent {
            revision: snapshot.revision,
            checked_at: snapshot.checked_at,
            changed,
            album_count: snapshot.albums.len(),
        };

        let snapshot_json = serde_json::to_value(snapshot).expect("serialize snapshot");
        assert_eq!(snapshot_json["checkedAt"], 123);
        assert!(snapshot_json.get("checked_at").is_none());
        let event_json = serde_json::to_value(event).expect("serialize event");
        assert_eq!(event_json["checkedAt"], 123);
        assert_eq!(event_json["albumCount"], 1);
        assert!(event_json.get("album_count").is_none());
    }

    fn test_service(
        delay: Duration,
    ) -> (
        AlbumCatalogService,
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
        Arc<StdMutex<Vec<AlbumCatalogRefreshedEvent>>>,
    ) {
        let fetch_count = Arc::new(AtomicUsize::new(0));
        let fetch_count_for_call = Arc::clone(&fetch_count);
        let fetcher: AlbumFetcher = Arc::new(move || {
            fetch_count_for_call.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                tokio::time::sleep(delay).await;
                Ok(vec![album("alpha", "Alpha")])
            })
        });

        let invalidation_count = Arc::new(AtomicUsize::new(0));
        let invalidation_count_for_call = Arc::clone(&invalidation_count);
        let invalidator: CacheInvalidator = Arc::new(move || {
            invalidation_count_for_call.fetch_add(1, Ordering::SeqCst);
        });

        let events = Arc::new(StdMutex::new(Vec::new()));
        let events_for_call = Arc::clone(&events);
        let emitter: RefreshEventEmitter = Arc::new(move |event| {
            events_for_call
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push(event);
        });

        (
            AlbumCatalogService::new_with_dependencies(fetcher, invalidator, emitter),
            fetch_count,
            invalidation_count,
            events,
        )
    }

    #[tokio::test]
    async fn concurrent_force_refreshes_share_one_remote_request() {
        let (service, fetch_count, invalidation_count, events) =
            test_service(Duration::from_millis(50));

        let (first, second) = tokio::join!(service.refresh(), service.refresh());

        assert_eq!(first.expect("first refresh").revision, 1);
        assert_eq!(second.expect("second refresh").revision, 1);
        assert_eq!(fetch_count.load(Ordering::SeqCst), 1);
        assert_eq!(invalidation_count.load(Ordering::SeqCst), 1);
        assert_eq!(events.lock().expect("events").len(), 1);
    }

    #[tokio::test]
    async fn force_refresh_joins_an_active_initial_get() {
        let (service, fetch_count, invalidation_count, events) =
            test_service(Duration::from_millis(50));
        let get_service = service.clone();
        let initial_get = tokio::spawn(async move { get_service.get().await });
        while fetch_count.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }

        let refresh = service.refresh().await.expect("refresh");
        let initial = initial_get
            .await
            .expect("initial get task")
            .expect("initial get");

        assert_eq!(initial.revision, 1);
        assert_eq!(refresh.revision, 1);
        assert_eq!(fetch_count.load(Ordering::SeqCst), 1);
        assert_eq!(invalidation_count.load(Ordering::SeqCst), 1);
        assert_eq!(events.lock().expect("events").len(), 1);
    }

    #[tokio::test]
    async fn unchanged_forced_check_emits_without_advancing_revision() {
        let (service, fetch_count, invalidation_count, events) =
            test_service(Duration::from_millis(1));

        let initial = service.get().await.expect("initial get");
        let refreshed = service.refresh().await.expect("refresh");

        assert_eq!(initial.revision, 1);
        assert_eq!(refreshed.revision, 1);
        assert_eq!(fetch_count.load(Ordering::SeqCst), 2);
        assert_eq!(invalidation_count.load(Ordering::SeqCst), 2);
        let events = events.lock().expect("events");
        assert_eq!(events.len(), 2);
        assert!(events[0].changed);
        assert!(!events[1].changed);
    }

    #[tokio::test]
    async fn clearing_snapshot_forces_reload_without_false_revision_change() {
        let (service, fetch_count, invalidation_count, events) =
            test_service(Duration::from_millis(1));

        let initial = service.get().await.expect("initial get");
        service.clear_snapshot().await;
        let reloaded = service.get().await.expect("reload");

        assert_eq!(initial.revision, 1);
        assert_eq!(reloaded.revision, 1);
        assert_eq!(fetch_count.load(Ordering::SeqCst), 2);
        assert_eq!(invalidation_count.load(Ordering::SeqCst), 2);
        let events = events.lock().expect("events");
        assert_eq!(events.len(), 2);
        assert!(events[0].changed);
        assert!(!events[1].changed);
    }
}
