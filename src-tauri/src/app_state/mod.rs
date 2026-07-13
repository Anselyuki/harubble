mod api_clients;
mod download_subsystem;
mod media_controls;
mod playback;
mod preferences;

pub(crate) use api_clients::ApiClients;
pub(crate) use download_subsystem::DownloadSubsystem;
pub(crate) use preferences::PreferencesSubsystem;

use crate::album_metadata_cache::AlbumMetadataCacheService;
use crate::background_tasks::TaskDirectory;
use crate::collection::CollectionService;
use crate::command_scheduling::{self, CommandDomain};
use crate::download_session::DownloadSessionStore;
use crate::listening_history::ListeningHistoryService;
use crate::local_inventory::LocalInventoryService;
use crate::local_inventory_provenance::LocalInventoryProvenanceStore;
use crate::logging::{LogCenter, LogLevel, LogPayload};
use crate::playback_actor::{start_playback_actor, PlaybackActor};
use crate::playback_load_gate::PlaybackLoadGate;
use crate::player::stream::PlaybackInput;
use crate::player::AudioPlayer;
use crate::player::PlaybackError;
use crate::preferences::{AppPreferences, PreferencesStore};
use crate::search::LibrarySearchService;
use crate::startup_recovery::prepare_local_database;
use crate::tag_editor::TagEditorService;
use crate::tag_registry::TagRegistryService;
use harubble_core::{DownloadManagerSnapshot, DownloadService};
use serde_json::json;
use std::future::Future;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};
use tokio::sync::{Mutex, MutexGuard};

/// 应用运行期间共享的后端状态容器。
///
/// 这是 Tauri command、下载桥接、播放器控制与日志系统共用的高层入口，适用于需要访问跨域后端能力的场景。
/// 其内部聚合播放器、API 客户端、下载服务、库存服务、偏好存储与日志中心等共享状态。
/// 该类型应作为长生命周期共享状态使用，而不是按请求临时构造；调用方也不应绕过它直接拼装各子系统依赖。
#[derive(Clone)]
pub struct AppState {
    pub(crate) player: Arc<AudioPlayer>,
    pub(crate) api_clients: ApiClients,
    pub(crate) playback_runtime: Arc<tokio::runtime::Runtime>,
    pub(crate) playback_actor: PlaybackActor,
    pub(crate) playback_load_gate: PlaybackLoadGate,
    pub(crate) visual_aux_lock: Arc<Mutex<()>>,
    pub(crate) download: DownloadSubsystem,
    pub(crate) prefs: PreferencesSubsystem,
    pub(crate) local_inventory_service: LocalInventoryService,
    pub(crate) local_inventory_provenance_store: Arc<LocalInventoryProvenanceStore>,
    pub(crate) log_center: Arc<LogCenter>,
    pub(crate) task_directory: TaskDirectory,
    pub(crate) library_search_service: LibrarySearchService,
    pub(crate) listening_history: Arc<ListeningHistoryService>,
    pub(crate) album_metadata_cache: AlbumMetadataCacheService,
    pub(crate) tag_registry: TagRegistryService,
    pub(crate) tag_editor: TagEditorService,
    pub(crate) collection: CollectionService,
}

struct PreparedPlaybackInput {
    input: PlaybackInput,
    cache_path: std::path::PathBuf,
}

