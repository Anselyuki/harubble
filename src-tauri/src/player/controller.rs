//! 播放控制器与播放队列编排逻辑。
//!
//! 该模块实现播放器主控制器、前后端共享的播放队列上下文，以及播放、暂停、跳转、
//! 上下曲切换与系统媒体控制绑定等核心编排能力。

use crate::player::backend::{create_backend, PlaybackBackend};
use crate::player::events::{
    emit_ended, emit_progress, emit_progress_snapshot, emit_state, PlaybackEndedEvent,
};
use crate::player::media::MediaSession;
use crate::player::state::PlayerState;
use crate::player::stream::{AudioFormat, SampleBuffer};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use souvlaki::MediaControlEvent;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Manager};

/// 播放进度事件向前端广播的最小间隔。
///
/// 底层音频输出回调按驱动缓冲帧率（约 80–160 次/秒）推进进度，但前端进度条无需
/// 如此高频更新。播放进度事件统一由独立的发射线程按该间隔节流广播，避免高频
/// 全量状态序列化灌满 WebView 事件通道导致界面逐渐卡死。
const PROGRESS_EMIT_INTERVAL: Duration = Duration::from_millis(100);

/// 前后端共享的播放队列条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackQueueEntry {
    /// 重新传回 `play_song` 时使用的歌曲 CID。
    pub cid: String,
    /// 队列与播放器界面展示用的歌曲名。
    pub name: String,
    /// 队列与播放器界面展示用的艺术家列表。
    pub artists: Vec<String>,
    /// 系统媒体会话使用的可选封面地址。
    pub cover_url: Option<String>,
}

/// 前端发起播放时传入的播放队列上下文。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackContext {
    /// 可用于上一首/下一首导航的有序队列。
    pub entries: Vec<PlaybackQueueEntry>,
    /// 播放开始时前端选中的索引。
    pub current_index: usize,
}

#[derive(Default)]
struct PlaybackQueueState {
    entries: Vec<PlaybackQueueEntry>,
    current_index: Option<usize>,
}

impl PlaybackQueueState {
    fn set_context(&mut self, context: PlaybackContext, current_cid: &str) {
        self.entries = context.entries;
        if self.entries.is_empty() {
            self.current_index = None;
            return;
        }

        let bounded_index = context.current_index.min(self.entries.len() - 1);
        let resolved_index = self
            .entries
            .iter()
            .position(|entry| entry.cid == current_cid)
            .unwrap_or(bounded_index);
        self.current_index = Some(resolved_index);
    }

    fn sync_or_replace_current(&mut self, entry: PlaybackQueueEntry) {
        if let Some(index) = self.entries.iter().position(|item| item.cid == entry.cid) {
            self.entries[index] = entry;
            self.current_index = Some(index);
            return;
        }

        self.entries = vec![entry];
        self.current_index = Some(0);
    }

    fn select_next(&mut self) -> Option<PlaybackQueueEntry> {
        if self.entries.len() <= 1 {
            return None;
        }
        let next_index = self
            .current_index
            .map(|index| (index + 1) % self.entries.len())
            .unwrap_or(0);
        self.current_index = Some(next_index);
        self.entries.get(next_index).cloned()
    }

    fn select_previous(&mut self) -> Option<PlaybackQueueEntry> {
        if self.entries.len() <= 1 {
            return None;
        }
        let previous_index = self
            .current_index
            .map(|index| {
                if index == 0 {
                    self.entries.len() - 1
                } else {
                    index - 1
                }
            })
            .unwrap_or(0);
        self.current_index = Some(previous_index);
        self.entries.get(previous_index).cloned()
    }

    fn has_previous(&self) -> bool {
        self.current_index.is_some() && self.entries.len() > 1
    }

    fn has_next(&self) -> bool {
        self.current_index.is_some() && self.entries.len() > 1
    }
}

/// Tauri command 与系统媒体控制共用的后端播放控制器。
///
/// 负责维护播放器状态、驱动底层播放后端、同步系统媒体会话，并为前端提供统一的
/// 播放控制入口；同一应用生命周期内通常只需要持有一个实例。
pub struct AudioPlayer {
    app: AppHandle,
    state: Arc<Mutex<PlayerState>>,
    backend: Mutex<Box<dyn PlaybackBackend>>,
    media_session: Arc<Mutex<Option<MediaSession>>>,
    queue: Arc<Mutex<PlaybackQueueState>>,
    volume: Arc<AtomicU64>,
    active_session_id: Arc<AtomicU64>,
    active_request_id: AtomicU64,
    stop_flag: Mutex<Arc<AtomicBool>>,
    pause_flag: Mutex<Arc<AtomicBool>>,
}

