use crate::album_metadata_cache::AlbumMetadataCacheService;
use crate::audio_cache;
use crate::collection::CollectionService;
use crate::download_session::DownloadSessionStore;
use crate::i18n;
use crate::listening_history::ListeningHistoryService;
use crate::local_inventory::LocalInventoryService;
use crate::local_inventory_provenance::LocalInventoryProvenanceStore;
use crate::logging::{LogCenter, LogLevel, LogPayload};
use crate::player::stream::{GrowingFileHandle, PlaybackInput, SampleBuffer};
use crate::player::{AudioPlayer, PlaybackContext, PlaybackQueueEntry};
use crate::preferences::{AppPreferences, PreferencesStore};
use crate::search::LibrarySearchService;
use crate::tag_editor::TagEditorService;
use crate::tag_registry::TagRegistryService;
use anyhow::{Context, Result};
use harubble_core::{DownloadManagerSnapshot, DownloadService};
use souvlaki::{MediaControlEvent, SeekDirection};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

/// 应用运行期间共享的后端状态容器。
///
/// 这是 Tauri command、下载桥接、播放器控制与日志系统共用的高层入口，适用于需要访问跨域后端能力的场景。
/// 其内部聚合播放器、API 客户端、下载服务、库存服务、偏好存储与日志中心等共享状态。
/// 该类型应作为长生命周期共享状态使用，而不是按请求临时构造；调用方也不应绕过它直接拼装各子系统依赖。
#[derive(Clone)]
pub struct AppState {
    pub(crate) player: Arc<AudioPlayer>,
    pub(crate) api: Arc<harubble_core::ApiClient>,
    pub(crate) download_service: Arc<Mutex<DownloadService>>,
    pub(crate) local_inventory_service: LocalInventoryService,
    pub(crate) local_inventory_provenance_store: Arc<LocalInventoryProvenanceStore>,
    pub(crate) download_session_store: Arc<DownloadSessionStore>,
    pub(crate) preferences_store: Arc<PreferencesStore>,
    pub(crate) preferences: Arc<StdMutex<AppPreferences>>,
    pub(crate) log_center: Arc<LogCenter>,
    pub(crate) library_search_service: LibrarySearchService,
    pub(crate) listening_history: Arc<ListeningHistoryService>,
    pub(crate) album_metadata_cache: AlbumMetadataCacheService,
    pub(crate) tag_registry: TagRegistryService,
    pub(crate) tag_editor: TagEditorService,
    pub(crate) collection: CollectionService,
}

