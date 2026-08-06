//! 播放控制器与播放队列编排逻辑。
//!
//! 该模块实现播放器主控制器、前后端共享的播放队列上下文，以及播放、暂停、跳转、
//! 上下曲切换与系统媒体控制绑定等核心编排能力。

use crate::playback_load_gate::PlaybackLoadGate;
use crate::player::backend::{
    create_backend, AudioCallbackMetrics, AudioMetricsHandler, OutputFormat, PlaybackBackend,
};
use crate::player::events::{
    emit_ended, emit_progress, emit_progress_snapshot, emit_state, PlaybackEndedEvent,
};
use crate::player::media::MediaSession;
use crate::player::state::{PlaybackFormatState, PlayerState};
use crate::player::stream::{AudioFormat, SampleBuffer, SampleWaitOutcome};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;
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
const REBUFFER_TARGET_SECONDS: usize = 3;
const FINISHED_STREAM_KEEPALIVE_GRACE: Duration = Duration::from_secs(10);

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

    fn peek_next(&self) -> Option<PlaybackQueueEntry> {
        self.peek_next_with_index().map(|(_, entry)| entry)
    }

    fn peek_previous(&self) -> Option<PlaybackQueueEntry> {
        self.peek_previous_with_index().map(|(_, entry)| entry)
    }

    fn peek_next_with_index(&self) -> Option<(usize, PlaybackQueueEntry)> {
        if self.entries.len() <= 1 {
            return None;
        }
        let next_index = self
            .current_index
            .map(|index| (index + 1) % self.entries.len())
            .unwrap_or(0);
        self.entries
            .get(next_index)
            .cloned()
            .map(|entry| (next_index, entry))
    }

    fn peek_previous_with_index(&self) -> Option<(usize, PlaybackQueueEntry)> {
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
        self.entries
            .get(previous_index)
            .cloned()
            .map(|entry| (previous_index, entry))
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
    rebuffering_session_id: AtomicU64,
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
            rebuffering_session_id: AtomicU64::new(0),
            stop_flag: Mutex::new(Arc::new(AtomicBool::new(false))),
            pause_flag: Mutex::new(Arc::new(AtomicBool::new(false))),
        })
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

    /// 读取下一首队列条目但不移动队列游标。
    pub fn peek_next_entry(&self) -> Option<PlaybackQueueEntry> {
        self.queue.lock().unwrap().peek_next()
    }

    /// 读取上一首队列条目但不移动队列游标。
    pub fn peek_previous_entry(&self) -> Option<PlaybackQueueEntry> {
        self.queue.lock().unwrap().peek_previous()
    }

    /// 初始化新的加载会话并替换当前播放状态。
    ///
    /// 该方法会先停止旧会话，并让已有设备流在切换期持续输出数字静音，但不会广播
    /// 一个短暂的空播放器状态；随后写入歌曲元数据、初始进度、初始时长与当前音量，
    /// 并返回新的会话 ID 供后续流式播放阶段校验。
    pub fn begin_loading_session(
        &self,
        song_cid: String,
        song_name: String,
        artists: Vec<String>,
        cover_url: Option<String>,
        initial_progress: f64,
        initial_duration: Option<f64>,
    ) -> Result<u64> {
        self.quiesce_active_backend_for_transition()?;

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
            state.playback_format = None;
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
    pub fn negotiate_output_format(&self, source_format: AudioFormat) -> Result<OutputFormat> {
        self.backend
            .lock()
            .unwrap()
            .negotiate_output_format(source_format)
    }

    /// 更新当前会话的输入与输出音频格式摘要。
    pub fn set_playback_format(
        &self,
        session_id: u64,
        source_format: AudioFormat,
        output_format: &OutputFormat,
    ) {
        {
            let mut state = self.state.lock().unwrap();
            if state.session_id != session_id {
                return;
            }
            state.playback_format = Some(PlaybackFormatState {
                source_sample_rate: source_format.sample_rate,
                source_channels: source_format.channels,
                source_bits_per_sample: source_format.bits_per_sample,
                output_sample_rate: output_format.audio_format.sample_rate,
                output_channels: output_format.audio_format.channels,
                output_bits_per_sample: output_format.audio_format.bits_per_sample,
                output_sample_format: output_format.sample_format.as_str().to_string(),
                resampling: source_format.sample_rate != output_format.audio_format.sample_rate,
                channel_remix: source_format.channels != output_format.audio_format.channels,
            });
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);
    }

    /// 启动当前会话的流式播放并返回最终时长。
    ///
    /// 该方法会校验会话 ID 是否仍有效，向后端注册进度、完成与错误回调，并在
    /// 启动成功后把播放器状态切换为播放中。
    pub fn start_stream_playback(
        &self,
        session_id: u64,
        format: OutputFormat,
        samples: SampleBuffer,
        initial_progress: f64,
    ) -> Result<f64> {
        if !self.is_session_active(session_id) {
            anyhow::bail!("Playback session expired");
        }

        self.pause_signal().store(false, Ordering::SeqCst);
        let audio_format = format.audio_format;
        let initial_progress = if audio_format.duration_secs > 0.0 {
            initial_progress.clamp(0.0, audio_format.duration_secs)
        } else {
            initial_progress.max(0.0)
        };

        let did_update_timing = {
            let mut state = self.state.lock().unwrap();
            let next_duration = audio_format.duration_secs.max(initial_progress);
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
        let metrics_handler = self.build_audio_metrics_callback(session_id, &format);
        let underrun_handler =
            self.build_audio_underrun_callback(session_id, samples.clone(), audio_format);

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
                metrics_handler,
                underrun_handler,
            )
            .context("Failed to start audio backend")?;

        // 使该会话失效的 transition 已负责 quiesce 或 stop。这里不能再无条件 stop：
        // 新请求可能已经接管刚启动的静音流，用它覆盖后续加载空档。
        ensure_started_session_current(self.is_session_active(session_id))?;

        {
            let mut state = self.state.lock().unwrap();
            state.is_loading = false;
            state.is_playing = true;
            state.is_paused = false;
            state.progress = initial_progress;
            if audio_format.duration_secs > 0.0 {
                state.duration = audio_format.duration_secs;
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
        build_progress_callback(
            Arc::clone(&self.state),
            Arc::clone(&self.active_session_id),
            self.stop_signal(),
            session_id,
            initial_progress,
        )
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
                state.log_center().record(
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
        let app_for_finish = self.app.clone();

        Arc::new(move || {
            if let Some(state) = app_for_finish.try_state::<crate::app_state::AppState>() {
                state.player().finish_session(session_id);
            }
        })
    }

    fn build_audio_metrics_callback(
        &self,
        session_id: u64,
        output_format: &OutputFormat,
    ) -> AudioMetricsHandler {
        let app_for_metrics = self.app.clone();
        let output_device_identity = output_format.device_identity.clone();
        let output_sample_rate = output_format.audio_format.sample_rate;
        let output_channels = output_format.audio_format.channels;
        let output_sample_format = output_format.sample_format.as_str();

        Arc::new(move |metrics: AudioCallbackMetrics| {
            if let Some(state) = app_for_metrics.try_state::<crate::app_state::AppState>() {
                let log_center = Arc::clone(state.log_center());
                let playback_runtime = Arc::clone(state.playback_runtime());
                let output_device_identity = output_device_identity.clone();
                playback_runtime.spawn(async move {
                    let avg_ns = metrics
                        .callback_elapsed_ns_total
                        .checked_div(metrics.callback_count)
                        .unwrap_or(0);
                    let callback_period_ns_min = metrics
                        .callback_frames_min
                        .saturating_mul(1_000_000_000)
                        .checked_div(u64::from(output_sample_rate.max(1)))
                        .unwrap_or(0);
                    let log_level = if metrics.callback_over_period_count > 0
                        || metrics.callback_metrics_dropped_count > 0
                        || metrics.stream_start_timeout_count > 0
                        || metrics.stream_start_failure_count > 0
                        || metrics.output_xrun_count > 0
                        || metrics.output_clipped_samples > 0
                        || metrics.output_nonfinite_samples > 0
                    {
                        crate::logging::LogLevel::Warn
                    } else {
                        crate::logging::LogLevel::Debug
                    };
                    log_center.record(
                        crate::logging::LogPayload::new(
                            log_level,
                            "player",
                            "audio.callback_metrics",
                            "Audio callback metrics",
                        )
                        .context(json!({
                            "command.domain": "RealtimeAudio",
                            "command.priority": "Realtime",
                            "playback.session_id": session_id,
                            "audio.output_device_id": output_device_identity,
                            "audio.output_sample_rate": output_sample_rate,
                            "audio.output_channels": output_channels,
                            "audio.output_sample_format": output_sample_format,
                            "audio.callback_silence_due_to_lock": metrics.silence_due_to_lock,
                            "audio.callback_underrun_frames": metrics.underrun_frames,
                            "audio.callback_count": metrics.callback_count,
                            "audio.callback_elapsed_ns_total": metrics.callback_elapsed_ns_total,
                            "audio.callback_elapsed_ns_avg": avg_ns,
                            "audio.callback_elapsed_ns_max": metrics.callback_elapsed_ns_max,
                            "audio.callback_over_period_count": metrics.callback_over_period_count,
                            "audio.callback_metrics_dropped_count": metrics.callback_metrics_dropped_count,
                            "audio.callback_frames_total": metrics.callback_frames_total,
                            "audio.callback_frames_min": metrics.callback_frames_min,
                            "audio.callback_frames_max": metrics.callback_frames_max,
                            "audio.callback_period_ns_min": callback_period_ns_min,
                            "audio.callback_duration_buckets": metrics.callback_duration_buckets,
                            "audio.stream_start_wait_ns": metrics.stream_start_wait_ns,
                            "audio.stream_start_timeout_count": metrics.stream_start_timeout_count,
                            "audio.stream_start_failure_count": metrics.stream_start_failure_count,
                            "audio.output_xrun_count": metrics.output_xrun_count,
                            "audio.output_sample_count": metrics.output_sample_count,
                            "audio.output_peak_abs": metrics.output_peak_abs(),
                            "audio.output_rms": metrics.output_rms(),
                            "audio.output_clipped_samples": metrics.output_clipped_samples,
                            "audio.output_nonfinite_samples": metrics.output_nonfinite_samples,
                        })),
                    );
                });
            }
        })
    }

    fn build_audio_underrun_callback(
        &self,
        session_id: u64,
        sample_buffer: SampleBuffer,
        audio_format: AudioFormat,
    ) -> crate::player::backend::AudioUnderrunHandler {
        let active_session = Arc::clone(&self.active_session_id);
        let app_for_underrun = self.app.clone();

        Arc::new(move || {
            if active_session.load(Ordering::SeqCst) != session_id {
                return;
            }
            if let Some(state) = app_for_underrun.try_state::<crate::app_state::AppState>() {
                state
                    .player()
                    .start_rebuffering(session_id, sample_buffer.clone(), audio_format);
            }
        })
    }

    fn start_rebuffering(
        &self,
        session_id: u64,
        sample_buffer: SampleBuffer,
        audio_format: AudioFormat,
    ) {
        if !self.is_session_active(session_id) {
            return;
        }
        if !claim_rebuffering_session(&self.rebuffering_session_id, session_id) {
            return;
        }

        let Some(app_state) = self.app.try_state::<crate::app_state::AppState>() else {
            clear_rebuffering_session(&self.rebuffering_session_id, session_id);
            return;
        };

        match begin_rebuffering_session(
            &self.backend,
            &self.state,
            &self.active_session_id,
            &self.rebuffering_session_id,
            session_id,
        ) {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                app_state.log_center().record(
                    crate::logging::LogPayload::new(
                        crate::logging::LogLevel::Warn,
                        "player",
                        "player.rebuffer_pause_failed",
                        "Failed to pause output stream for rebuffering",
                    )
                    .details(format!("{error:#}")),
                );
                return;
            }
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);

        let player = Arc::clone(app_state.player());
        let playback_runtime = Arc::clone(app_state.playback_runtime());
        let log_center = Arc::clone(app_state.log_center());
        let stop_flag = self.stop_signal();
        let target_samples = rebuffer_target_samples(audio_format);

        playback_runtime.clone().spawn(async move {
            let wait_buffer = sample_buffer.clone();
            let wait_stop = Arc::clone(&stop_flag);
            let wait_result = playback_runtime
                .handle()
                .spawn_blocking(move || {
                    wait_buffer.wait_for_samples_or_end(target_samples, &wait_stop)
                })
                .await
                .map_err(|error| anyhow::anyhow!(error.to_string()))
                .and_then(|result| result);

            if stop_flag.load(Ordering::SeqCst) || !player.is_session_active(session_id) {
                clear_rebuffering_session(&player.rebuffering_session_id, session_id);
                return;
            }

            match wait_result {
                Ok(SampleWaitOutcome::Ready) => {}
                Ok(SampleWaitOutcome::Ended) => {
                    clear_rebuffering_session(&player.rebuffering_session_id, session_id);
                    player.finish_session(session_id);
                    return;
                }
                Err(error) => {
                    clear_rebuffering_session(&player.rebuffering_session_id, session_id);
                    log_center.record(
                        crate::logging::LogPayload::new(
                            crate::logging::LogLevel::Warn,
                            "player",
                            "player.rebuffer_wait_failed",
                            "Failed while waiting for playback buffer to recover",
                        )
                        .details(format!("{error:#}")),
                    );
                    player.fail_session(session_id);
                    return;
                }
            }

            let Some(resume_result) = resume_rebuffered_session(
                &player.backend,
                &player.state,
                &player.active_session_id,
                &player.rebuffering_session_id,
                session_id,
            ) else {
                return;
            };
            if let Err(error) = resume_result {
                log_center.record(
                    crate::logging::LogPayload::new(
                        crate::logging::LogLevel::Error,
                        "player",
                        "player.rebuffer_resume_failed",
                        "Failed to resume output stream after rebuffering",
                    )
                    .details(format!("{error:#}")),
                );
                player.fail_session(session_id);
                return;
            }

            emit_state_and_sync(&player.app, &player.state, &player.media_session);
        });
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
                let log_center = Arc::clone(state.log_center());
                let player = Arc::clone(state.player());
                let playback_runtime = Arc::clone(state.playback_runtime());
                let message_for_log = message.clone();
                let active_session_for_error = Arc::clone(&active_session);
                let stop_flag_for_error = Arc::clone(&stop_flag);
                playback_runtime.spawn(async move {
                    if stop_flag_for_error.load(Ordering::SeqCst)
                        || active_session_for_error.load(Ordering::SeqCst) != session_id
                    {
                        return;
                    }
                    log_center.record(
                        crate::logging::LogPayload::new(
                            crate::logging::LogLevel::Error,
                            "player",
                            "player.output_stream_failed",
                            "Audio output stream failed",
                        )
                        .details(message_for_log),
                    );
                    player.fail_session(session_id);
                });
            }
        })
    }

    /// 暂停当前播放中的会话。
    ///
    /// 仅当播放器处于非加载中的播放态时才会真正调用后端暂停；否则直接返回成功。
    pub fn pause(&self) -> Result<()> {
        let observed_rebuffering_session = self.rebuffering_session_id.load(Ordering::SeqCst);
        let outcome = pause_with_rebuffer_snapshot(
            &self.backend,
            &self.state,
            &self.active_session_id,
            &self.rebuffering_session_id,
            observed_rebuffering_session,
        )?;
        if outcome == PlaybackControlOutcome::BackendStateChanged {
            self.pause_signal().store(true, Ordering::SeqCst);
        }
        if outcome != PlaybackControlOutcome::Unchanged {
            emit_state_and_sync(&self.app, &self.state, &self.media_session);
        }
        Ok(())
    }

    /// 恢复当前已暂停的会话。
    ///
    /// 仅当播放器仍保留歌曲上下文且状态为已暂停时才会真正调用后端恢复；否则直接返回成功。
    pub fn resume(&self) -> Result<()> {
        let observed_rebuffering_session = self.rebuffering_session_id.load(Ordering::SeqCst);
        let outcome = resume_with_rebuffer_snapshot(
            &self.backend,
            &self.state,
            &self.active_session_id,
            &self.rebuffering_session_id,
            observed_rebuffering_session,
        )?;
        if outcome == PlaybackControlOutcome::BackendStateChanged {
            self.pause_signal().store(false, Ordering::SeqCst);
        }
        if outcome != PlaybackControlOutcome::Unchanged {
            emit_state_and_sync(&self.app, &self.state, &self.media_session);
        }
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
        let stop_flag = self.stop_signal();
        let pause_flag = self.pause_signal();
        let claimed_session_id = session_id.saturating_add(1);
        if !claim_session_end(&self.active_session_id, session_id, claimed_session_id) {
            return;
        }

        stop_flag.store(true, Ordering::SeqCst);
        pause_flag.store(false, Ordering::SeqCst);
        {
            let mut backend = self.backend.lock().unwrap();
            if self.active_session_id.load(Ordering::SeqCst) != claimed_session_id {
                return;
            }
            clear_rebuffering_session(&self.rebuffering_session_id, session_id);
            let _ = backend.stop();
            let mut state = self.state.lock().unwrap();
            *state = PlayerState::default();
            state.session_id = claimed_session_id;
            state.volume = atomic_volume_load(&self.volume);
        }
        emit_state_and_sync(&self.app, &self.state, &self.media_session);
    }

    /// 将指定会话标记为自然结束，并在短暂续播窗口内保留静音输出流。
    ///
    /// 对外快照保留刚结束的 `session_id`，便于前端用 `player-ended` 匹配自动续播；
    /// 内部活跃会话会立即推进，使进度线程、输出错误和迟到回调都被视为过期。
    pub fn finish_session(&self, session_id: u64) {
        let next_session_id = session_id.saturating_add(1);
        let stop_flag = self.stop_signal();
        let pause_flag = self.pause_signal();
        if !claim_session_end(&self.active_session_id, session_id, next_session_id) {
            return;
        }

        stop_flag.store(true, Ordering::SeqCst);
        pause_flag.store(false, Ordering::SeqCst);

        let (ended_event, keepalive_active, quiesce_warning) = {
            let mut backend = self.backend.lock().unwrap();
            if self.active_session_id.load(Ordering::SeqCst) != next_session_id {
                return;
            }
            clear_rebuffering_session(&self.rebuffering_session_id, session_id);
            let mut state = self.state.lock().unwrap();
            let ended_event = if state.session_id != session_id {
                None
            } else {
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
            drop(state);

            match quiesce_or_stop_backend(backend.as_mut()) {
                Ok(None) => (ended_event, true, None),
                Ok(Some(details)) => (ended_event, false, Some(details)),
                Err(error) => (ended_event, false, Some(format!("{error:#}"))),
            }
        };

        if let Some(details) = quiesce_warning {
            if let Some(state) = self.app.try_state::<crate::app_state::AppState>() {
                state.log_center().record(
                    crate::logging::LogPayload::new(
                        crate::logging::LogLevel::Warn,
                        "player",
                        "player.finished_stream_quiesce_failed",
                        "Failed to keep the finished output stream active for automatic playback",
                    )
                    .details(details),
                );
            }
        }

        if let Some(event) = ended_event {
            emit_state_and_sync(&self.app, &self.state, &self.media_session);
            emit_ended(&self.app, event);
        }
        if keepalive_active {
            self.schedule_finished_stream_cleanup(next_session_id);
        }
    }

    fn schedule_finished_stream_cleanup(&self, expected_session_id: u64) {
        let Some(state) = self.app.try_state::<crate::app_state::AppState>() else {
            return;
        };
        let player = Arc::clone(state.player());
        let playback_load_gate = state.playback_load_gate().clone();
        let playback_runtime = Arc::clone(state.playback_runtime());
        playback_runtime.spawn(async move {
            tokio::time::sleep(FINISHED_STREAM_KEEPALIVE_GRACE).await;
            loop {
                if player
                    .try_release_finished_stream_keepalive(expected_session_id, &playback_load_gate)
                {
                    break;
                }
                playback_load_gate.wait_until_inactive().await;
            }
        });
    }

    fn try_release_finished_stream_keepalive(
        &self,
        expected_session_id: u64,
        playback_load_gate: &PlaybackLoadGate,
    ) -> bool {
        try_cleanup_finished_stream_keepalive(
            playback_load_gate,
            self.active_session_id.as_ref(),
            &self.backend,
            expected_session_id,
        )
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

    /// 判断播放请求是否仍为最新请求。
    pub fn is_playback_request_active(&self, request_id: u64) -> bool {
        self.active_request_id.load(Ordering::SeqCst) == request_id
    }

    /// 让当前播放启动请求立即失效。
    ///
    /// 该方法不会停止已经进入播放中的会话，只用于调度层在提交新 play/seek/next/previous
    /// transition 前让旧的异步启动任务尽快返回 `Superseded`。
    pub fn supersede_playback_request(&self) -> u64 {
        self.active_request_id.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// 如果当前会话仍处于加载阶段，则立即让它失效。
    ///
    /// 已经进入 `Playing` 的会话不会被提前打断；新的 transition 会在完成准备并调用
    /// `begin_loading_session` 时按正常路径切换输出。
    pub fn supersede_loading_session(&self) {
        if !self.get_state().is_loading {
            return;
        }

        let _ = self.quiesce_active_backend_for_transition();
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
        self.rebuffering_session_id.store(0, Ordering::SeqCst);
        self.active_session_id.fetch_add(1, Ordering::SeqCst);
        self.backend.lock().unwrap().stop()
    }

    fn quiesce_active_backend_for_transition(&self) -> Result<()> {
        self.stop_signal().store(true, Ordering::SeqCst);
        self.pause_signal().store(false, Ordering::SeqCst);
        self.rebuffering_session_id.store(0, Ordering::SeqCst);
        self.active_session_id.fetch_add(1, Ordering::SeqCst);
        let quiesce_error = {
            let mut backend = self.backend.lock().unwrap();
            quiesce_or_stop_backend(backend.as_mut())?
        };

        if let Some(details) = quiesce_error {
            if let Some(state) = self.app.try_state::<crate::app_state::AppState>() {
                state.log_center().record(
                    crate::logging::LogPayload::new(
                        crate::logging::LogLevel::Warn,
                        "player",
                        "player.transition_quiesce_failed",
                        "Failed to keep output stream active during playback transition",
                    )
                    .details(details),
                );
            }
        }
        Ok(())
    }

    fn install_session_flags(&self) {
        self.rebuffering_session_id.store(0, Ordering::SeqCst);
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
        let safe_volume = normalize_volume(volume);
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
        let safe_volume = normalize_volume(volume);
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

fn quiesce_or_stop_backend(backend: &mut dyn PlaybackBackend) -> Result<Option<String>> {
    let Err(error) = backend.quiesce_for_transition() else {
        return Ok(None);
    };
    let details = format!("{error:#}");
    backend.stop().with_context(|| {
        format!("Failed to stop audio backend after transition quiesce failed: {details}")
    })?;
    Ok(Some(details))
}

fn ensure_started_session_current(is_current: bool) -> Result<()> {
    anyhow::ensure!(is_current, "Playback session expired");
    Ok(())
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

fn build_progress_callback(
    state_for_progress: Arc<Mutex<PlayerState>>,
    active_session: Arc<AtomicU64>,
    stop_flag: Arc<AtomicBool>,
    session_id: u64,
    initial_progress: f64,
) -> Arc<dyn Fn(f64, f64) + Send + Sync> {
    // 该回调运行在音频实时输出线程上，按驱动缓冲帧率高频触发，因此只做最廉价的
    // 非阻塞状态写入。事件广播与系统媒体会话同步交由独立的节流发射线程处理
    // （见 `spawn_progress_emitter`），避免在实时音频线程上执行 IPC / 平台调用或等待共享锁。
    Arc::new(move |progress, duration| {
        if active_session.load(Ordering::SeqCst) != session_id || stop_flag.load(Ordering::SeqCst) {
            return;
        }
        let Ok(mut state) = state_for_progress.try_lock() else {
            return;
        };
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

fn claim_rebuffering_session(owner: &AtomicU64, session_id: u64) -> bool {
    session_id != 0
        && owner
            .compare_exchange(0, session_id, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
}

fn clear_rebuffering_session(owner: &AtomicU64, session_id: u64) {
    let _ = owner.compare_exchange(session_id, 0, Ordering::SeqCst, Ordering::SeqCst);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlaybackControlOutcome {
    Unchanged,
    RebufferStateChanged,
    BackendStateChanged,
}

fn begin_rebuffering_session(
    backend: &Mutex<Box<dyn PlaybackBackend>>,
    state: &Mutex<PlayerState>,
    active_session_id: &AtomicU64,
    owner: &AtomicU64,
    session_id: u64,
) -> Result<bool> {
    let mut backend = backend.lock().unwrap();
    let mut state = state.lock().unwrap();
    if active_session_id.load(Ordering::SeqCst) != session_id
        || owner.load(Ordering::SeqCst) != session_id
        || state.session_id != session_id
        || !state.is_playing
        || state.is_loading
    {
        clear_rebuffering_session(owner, session_id);
        return Ok(false);
    }

    if let Err(error) = backend.pause() {
        clear_rebuffering_session(owner, session_id);
        return Err(error).context("Failed to pause audio backend for rebuffering");
    }
    if active_session_id.load(Ordering::SeqCst) != session_id
        || owner.load(Ordering::SeqCst) != session_id
    {
        clear_rebuffering_session(owner, session_id);
        return Ok(false);
    }

    state.is_playing = false;
    state.is_paused = false;
    state.is_loading = true;
    Ok(true)
}

fn pause_with_rebuffer_snapshot(
    backend: &Mutex<Box<dyn PlaybackBackend>>,
    state: &Mutex<PlayerState>,
    active_session_id: &AtomicU64,
    owner: &AtomicU64,
    observed_rebuffering_session: u64,
) -> Result<PlaybackControlOutcome> {
    if observed_rebuffering_session != 0 {
        let mut state = state.lock().unwrap();
        if owner.load(Ordering::SeqCst) == observed_rebuffering_session
            && active_session_id.load(Ordering::SeqCst) == observed_rebuffering_session
            && state.session_id == observed_rebuffering_session
            && state.is_loading
            && state.song_cid.is_some()
        {
            state.is_loading = false;
            state.is_playing = false;
            state.is_paused = true;
            return Ok(PlaybackControlOutcome::RebufferStateChanged);
        }
    }

    let mut backend = backend.lock().unwrap();
    let mut state = state.lock().unwrap();
    let session_id = state.session_id;
    if active_session_id.load(Ordering::SeqCst) != session_id
        || !state.is_playing
        || state.is_loading
    {
        return Ok(PlaybackControlOutcome::Unchanged);
    }

    backend.pause().context("Failed to pause audio backend")?;
    if active_session_id.load(Ordering::SeqCst) != session_id {
        return Ok(PlaybackControlOutcome::Unchanged);
    }
    state.is_playing = false;
    state.is_paused = true;
    Ok(PlaybackControlOutcome::BackendStateChanged)
}

fn resume_with_rebuffer_snapshot(
    backend: &Mutex<Box<dyn PlaybackBackend>>,
    state: &Mutex<PlayerState>,
    active_session_id: &AtomicU64,
    owner: &AtomicU64,
    observed_rebuffering_session: u64,
) -> Result<PlaybackControlOutcome> {
    if observed_rebuffering_session != 0 {
        let mut state = state.lock().unwrap();
        if owner.load(Ordering::SeqCst) == observed_rebuffering_session
            && active_session_id.load(Ordering::SeqCst) == observed_rebuffering_session
            && state.session_id == observed_rebuffering_session
            && state.is_paused
            && state.song_cid.is_some()
        {
            state.is_loading = true;
            state.is_playing = false;
            state.is_paused = false;
            return Ok(PlaybackControlOutcome::RebufferStateChanged);
        }
    }

    let mut backend = backend.lock().unwrap();
    let mut state = state.lock().unwrap();
    let session_id = state.session_id;
    if active_session_id.load(Ordering::SeqCst) != session_id
        || !state.is_paused
        || state.song_cid.is_none()
    {
        return Ok(PlaybackControlOutcome::Unchanged);
    }

    backend.resume().context("Failed to resume audio backend")?;
    if active_session_id.load(Ordering::SeqCst) != session_id {
        return Ok(PlaybackControlOutcome::Unchanged);
    }
    state.is_playing = true;
    state.is_paused = false;
    Ok(PlaybackControlOutcome::BackendStateChanged)
}

fn resume_rebuffered_session(
    backend: &Mutex<Box<dyn PlaybackBackend>>,
    state: &Mutex<PlayerState>,
    active_session_id: &AtomicU64,
    owner: &AtomicU64,
    session_id: u64,
) -> Option<Result<()>> {
    let mut backend = backend.lock().unwrap();
    let mut state = state.lock().unwrap();
    if active_session_id.load(Ordering::SeqCst) != session_id
        || owner.load(Ordering::SeqCst) != session_id
        || state.session_id != session_id
        || !state.is_loading
        || state.is_paused
    {
        clear_rebuffering_session(owner, session_id);
        return None;
    }

    let result = backend.resume();
    if result.is_ok()
        && active_session_id.load(Ordering::SeqCst) == session_id
        && owner.load(Ordering::SeqCst) == session_id
    {
        state.is_loading = false;
        state.is_playing = true;
        state.is_paused = false;
    }
    clear_rebuffering_session(owner, session_id);
    Some(result)
}

fn rebuffer_target_samples(audio_format: AudioFormat) -> usize {
    let sample_rate = audio_format.sample_rate.max(1) as usize;
    let channels = audio_format.channels.max(1) as usize;
    let target = sample_rate * channels * REBUFFER_TARGET_SECONDS;
    let minimum = channels * 4096;
    let maximum = SampleBuffer::max_capacity_samples() / 2;

    target.max(minimum).min(maximum)
}

fn atomic_volume_load(volume: &AtomicU64) -> f64 {
    normalize_volume(f64::from_bits(volume.load(Ordering::SeqCst)))
}

fn atomic_volume_store(volume: &AtomicU64, value: f64) {
    volume.store(normalize_volume(value).to_bits(), Ordering::SeqCst);
}

fn normalize_volume(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn claim_session_end(active_session_id: &AtomicU64, session_id: u64, next_session_id: u64) -> bool {
    active_session_id
        .compare_exchange(
            session_id,
            next_session_id,
            Ordering::SeqCst,
            Ordering::SeqCst,
        )
        .is_ok()
}

fn cleanup_finished_stream_keepalive(
    active_session_id: &AtomicU64,
    backend: &Mutex<Box<dyn PlaybackBackend>>,
    expected_session_id: u64,
) {
    let cleanup_session_id = expected_session_id.saturating_add(1);
    if !claim_session_end(active_session_id, expected_session_id, cleanup_session_id) {
        return;
    }

    let mut backend = backend.lock().unwrap();
    if active_session_id.load(Ordering::SeqCst) == cleanup_session_id {
        let _ = backend.stop();
    }
}

fn try_cleanup_finished_stream_keepalive(
    playback_load_gate: &PlaybackLoadGate,
    active_session_id: &AtomicU64,
    backend: &Mutex<Box<dyn PlaybackBackend>>,
    expected_session_id: u64,
) -> bool {
    playback_load_gate
        .run_if_inactive(|| {
            cleanup_finished_stream_keepalive(active_session_id, backend, expected_session_id);
        })
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::{
        begin_rebuffering_session, build_progress_callback, claim_rebuffering_session,
        claim_session_end, cleanup_finished_stream_keepalive, clear_rebuffering_session,
        ensure_started_session_current, normalize_volume, pause_with_rebuffer_snapshot,
        quiesce_or_stop_backend, rebuffer_target_samples, resume_rebuffered_session,
        resume_with_rebuffer_snapshot, try_cleanup_finished_stream_keepalive,
        PlaybackControlOutcome, PlaybackQueueEntry, PlaybackQueueState,
    };
    use crate::playback_load_gate::PlaybackLoadGate;
    use crate::player::backend::{
        AudioMetricsHandler, AudioUnderrunHandler, OutputFormat, PlaybackBackend,
    };
    use crate::player::state::PlayerState;
    use crate::player::stream::{AudioFormat, PlaybackErrorHandler, SampleBuffer};
    use anyhow::Result;
    use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier, Mutex};
    use std::time::{Duration, Instant};

    #[derive(Default)]
    struct TransitionBackend {
        events: Vec<&'static str>,
        fail_quiesce: bool,
        fail_stop: bool,
        pause_count: Arc<AtomicUsize>,
        resume_count: Arc<AtomicUsize>,
        stop_count: Arc<AtomicUsize>,
    }

    impl PlaybackBackend for TransitionBackend {
        fn negotiate_output_format(&self, _source_format: AudioFormat) -> Result<OutputFormat> {
            unreachable!("format negotiation is not used by transition tests")
        }

        fn play_stream(
            &mut self,
            _format: OutputFormat,
            _samples: SampleBuffer,
            _stop_flag: Arc<AtomicBool>,
            _volume: Arc<AtomicU64>,
            _progress_callback: Arc<dyn Fn(f64, f64) + Send + Sync>,
            _finish_callback: Arc<dyn Fn() + Send + Sync>,
            _error_handler: PlaybackErrorHandler,
            _metrics_handler: AudioMetricsHandler,
            _underrun_handler: AudioUnderrunHandler,
        ) -> Result<()> {
            unreachable!("stream playback is not used by transition tests")
        }

        fn quiesce_for_transition(&mut self) -> Result<()> {
            self.events.push("quiesce");
            anyhow::ensure!(!self.fail_quiesce, "quiesce failed");
            Ok(())
        }

        fn pause(&mut self) -> Result<()> {
            self.pause_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn resume(&mut self) -> Result<()> {
            self.resume_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn stop(&mut self) -> Result<()> {
            self.events.push("stop");
            self.stop_count.fetch_add(1, Ordering::SeqCst);
            anyhow::ensure!(!self.fail_stop, "stop failed");
            Ok(())
        }
    }

    fn entry(cid: &str) -> PlaybackQueueEntry {
        PlaybackQueueEntry {
            cid: cid.to_string(),
            name: cid.to_string(),
            artists: Vec::new(),
            cover_url: None,
        }
    }

    #[test]
    fn queue_navigation_peeks_across_boundaries_without_moving_cursor() {
        let queue = PlaybackQueueState {
            entries: vec![entry("a"), entry("b")],
            current_index: Some(1),
        };

        assert_eq!(queue.peek_next().map(|entry| entry.cid), Some("a".into()));
        assert_eq!(
            queue.peek_previous().map(|entry| entry.cid),
            Some("a".into())
        );
        assert_eq!(queue.current_index, Some(1));
    }

    #[test]
    fn single_entry_queue_has_no_navigation_targets() {
        let queue = PlaybackQueueState {
            entries: vec![entry("a")],
            current_index: Some(0),
        };

        assert!(!queue.has_next());
        assert!(!queue.has_previous());
        assert!(queue.peek_next().is_none());
        assert!(queue.peek_previous().is_none());
    }

    #[test]
    fn normalize_volume_handles_out_of_range_and_non_finite_values() {
        assert_eq!(normalize_volume(-1.0), 0.0);
        assert_eq!(normalize_volume(0.4), 0.4);
        assert_eq!(normalize_volume(2.0), 1.0);
        assert_eq!(normalize_volume(f64::NAN), 0.0);
        assert_eq!(normalize_volume(f64::INFINITY), 0.0);
    }

    #[test]
    fn transition_keeps_backend_alive_when_quiesce_succeeds() {
        let mut backend = TransitionBackend::default();

        let warning = quiesce_or_stop_backend(&mut backend).expect("quiesce");

        assert_eq!(backend.events, ["quiesce"]);
        assert!(warning.is_none());
    }

    #[test]
    fn superseded_stream_start_preserves_transition_keepalive() {
        let mut backend = TransitionBackend::default();
        quiesce_or_stop_backend(&mut backend).expect("quiesce");

        let error = ensure_started_session_current(false).expect_err("session must expire");

        assert_eq!(format!("{error:#}"), "Playback session expired");
        assert_eq!(backend.events, ["quiesce"]);
    }

    #[test]
    fn transition_falls_back_to_stop_when_quiesce_fails() {
        let mut backend = TransitionBackend {
            fail_quiesce: true,
            ..Default::default()
        };

        let warning = quiesce_or_stop_backend(&mut backend).expect("fallback stop");

        assert_eq!(backend.events, ["quiesce", "stop"]);
        assert_eq!(warning.as_deref(), Some("quiesce failed"));
    }

    #[test]
    fn transition_reports_stop_failure_after_quiesce_failure() {
        let mut backend = TransitionBackend {
            fail_quiesce: true,
            fail_stop: true,
            ..Default::default()
        };

        let error = quiesce_or_stop_backend(&mut backend).expect_err("stop must fail");

        assert_eq!(backend.events, ["quiesce", "stop"]);
        assert!(format!("{error:#}").contains("stop failed"), "{error:#}");
    }

    #[test]
    fn session_end_can_only_be_claimed_once() {
        let active_session_id = AtomicU64::new(7);

        assert!(claim_session_end(&active_session_id, 7, 8));
        assert!(!claim_session_end(&active_session_id, 7, 8));
        assert_eq!(active_session_id.load(Ordering::SeqCst), 8);
    }

    #[test]
    fn concurrent_session_end_claim_has_exactly_one_winner() {
        const CONTENDERS: usize = 16;
        let active_session_id = Arc::new(AtomicU64::new(7));
        let winners = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(CONTENDERS + 1));
        let handles = (0..CONTENDERS)
            .map(|_| {
                let active_session_id = Arc::clone(&active_session_id);
                let winners = Arc::clone(&winners);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    if claim_session_end(&active_session_id, 7, 8) {
                        winners.fetch_add(1, Ordering::SeqCst);
                    }
                })
            })
            .collect::<Vec<_>>();

        barrier.wait();
        for handle in handles {
            handle.join().expect("claim contender should finish");
        }

        assert_eq!(winners.load(Ordering::SeqCst), 1);
        assert_eq!(active_session_id.load(Ordering::SeqCst), 8);
    }

    #[test]
    fn finished_stream_cleanup_stops_owned_keepalive_once() {
        let expected_session_id = 7;
        let active_session_id = AtomicU64::new(expected_session_id);
        let stop_count = Arc::new(AtomicUsize::new(0));
        let backend: Mutex<Box<dyn PlaybackBackend>> = Mutex::new(Box::new(TransitionBackend {
            stop_count: Arc::clone(&stop_count),
            ..Default::default()
        }));

        cleanup_finished_stream_keepalive(&active_session_id, &backend, expected_session_id);
        cleanup_finished_stream_keepalive(&active_session_id, &backend, expected_session_id);

        assert_eq!(active_session_id.load(Ordering::SeqCst), 8);
        assert_eq!(stop_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn finished_stream_cleanup_preserves_already_superseded_stream() {
        let expected_session_id = 7;
        let successor_session_id = 8;
        let active_session_id = AtomicU64::new(successor_session_id);
        let stop_count = Arc::new(AtomicUsize::new(0));
        let backend: Mutex<Box<dyn PlaybackBackend>> = Mutex::new(Box::new(TransitionBackend {
            stop_count: Arc::clone(&stop_count),
            ..Default::default()
        }));

        cleanup_finished_stream_keepalive(&active_session_id, &backend, expected_session_id);

        assert_eq!(
            active_session_id.load(Ordering::SeqCst),
            successor_session_id
        );
        assert_eq!(stop_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn finished_stream_cleanup_waits_for_pending_playback_request() {
        let expected_session_id = 7;
        let active_session_id = AtomicU64::new(expected_session_id);
        let stop_count = Arc::new(AtomicUsize::new(0));
        let backend: Mutex<Box<dyn PlaybackBackend>> = Mutex::new(Box::new(TransitionBackend {
            stop_count: Arc::clone(&stop_count),
            ..Default::default()
        }));
        let playback_load_gate = PlaybackLoadGate::new();
        let pending_request = playback_load_gate.enter();

        assert!(!try_cleanup_finished_stream_keepalive(
            &playback_load_gate,
            &active_session_id,
            &backend,
            expected_session_id,
        ));
        assert_eq!(
            active_session_id.load(Ordering::SeqCst),
            expected_session_id
        );
        assert_eq!(stop_count.load(Ordering::SeqCst), 0);

        drop(pending_request);
        assert!(try_cleanup_finished_stream_keepalive(
            &playback_load_gate,
            &active_session_id,
            &backend,
            expected_session_id,
        ));
        assert_eq!(active_session_id.load(Ordering::SeqCst), 8);
        assert_eq!(stop_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn finished_stream_cleanup_rechecks_ownership_after_backend_lock() {
        let expected_session_id = 7;
        let cleanup_session_id = 8;
        let successor_session_id = 9;
        let active_session_id = Arc::new(AtomicU64::new(expected_session_id));
        let stop_count = Arc::new(AtomicUsize::new(0));
        let backend: Arc<Mutex<Box<dyn PlaybackBackend>>> =
            Arc::new(Mutex::new(Box::new(TransitionBackend {
                stop_count: Arc::clone(&stop_count),
                ..Default::default()
            })));
        let cleanup_backend = Arc::clone(&backend);
        let cleanup_active_session_id = Arc::clone(&active_session_id);
        let cleanup_started = Arc::new(Barrier::new(2));
        let cleanup_thread_started = Arc::clone(&cleanup_started);
        let backend_guard = backend.lock().unwrap();
        let cleanup = std::thread::spawn(move || {
            cleanup_thread_started.wait();
            cleanup_finished_stream_keepalive(
                cleanup_active_session_id.as_ref(),
                cleanup_backend.as_ref(),
                expected_session_id,
            );
        });

        cleanup_started.wait();
        let claim_deadline = Instant::now() + Duration::from_secs(5);
        while active_session_id.load(Ordering::SeqCst) != cleanup_session_id {
            assert!(
                Instant::now() < claim_deadline,
                "cleanup did not claim the finished session"
            );
            std::thread::yield_now();
        }
        assert_eq!(
            active_session_id.fetch_add(1, Ordering::SeqCst),
            cleanup_session_id
        );
        drop(backend_guard);
        cleanup.join().expect("cleanup thread should finish");

        assert_eq!(
            active_session_id.load(Ordering::SeqCst),
            successor_session_id
        );
        assert_eq!(stop_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn superseding_request_invalidates_older_request_ids() {
        let active_request_id = AtomicU64::new(0);

        let first = active_request_id.fetch_add(1, Ordering::SeqCst) + 1;
        let superseding = active_request_id.fetch_add(1, Ordering::SeqCst) + 1;

        assert_eq!(first, 1);
        assert_eq!(superseding, 2);
        assert_ne!(active_request_id.load(Ordering::SeqCst), first);
        assert_eq!(active_request_id.load(Ordering::SeqCst), superseding);
    }

    #[test]
    fn rebuffer_target_samples_uses_three_seconds_with_capacity_cap() {
        let stereo_48k = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 60.0,
            bits_per_sample: None,
        };
        assert_eq!(rebuffer_target_samples(stereo_48k), 288_000);

        let high_rate = AudioFormat {
            channels: 2,
            sample_rate: 768_000,
            duration_secs: 60.0,
            bits_per_sample: None,
        };
        assert_eq!(
            rebuffer_target_samples(high_rate),
            SampleBuffer::max_capacity_samples() / 2
        );
    }

    #[test]
    fn stale_rebuffer_task_cannot_clear_new_session_owner() {
        let owner = AtomicU64::new(0);

        assert!(claim_rebuffering_session(&owner, 7));
        clear_rebuffering_session(&owner, 7);
        assert!(claim_rebuffering_session(&owner, 8));

        clear_rebuffering_session(&owner, 7);
        assert_eq!(owner.load(Ordering::SeqCst), 8);

        clear_rebuffering_session(&owner, 8);
        assert_eq!(owner.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn stale_rebuffer_claim_cannot_pause_after_session_transition() {
        let pause_count = Arc::new(AtomicUsize::new(0));
        let backend: Mutex<Box<dyn PlaybackBackend>> = Mutex::new(Box::new(TransitionBackend {
            pause_count: Arc::clone(&pause_count),
            ..Default::default()
        }));
        let state = Mutex::new(PlayerState {
            session_id: 7,
            is_playing: true,
            ..Default::default()
        });
        let active_session_id = AtomicU64::new(8);
        let owner = AtomicU64::new(7);

        assert!(
            !begin_rebuffering_session(&backend, &state, &active_session_id, &owner, 7,)
                .expect("stale rebuffer claim should be ignored")
        );
        assert_eq!(pause_count.load(Ordering::SeqCst), 0);
        assert_eq!(owner.load(Ordering::SeqCst), 0);
        assert!(state.lock().unwrap().is_playing);
    }

    #[test]
    fn stale_rebuffer_snapshot_falls_through_to_backend_pause() {
        let pause_count = Arc::new(AtomicUsize::new(0));
        let backend: Mutex<Box<dyn PlaybackBackend>> = Mutex::new(Box::new(TransitionBackend {
            pause_count: Arc::clone(&pause_count),
            ..Default::default()
        }));
        let state = Mutex::new(PlayerState {
            session_id: 7,
            song_cid: Some("track".into()),
            is_playing: true,
            ..Default::default()
        });
        let active_session_id = AtomicU64::new(7);
        let owner = AtomicU64::new(0);

        let outcome = pause_with_rebuffer_snapshot(&backend, &state, &active_session_id, &owner, 7)
            .expect("pause should fall through after automatic resume clears owner");

        assert_eq!(outcome, PlaybackControlOutcome::BackendStateChanged);
        assert_eq!(pause_count.load(Ordering::SeqCst), 1);
        let state = state.lock().unwrap();
        assert!(!state.is_playing);
        assert!(state.is_paused);
    }

    #[test]
    fn stale_rebuffer_snapshot_falls_through_to_backend_resume() {
        let resume_count = Arc::new(AtomicUsize::new(0));
        let backend: Mutex<Box<dyn PlaybackBackend>> = Mutex::new(Box::new(TransitionBackend {
            resume_count: Arc::clone(&resume_count),
            ..Default::default()
        }));
        let state = Mutex::new(PlayerState {
            session_id: 7,
            song_cid: Some("track".into()),
            is_paused: true,
            ..Default::default()
        });
        let active_session_id = AtomicU64::new(7);
        let owner = AtomicU64::new(0);

        let outcome =
            resume_with_rebuffer_snapshot(&backend, &state, &active_session_id, &owner, 7)
                .expect("resume should fall through after rebuffer worker clears owner");

        assert_eq!(outcome, PlaybackControlOutcome::BackendStateChanged);
        assert_eq!(resume_count.load(Ordering::SeqCst), 1);
        let state = state.lock().unwrap();
        assert!(state.is_playing);
        assert!(!state.is_loading);
        assert!(!state.is_paused);
    }

    #[test]
    fn pause_committed_while_resume_waits_prevents_backend_resume() {
        let resume_count = Arc::new(AtomicUsize::new(0));
        let backend: Arc<Mutex<Box<dyn PlaybackBackend>>> =
            Arc::new(Mutex::new(Box::new(TransitionBackend {
                resume_count: Arc::clone(&resume_count),
                ..Default::default()
            })));
        let state = Arc::new(Mutex::new(PlayerState {
            session_id: 7,
            is_loading: true,
            ..Default::default()
        }));
        let owner = Arc::new(AtomicU64::new(7));
        let active_session_id = Arc::new(AtomicU64::new(7));
        let mut state_guard = state.lock().unwrap();
        let resume_backend = Arc::clone(&backend);
        let resume_state = Arc::clone(&state);
        let resume_active_session_id = Arc::clone(&active_session_id);
        let resume_owner = Arc::clone(&owner);
        let resume_thread = std::thread::spawn(move || {
            resume_rebuffered_session(
                &resume_backend,
                &resume_state,
                &resume_active_session_id,
                &resume_owner,
                7,
            )
        });

        let lock_deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match backend.try_lock() {
                Err(std::sync::TryLockError::WouldBlock) => break,
                Err(std::sync::TryLockError::Poisoned(error)) => panic!("{error}"),
                Ok(guard) => drop(guard),
            }
            assert!(
                Instant::now() < lock_deadline,
                "resume thread did not acquire the backend lock"
            );
            std::thread::yield_now();
        }
        state_guard.is_loading = false;
        state_guard.is_paused = true;
        drop(state_guard);

        assert!(resume_thread.join().expect("resume thread").is_none());
        assert_eq!(resume_count.load(Ordering::SeqCst), 0);
        assert_eq!(owner.load(Ordering::SeqCst), 0);
        assert!(state.lock().unwrap().is_paused);
    }

    #[test]
    fn progress_callback_skips_update_when_state_lock_is_busy() {
        let state = Arc::new(Mutex::new(crate::player::state::PlayerState::default()));
        let active_session = Arc::new(AtomicU64::new(1));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let callback =
            build_progress_callback(Arc::clone(&state), active_session, stop_flag, 1, 0.0);

        {
            let _guard = state.lock().unwrap();
            callback(12.0, 120.0);
        }

        assert_eq!(state.lock().unwrap().progress, 0.0);

        callback(12.0, 120.0);

        let snapshot = state.lock().unwrap();
        assert_eq!(snapshot.progress, 12.0);
        assert_eq!(snapshot.duration, 120.0);
    }
}