impl AudioPlayer {
    /// 创建新的播放器控制器实例。
    ///
    /// 该方法会初始化底层播放后端、空白播放器状态与默认音量；返回值可直接用于
    /// Tauri command 注入和媒体控制绑定。
    pub fn new(app: AppHandle) -> Result<Self> {
        let backend = create_backend()?;
        Ok(Self {
            app,
            state: Arc::new(Mutex::new(PlayerState::default())),
            backend: Mutex::new(backend),
            media_session: Arc::new(Mutex::new(None)),
            queue: Arc::new(Mutex::new(PlaybackQueueState::default())),
            volume: Arc::new(AtomicU64::new(1.0_f64.to_bits())),
            active_session_id: Arc::new(AtomicU64::new(0)),
            active_request_id: AtomicU64::new(0),
            stop_flag: Mutex::new(Arc::new(AtomicBool::new(false))),
            pause_flag: Mutex::new(Arc::new(AtomicBool::new(false))),
        })
    }

    /// 返回底层 Tauri 应用句柄。
    ///
    /// 适用于需要向外部组件转交 `AppHandle`、发出事件或访问应用级状态的场景。
    pub fn app_handle(&self) -> AppHandle {
        self.app.clone()
    }

    /// 绑定系统媒体控制事件处理器。
    pub fn bind_media_controls<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(MediaControlEvent) + Send + 'static,
    {
        let session = MediaSession::new(&self.app)?;
        session.attach(handler)?;
        *self.media_session.lock().unwrap() = Some(session);
        self.sync_media_controls(false);
        Ok(())
    }

    /// 准备当前播放会话对应的队列上下文。
    pub fn prepare_playback_context(
        &self,
        context: Option<PlaybackContext>,
        current_entry: PlaybackQueueEntry,
    ) {
        {
            let mut queue = self.queue.lock().unwrap();
            if let Some(context) = context {
                queue.set_context(context, &current_entry.cid);
                if let Some(index) = queue.current_index {
                    queue.entries[index] = current_entry;
                }
            } else {
                queue.sync_or_replace_current(current_entry);
            }
        }
        self.sync_navigation_flags();
    }

    /// 选择下一首队列条目并更新导航标志。
    pub fn select_next_entry(&self) -> Option<PlaybackQueueEntry> {
        let next = {
            let mut queue = self.queue.lock().unwrap();
            queue.select_next()
        };
        self.sync_navigation_flags();
        next
    }

    /// 选择上一首队列条目并更新导航标志。
    pub fn select_previous_entry(&self) -> Option<PlaybackQueueEntry> {
        let previous = {
            let mut queue = self.queue.lock().unwrap();
            queue.select_previous()
        };
        self.sync_navigation_flags();
        previous
    }

    /// 初始化新的加载会话并替换当前播放状态。
    ///
    /// 该方法会先停止已有音频输出，但不会广播一个短暂的空播放器状态；随后写入
    /// 歌曲元数据、初始进度、初始时长与当前音量，并返回新的会话 ID 供后续流式
    /// 播放阶段校验。
    pub fn begin_loading_session(
        &self,
        song_cid: String,
        song_name: String,
        artists: Vec<String>,
        cover_url: Option<String>,
        initial_progress: f64,
        initial_duration: Option<f64>,
    ) -> Result<u64> {
        self.stop_active_backend()?;

        let session_id = self.active_session_id.fetch_add(1, Ordering::SeqCst) + 1;
        self.install_session_flags();
        let initial_progress = initial_progress.max(0.0);
        let initial_duration = initial_duration.unwrap_or(0.0).max(initial_progress);

        let (has_previous, has_next) = self.queue_navigation_flags();
        let volume = atomic_volume_load(&self.volume);
        {
            let mut state = self.state.lock().unwrap();
            state.session_id = session_id;
            state.song_cid = Some(song_cid);
            state.song_name = Some(song_name);
            state.artists = artists;
            state.cover_url = cover_url;
            state.is_loading = true;
            state.is_playing = false;
            state.is_paused = false;
            state.progress = initial_progress;
            state.duration = initial_duration;
            state.volume = volume;
            state.has_previous = has_previous;
            state.has_next = has_next;
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);

        Ok(session_id)
    }