impl AppState {
    /// 根据 Tauri 应用句柄初始化全部后端服务与运行时状态。
    ///
    /// 适用于应用启动阶段创建唯一的全局后端状态入口。
    /// 入参 `app` 提供路径、插件与运行时访问能力；返回值为可注入到 Tauri `State` 中的 `AppState`。
    /// 该方法会加载偏好、恢复下载会话、初始化日志中心与搜索服务，因此应在应用生命周期内只调用一次。
    pub fn new(app: tauri::AppHandle) -> Result<Self, String> {
        let log_center = Arc::new(LogCenter::new(app.clone())?);
        let player = AudioPlayer::new(app.clone()).map_err(|e| e.to_string())?;
        let api = harubble_core::ApiClient::new().map_err(|e| e.to_string())?;
        let playback_api = harubble_core::ApiClient::new().map_err(|e| e.to_string())?;
        let image_api = harubble_core::ApiClient::new_image().map_err(|e| e.to_string())?;
        let download_api = harubble_core::ApiClient::new_download().map_err(|e| e.to_string())?;
        let playback_runtime = tokio::runtime::Builder::new_multi_thread()
            .thread_name("harubble-playback")
            .worker_threads(2)
            .max_blocking_threads(4)
            .enable_all()
            .build()
            .map_err(|e| format!("failed to initialize playback runtime: {e}"))?;
        let (playback_actor, playback_actor_inbox) = PlaybackActor::new();
        crate::migration::migrate_legacy_data(&app.path().app_data_dir().unwrap_or_default());
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("failed to get app data dir: {e}"))?;
        let store = PreferencesStore::new(app_data_dir.clone());
        let preferences = store.load(Some(log_center.as_ref()));
        let download_session_store = Arc::new(DownloadSessionStore::new(app_data_dir.clone()));
        let loaded_download_session =
            download_session_store.load(Some(log_center.as_ref()), preferences.locale);
        let download_service = Arc::new(Mutex::new(DownloadService::from_manager_snapshot(
            loaded_download_session.snapshot.clone(),
        )));
        let local_inventory_provenance_store = Arc::new(
            LocalInventoryProvenanceStore::new_with_logger(
                app_data_dir.clone(),
                Some(log_center.as_ref()),
            )
            .map_err(|e| e.to_string())?,
        );
        let local_inventory_service =
            LocalInventoryService::new(local_inventory_provenance_store.clone());
        let legacy_search_dir = app_data_dir.join("library-search");
        let new_search_dir = crate::storage_paths::search_index_root(&app)?;
        if legacy_search_dir.exists() && !new_search_dir.exists() {
            if let Err(err) = std::fs::rename(&legacy_search_dir, &new_search_dir) {
                log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "storage",
                        "storage.search_migration_failed",
                        "Failed to migrate search index to cache dir, will rebuild",
                    )
                    .details(err.to_string()),
                );
                // 迁移失败：删除旧目录以避免重复迁移尝试，索引将在下次 inventory scan 时重建
                let _ = std::fs::remove_dir_all(&legacy_search_dir);
            } else {
                log_center.record(LogPayload::new(
                    LogLevel::Info,
                    "storage",
                    "storage.search_migrated",
                    "Migrated search index from app_data_dir to app_cache_dir",
                ));
            }
        } else if legacy_search_dir.exists() && new_search_dir.exists() {
            // 两处都存在：删除旧位置（新位置已经是权威）
            let _ = std::fs::remove_dir_all(&legacy_search_dir);
        }
        let search_data_dir = new_search_dir;
        let library_search_service =
            LibrarySearchService::new(search_data_dir, preferences.output_dir.clone());
        let db_path = prepare_local_database(&app_data_dir, Some(log_center.as_ref()))?;
        let listening_history = Arc::new(
            ListeningHistoryService::new(&db_path)
                .map_err(|e| format!("初始化收听历史服务失败: {e}"))?,
        );
        let album_metadata_cache = AlbumMetadataCacheService::new(&db_path)
            .map_err(|e| format!("初始化元数据缓存服务失败: {e}"))?;
        let tag_registry = TagRegistryService::new(&app_data_dir);
        let tag_editor = TagEditorService::new(&app_data_dir);
        let official_collections_bytes = include_bytes!("../../../data/official_collections.json");
        let collection = CollectionService::new(&db_path, official_collections_bytes)
            .map_err(|e| format!("初始化合集服务失败: {e}"))?;
        let state = Self {
            player: Arc::new(player),
            api_clients: ApiClients {
                api: Arc::new(api),
                playback_api: Arc::new(playback_api),
                image_api: Arc::new(image_api),
                download_api: Arc::new(download_api),
            },
            playback_runtime: Arc::new(playback_runtime),
            playback_actor,
            playback_load_gate: PlaybackLoadGate::new(),
            visual_aux_lock: Arc::new(Mutex::new(())),
            download: DownloadSubsystem {
                download_service,
                download_job_creation_lock: Arc::new(Mutex::new(())),
                download_session_store,
            },
            prefs: PreferencesSubsystem {
                preferences_store: Arc::new(store),
                preferences: Arc::new(StdMutex::new(preferences)),
                preferences_write_lock: Arc::new(Mutex::new(())),
            },
            local_inventory_service,
            local_inventory_provenance_store,
            log_center,
            task_directory: TaskDirectory::new(),
            library_search_service,
            listening_history,
            album_metadata_cache,
            tag_registry,
            tag_editor,
            collection,
        };
        state.player.set_volume_silent(state.preferences().volume);
        start_playback_actor(Arc::clone(&state.playback_runtime), playback_actor_inbox);
        if loaded_download_session.should_persist {
            state.persist_download_snapshot(&loaded_download_session.snapshot);
        }
        Ok(state)
    }

    pub(crate) fn preferences(&self) -> AppPreferences {
        self.prefs
            .preferences
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    // ─── 领域访问器 ─────────────────────────────────────────────────────────
    // 为 command 层提供窄化访问入口。command 应通过这些方法访问各领域服务，
    // 而不是直接读写 pub(crate) 字段，以便未来能够拆分为独立的 Tauri State
    // 或按领域建立 Facade。app_state 内部（playback / media_controls）以及
    // 非-command 的模块（playback_actor / downloads/bridge 等）继续使用字段
    // 直接访问。

    /// 返回音频播放器实例（用于状态查询与音量控制）。
    pub(crate) fn player(&self) -> &Arc<AudioPlayer> {
        &self.player
    }

    /// 返回合集服务（列表、增删改、导入导出）。
    pub(crate) fn collection(&self) -> &CollectionService {
        &self.collection
    }

    /// 返回 Tag 编辑器服务（双层存储、三路合并）。
    pub(crate) fn tag_editor(&self) -> &TagEditorService {
        &self.tag_editor
    }

    /// 返回 Tag 注册表服务（维度定义、按 tag 查专辑）。
    pub(crate) fn tag_registry(&self) -> &TagRegistryService {
        &self.tag_registry
    }

    /// 返回库内搜索服务。
    pub(crate) fn library_search(&self) -> &LibrarySearchService {
        &self.library_search_service
    }

    /// 返回收听历史服务。
    pub(crate) fn listening_history(&self) -> &Arc<ListeningHistoryService> {
        &self.listening_history
    }

    /// 返回专辑元数据缓存服务。
    pub(crate) fn album_metadata_cache(&self) -> &AlbumMetadataCacheService {
        &self.album_metadata_cache
    }

    /// 返回本地库存服务。
    pub(crate) fn local_inventory(&self) -> &LocalInventoryService {
        &self.local_inventory_service
    }

    /// 返回日志中心（用于查询记录、状态检查等）。
    pub(crate) fn log_center(&self) -> &Arc<LogCenter> {
        &self.log_center
    }

    /// 返回后台任务目录（跨领域生命周期协调）。
    #[allow(dead_code)]
    pub(crate) fn task_directory(&self) -> &TaskDirectory {
        &self.task_directory
    }

    /// 返回通用业务 API 客户端。
    pub(crate) fn api_client(&self) -> &Arc<harubble_core::ApiClient> {
        &self.api_clients.api
    }

    /// 返回下载链路专用 API 客户端。
    pub(crate) fn download_api_client(&self) -> &Arc<harubble_core::ApiClient> {
        &self.api_clients.download_api
    }

    /// 返回图片资源专用 API 客户端。
    pub(crate) fn image_api_client(&self) -> &Arc<harubble_core::ApiClient> {
        &self.api_clients.image_api
    }

    /// 返回下载服务实例（需在 lock 后使用）。
    pub(crate) fn download_service(&self) -> &Arc<Mutex<DownloadService>> {
        &self.download.download_service
    }

    /// 返回下载批次创建互斥锁。
    pub(crate) fn download_job_creation_lock(&self) -> &Arc<Mutex<()>> {
        &self.download.download_job_creation_lock
    }

    /// 返回当前配置中的根输出目录。
    ///
    /// 适用于需要读取当前下载根目录的高层调用方。
    /// 返回值为当前内存中已生效的输出目录字符串。
    /// 该接口不会触发偏好重新加载；若调用方关心磁盘上的最新配置，应先完成偏好同步。
    pub fn output_dir(&self) -> String {
        self.prefs
            .preferences
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .output_dir
            .clone()
    }

    /// 记录一条结构化日志。
    ///
    /// 适用于高层业务在不直接依赖日志中心实现细节的前提下写入统一日志。
    /// 入参 `payload` 为结构化日志载荷；返回值为空。
    /// 该接口只负责投递日志，不保证日志已经持久化到磁盘；若需要落盘，应结合退出刷新接口使用。
    pub fn record_log(&self, payload: LogPayload) {
        self.log_center.record(payload);
    }

    /// 请求取消所有后台追踪任务。
    ///
    /// 应在应用退出前调用，以协作式通知各后台任务（库存扫描、搜索重建、tag 同步、
    /// 下载执行循环等）尽快退出，避免在进程终止时留下未完成的 I/O 操作。
    /// 该方法只发出取消信号，不等待各任务实际退出。
    pub async fn cancel_background_tasks(&self) {
        self.task_directory.cancel_all().await;
    }

    /// 按当前日志级别阈值将会话日志刷入持久化日志文件。
    ///
    /// 适用于应用退出前、崩溃恢复前置收尾，或需要显式落盘当前会话日志的场景。
    /// 成功时返回空值。
    /// 该接口会基于当前偏好的日志级别阈值过滤后再落盘，因此持久化文件内容不一定等于会话内全部日志。
    pub fn flush_logs_on_exit(&self) -> Result<(), String> {
        let threshold = LogLevel::parse(
            &self
                .prefs
                .preferences
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .log_level,
        )
        .unwrap_or(LogLevel::Error);
        self.log_center
            .flush_session_to_persistent(threshold)
            .map_err(|error| error.to_string())
    }

    pub(crate) fn set_preferences(&self, prefs: AppPreferences) {
        *self
            .prefs
            .preferences
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = prefs;
    }

    /// 在本地 tag_editor 修改成功后，异步刷新受影响专辑的搜索索引。
    ///
    /// 与 tag_registry_sync 通路复用 `build_snapshot_records_for_album` +
    /// `apply_incremental_tag_update`，任一步失败视为软失败，仅记日志不阻塞主流程。
    /// 无活跃索引或空 `album_cids` 时直接返回。
    pub(crate) fn schedule_local_tag_incremental_update(&self, album_cids: Vec<String>) {
        if album_cids.is_empty() {
            return;
        }
        let state = self.clone();
        let directory = state.task_directory.clone();
        tauri::async_runtime::spawn(async move {
            let task_id = directory
                .next_task_id("library_search", "local_tag_incremental")
                .await;
            crate::background_tasks::spawn_tracked(
                directory,
                task_id,
                move |_cancel_token| async move {
                    let locale = state.preferences().locale;
                    let mut updates = Vec::with_capacity(album_cids.len());
                    for cid in &album_cids {
                        match crate::search::build_snapshot_records_for_album(
                            state.api_clients.api.clone(),
                            state.tag_registry.clone(),
                            cid,
                            locale,
                        )
                        .await
                        {
                            Ok(records) => updates.push(records),
                            Err(error) => {
                                state.record_log(
                                    LogPayload::new(
                                        LogLevel::Warn,
                                        "library-search",
                                        "library_search.local_incremental_fetch_failed",
                                        "Failed to build incremental snapshot records for local tag edit",
                                    )
                                    .context(json!({
                                        "album_cid": cid,
                                    }))
                                    .details(error.to_string()),
                                );
                                return;
                            }
                        }
                    }
                    if let Err(error) = state
                        .library_search_service
                        .apply_incremental_tag_update(updates)
                        .await
                    {
                        state.record_log(
                            LogPayload::new(
                                LogLevel::Warn,
                                "library-search",
                                "library_search.local_incremental_apply_failed",
                                "Incremental tag update failed after local tag edit; will heal on next full rebuild",
                            )
                            .context(json!({
                                "changed_album_count": album_cids.len(),
                            }))
                            .details(error.to_string()),
                        );
                    }
                },
            );
        });
    }

    pub(crate) fn preferences_store(&self) -> Arc<PreferencesStore> {
        self.prefs.preferences_store.clone()
    }

    pub(crate) fn clear_api_response_caches(&self) {
        self.api_clients.api.clear_response_cache();
        self.api_clients.playback_api.clear_response_cache();
        self.api_clients.image_api.clear_response_cache();
        self.api_clients.download_api.clear_response_cache();
    }

    pub(crate) fn reset_http_clients(&self) -> Result<(), String> {
        let results = [
            ("app", self.api_clients.api.reset_http_client()),
            (
                "playback",
                self.api_clients.playback_api.reset_http_client(),
            ),
            ("image", self.api_clients.image_api.reset_http_client()),
            (
                "download",
                self.api_clients.download_api.reset_http_client(),
            ),
        ];
        let errors = results
            .into_iter()
            .filter_map(|(domain, result)| {
                result
                    .err()
                    .map(|error| format!("{domain} HTTP client reset failed: {error}"))
            })
            .collect::<Vec<_>>();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    pub(crate) fn begin_playback_transition(&self, command_name: &'static str) -> u64 {
        command_scheduling::debug_assert_command_domain(
            command_name,
            CommandDomain::PlaybackTransition,
        );
        let request_id = self.player.supersede_playback_request();
        self.player.supersede_loading_session();
        request_id
    }

    pub(crate) fn is_playback_load_gate_active(&self) -> bool {
        self.playback_load_gate.is_active()
    }

    pub(crate) async fn wait_for_playback_load_gate(&self, settle_delay: Duration) {
        self.playback_load_gate
            .wait_until_inactive_with_settle(settle_delay)
            .await;
    }

    pub(crate) async fn wait_for_background_io_gate(
        &self,
        command_name: &'static str,
        settle_delay: Duration,
    ) {
        command_scheduling::debug_assert_command_domain(command_name, CommandDomain::BackgroundIo);
        let was_active = self.is_playback_load_gate_active();
        let started_at = Instant::now();
        self.wait_for_playback_load_gate(settle_delay).await;
        let resource_gate_wait_ms = started_at.elapsed().as_millis();
        if was_active || resource_gate_wait_ms >= settle_delay.as_millis() {
            record_background_io_gate_metrics(
                Arc::clone(&self.log_center),
                command_name,
                resource_gate_wait_ms,
            );
        }
    }

    async fn enter_visual_aux(&self, command_name: &'static str) -> (MutexGuard<'_, ()>, u128) {
        const VISUAL_AUX_PLAYBACK_SETTLE_DELAY: Duration = Duration::from_millis(350);

        command_scheduling::debug_assert_command_domain(command_name, CommandDomain::VisualAux);

        let mut resource_gate_wait_ms = 0_u128;
        loop {
            let gate_wait_started_at = Instant::now();
            self.wait_for_playback_load_gate(VISUAL_AUX_PLAYBACK_SETTLE_DELAY)
                .await;
            resource_gate_wait_ms =
                resource_gate_wait_ms.saturating_add(gate_wait_started_at.elapsed().as_millis());
            let guard = self.visual_aux_lock.lock().await;
            if !self.is_playback_load_gate_active() {
                return (guard, resource_gate_wait_ms);
            }
        }
    }

    pub(crate) async fn dispatch_visual_aux<F, Fut, T>(
        &self,
        command_name: &'static str,
        task: F,
    ) -> T
    where
        F: FnOnce(Self) -> Fut,
        Fut: Future<Output = T>,
    {
        let submitted_at = Instant::now();
        let (visual_guard, resource_gate_wait_ms) = self.enter_visual_aux(command_name).await;
        let queue_wait_ms = submitted_at.elapsed().as_millis();
        let state = self.clone();
        let log_center = Arc::clone(&self.log_center);
        let started_at = Instant::now();
        let result = task(state).await;
        let run_ms = started_at.elapsed().as_millis();
        drop(visual_guard);
        record_visual_aux_metrics(
            log_center,
            command_name,
            queue_wait_ms,
            run_ms,
            resource_gate_wait_ms,
        );
        result
    }

    pub(crate) async fn dispatch_playback_transition<F, Fut, T>(
        &self,
        command_name: &'static str,
        task: F,
    ) -> Result<T, PlaybackError>
    where
        F: FnOnce(Self, u64) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, PlaybackError>> + Send + 'static,
        T: Send + 'static,
    {
        self.playback_actor
            .dispatch(self.clone(), command_name, task)
            .await
    }

    pub(crate) fn spawn_playback_transition<F, Fut>(&self, command_name: &'static str, task: F)
    where
        F: FnOnce(Self, u64) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        if let Err(error) = self.playback_actor.spawn(self.clone(), command_name, task) {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Error,
                    "playback",
                    "playback.transition_schedule_failed",
                    "Failed to schedule playback transition",
                )
                .details(error.to_string()),
            );
        }
    }

    pub(crate) async fn dispatch_playback_side_effect<F, Fut, T>(
        &self,
        command_name: &'static str,
        task: F,
    ) -> T
    where
        F: FnOnce(Self) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        command_scheduling::debug_assert_command_domain(
            command_name,
            CommandDomain::PlaybackSideEffect,
        );
        // Side effect 命令没有独立排队层，因此这里只统计 run_ms；避免记录一个恒为 0
        // 的 queue_wait_ms 掩盖真实的调度堆积（如果未来把它接入排队器时再补上）。
        let state = self.clone();
        let log_center = Arc::clone(&self.log_center);
        let started_at = Instant::now();
        let result = task(state).await;
        let run_ms = started_at.elapsed().as_millis();
        record_playback_side_effect_metrics(log_center, command_name, run_ms);
        result
    }

    pub(crate) async fn persist_preferences(&self, prefs: AppPreferences) -> Result<(), String> {
        let _guard = self.prefs.preferences_write_lock.lock().await;
        let locale = prefs.locale;
        let store = self.preferences_store();
        let prefs_to_save = prefs.clone();
        tokio::task::spawn_blocking(move || store.save(&prefs_to_save, locale))
            .await
            .map_err(|error| error.to_string())??;
        self.set_preferences(prefs);
        Ok(())
    }

    pub(crate) async fn update_preferences<F>(&self, update: F) -> Result<AppPreferences, String>
    where
        F: FnOnce(&mut AppPreferences),
    {
        let _guard = self.prefs.preferences_write_lock.lock().await;
        let mut prefs = self.preferences();
        update(&mut prefs);
        let locale = prefs.locale;
        let store = self.preferences_store();
        let prefs_to_save = prefs.clone();
        tokio::task::spawn_blocking(move || store.save(&prefs_to_save, locale))
            .await
            .map_err(|error| error.to_string())??;
        self.set_preferences(prefs.clone());
        Ok(prefs)
    }

    pub(crate) fn persist_download_snapshot(&self, snapshot: &DownloadManagerSnapshot) {
        if let Err(error) = self
            .download
            .download_session_store
            .save(snapshot, self.preferences().locale)
        {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Error,
                    "download-session",
                    "download_session.write_failed",
                    "Failed to persist download session",
                )
                .user_message(crate::i18n::tr(
                    self.preferences().locale,
                    "download-session-save-failed",
                ))
                .details(error),
            );
        }
    }

    pub(crate) async fn persist_download_snapshot_async(&self, snapshot: DownloadManagerSnapshot) {
        let store = self.download.download_session_store.clone();
        let locale = self.preferences().locale;
        let result = tokio::task::spawn_blocking(move || store.save(&snapshot, locale))
            .await
            .map_err(|error| error.to_string())
            .and_then(|result| result);

        if let Err(error) = result {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Error,
                    "download-session",
                    "download_session.write_failed",
                    "Failed to persist download session",
                )
                .user_message(crate::i18n::tr(
                    self.preferences().locale,
                    "download-session-save-failed",
                ))
                .details(error),
            );
        }
    }

    // ─── 跨领域应用服务 ────────────────────────────────────────────────────────
    // 收敛涉及多个领域的编排逻辑，避免 command 直接协调 api / local_inventory /
    // tag_registry / album_metadata_cache / download 等多个服务。command 只
    // 负责 IPC 参数解析与领域错误映射。

    /// 为已获取的专辑列表补齐本地库存徽标与 tag 元数据。
    ///
    /// 跨领域协调：local_inventory.enrich_albums + tag_registry.get_album_tags + preferences.locale。
    /// 该方法不产生错误，纯数据装配。
    pub(crate) async fn attach_album_enrichment(
        &self,
        albums: Vec<harubble_core::api::Album>,
    ) -> Vec<harubble_core::api::Album> {
        let mut enriched = self.local_inventory_service.enrich_albums(albums).await;
        let locale = self.preferences().locale;
        for album in &mut enriched {
            album.tags = self.tag_registry.get_album_tags(&album.cid, locale);
        }
        enriched
    }

    /// 为已获取的专辑详情补齐本地库存徽标、tag，并 upsert belong 缓存（fire-and-forget）。
    ///
    /// 跨领域协调：album_metadata_cache.upsert_belong + local_inventory.enrich_album_detail
    /// + tag_registry.get_album_tags + tag_registry.get_song_tags + preferences.locale。
    ///   belong 缓存更新失败不影响主流程返回值。
    pub(crate) async fn attach_album_detail_enrichment(
        &self,
        album: harubble_core::api::AlbumDetail,
    ) -> harubble_core::api::AlbumDetail {
        let cache = self.album_metadata_cache.clone();
        let album_cid_for_cache = album.cid.clone();
        let album_belong_for_cache = album.belong.clone();
        let _ = tokio::task::spawn_blocking(move || {
            cache.upsert_belong(&album_cid_for_cache, &album_belong_for_cache)
        })
        .await;
        let mut enriched = self
            .local_inventory_service
            .enrich_album_detail(album)
            .await;
        let locale = self.preferences().locale;
        enriched.tags = self.tag_registry.get_album_tags(&enriched.cid, locale);
        for song in &mut enriched.songs {
            song.tags = self
                .tag_registry
                .get_song_tags(&song.cid, &enriched.cid, locale);
        }
        enriched
    }

    /// 为已获取的歌曲详情补齐所属专辑上下文的本地库存徽标与 tag。
    ///
    /// 跨领域协调：local_inventory.enrich_song_detail + tag_registry.get_song_tags + preferences.locale。
    pub(crate) async fn attach_song_detail_enrichment(
        &self,
        song: harubble_core::api::SongDetail,
        album_name: &str,
    ) -> harubble_core::api::SongDetail {
        let mut enriched = self
            .local_inventory_service
            .enrich_song_detail(song, album_name)
            .await;
        let locale = self.preferences().locale;
        enriched.tags = self
            .tag_registry
            .get_song_tags(&enriched.cid, &enriched.album_cid, locale);
        enriched
    }

    /// 收集首页仪表盘状态，聚合平台专辑总数、本地库存与下载会话。
    ///
    /// 跨领域协调：api.get_albums + local_inventory.snapshot + download.download_service.lock().await.snapshot。
    /// 避免 command 直接持有下载服务锁。
    pub(crate) async fn homepage_status(
        &self,
    ) -> Result<harubble_core::homepage::HomepageStatus, String> {
        let albums = self
            .api_clients
            .api
            .get_albums()
            .await
            .map_err(|e| e.to_string())?;
        let platform_album_count = albums.len() as u32;

        let inventory_snapshot = self.local_inventory_service.snapshot().await;
        let local_downloaded_count = inventory_snapshot.matched_track_count as u32;

        let download_snapshot = self.download.download_service.lock().await.snapshot();
        let active_download_count = download_snapshot
            .jobs
            .iter()
            .filter(|j| matches!(j.status, harubble_core::DownloadJobStatus::Running))
            .count() as u32;
        let completed_download_count = download_snapshot
            .jobs
            .iter()
            .filter(|j| matches!(j.status, harubble_core::DownloadJobStatus::Completed))
            .count() as u32;

        Ok(harubble_core::homepage::HomepageStatus {
            platform_album_count,
            platform_song_count: 0,
            local_downloaded_count,
            local_storage_bytes: 0,
            active_download_count,
            completed_download_count,
        })
    }
}