struct PreparedPlaybackInput {
    input: PlaybackInput,
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
            LocalInventoryProvenanceStore::new(app_data_dir.clone()).map_err(|e| e.to_string())?,
        );
        let local_inventory_service =
            LocalInventoryService::new(local_inventory_provenance_store.clone());
        let search_data_dir = app_data_dir.join("library-search");
        let library_search_service =
            LibrarySearchService::new(search_data_dir, preferences.output_dir.clone());
        let db_path = app_data_dir.join("harubble_local.db");
        let listening_history = Arc::new(
            ListeningHistoryService::new(&db_path)
                .map_err(|e| format!("初始化收听历史服务失败: {e}"))?,
        );
        let album_metadata_cache = AlbumMetadataCacheService::new(&db_path)
            .map_err(|e| format!("初始化元数据缓存服务失败: {e}"))?;
        let tag_registry = TagRegistryService::new(&app_data_dir);
        let tag_editor = TagEditorService::new(&app_data_dir);
        let official_collections_bytes = include_bytes!("../../data/official_collections.json");
        let collection = CollectionService::new(&db_path, official_collections_bytes)
            .map_err(|e| format!("初始化合集服务失败: {e}"))?;
        let state = Self {
            player: Arc::new(player),
            api: Arc::new(api),
            download_service,
            local_inventory_service,
            local_inventory_provenance_store,
            download_session_store,
            preferences_store: Arc::new(store),
            preferences: Arc::new(StdMutex::new(preferences)),
            log_center,
            library_search_service,
            listening_history,
            album_metadata_cache,
            tag_registry,
            tag_editor,
            collection,
        };
        state.player.set_volume_silent(state.preferences().volume);
        if loaded_download_session.should_persist {
            state.persist_download_snapshot(&loaded_download_session.snapshot);
        }
        Ok(state)
    }

    pub(crate) fn preferences(&self) -> AppPreferences {
        self.preferences
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 返回当前配置中的根输出目录。
    ///
    /// 适用于需要读取当前下载根目录的高层调用方。
    /// 返回值为当前内存中已生效的输出目录字符串。
    /// 该接口不会触发偏好重新加载；若调用方关心磁盘上的最新配置，应先完成偏好同步。
    pub fn output_dir(&self) -> String {
        self.preferences
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .output_dir
            .clone()
    }

    /// 绑定系统媒体控制事件。
    ///
    /// 适用于应用启动后启用系统级播放/暂停/切歌控制的场景。
    /// 成功时返回空值。
    /// 该接口通常只需要在启动阶段绑定一次；重复绑定可能导致媒体事件处理关系变得难以推断。
    pub fn bind_media_controls(&self) -> Result<(), String> {
        let media_state = self.clone();
        self.player
            .bind_media_controls(move |event| media_state.handle_media_control(event))
            .map_err(|error| error.to_string())
    }

    /// 记录一条结构化日志。
    ///
    /// 适用于高层业务在不直接依赖日志中心实现细节的前提下写入统一日志。
    /// 入参 `payload` 为结构化日志载荷；返回值为空。
    /// 该接口只负责投递日志，不保证日志已经持久化到磁盘；若需要落盘，应结合退出刷新接口使用。
    pub fn record_log(&self, payload: LogPayload) {
        self.log_center.record(payload);
    }

    /// 按当前日志级别阈值将会话日志刷入持久化日志文件。
    ///
    /// 适用于应用退出前、崩溃恢复前置收尾，或需要显式落盘当前会话日志的场景。
    /// 成功时返回空值。
    /// 该接口会基于当前偏好的日志级别阈值过滤后再落盘，因此持久化文件内容不一定等于会话内全部日志。
    pub fn flush_logs_on_exit(&self) -> Result<(), String> {
        let threshold = LogLevel::parse(
            &self
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
        *self.preferences.lock().unwrap_or_else(|e| e.into_inner()) = prefs;
    }

    pub(crate) fn preferences_store(&self) -> Arc<PreferencesStore> {
        self.preferences_store.clone()
    }

    pub(crate) fn persist_download_snapshot(&self, snapshot: &DownloadManagerSnapshot) {
        if let Err(error) = self
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

    pub(crate) async fn play_song_internal(
        &self,
        song_cid: String,
        cover_url: Option<String>,
        playback_context: Option<PlaybackContext>,
    ) -> Result<f64, String> {
        let song_detail = self
            .api
            .get_song_detail(&song_cid)
            .await
            .map_err(|e| e.to_string())?;

        self.player.prepare_playback_context(
            playback_context,
            PlaybackQueueEntry {
                cid: song_cid.clone(),
                name: song_detail.name.clone(),
                artists: song_detail.artists.clone(),
                cover_url: cover_url.clone(),
            },
        );

        let session_id = self
            .player
            .begin_loading_session(
                song_cid.clone(),
                song_detail.name.clone(),
                song_detail.artists.clone(),
                cover_url.clone(),
                0.0,
                None,
            )
            .map_err(|e| e.to_string())?;

        let result: Result<f64> = async {
            self.start_playback_session(session_id, &song_cid, &song_detail.source_url, 0.0)
                .await
        }
        .await;

        match result {
            Ok(duration) => {
                let listening_event = harubble_core::ListeningEvent {
                    song_cid: song_cid.clone(),
                    song_name: song_detail.name.clone(),
                    album_cid: song_detail.album_cid.clone(),
                    album_name: String::new(),
                    cover_url: cover_url.clone(),
                    artists: song_detail.artists.clone(),
                };
                if let Err(e) = self.listening_history.record(&listening_event) {
                    self.log_center.record(
                        LogPayload::new(
                            LogLevel::Warn,
                            "listening-history",
                            "listening_history.record_failed",
                            "Failed to record listening history",
                        )
                        .details(e),
                    );
                }
                Ok(duration)
            }
            Err(error) => {
                self.player.fail_session(session_id);
                Err(error.to_string())
            }
        }
    }

    pub(crate) async fn seek_current_internal(&self, position_secs: f64) -> Result<f64, String> {
        let current_state = self.player.get_state();
        let song_cid = current_state
            .song_cid
            .clone()
            .ok_or_else(|| i18n::tr(self.preferences().locale, "player-no-active-track"))?;

        if current_state.is_loading {
            return Err(i18n::tr(self.preferences().locale, "player-still-loading"));
        }

        let target_position = normalize_seek_position(position_secs, current_state.duration);
        if (current_state.progress - target_position).abs() < 0.05 {
            return Ok(current_state.duration);
        }

        let song_detail = self
            .api
            .get_song_detail(&song_cid)
            .await
            .map_err(|e| e.to_string())?;

        let session_id = self
            .player
            .begin_loading_session(
                song_cid.clone(),
                song_detail.name.clone(),
                song_detail.artists.clone(),
                current_state.cover_url.clone(),
                target_position,
                (current_state.duration > 0.0).then_some(current_state.duration),
            )
            .map_err(|e| e.to_string())?;

        let should_pause_after_seek = current_state.is_paused;
        let result: Result<f64> = async {
            let duration = self
                .start_playback_session(
                    session_id,
                    &song_cid,
                    &song_detail.source_url,
                    target_position,
                )
                .await?;

            if should_pause_after_seek {
                self.player.pause()?;
            }

            Ok(duration)
        }
        .await;

        match result {
            Ok(duration) => Ok(duration),
            Err(error) => {
                self.player.fail_session(session_id);
                Err(error.to_string())
            }
        }
    }

    async fn start_playback_session(
        &self,
        session_id: u64,
        song_cid: &str,
        source_url: &str,
        start_position_secs: f64,
    ) -> Result<f64> {
        let stop_flag = self.player.stop_signal();
        let pause_flag = self.player.pause_signal();
        let prepared_input = self.prepare_playback_input(song_cid, source_url, &stop_flag)?;
        let input = prepared_input.input.clone();

        let inspect_input = input.clone();
        let source_format = tokio::task::spawn_blocking(move || inspect_input.inspect_format())
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))??;

        anyhow::ensure!(
            self.player.is_session_active(session_id),
            "Playback stopped"
        );

        let output_format = self.player.negotiate_output_format(source_format)?;
        let start_position_secs =
            normalize_seek_position(start_position_secs, source_format.duration_secs);
        let sample_buffer = SampleBuffer::new();

        let log_center = Arc::clone(&self.log_center);
        let error_handler: crate::player::stream::PlaybackErrorHandler = Arc::new(move |message| {
            log_center.record(
                crate::logging::LogPayload::new(
                    crate::logging::LogLevel::Error,
                    "player",
                    "player.decode_worker_failed",
                    "Audio decode worker failed",
                )
                .details(message),
            );
        });

        let _decode_worker = input.spawn_decode_worker(
            source_format,
            output_format,
            sample_buffer.clone(),
            Arc::clone(&stop_flag),
            Arc::clone(&pause_flag),
            start_position_secs,
            error_handler,
        )?;

        self.wait_for_initial_buffer(&sample_buffer, output_format, &stop_flag)
            .await?;

        self.start_prepared_playback(
            session_id,
            output_format,
            sample_buffer,
            start_position_secs,
        )
    }

    pub(crate) async fn play_next_internal(&self) -> Result<f64, String> {
        let target = self
            .player
            .select_next_entry()
            .ok_or_else(|| i18n::tr(self.preferences().locale, "player-no-next-track"))?;
        self.play_song_internal(target.cid, target.cover_url, None)
            .await
    }

    pub(crate) async fn play_previous_internal(&self) -> Result<f64, String> {
        let target = self
            .player
            .select_previous_entry()
            .ok_or_else(|| i18n::tr(self.preferences().locale, "player-no-previous-track"))?;
        self.play_song_internal(target.cid, target.cover_url, None)
            .await
    }

    fn handle_media_control(&self, event: MediaControlEvent) {
        match event {
            MediaControlEvent::Play => self.handle_media_play(),
            MediaControlEvent::Pause => self.handle_media_pause(),
            MediaControlEvent::Toggle => self.handle_media_toggle(),
            MediaControlEvent::Stop | MediaControlEvent::Quit => self.handle_media_stop_or_quit(),
            MediaControlEvent::Next => self.handle_media_next(),
            MediaControlEvent::Previous => self.handle_media_previous(),
            MediaControlEvent::SetPosition(position) => {
                self.handle_media_set_position(position.0.as_secs_f64())
            }
            MediaControlEvent::SeekBy(direction, delta) => {
                self.handle_media_seek_by(direction, delta.as_secs_f64())
            }
            MediaControlEvent::Seek(direction) => self.handle_media_seek(direction),
            _ => {}
        }
    }

    fn handle_media_play(&self) {
        if let Err(error) = self.player.resume() {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Warn,
                    "media-session",
                    "media_session.resume_failed",
                    "Failed to resume playback",
                )
                .details(format!("{error:#}")),
            );
        }
    }

    fn handle_media_pause(&self) {
        if let Err(error) = self.player.pause() {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Warn,
                    "media-session",
                    "media_session.pause_failed",
                    "Failed to pause playback",
                )
                .details(format!("{error:#}")),
            );
        }
    }

    fn handle_media_toggle(&self) {
        if let Err(error) = self.player.toggle_playback() {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Warn,
                    "media-session",
                    "media_session.toggle_failed",
                    "Failed to toggle playback",
                )
                .details(format!("{error:#}")),
            );
        }
    }

    fn handle_media_stop_or_quit(&self) {
        if let Err(error) = self.player.stop() {
            self.log_center.record(
                LogPayload::new(
                    LogLevel::Warn,
                    "media-session",
                    "media_session.stop_failed",
                    "Failed to stop playback",
                )
                .details(format!("{error:#}")),
            );
        }
    }

    fn handle_media_next(&self) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = state.play_next_internal().await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.next_track_failed",
                        "Failed to play next track",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn handle_media_previous(&self) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = state.play_previous_internal().await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.previous_track_failed",
                        "Failed to play previous track",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn handle_media_set_position(&self, position_secs: f64) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = state.seek_current_internal(position_secs).await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.seek_failed",
                        "Failed to seek playback",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn handle_media_seek_by(&self, direction: SeekDirection, delta_secs: f64) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            let current = state.player.get_state();
            let target = match direction {
                SeekDirection::Forward => current.progress + delta_secs,
                SeekDirection::Backward => current.progress - delta_secs,
            };
            if let Err(error) = state.seek_current_internal(target).await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.seek_by_delta_failed",
                        "Failed to seek by delta",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn handle_media_seek(&self, direction: SeekDirection) {
        let state = self.clone();
        tauri::async_runtime::spawn(async move {
            let current = state.player.get_state();
            let target = match direction {
                SeekDirection::Forward => current.progress + 10.0,
                SeekDirection::Backward => current.progress - 10.0,
            };
            if let Err(error) = state.seek_current_internal(target).await {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.seek_forward_failed",
                        "Failed to seek forward/backward",
                    )
                    .details(error.to_string()),
                );
            }
        });
    }

    fn prepare_playback_input(
        &self,
        song_cid: &str,
        source_url: &str,
        stop_flag: &Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<PreparedPlaybackInput> {
        let cache_path = audio_cache::cached_song_path(song_cid, source_url)?;
        let pending_marker = audio_cache::pending_marker_path(&cache_path);

        let input = if audio_cache::is_song_cached(&cache_path) {
            PlaybackInput::cached_file(cache_path.clone())
        } else {
            let _ = std::fs::remove_file(&cache_path);
            let _ = std::fs::remove_file(&pending_marker);
            std::fs::write(&pending_marker, b"pending").with_context(|| {
                format!("Failed to create cache marker {}", pending_marker.display())
            })?;

            let (handle, writer) = GrowingFileHandle::new(cache_path.clone())?;
            self.spawn_stream_download(
                source_url.to_string(),
                Arc::clone(stop_flag),
                handle.clone(),
                writer,
                cache_path.clone(),
                pending_marker.clone(),
            );
            PlaybackInput::growing_file(handle)
        };

        Ok(PreparedPlaybackInput { input })
    }

    fn spawn_stream_download(
        &self,
        source_url: String,
        stop_flag: Arc<std::sync::atomic::AtomicBool>,
        handle: GrowingFileHandle,
        mut writer: std::fs::File,
        cache_path: PathBuf,
        pending_marker: PathBuf,
    ) {
        let api = Arc::clone(&self.api);
        let log_center = Arc::clone(&self.log_center);

        tokio::spawn(async move {
            let total_len_set = std::sync::atomic::AtomicBool::new(false);
            let download_result = api
                .download_stream(&source_url, |chunk, _, total| {
                    if stop_flag.load(Ordering::SeqCst) {
                        return Ok(false);
                    }
                    if !total_len_set.load(Ordering::Relaxed) {
                        if let Some(total) = total {
                            handle.set_expected_total_len(total);
                            total_len_set.store(true, Ordering::Relaxed);
                        }
                    }
                    handle.append_chunk(&mut writer, chunk)?;
                    Ok(true)
                })
                .await;

            match download_result {
                Ok(()) if !stop_flag.load(Ordering::SeqCst) => {
                    handle.mark_complete();
                    let _ = std::fs::remove_file(&pending_marker);
                    audio_cache::spawn_cleanup_if_needed();
                }
                Ok(()) => {
                    handle.mark_error("Playback stopped");
                    let _ = std::fs::remove_file(&pending_marker);
                    let _ = std::fs::remove_file(&cache_path);
                }
                Err(error) => {
                    log_center.record(
                        LogPayload::new(
                            LogLevel::Error,
                            "player",
                            "player.stream_download_failed",
                            "Streaming download failed during playback",
                        )
                        .details(format!("{error:#}")),
                    );
                    handle.mark_error(error.to_string());
                    let _ = std::fs::remove_file(&pending_marker);
                    let _ = std::fs::remove_file(&cache_path);
                }
            }
        });
    }

    async fn wait_for_initial_buffer(
        &self,
        sample_buffer: &SampleBuffer,
        output_format: crate::player::stream::AudioFormat,
        stop_flag: &Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<()> {
        let minimum_samples =
            ((output_format.sample_rate as usize * output_format.channels as usize) / 3)
                .max(output_format.channels as usize * 4096)
                .min(output_format.channels as usize * 32_768);

        let wait_buffer = sample_buffer.clone();
        let wait_stop = Arc::clone(stop_flag);
        tokio::task::spawn_blocking(move || {
            wait_buffer.wait_for_samples(minimum_samples, &wait_stop)
        })
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))??;
        Ok(())
    }

    fn start_prepared_playback(
        &self,
        session_id: u64,
        output_format: crate::player::stream::AudioFormat,
        sample_buffer: SampleBuffer,
        start_position_secs: f64,
    ) -> Result<f64> {
        anyhow::ensure!(
            self.player.is_session_active(session_id),
            "Playback stopped"
        );

        self.player.start_stream_playback(
            session_id,
            output_format,
            sample_buffer,
            start_position_secs,
        )
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
    let api = state.api.clone();
    let cache = state.album_metadata_cache.clone();
    let log_center = state.log_center.clone();

    tauri::async_runtime::spawn(async move {
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
        let missing = match cache.get_missing_album_cids(&all_cids) {
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
                        if let Err(e) = cache.upsert_belong(&cid, &detail.belong) {
                            log_center.record(
                                LogPayload::new(
                                    LogLevel::Warn,
                                    "homepage",
                                    "homepage.belong_warmup_upsert_failed",
                                    "belong 预热: 写入缓存失败",
                                )
                                .details(format!("{cid}: {e}")),
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
    let path = std::path::Path::new(crate::tag_registry::DEV_LOCAL_PATH);
    std::fs::read(path).map_err(|e| {
        anyhow::anyhow!(
            "failed to read local tag registry at {}: {e}",
            path.display()
        )
    })
}

#[cfg(not(debug_assertions))]
async fn load_tag_registry_bytes(state: &AppState) -> anyhow::Result<Vec<u8>> {
    state
        .api
        .download_bytes(crate::tag_registry::REMOTE_URL, |_, _| {})
        .await
}

/// 启动 tag registry 远程同步后台任务。
///
/// 在应用启动后异步从远程拉取最新 tag JSON，与本地版本比对后按需替换。
/// 若注册表发生更新且当前已有库存快照，自动触发搜索索引重建以同步新 tag 数据。
/// 网络失败时静默使用本地缓存，不阻塞应用启动。
pub fn spawn_tag_registry_sync(state: &AppState) {
    let state = state.clone();

    tauri::async_runtime::spawn(async move {
        let updated = async {
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
                if new_registry.updated_at == current_updated_at && !current_updated_at.is_empty() {
                    return Ok(false);
                }
            }

            state.tag_registry.update(new_registry)?;
            Ok::<bool, anyhow::Error>(true)
        }
        .await;

        match updated {
            Ok(true) => {
                let inventory = state.local_inventory_service.snapshot().await;
                state
                    .library_search_service
                    .schedule_rebuild(state.clone(), inventory);
            }
            Ok(false) => {}
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
    });
}

fn normalize_seek_position(position_secs: f64, duration_secs: f64) -> f64 {
    let position_secs = position_secs.max(0.0);
    if duration_secs > 0.0 {
        position_secs.min((duration_secs - 0.05).max(0.0))
    } else {
        position_secs
    }
}