    /// 与底层播放后端协商最终输出音频格式。
    ///
    /// 传入的 `source_format` 通常来自解码器探测结果，返回值表示当前音频设备或
    /// 后端实现可稳定消费的实际输出格式。
    pub fn negotiate_output_format(&self, source_format: AudioFormat) -> Result<AudioFormat> {
        self.backend
            .lock()
            .unwrap()
            .negotiate_output_format(source_format)
    }

    /// 启动当前会话的流式播放并返回最终时长。
    ///
    /// 该方法会校验会话 ID 是否仍有效，向后端注册进度、完成与错误回调，并在
    /// 启动成功后把播放器状态切换为播放中。
    pub fn start_stream_playback(
        &self,
        session_id: u64,
        format: AudioFormat,
        samples: SampleBuffer,
        initial_progress: f64,
    ) -> Result<f64> {
        if !self.is_session_active(session_id) {
            anyhow::bail!("Playback session expired");
        }

        self.pause_signal().store(false, Ordering::SeqCst);
        let initial_progress = if format.duration_secs > 0.0 {
            initial_progress.clamp(0.0, format.duration_secs)
        } else {
            initial_progress.max(0.0)
        };

        let did_update_timing = {
            let mut state = self.state.lock().unwrap();
            let next_duration = format.duration_secs.max(initial_progress);
            let mut did_update = false;

            if (state.progress - initial_progress).abs() > 0.001 {
                state.progress = initial_progress;
                did_update = true;
            }
            if (state.duration - next_duration).abs() > 0.001 {
                state.duration = next_duration;
                did_update = true;
            }

            did_update
        };
        if did_update_timing {
            emit_progress_and_sync(&self.app, &self.state, &self.media_session);
        }

        let progress_callback = self.build_progress_callback(session_id, initial_progress);
        let finish_callback = self.build_finish_callback(session_id);
        let error_handler = self.build_error_callback(session_id);

        self.backend
            .lock()
            .unwrap()
            .play_stream(
                format,
                samples,
                self.stop_signal(),
                Arc::clone(&self.volume),
                progress_callback,
                finish_callback,
                error_handler,
            )
            .context("Failed to start audio backend")?;

        {
            let mut state = self.state.lock().unwrap();
            state.is_loading = false;
            state.is_playing = true;
            state.is_paused = false;
            state.progress = initial_progress;
            if format.duration_secs > 0.0 {
                state.duration = format.duration_secs;
            }
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);
        self.spawn_progress_emitter(session_id);

        Ok(self.state.lock().unwrap().duration)
    }

    fn build_progress_callback(
        &self,
        session_id: u64,
        initial_progress: f64,
    ) -> Arc<dyn Fn(f64, f64) + Send + Sync> {
        let state_for_progress = Arc::clone(&self.state);
        let active_session = Arc::clone(&self.active_session_id);
        let stop_flag = self.stop_signal();

        // 该回调运行在音频实时输出线程上，按驱动缓冲帧率高频触发，因此只做最廉价的
        // 状态写入：更新进度与时长。事件广播与系统媒体会话同步交由独立的节流发射
        // 线程处理（见 `spawn_progress_emitter`），避免在实时音频线程上执行 IPC /
        // 平台调用或高频全量序列化。
        Arc::new(move |progress, duration| {
            if active_session.load(Ordering::SeqCst) != session_id
                || stop_flag.load(Ordering::SeqCst)
            {
                return;
            }
            let mut state = state_for_progress.lock().unwrap();
            let absolute_progress = if duration > 0.0 {
                (initial_progress + progress).min(duration)
            } else {
                initial_progress + progress
            };
            state.progress = absolute_progress;
            if duration > 0.0 {
                state.duration = duration.max(initial_progress);
            }
        })
    }