/// 启动 belong 预热后台任务。
///
/// 获取全量专辑列表，找出缓存中缺失的专辑，并发获取其详情以填充 belong 缓存。
/// 完成后向前端发送 `homepage-belong-ready` 事件。
///
/// 适用于应用启动阶段在后台异步预热 belong 缓存，以便首页"按系列浏览"功能在用户打开时
/// 能够立即展示分组数据，而不需要等待实时拉取。
/// 入参 `app_handle` 用于在任务完成后向前端发送事件；`state` 提供 API 客户端、缓存服务与日志中心。
/// 该函数立即返回，实际工作在后台 tokio 任务中异步执行；调用方无需等待其完成。
/// 若获取专辑列表或查询缺失 CID 失败，任务会记录警告日志后提前退出，不会 panic。
/// 并发度上限为 5，避免对上游 API 造成过大压力。
pub fn spawn_belong_warmup(app_handle: tauri::AppHandle, state: &AppState) {
    let api = state.api_clients.api.clone();
    let cache = state.album_metadata_cache.clone();
    let log_center = state.log_center.clone();
    let state_for_gate = state.clone();

    tauri::async_runtime::spawn(async move {
        state_for_gate
            .wait_for_background_io_gate("belong_warmup", Duration::from_millis(250))
            .await;

        let albums = match api.get_albums().await {
            Ok(albums) => albums,
            Err(e) => {
                log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "homepage",
                        "homepage.belong_warmup_albums_failed",
                        "belong 预热: 获取专辑列表失败",
                    )
                    .details(e.to_string()),
                );
                return;
            }
        };

        let all_cids: Vec<String> = albums.iter().map(|a| a.cid.clone()).collect();
        let missing_result = {
            let cache = cache.clone();
            tokio::task::spawn_blocking(move || cache.get_missing_album_cids(&all_cids))
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result)
        };
        let missing = match missing_result {
            Ok(m) => m,
            Err(e) => {
                log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "homepage",
                        "homepage.belong_warmup_missing_cids_failed",
                        "belong 预热: 查询缺失 CID 失败",
                    )
                    .details(e),
                );
                return;
            }
        };

        if missing.is_empty() {
            let _ = app_handle.emit("homepage-belong-ready", ());
            return;
        }

        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(5));
        let mut handles = Vec::new();

        for cid in missing {
            let api = api.clone();
            let cache = cache.clone();
            let permit = semaphore.clone();
            let log_center = log_center.clone();

            handles.push(tokio::spawn(async move {
                let _permit = permit.acquire().await;
                match api.get_album_detail(&cid).await {
                    Ok(detail) => {
                        let cid_for_log = cid.clone();
                        let belong = detail.belong;
                        let upsert_result =
                            tokio::task::spawn_blocking(move || cache.upsert_belong(&cid, &belong))
                                .await
                                .map_err(|error| error.to_string())
                                .and_then(|result| result);
                        if let Err(e) = upsert_result {
                            log_center.record(
                                LogPayload::new(
                                    LogLevel::Warn,
                                    "homepage",
                                    "homepage.belong_warmup_upsert_failed",
                                    "belong 预热: 写入缓存失败",
                                )
                                .details(format!("{cid_for_log}: {e}")),
                            );
                        }
                    }
                    Err(e) => {
                        log_center.record(
                            LogPayload::new(
                                LogLevel::Warn,
                                "homepage",
                                "homepage.belong_warmup_detail_failed",
                                "belong 预热: 获取专辑详情失败",
                            )
                            .details(format!("{cid}: {e}")),
                        );
                    }
                }
            }));
        }

        for handle in handles {
            let _ = handle.await;
        }

        let _ = app_handle.emit("homepage-belong-ready", ());
    });
}

/// dev 模式下从本地项目文件加载 tag registry，release 模式下从远端拉取。
#[cfg(debug_assertions)]
async fn load_tag_registry_bytes(_state: &AppState) -> anyhow::Result<Vec<u8>> {
    let path = std::path::PathBuf::from(crate::tag_registry::DEV_LOCAL_PATH);
    tokio::task::spawn_blocking(move || {
        std::fs::read(&path).map_err(|e| {
            anyhow::anyhow!(
                "failed to read local tag registry at {}: {e}",
                path.display()
            )
        })
    })
    .await
    .map_err(|error| anyhow::anyhow!(error.to_string()))?
}

#[cfg(not(debug_assertions))]
async fn load_tag_registry_bytes(state: &AppState) -> anyhow::Result<Vec<u8>> {
    state
        .api_clients
        .api
        .download_bytes(crate::tag_registry::REMOTE_URL, |_, _| {})
        .await
}

fn record_background_io_gate_metrics(
    log_center: Arc<LogCenter>,
    command_name: &'static str,
    resource_gate_wait_ms: u128,
) {
    if let Some(spec) = command_scheduling::command_spec(command_name) {
        log_center.record(
            LogPayload::new(
                LogLevel::Debug,
                "playback",
                "playback.background_gate_wait_completed",
                "Background I/O waited for playback loading gate",
            )
            .context(json!({
                "command.name": command_name,
                "command.domain": spec.domain.as_label(),
                "command.priority": spec.priority.as_label(),
                "resource_gate.wait_ms": resource_gate_wait_ms,
            })),
        );
    }
}