    /// 启动当前会话的进度发射线程。
    ///
    /// 该线程按 [`PROGRESS_EMIT_INTERVAL`] 节流读取播放器状态快照，仅在仍处于播放
    /// 中时广播进度事件并同步系统媒体会话。当活跃会话 ID 不再匹配或收到停止信号时
    /// 线程自动结束，因此每个播放会话至多对应一个发射线程，不会随时间累积。
    fn spawn_progress_emitter(&self, session_id: u64) {
        let state = Arc::clone(&self.state);
        let app = self.app.clone();
        let media_session = Arc::clone(&self.media_session);
        let active_session = Arc::clone(&self.active_session_id);
        let stop_flag = self.stop_signal();

        let spawn_result = thread::Builder::new()
            .name("player-progress-emitter".into())
            .spawn(move || loop {
                thread::sleep(PROGRESS_EMIT_INTERVAL);
                if active_session.load(Ordering::SeqCst) != session_id
                    || stop_flag.load(Ordering::SeqCst)
                {
                    break;
                }
                let snapshot = state.lock().unwrap().clone();
                if !snapshot.is_playing {
                    continue;
                }
                emit_progress_snapshot(&app, &snapshot);
                sync_media_session_snapshot(&media_session, &snapshot, true);
            });

        if let Err(error) = spawn_result {
            if let Some(state) = self.app.try_state::<crate::app_state::AppState>() {
                state.log_center.record(
                    crate::logging::LogPayload::new(
                        crate::logging::LogLevel::Warn,
                        "player",
                        "player.progress_emitter_spawn_failed",
                        "Failed to spawn progress emitter thread",
                    )
                    .details(error.to_string()),
                );
            }
        }
    }

    fn build_finish_callback(&self, session_id: u64) -> Arc<dyn Fn() + Send + Sync> {
        let state_for_finish = Arc::clone(&self.state);
        let app_for_finish = self.app.clone();
        let media_for_finish = Arc::clone(&self.media_session);
        let active_session = Arc::clone(&self.active_session_id);
        let stop_flag = self.stop_signal();
        let pause_flag = self.pause_signal();

        Arc::new(move || {
            if active_session.load(Ordering::SeqCst) != session_id
                || stop_flag.load(Ordering::SeqCst)
            {
                return;
            }
            pause_flag.store(false, Ordering::SeqCst);
            let ended_event = {
                let mut state = state_for_finish.lock().unwrap();
                state.is_playing = false;
                state.is_paused = false;
                state.is_loading = false;
                if state.duration <= 0.0 {
                    state.duration = state.progress;
                }
                state.progress = state.duration;
                state.song_cid.clone().map(|song_cid| PlaybackEndedEvent {
                    session_id,
                    song_cid,
                    progress: state.progress,
                    duration: state.duration,
                })
            };
            emit_state_and_sync(&app_for_finish, &state_for_finish, &media_for_finish);
            if let Some(event) = ended_event {
                emit_ended(&app_for_finish, event);
            }
        })
    }

    fn build_error_callback(&self, session_id: u64) -> Arc<dyn Fn(String) + Send + Sync> {
        let active_session = Arc::clone(&self.active_session_id);
        let stop_flag = self.stop_signal();
        let app_for_error = self.app.clone();

        Arc::new(move |message: String| {
            if stop_flag.load(Ordering::SeqCst)
                || active_session.load(Ordering::SeqCst) != session_id
            {
                return;
            }
            if let Some(state) = app_for_error.try_state::<crate::app_state::AppState>() {
                state.log_center.record(
                    crate::logging::LogPayload::new(
                        crate::logging::LogLevel::Error,
                        "player",
                        "player.output_stream_failed",
                        "Audio output stream failed",
                    )
                    .details(message),
                );
            }
        })
    }

    /// 暂停当前播放中的会话。
    ///
    /// 仅当播放器处于非加载中的播放态时才会真正调用后端暂停；否则直接返回成功。
    pub fn pause(&self) -> Result<()> {
        let should_pause = {
            let state = self.state.lock().unwrap();
            state.is_playing && !state.is_loading
        };
        if !should_pause {
            return Ok(());
        }

        self.backend
            .lock()
            .unwrap()
            .pause()
            .context("Failed to pause audio backend")?;
        self.pause_signal().store(true, Ordering::SeqCst);

        {
            let mut state = self.state.lock().unwrap();
            state.is_playing = false;
            state.is_paused = true;
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);
        Ok(())
    }