fn record_visual_aux_metrics(
    log_center: Arc<LogCenter>,
    command_name: &'static str,
    queue_wait_ms: u128,
    run_ms: u128,
    resource_gate_wait_ms: u128,
) {
    if let Some(spec) = command_scheduling::command_spec(command_name) {
        log_center.record(
            LogPayload::new(
                LogLevel::Debug,
                "playback",
                "playback.visual_aux_completed",
                "Playback visual auxiliary command completed",
            )
            .context(json!({
                "command.name": command_name,
                "command.domain": spec.domain.as_label(),
                "command.priority": spec.priority.as_label(),
                "command.queue_wait_ms": queue_wait_ms,
                "command.run_ms": run_ms,
                "resource_gate.wait_ms": resource_gate_wait_ms,
            })),
        );
    }
}

fn record_playback_side_effect_metrics(
    log_center: Arc<LogCenter>,
    command_name: &'static str,
    run_ms: u128,
) {
    if let Some(spec) = command_scheduling::command_spec(command_name) {
        log_center.record(
            LogPayload::new(
                LogLevel::Debug,
                "playback",
                "playback.side_effect_completed",
                "Playback side-effect command completed",
            )
            .context(json!({
                "command.name": command_name,
                "command.domain": spec.domain.as_label(),
                "command.priority": spec.priority.as_label(),
                "command.run_ms": run_ms,
            })),
        );
    }
}

/// 启动 tag registry 远程同步后台任务。
///
/// 在应用启动后异步从远程拉取最新 tag JSON，与本地版本比对后按需替换。
/// 若注册表发生更新，会先尝试增量刷新受影响专辑的搜索索引：
/// 变更专辑数量少于阈值时，只重建这些专辑的快照记录与 Tantivy 文档；
/// 变更规模较大或增量过程失败时，回退为全量重建。
/// 网络失败时静默使用本地缓存，不阻塞应用启动。
pub fn spawn_tag_registry_sync(state: &AppState) {
    let state = state.clone();
    let directory = state.task_directory.clone();

    tauri::async_runtime::spawn(async move {
        let task_id = directory.next_task_id("tag_registry", "sync").await;
        crate::background_tasks::spawn_tracked(
            directory,
            task_id,
            move |cancel_token| async move {
                tokio::select! {
                    _ = cancel_token.cancelled() => { return; }
                    _ = state.wait_for_background_io_gate("tag_registry_sync", Duration::from_millis(250)) => {}
                }
                if cancel_token.is_cancelled() {
                    return;
                }

                let sync_result = async {
                    let response_bytes = load_tag_registry_bytes(&state).await?;
                    let new_registry: crate::tag_registry::TagRegistry =
                        serde_json::from_slice(&response_bytes)
                            .map_err(|e| anyhow::anyhow!("failed to parse tag registry: {e}"))?;

                    if new_registry.schema_version != crate::tag_registry::CURRENT_SCHEMA_VERSION {
                        anyhow::bail!(
                            "tag registry schema version {} does not match expected {}",
                            new_registry.schema_version,
                            crate::tag_registry::CURRENT_SCHEMA_VERSION
                        );
                    }

                    #[cfg(not(debug_assertions))]
                    {
                        let current_updated_at = state.tag_registry.current_updated_at();
                        if new_registry.updated_at == current_updated_at
                            && !current_updated_at.is_empty()
                        {
                            return Ok(None);
                        }
                    }

                    let tag_registry = state.tag_registry.clone();
                    let registry_for_persist = new_registry.clone();
                    let persist_result = tokio::task::spawn_blocking(move || {
                        tag_registry.persist_registry(&registry_for_persist)
                    })
                    .await
                    .map_err(|error| anyhow::anyhow!(error.to_string()))
                    .and_then(|result| result);
                    persist_result.map_err(|error| {
                        anyhow::anyhow!("failed to persist synced tag registry: {error}")
                    })?;

                    let old_registry = state.tag_registry.clone_current();
                    let new_registry_snapshot = new_registry.clone();
                    state.tag_registry.replace_in_memory(new_registry);

                    Ok::<
                        Option<(
                            crate::tag_registry::TagRegistry,
                            crate::tag_registry::TagRegistry,
                        )>,
                        anyhow::Error,
                    >(Some((old_registry, new_registry_snapshot)))
                }
                .await;

                match sync_result {
                    Ok(Some((old_registry, new_registry))) => {
                        apply_tag_registry_change(state, old_registry, new_registry).await;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        state.log_center.record(
                            LogPayload::new(
                                LogLevel::Warn,
                                "tag-registry",
                                "tag_registry.sync_failed",
                                "Failed to sync tag registry from remote",
                            )
                            .details(error.to_string()),
                        );
                    }
                }
            },
        );
    });
}