    /// 恢复当前已暂停的会话。
    ///
    /// 仅当播放器仍保留歌曲上下文且状态为已暂停时才会真正调用后端恢复；否则直接返回成功。
    pub fn resume(&self) -> Result<()> {
        let should_resume = {
            let state = self.state.lock().unwrap();
            state.is_paused && state.song_cid.is_some()
        };
        if !should_resume {
            return Ok(());
        }

        self.backend
            .lock()
            .unwrap()
            .resume()
            .context("Failed to resume audio backend")?;
        self.pause_signal().store(false, Ordering::SeqCst);

        {
            let mut state = self.state.lock().unwrap();
            state.is_playing = true;
            state.is_paused = false;
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);
        Ok(())
    }

    /// 根据当前状态在暂停与恢复之间切换。
    ///
    /// 若既不处于播放中也不处于已暂停状态，则该方法不会启动新的播放流程。
    pub fn toggle_playback(&self) -> Result<()> {
        let state = self.get_state();
        if state.is_paused {
            self.resume()
        } else if state.is_playing {
            self.pause()
        } else {
            Ok(())
        }
    }

    /// 将指定会话标记为失败并清空播放器状态。
    ///
    /// 仅当 `session_id` 仍为当前活跃会话时才会生效，通常用于流式解码或输出阶段发生不可恢复错误时。
    pub fn fail_session(&self, session_id: u64) {
        if self.active_session_id.load(Ordering::SeqCst) != session_id {
            return;
        }

        self.stop_signal().store(true, Ordering::SeqCst);
        self.pause_signal().store(false, Ordering::SeqCst);
        self.active_session_id.fetch_add(1, Ordering::SeqCst);
        let _ = self.backend.lock().unwrap().stop();

        {
            let mut state = self.state.lock().unwrap();
            *state = PlayerState::default();
            state.session_id = self.active_session_id.load(Ordering::SeqCst);
            state.volume = atomic_volume_load(&self.volume);
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);
    }

    /// 返回当前播放会话共享的停止信号。
    ///
    /// 后台下载、解码或输出线程可通过该标志感知会话是否已被停止。
    pub fn stop_signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop_flag.lock().unwrap())
    }

    /// 返回当前播放会话共享的暂停信号。
    ///
    /// 后台解码线程可通过该标志在不销毁会话的前提下暂时挂起处理。
    pub fn pause_signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.pause_flag.lock().unwrap())
    }

    /// 判断给定会话是否仍为当前活跃播放会话。
    ///
    /// 当会话 ID 不匹配或已收到停止信号时返回 `false`。
    pub fn is_session_active(&self, session_id: u64) -> bool {
        self.active_session_id.load(Ordering::SeqCst) == session_id
            && !self.stop_signal().load(Ordering::SeqCst)
    }

    /// 创建一个新的播放请求 ID，用于异步阶段判断旧请求是否已被更新请求取代。
    pub fn begin_playback_request(&self) -> u64 {
        self.active_request_id.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// 判断播放请求是否仍为最新请求。
    pub fn is_playback_request_active(&self, request_id: u64) -> bool {
        self.active_request_id.load(Ordering::SeqCst) == request_id
    }

    /// 停止当前播放并将播放器状态重置为默认值。
    ///
    /// 该方法会推进会话 ID，使旧回调与后台线程自动失效。
    pub fn stop(&self) -> Result<()> {
        self.stop_active_backend()?;

        {
            let mut state = self.state.lock().unwrap();
            *state = PlayerState::default();
            state.session_id = self.active_session_id.load(Ordering::SeqCst);
            state.volume = atomic_volume_load(&self.volume);
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);
        Ok(())
    }

    fn stop_active_backend(&self) -> Result<()> {
        self.stop_signal().store(true, Ordering::SeqCst);
        self.pause_signal().store(false, Ordering::SeqCst);
        self.active_session_id.fetch_add(1, Ordering::SeqCst);
        self.backend.lock().unwrap().stop()
    }

    fn install_session_flags(&self) {
        *self.stop_flag.lock().unwrap() = Arc::new(AtomicBool::new(false));
        *self.pause_flag.lock().unwrap() = Arc::new(AtomicBool::new(false));
    }

    /// 返回当前播放器状态快照。
    pub fn get_state(&self) -> PlayerState {
        self.state.lock().unwrap().clone()
    }

    /// 设置播放器音量并返回裁剪后的实际值。
    ///
    /// 输入音量会被限制在 `0.0..=1.0` 范围内，同时同步更新对外状态与媒体会话。
    pub fn set_volume(&self, volume: f64) -> f64 {
        let safe_volume = volume.clamp(0.0, 1.0);
        atomic_volume_store(&self.volume, safe_volume);

        {
            let mut state = self.state.lock().unwrap();
            state.volume = safe_volume;
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);

        safe_volume
    }

    /// 静默设置播放器音量，不触发事件广播。
    ///
    /// 仅用于应用启动阶段从偏好恢复音量，此时 Tauri 状态尚未注册，无法发送事件。
    pub fn set_volume_silent(&self, volume: f64) {
        let safe_volume = volume.clamp(0.0, 1.0);
        atomic_volume_store(&self.volume, safe_volume);
        self.state.lock().unwrap().volume = safe_volume;
    }

    fn sync_media_controls(&self, progress_only: bool) {
        sync_media_session(&self.media_session, &self.state, progress_only);
    }

    /// 读取当前队列导航标志快照（是否有上一首/下一首）。
    ///
    /// 该方法仅短暂持有 `queue` 锁并立即释放，便于调用方在不与 `state` 锁嵌套的前提下
    /// 获取导航标志，从而避免 `state` 与 `queue` 两把锁因获取顺序相反而产生死锁。
    fn queue_navigation_flags(&self) -> (bool, bool) {
        let queue = self.queue.lock().unwrap();
        (queue.has_previous(), queue.has_next())
    }

    fn sync_navigation_flags(&self) {
        let (has_previous, has_next) = self.queue_navigation_flags();
        {
            let mut state = self.state.lock().unwrap();
            state.has_previous = has_previous;
            state.has_next = has_next;
        }
        self.sync_media_controls(false);
    }
}