/// 阈值：变更专辑数在此以下才尝试增量搜索索引刷新，否则回退到全量重建。
const TAG_REGISTRY_INCREMENTAL_THRESHOLD: usize = 50;

/// 处理 tag registry 变更后的搜索索引刷新策略。
///
/// 步骤：
/// 1. 计算 `old` 与 `new` 之间 tag 发生变化的专辑 CID 集合（含歌曲级 tag 变更
///    通过快照歌曲→专辑映射回溯得到的父专辑）。
/// 2. 若集合为空，记录一次 no-op 日志后直接返回。
/// 3. 若规模低于阈值，逐个构建增量记录并调用 `apply_incremental_tag_update`；
///    构建失败或增量返回 `Ok(false)` / `Err(_)` 则回退为全量重建。
/// 4. 否则直接触发全量重建。
async fn apply_tag_registry_change(
    state: AppState,
    old_registry: crate::tag_registry::TagRegistry,
    new_registry: crate::tag_registry::TagRegistry,
) {
    let song_album_map = state
        .library_search_service
        .current_song_album_map()
        .await
        .unwrap_or_default();
    let changed_albums = compute_changed_album_cids(&old_registry, &new_registry, &song_album_map);

    if changed_albums.is_empty() {
        state.record_log(
            LogPayload::new(
                LogLevel::Info,
                "tag-registry",
                "tag_registry.sync_no_op",
                "Tag registry sync produced no album-level changes",
            )
            .context(json!({
                "old_updated_at": old_registry.updated_at,
                "new_updated_at": new_registry.updated_at,
            })),
        );
        return;
    }

    if changed_albums.len() < TAG_REGISTRY_INCREMENTAL_THRESHOLD {
        if try_incremental_tag_update(&state, &changed_albums).await {
            return;
        }
    } else {
        state.record_log(
            LogPayload::new(
                LogLevel::Info,
                "tag-registry",
                "tag_registry.sync_full_rebuild",
                "Tag registry change exceeds incremental threshold; scheduling full rebuild",
            )
            .context(json!({
                "changed_album_count": changed_albums.len(),
                "threshold": TAG_REGISTRY_INCREMENTAL_THRESHOLD,
            })),
        );
    }

    let inventory = state.local_inventory_service.snapshot().await;
    state
        .library_search_service
        .schedule_rebuild(state.clone(), inventory);
}

/// 尝试执行增量搜索索引刷新。
///
/// # 返回值
/// - `true` 增量成功，无需再触发全量重建。
/// - `false` 拉取快照失败、当前无活跃索引，或增量写入失败；调用方应回退到全量重建。
async fn try_incremental_tag_update(state: &AppState, changed_albums: &[String]) -> bool {
    let mut updates = Vec::with_capacity(changed_albums.len());
    let locale = state.preferences().locale;
    for cid in changed_albums {
        match crate::search::build_snapshot_records_for_album(
            state.api_clients.api.clone(),
            state.tag_registry.clone(),
            cid,
            locale,
        )
        .await
        {
            Ok(records) => updates.push(records),
            Err(error) => {
                state.record_log(
                    LogPayload::new(
                        LogLevel::Warn,
                        "library-search",
                        "library_search.incremental_fetch_failed",
                        "Failed to fetch album detail for incremental tag update",
                    )
                    .context(json!({
                        "album_cid": cid,
                    }))
                    .details(error.to_string()),
                );
                return false;
            }
        }
    }

    match state
        .library_search_service
        .apply_incremental_tag_update(updates)
        .await
    {
        Ok(true) => {
            state.record_log(
                LogPayload::new(
                    LogLevel::Info,
                    "library-search",
                    "library_search.incremental_updated",
                    "Applied incremental tag update",
                )
                .context(json!({
                    "changed_album_count": changed_albums.len(),
                })),
            );
            true
        }
        Ok(false) => {
            state.record_log(
                LogPayload::new(
                    LogLevel::Info,
                    "library-search",
                    "library_search.incremental_skipped",
                    "No active search index; falling back to full rebuild",
                )
                .context(json!({
                    "changed_album_count": changed_albums.len(),
                })),
            );
            false
        }
        Err(error) => {
            state.record_log(
                LogPayload::new(
                    LogLevel::Warn,
                    "library-search",
                    "library_search.incremental_apply_failed",
                    "Incremental tag update failed; falling back to full rebuild",
                )
                .context(json!({
                    "changed_album_count": changed_albums.len(),
                }))
                .details(error.to_string()),
            );
            false
        }
    }
}

/// 计算两版 tag registry 之间发生 tag 变更的专辑 CID 集合。
///
/// 覆盖两类变更：
/// - 专辑级 tag 集合发生变化（含新增、删除、内容修改）。
/// - 歌曲级 tag 集合发生变化，通过 `song_album_map` 回溯到所属专辑 CID。
///
/// # 参数
/// - `old`：旧的 tag registry 快照。
/// - `new`：新的 tag registry 快照。
/// - `song_album_map`：来自当前搜索快照的"歌曲 CID → 专辑 CID"映射；缺失映射的
///   歌曲变更会被忽略（这些专辑通常不在当前库存中，即便刷新也不会命中搜索）。
///
/// # 返回值
/// 变更专辑 CID 的去重列表，顺序不保证。
fn compute_changed_album_cids(
    old: &crate::tag_registry::TagRegistry,
    new: &crate::tag_registry::TagRegistry,
    song_album_map: &std::collections::HashMap<String, String>,
) -> Vec<String> {
    use std::collections::HashSet;

    let old_album_tags = crate::tag_registry::albums_to_tag_map(&old.albums, &old.type_definitions);
    let new_album_tags = crate::tag_registry::albums_to_tag_map(&new.albums, &new.type_definitions);
    let old_song_tags = crate::tag_registry::songs_to_tag_map(&old.songs);
    let new_song_tags = crate::tag_registry::songs_to_tag_map(&new.songs);

    let mut changed: HashSet<String> = HashSet::new();

    let mut album_cids: HashSet<&String> = HashSet::new();
    album_cids.extend(old_album_tags.keys());
    album_cids.extend(new_album_tags.keys());
    for cid in album_cids {
        let old_tags = old_album_tags.get(cid).map(|s| &s.tags);
        let new_tags = new_album_tags.get(cid).map(|s| &s.tags);
        if old_tags != new_tags {
            changed.insert(cid.clone());
        }
    }

    let mut song_cids: HashSet<&String> = HashSet::new();
    song_cids.extend(old_song_tags.keys());
    song_cids.extend(new_song_tags.keys());
    for song_cid in song_cids {
        let old_tags = old_song_tags.get(song_cid).map(|s| &s.tags);
        let new_tags = new_song_tags.get(song_cid).map(|s| &s.tags);
        if old_tags != new_tags {
            if let Some(album_cid) = song_album_map.get(song_cid) {
                changed.insert(album_cid.clone());
            }
        }
    }

    changed.into_iter().collect()
}

fn normalize_seek_position(position_secs: f64, duration_secs: f64) -> f64 {
    let position_secs = position_secs.max(0.0);
    if duration_secs > 0.0 {
        position_secs.min((duration_secs - 0.05).max(0.0))
    } else {
        position_secs
    }
}