fn emit_state_and_sync(
    app: &AppHandle,
    state: &Arc<Mutex<PlayerState>>,
    media_session: &Arc<Mutex<Option<MediaSession>>>,
) {
    let snapshot = state.lock().unwrap().clone();
    emit_state(app, state);
    sync_media_session(media_session, state, false);
    crate::notification::notify_playback_changed(app, &snapshot);
}

fn emit_progress_and_sync(
    app: &AppHandle,
    state: &Arc<Mutex<PlayerState>>,
    media_session: &Arc<Mutex<Option<MediaSession>>>,
) {
    emit_progress(app, state);
    sync_media_session(media_session, state, true);
}

fn sync_media_session(
    media_session: &Arc<Mutex<Option<MediaSession>>>,
    state: &Arc<Mutex<PlayerState>>,
    progress_only: bool,
) {
    let snapshot = state.lock().unwrap().clone();
    sync_media_session_snapshot(media_session, &snapshot, progress_only);
}

fn sync_media_session_snapshot(
    media_session: &Arc<Mutex<Option<MediaSession>>>,
    snapshot: &PlayerState,
    progress_only: bool,
) {
    if let Some(session) = media_session.lock().unwrap().as_ref() {
        session.sync_state(snapshot, progress_only);
    }
}

fn atomic_volume_load(volume: &AtomicU64) -> f64 {
    f64::from_bits(volume.load(Ordering::SeqCst)).clamp(0.0, 1.0)
}

fn atomic_volume_store(volume: &AtomicU64, value: f64) {
    volume.store(value.clamp(0.0, 1.0).to_bits(), Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::{PlaybackQueueEntry, PlaybackQueueState};

    fn entry(cid: &str) -> PlaybackQueueEntry {
        PlaybackQueueEntry {
            cid: cid.to_string(),
            name: cid.to_string(),
            artists: Vec::new(),
            cover_url: None,
        }
    }

    #[test]
    fn queue_navigation_wraps_across_boundaries() {
        let mut queue = PlaybackQueueState {
            entries: vec![entry("a"), entry("b")],
            current_index: Some(1),
        };

        assert_eq!(queue.select_next().map(|entry| entry.cid), Some("a".into()));
        assert_eq!(
            queue.select_previous().map(|entry| entry.cid),
            Some("b".into())
        );
    }

    #[test]
    fn single_entry_queue_has_no_navigation_targets() {
        let mut queue = PlaybackQueueState {
            entries: vec![entry("a")],
            current_index: Some(0),
        };

        assert!(!queue.has_next());
        assert!(!queue.has_previous());
        assert!(queue.select_next().is_none());
        assert!(queue.select_previous().is_none());
    }
}
