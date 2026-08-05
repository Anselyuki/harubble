//! 基于 CPAL 的默认音频播放后端实现。
//!
//! 该模块负责选择可用输出设备与格式、创建音频输出流，并把播放器解码后的样本缓冲
//! 推送到系统音频设备，供桌面端实际发声使用。

use super::cpal_helpers::*;
use crate::player::backend::{
    AudioCallbackMetrics, AudioMetricsHandler, AudioUnderrunHandler, OutputFormat, PlaybackBackend,
    CALLBACK_DURATION_BUCKETS,
};
use crate::player::stream::{AudioFormat, PlaybackErrorHandler, SampleBuffer};
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    FromSample, Sample, SampleFormat, SizedSample, Stream, StreamError, SupportedStreamConfig,
    SupportedStreamConfigRange, I24, U24,
};
use crossbeam_queue::ArrayQueue;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

// 该等待只运行在旧静音流的退休线程中，不会阻塞播放 actor；给慢唤醒、AirPlay
// 和聚合设备留出足够时间完成首个 callback。
#[cfg(target_os = "macos")]
const STREAM_START_CALLBACK_TIMEOUT: Duration = Duration::from_secs(10);
#[cfg(target_os = "macos")]
const STREAM_START_CALLBACK_POLL_INTERVAL: Duration = Duration::from_millis(1);
const OUTPUT_SQUARE_Q32_SCALE: f64 = (1_u64 << 32) as f64;
const CALLBACK_METRIC_QUEUE_CAPACITY: usize = 16_384;
const INITIAL_OUTPUT_SCRATCH_FRAMES: usize = 8_192;
const STREAM_START_PENDING: u8 = 0;
const STREAM_START_READY: u8 = 1;
const STREAM_START_FAILED: u8 = 2;

fn choose_negotiated_output_config(
    device: &cpal::Device,
    audio_format: AudioFormat,
) -> Result<SupportedStreamConfig> {
    let default_config = device.default_output_config().ok();
    let configs = match device.supported_output_configs() {
        Ok(configs) => configs.collect::<Vec<_>>(),
        Err(error) => {
            if let Some(config) = default_config.filter(is_supported_output_config) {
                return Ok(config);
            }
            return Err(error).context("Failed to query supported output configs");
        }
    };

    choose_output_config_from_ranges(default_config, &configs, audio_format)
}

fn choose_stream_output_config(
    device: &cpal::Device,
    output_format: &OutputFormat,
) -> Result<SupportedStreamConfig> {
    ensure_negotiated_device_is_current(device, output_format)?;

    let default_config = device.default_output_config().ok();
    let configs = match device.supported_output_configs() {
        Ok(configs) => configs.collect::<Vec<_>>(),
        Err(error) => {
            if let Some(config) =
                choose_exact_output_config_from_default(default_config, output_format)
            {
                return Ok(config);
            }
            return Err(error).context("Failed to query supported output configs");
        }
    };

    choose_exact_output_config_from_ranges(&configs, output_format.clone())
        .or_else(|| choose_exact_output_config_from_default(default_config, output_format))
        .context("Selected output configuration is no longer supported")
}

fn ensure_negotiated_device_is_current(
    device: &cpal::Device,
    output_format: &OutputFormat,
) -> Result<()> {
    let current_identity = output_device_identity(device)?;
    anyhow::ensure!(
        current_identity == output_format.device_identity,
        "Default output device changed from {} to {current_identity}",
        output_format.device_identity
    );
    Ok(())
}

fn output_device_identity(device: &cpal::Device) -> Result<String> {
    Ok(format!(
        "id:{}",
        device.id().context("Failed to identify output device")?
    ))
}

fn choose_output_config_from_ranges(
    default_config: Option<SupportedStreamConfig>,
    configs: &[SupportedStreamConfigRange],
    audio_format: AudioFormat,
) -> Result<SupportedStreamConfig> {
    if let Some(config) = choose_exact_output_config_from_ranges(configs, audio_format) {
        return Ok(config);
    }

    if let Some(config) = default_config.filter(is_supported_output_config) {
        return Ok(config);
    }

    let fallback = configs
        .iter()
        .filter(|config| is_supported_sample_format(config.sample_format()))
        .min_by_key(|config| {
            (
                usize::from(config.channels() != audio_format.channels),
                config.channels().abs_diff(audio_format.channels),
                sample_rate_distance(config, audio_format.sample_rate),
                sample_format_priority(config.sample_format()).unwrap_or(usize::MAX),
            )
        })
        .context("No supported output configuration found")?;

    Ok(fallback.with_sample_rate(clamp_sample_rate(fallback, audio_format.sample_rate)))
}

fn choose_exact_output_config_from_ranges(
    configs: &[SupportedStreamConfigRange],
    output_format: impl Into<ExactOutputFormat>,
) -> Option<SupportedStreamConfig> {
    let output_format = output_format.into();
    let audio_format = output_format.audio_format;
    configs
        .iter()
        .filter(|config| {
            is_supported_sample_format(config.sample_format())
                && config.channels() == audio_format.channels
                && config.min_sample_rate() <= audio_format.sample_rate
                && config.max_sample_rate() >= audio_format.sample_rate
                && output_format
                    .sample_format
                    .is_none_or(|sample_format| sample_format == config.sample_format())
        })
        .min_by_key(|config| sample_format_priority(config.sample_format()).unwrap_or(usize::MAX))
        .map(|config| (*config).with_sample_rate(audio_format.sample_rate))
}

fn choose_exact_output_config_from_default(
    default_config: Option<SupportedStreamConfig>,
    output_format: &OutputFormat,
) -> Option<SupportedStreamConfig> {
    default_config.filter(|config| {
        is_supported_output_config(config)
            && config.channels() == output_format.audio_format.channels
            && config.sample_rate() == output_format.audio_format.sample_rate
            && config.sample_format() == cpal_sample_format(output_format.sample_format)
    })
}

struct ExactOutputFormat {
    audio_format: AudioFormat,
    sample_format: Option<SampleFormat>,
}

impl From<AudioFormat> for ExactOutputFormat {
    fn from(audio_format: AudioFormat) -> Self {
        Self {
            audio_format,
            sample_format: None,
        }
    }
}

impl From<OutputFormat> for ExactOutputFormat {
    fn from(output_format: OutputFormat) -> Self {
        Self {
            audio_format: output_format.audio_format,
            sample_format: Some(cpal_sample_format(output_format.sample_format)),
        }
    }
}

struct CallbackMetricCounters {
    output_rate: u64,
    callback_observations: ArrayQueue<CallbackObservation>,
    callback_metrics_dropped_count: AtomicU64,
    stream_start_wait_ns: AtomicU64,
    stream_start_timeout_count: AtomicU64,
    stream_start_failure_count: AtomicU64,
    output_xrun_count: AtomicU64,
}

impl Default for CallbackMetricCounters {
    fn default() -> Self {
        Self::new(48_000)
    }
}

impl CallbackMetricCounters {
    fn new(output_rate: u32) -> Self {
        Self {
            output_rate: u64::from(output_rate.max(1)),
            callback_observations: ArrayQueue::new(CALLBACK_METRIC_QUEUE_CAPACITY),
            callback_metrics_dropped_count: AtomicU64::new(0),
            stream_start_wait_ns: AtomicU64::new(0),
            stream_start_timeout_count: AtomicU64::new(0),
            stream_start_failure_count: AtomicU64::new(0),
            output_xrun_count: AtomicU64::new(0),
        }
    }

    fn record_callback(&self, observation: CallbackObservation) {
        if self.callback_observations.push(observation).is_err() {
            self.callback_metrics_dropped_count
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    #[cfg(target_os = "macos")]
    fn record_stream_start_wait(&self, elapsed: Duration, outcome: StreamStartWaitOutcome) {
        let elapsed_ns = elapsed.as_nanos().min(u64::MAX as u128) as u64;
        update_atomic_max(&self.stream_start_wait_ns, elapsed_ns);
        match outcome {
            StreamStartWaitOutcome::TimedOut => {
                self.stream_start_timeout_count
                    .fetch_add(1, Ordering::Relaxed);
            }
            StreamStartWaitOutcome::Failed => {
                self.stream_start_failure_count
                    .fetch_add(1, Ordering::Relaxed);
            }
            StreamStartWaitOutcome::Ready | StreamStartWaitOutcome::Cancelled => {}
        }
    }

    fn record_nonfatal_stream_error(&self, disposition: StreamErrorDisposition) {
        match disposition {
            StreamErrorDisposition::Xrun => {
                self.output_xrun_count.fetch_add(1, Ordering::Relaxed);
            }
            StreamErrorDisposition::Fatal => {}
        }
    }

    fn drain(&self) -> AudioCallbackMetrics {
        let mut metrics = AudioCallbackMetrics {
            callback_frames_min: u64::MAX,
            ..AudioCallbackMetrics::default()
        };
        let mut output_square_sum = 0.0_f64;

        while let Some(observation) = self.callback_observations.pop() {
            metrics.silence_due_to_lock = metrics
                .silence_due_to_lock
                .saturating_add(observation.silence_due_to_lock);
            metrics.underrun_frames = metrics
                .underrun_frames
                .saturating_add(observation.underrun_frames);
            metrics.callback_count = metrics.callback_count.saturating_add(1);
            metrics.callback_elapsed_ns_total = metrics
                .callback_elapsed_ns_total
                .saturating_add(observation.elapsed_ns);
            metrics.callback_elapsed_ns_max =
                metrics.callback_elapsed_ns_max.max(observation.elapsed_ns);
            metrics.callback_frames_total = metrics
                .callback_frames_total
                .saturating_add(observation.callback_frames);
            metrics.callback_frames_min =
                metrics.callback_frames_min.min(observation.callback_frames);
            metrics.callback_frames_max =
                metrics.callback_frames_max.max(observation.callback_frames);

            let callback_period_ns = observation
                .callback_frames
                .saturating_mul(1_000_000_000)
                .checked_div(self.output_rate)
                .unwrap_or(0);
            if callback_period_ns > 0 && observation.elapsed_ns > callback_period_ns {
                metrics.callback_over_period_count =
                    metrics.callback_over_period_count.saturating_add(1);
            }
            let bucket = callback_duration_bucket(observation.elapsed_ns);
            metrics.callback_duration_buckets[bucket] =
                metrics.callback_duration_buckets[bucket].saturating_add(1);

            metrics.output_sample_count = metrics
                .output_sample_count
                .saturating_add(observation.output.sample_count);
            output_square_sum += observation.output.square_sum;
            metrics.output_peak_abs_bits = metrics
                .output_peak_abs_bits
                .max(observation.output.peak_abs_bits);
            metrics.output_clipped_samples = metrics
                .output_clipped_samples
                .saturating_add(observation.output.clipped_samples);
            metrics.output_nonfinite_samples = metrics
                .output_nonfinite_samples
                .saturating_add(observation.output.nonfinite_samples);
        }

        if metrics.callback_frames_min == u64::MAX {
            metrics.callback_frames_min = 0;
        }
        metrics.callback_metrics_dropped_count = self
            .callback_metrics_dropped_count
            .swap(0, Ordering::Relaxed);
        metrics.stream_start_wait_ns = self.stream_start_wait_ns.swap(0, Ordering::Relaxed);
        metrics.stream_start_timeout_count =
            self.stream_start_timeout_count.swap(0, Ordering::Relaxed);
        metrics.stream_start_failure_count =
            self.stream_start_failure_count.swap(0, Ordering::Relaxed);
        metrics.output_xrun_count = self.output_xrun_count.swap(0, Ordering::Relaxed);
        metrics.output_square_sum_q32 =
            (output_square_sum * OUTPUT_SQUARE_Q32_SCALE).round() as u64;
        metrics
    }
}

#[cfg(target_os = "macos")]
fn update_atomic_max(target: &AtomicU64, value: u64) {
    let mut current = target.load(Ordering::Relaxed);
    while value > current {
        match target.compare_exchange_weak(current, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct OutputSampleObservation {
    sample_count: u64,
    square_sum: f64,
    peak_abs_bits: u64,
    clipped_samples: u64,
    nonfinite_samples: u64,
}

impl OutputSampleObservation {
    fn observe(&mut self, sample: f32) -> f32 {
        self.observe_with_source(sample, sample)
    }

    fn observe_with_source(&mut self, output_sample: f32, source_sample: f32) -> f32 {
        self.sample_count = self.sample_count.saturating_add(1);
        if !source_sample.is_finite() {
            self.nonfinite_samples = self.nonfinite_samples.saturating_add(1);
        } else if source_sample.abs() > 1.0 {
            self.clipped_samples = self.clipped_samples.saturating_add(1);
        }

        let sanitized = sanitize_output_sample(output_sample);
        self.peak_abs_bits = self.peak_abs_bits.max(u64::from(sanitized.abs().to_bits()));
        self.square_sum += f64::from(sanitized) * f64::from(sanitized);
        sanitized
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct CallbackObservation {
    silence_due_to_lock: u64,
    underrun_frames: u64,
    elapsed_ns: u64,
    callback_frames: u64,
    output: OutputSampleObservation,
}

struct CallbackMetricRecord<'a> {
    counters: &'a CallbackMetricCounters,
    started_at: Instant,
    observation: CallbackObservation,
}

impl<'a> CallbackMetricRecord<'a> {
    fn new(counters: &'a CallbackMetricCounters, callback_frames: u64) -> Self {
        Self {
            counters,
            started_at: Instant::now(),
            observation: CallbackObservation {
                callback_frames,
                ..CallbackObservation::default()
            },
        }
    }

    fn record_silence_due_to_lock(&mut self) {
        self.observation.silence_due_to_lock = 1;
    }

    fn record_underrun_frames(&mut self, frames: u64) {
        self.observation.underrun_frames = frames;
    }

    fn record_output_observation(&mut self, observation: OutputSampleObservation) {
        self.observation.output = observation;
    }
}

impl Drop for CallbackMetricRecord<'_> {
    fn drop(&mut self) {
        self.observation.elapsed_ns =
            self.started_at.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        self.counters.record_callback(self.observation);
    }
}

/// 把纳秒转成 log2μs 直方图桶索引。
///
/// 桶 i 覆盖 `[2^i μs, 2^(i+1) μs)`；桶 15（最后一桶）为 ≥32768μs 的溢出桶。
/// monitor 聚合完整回调观测时使用整数移位计算。
fn callback_duration_bucket(elapsed_ns: u64) -> usize {
    let micros = elapsed_ns / 1000;
    if micros == 0 {
        return 0;
    }
    let log2 = 63 - micros.leading_zeros() as usize;
    log2.min(CALLBACK_DURATION_BUCKETS - 1)
}

#[cfg(any(target_os = "macos", test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamStartWaitOutcome {
    Ready,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamErrorDisposition {
    Xrun,
    Fatal,
}

fn stream_error_disposition(error: &StreamError) -> StreamErrorDisposition {
    match error {
        StreamError::BufferUnderrun => StreamErrorDisposition::Xrun,
        _ => StreamErrorDisposition::Fatal,
    }
}

#[cfg(any(target_os = "macos", test))]
fn current_stream_start_outcome(
    state: &AtomicU8,
    stop_flag: &AtomicBool,
) -> Option<StreamStartWaitOutcome> {
    if stop_flag.load(Ordering::SeqCst) {
        return Some(StreamStartWaitOutcome::Cancelled);
    }
    match state.load(Ordering::Acquire) {
        STREAM_START_READY => Some(StreamStartWaitOutcome::Ready),
        STREAM_START_FAILED => Some(StreamStartWaitOutcome::Failed),
        _ => None,
    }
}

#[cfg(any(target_os = "macos", test))]
fn wait_for_stream_start(
    state: &AtomicU8,
    stop_flag: &AtomicBool,
    timeout: Duration,
    poll_interval: Duration,
) -> (StreamStartWaitOutcome, Duration) {
    let started_at = Instant::now();
    loop {
        if let Some(outcome) = current_stream_start_outcome(state, stop_flag) {
            return (outcome, started_at.elapsed());
        }

        let elapsed = started_at.elapsed();
        if elapsed >= timeout {
            if let Some(outcome) = current_stream_start_outcome(state, stop_flag) {
                return (outcome, started_at.elapsed());
            }
            return (StreamStartWaitOutcome::TimedOut, elapsed);
        }
        thread::sleep(poll_interval.min(timeout.saturating_sub(elapsed)));
    }
}

pub struct CpalBackend {
    stream: Option<Stream>,
    samples: Option<SampleBuffer>,
}

struct PendingSampleBufferGuard(Option<SampleBuffer>);

impl PendingSampleBufferGuard {
    fn new(samples: SampleBuffer) -> Self {
        Self(Some(samples))
    }

    fn disarm(&mut self) {
        self.0.take();
    }
}

impl Drop for PendingSampleBufferGuard {
    fn drop(&mut self) {
        if let Some(samples) = self.0.take() {
            samples.cancel();
        }
    }
}

impl CpalBackend {
    pub fn new() -> Result<Self> {
        Ok(Self {
            stream: None,
            samples: None,
        })
    }
}

impl PlaybackBackend for CpalBackend {
    fn negotiate_output_format(&self, source_format: AudioFormat) -> Result<OutputFormat> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No default output device available")?;
        let config = choose_negotiated_output_config(&device, source_format)?;
        let sample_format = output_sample_format(config.sample_format())
            .context("Unsupported output sample format")?;
        Ok(OutputFormat {
            audio_format: AudioFormat::with_bits_per_sample(
                config.channels(),
                config.sample_rate(),
                source_format.duration_secs,
                Some(sample_format.bits_per_sample()),
            ),
            sample_format,
            device_identity: output_device_identity(&device)?,
        })
    }

    fn play_stream(
        &mut self,
        format: OutputFormat,
        samples: SampleBuffer,
        stop_flag: Arc<AtomicBool>,
        volume: Arc<AtomicU64>,
        progress_callback: Arc<dyn Fn(f64, f64) + Send + Sync>,
        finish_callback: Arc<dyn Fn() + Send + Sync>,
        error_handler: PlaybackErrorHandler,
        metrics_handler: AudioMetricsHandler,
        underrun_handler: AudioUnderrunHandler,
    ) -> Result<()> {
        let mut pending_samples = PendingSampleBufferGuard::new(samples.clone());

        #[cfg(not(target_os = "macos"))]
        self.stop()?;

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No default output device available")?;
        let config = choose_stream_output_config(&device, &format)?;
        let stream_config: cpal::StreamConfig = config.clone().into();
        let audio_format = format.audio_format;

        let total_duration = audio_format.duration_secs;
        let output_rate = config.sample_rate().max(1);
        let output_channels = config.channels().max(1);
        let frames_rendered = Arc::new(AtomicU64::new(0));
        let finish_fired = Arc::new(AtomicBool::new(false));
        let buffer_error_reported = Arc::new(AtomicBool::new(false));
        let callback_metrics = Arc::new(CallbackMetricCounters::new(output_rate));
        let underrun_requested = Arc::new(AtomicBool::new(false));
        let stream_start_state = Arc::new(AtomicU8::new(STREAM_START_PENDING));

        macro_rules! build_stream_for_sample {
            ($sample_type:ty) => {
                build_stream::<$sample_type>(
                    &device,
                    &stream_config,
                    samples.clone(),
                    Arc::clone(&stop_flag),
                    Arc::clone(&volume),
                    Arc::clone(&frames_rendered),
                    Arc::clone(&finish_fired),
                    Arc::clone(&buffer_error_reported),
                    Arc::clone(&error_handler),
                    Arc::clone(&callback_metrics),
                    Arc::clone(&underrun_requested),
                    Arc::clone(&stream_start_state),
                    output_channels,
                    output_dither_lsb(config.sample_format()),
                )?
            };
        }

        let stream = match config.sample_format() {
            SampleFormat::F32 => build_stream_for_sample!(f32),
            SampleFormat::F64 => build_stream_for_sample!(f64),
            SampleFormat::I8 => build_stream_for_sample!(i8),
            SampleFormat::I16 => build_stream_for_sample!(i16),
            SampleFormat::I24 => build_stream_for_sample!(I24),
            SampleFormat::I32 => build_stream_for_sample!(i32),
            SampleFormat::I64 => build_stream_for_sample!(i64),
            SampleFormat::U8 => build_stream_for_sample!(u8),
            SampleFormat::U16 => build_stream_for_sample!(u16),
            SampleFormat::U24 => build_stream_for_sample!(U24),
            SampleFormat::U32 => build_stream_for_sample!(u32),
            SampleFormat::U64 => build_stream_for_sample!(u64),
            sample_format => anyhow::bail!("Unsupported output sample format {sample_format}"),
        };

        stream.play().context("Failed to start output stream")?;

        // macOS 上旧流已被会话 stop flag 切到 equilibrium。新流先接管后，旧流交给
        // 独立退休线程等待首个 callback，再释放它，避免慢唤醒设备阻塞播放 actor 或
        // 在歌曲加载期间失去稳定的静音时钟。其他平台已在开流前 stop。
        let previous_stream = self.stream.replace(stream);
        let previous_samples = self.samples.replace(samples);
        pending_samples.disarm();
        if let Some(previous_samples) = previous_samples {
            previous_samples.cancel();
        }

        #[cfg(target_os = "macos")]
        if let Some(previous_stream) = previous_stream {
            spawn_previous_stream_retirement(
                previous_stream,
                Arc::clone(&stream_start_state),
                Arc::clone(&stop_flag),
                Arc::clone(&callback_metrics),
                Arc::clone(&error_handler),
            );
        }
        #[cfg(not(target_os = "macos"))]
        drop(previous_stream);

        spawn_stream_monitor(
            Arc::clone(&stop_flag),
            frames_rendered,
            finish_fired,
            progress_callback,
            finish_callback,
            output_rate,
            total_duration,
            callback_metrics,
            metrics_handler,
            underrun_requested,
            underrun_handler,
        );
        Ok(())
    }

    fn quiesce_for_transition(&mut self) -> Result<()> {
        if let Some(samples) = self.samples.take() {
            samples.cancel();
        }
        if let Some(stream) = &self.stream {
            stream
                .play()
                .context("Failed to keep output stream active during playback transition")?;
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(samples) = self.samples.take() {
            samples.cancel();
        }
        self.stream.take();
        Ok(())
    }

    fn pause(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            stream.pause().context("Failed to pause output stream")?;
        }
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            stream.play().context("Failed to resume output stream")?;
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn spawn_previous_stream_retirement(
    previous_stream: Stream,
    stream_start_state: Arc<AtomicU8>,
    stop_flag: Arc<AtomicBool>,
    callback_metrics: Arc<CallbackMetricCounters>,
    error_handler: PlaybackErrorHandler,
) {
    let _ = thread::Builder::new()
        .name("player-stream-retirement".into())
        .spawn(move || {
            let (outcome, elapsed) = wait_for_stream_start(
                &stream_start_state,
                &stop_flag,
                STREAM_START_CALLBACK_TIMEOUT,
                STREAM_START_CALLBACK_POLL_INTERVAL,
            );
            callback_metrics.record_stream_start_wait(elapsed, outcome);
            if outcome == StreamStartWaitOutcome::TimedOut {
                stream_start_state.store(STREAM_START_FAILED, Ordering::Release);
                error_handler(format!(
                    "Output stream did not produce its first callback within {} seconds",
                    STREAM_START_CALLBACK_TIMEOUT.as_secs()
                ));
            }
            drop(previous_stream);
        });
}

#[allow(clippy::too_many_arguments)]
fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    samples: SampleBuffer,
    stop_flag: Arc<AtomicBool>,
    volume: Arc<AtomicU64>,
    frames_rendered: Arc<AtomicU64>,
    finish_fired: Arc<AtomicBool>,
    buffer_error_reported: Arc<AtomicBool>,
    error_handler: PlaybackErrorHandler,
    callback_metrics: Arc<CallbackMetricCounters>,
    underrun_requested: Arc<AtomicBool>,
    stream_start_state: Arc<AtomicU8>,
    output_channels: u16,
    dither_lsb: f32,
) -> Result<Stream>
where
    T: Sample + SizedSample + FromSample<f32>,
{
    let channels = usize::from(output_channels.max(1));
    let mut scratch =
        Vec::<f32>::with_capacity(INITIAL_OUTPUT_SCRATCH_FRAMES.saturating_mul(channels));
    let mut smoother = OutputSmoother::new(channels);
    let mut dither = TpdfDither::new(dither_lsb);
    let error_handler_for_stream = Arc::clone(&error_handler);
    let stream_start_state_for_error = Arc::clone(&stream_start_state);
    let callback_metrics_for_error = Arc::clone(&callback_metrics);
    let mut first_callback_pending = true;

    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                write_output_data_with_metrics(
                    data,
                    &samples,
                    &stop_flag,
                    &volume,
                    &frames_rendered,
                    &finish_fired,
                    &buffer_error_reported,
                    &error_handler,
                    &callback_metrics,
                    &underrun_requested,
                    channels,
                    &mut scratch,
                    &mut smoother,
                    &mut dither,
                );
                if first_callback_pending {
                    let _ = stream_start_state.compare_exchange(
                        STREAM_START_PENDING,
                        STREAM_START_READY,
                        Ordering::Release,
                        Ordering::Relaxed,
                    );
                    first_callback_pending = false;
                }
            },
            move |err| {
                let disposition = stream_error_disposition(&err);
                if disposition == StreamErrorDisposition::Fatal {
                    stream_start_state_for_error.store(STREAM_START_FAILED, Ordering::Release);
                    error_handler_for_stream(format!("output stream error ({err:?}): {err}"));
                } else {
                    callback_metrics_for_error.record_nonfatal_stream_error(disposition);
                }
            },
            None,
        )
        .context("Failed to build output stream")
}

struct OutputSmoother {
    channels: usize,
    gain: f32,
    last_frame: Vec<f32>,
}

impl OutputSmoother {
    fn new(channels: usize) -> Self {
        let channels = channels.max(1);
        Self {
            channels,
            gain: 0.0,
            last_frame: vec![0.0; channels],
        }
    }

    #[cfg(test)]
    fn primed(channels: usize) -> Self {
        let channels = channels.max(1);
        Self {
            channels,
            gain: 1.0,
            last_frame: vec![0.0; channels],
        }
    }

    fn smooth_audio(&mut self, output: &mut [f32], written_samples: usize, channels: usize) {
        self.ensure_channels(channels);
        let writable_samples = written_samples.min(output.len());
        if writable_samples == 0 {
            self.smooth_silence(output, channels);
            return;
        }

        let channels = channels.max(1);
        let frames = writable_samples / channels;
        for frame in 0..frames {
            self.gain = step_toward(self.gain, 1.0);
            let base = frame * channels;
            for channel in 0..channels {
                output[base + channel] *= self.gain;
            }
        }
        self.capture_last_frame(&output[..writable_samples], channels);

        if writable_samples < output.len() {
            self.smooth_silence(&mut output[writable_samples..], channels);
        }
    }

    fn smooth_silence(&mut self, output: &mut [f32], channels: usize) {
        self.ensure_channels(channels);
        let channels = channels.max(1);
        output.fill(0.0);
        let frames = output.len() / channels;
        for frame in 0..frames {
            self.gain = step_toward(self.gain, 0.0);
            let base = frame * channels;
            for channel in 0..channels {
                output[base + channel] = self.last_frame[channel] * self.gain;
            }
        }
        if self.gain <= f32::EPSILON {
            self.last_frame.fill(0.0);
            self.gain = 0.0;
        }
    }

    fn ensure_channels(&mut self, channels: usize) {
        let channels = channels.max(1);
        if self.channels != channels {
            self.channels = channels;
            self.last_frame.resize(channels, 0.0);
            self.gain = self.gain.clamp(0.0, 1.0);
        }
    }

    fn capture_last_frame(&mut self, output: &[f32], channels: usize) {
        let channels = channels.max(1);
        if output.len() < channels {
            return;
        }
        let start = output.len() - channels;
        self.last_frame[..channels].copy_from_slice(&output[start..start + channels]);
    }
}

struct TpdfDither {
    lsb: f32,
    state: u64,
}

impl TpdfDither {
    fn new(lsb: f32) -> Self {
        Self {
            lsb: lsb.max(0.0),
            state: 0x8a5c_91d2_4f3e_7b60,
        }
    }

    #[cfg(test)]
    fn disabled() -> Self {
        Self::new(0.0)
    }

    fn next(&mut self) -> f32 {
        if self.lsb == 0.0 {
            return 0.0;
        }
        (self.next_unit() - self.next_unit()) * self.lsb
    }

    fn next_unit(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        let value = (self.state >> 40) as u32;
        value as f32 / 16_777_216.0
    }
}

fn write_smoothed_silence<T>(
    data: &mut [T],
    output: &mut [f32],
    channels: usize,
    smoother: &mut OutputSmoother,
    output_gain: f32,
) -> OutputSampleObservation
where
    T: Sample + FromSample<f32>,
{
    smoother.smooth_silence(output, channels);
    let mut observation = OutputSampleObservation::default();
    for (target, sample) in data.iter_mut().zip(output.iter().copied()) {
        *target = T::from_sample(observation.observe(sample * output_gain));
    }
    observation
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn write_output_data<T>(
    data: &mut [T],
    samples: &SampleBuffer,
    stop_flag: &AtomicBool,
    volume: &AtomicU64,
    frames_rendered: &AtomicU64,
    finish_fired: &AtomicBool,
    buffer_error_reported: &AtomicBool,
    error_handler: &PlaybackErrorHandler,
    channels: usize,
    scratch: &mut Vec<f32>,
) where
    T: Sample + FromSample<f32>,
{
    let callback_metrics = CallbackMetricCounters::default();
    let underrun_requested = AtomicBool::new(false);
    let mut smoother = OutputSmoother::primed(channels.max(1));
    let mut dither = TpdfDither::disabled();
    write_output_data_with_metrics(
        data,
        samples,
        stop_flag,
        volume,
        frames_rendered,
        finish_fired,
        buffer_error_reported,
        error_handler,
        &callback_metrics,
        &underrun_requested,
        channels,
        scratch,
        &mut smoother,
        &mut dither,
    );
}

#[allow(clippy::too_many_arguments)]
fn write_output_data_with_metrics<T>(
    data: &mut [T],
    samples: &SampleBuffer,
    stop_flag: &AtomicBool,
    volume: &AtomicU64,
    frames_rendered: &AtomicU64,
    finish_fired: &AtomicBool,
    buffer_error_reported: &AtomicBool,
    error_handler: &PlaybackErrorHandler,
    callback_metrics: &CallbackMetricCounters,
    underrun_requested: &AtomicBool,
    channels: usize,
    scratch: &mut Vec<f32>,
    smoother: &mut OutputSmoother,
    dither: &mut TpdfDither,
) where
    T: Sample + FromSample<f32>,
{
    let callback_frames = (data.len() / channels.max(1)) as u64;

    if stop_flag.load(Ordering::SeqCst) {
        data.fill(T::EQUILIBRIUM);
        return;
    }
    let mut metric_record = CallbackMetricRecord::new(callback_metrics, callback_frames);

    if scratch.len() < data.len() {
        scratch.resize(data.len(), 0.0);
    }
    let output = &mut scratch[..data.len()];
    output.fill(0.0);

    let gain = output_gain(volume);

    let Some(status) = samples.try_pop_realtime_frames_into(output, channels) else {
        let observation = write_smoothed_silence(data, output, channels, smoother, gain);
        metric_record.record_output_observation(observation);
        metric_record.record_silence_due_to_lock();
        return;
    };
    if let Some(error) = status.error {
        let observation = write_smoothed_silence(data, output, channels, smoother, gain);
        metric_record.record_output_observation(observation);
        if !buffer_error_reported.swap(true, Ordering::SeqCst) {
            error_handler(error);
        }
        return;
    }

    smoother.smooth_audio(output, status.written, channels);

    let mut observation = OutputSampleObservation::default();
    for (index, (target, sample)) in data.iter_mut().zip(output.iter().copied()).enumerate() {
        let scaled = sample * gain;
        let dithered = if index < status.written && gain > 0.0 {
            scaled + dither.next()
        } else {
            scaled
        };
        *target = T::from_sample(observation.observe_with_source(dithered, scaled));
    }
    metric_record.record_output_observation(observation);

    let channels = channels.max(1);
    frames_rendered.fetch_add((status.written / channels) as u64, Ordering::Relaxed);

    let writable_samples = data.len() - (data.len() % channels);
    if status.written < writable_samples && !status.finished {
        metric_record
            .record_underrun_frames(((writable_samples - status.written) / channels) as u64);
        underrun_requested.store(true, Ordering::SeqCst);
    }

    if status.finished {
        finish_fired.store(true, Ordering::SeqCst);
    }
}

fn output_gain(volume: &AtomicU64) -> f32 {
    let value = f64::from_bits(volume.load(Ordering::Relaxed));
    if value.is_finite() {
        value.clamp(0.0, 1.0) as f32
    } else {
        0.0
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_stream_monitor(
    stop_flag: Arc<AtomicBool>,
    frames_rendered: Arc<AtomicU64>,
    finish_fired: Arc<AtomicBool>,
    progress_callback: Arc<dyn Fn(f64, f64) + Send + Sync>,
    finish_callback: Arc<dyn Fn() + Send + Sync>,
    output_rate: u32,
    total_duration: f64,
    callback_metrics: Arc<CallbackMetricCounters>,
    metrics_handler: AudioMetricsHandler,
    underrun_requested: Arc<AtomicBool>,
    underrun_handler: AudioUnderrunHandler,
) {
    let _ = thread::Builder::new()
        .name("player-output-monitor".into())
        .spawn(move || {
            let mut next_metrics_report_at = Instant::now() + Duration::from_secs(1);
            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    report_callback_metrics(&callback_metrics, &metrics_handler);
                    break;
                }

                let progress =
                    frames_rendered.load(Ordering::Relaxed) as f64 / f64::from(output_rate.max(1));
                progress_callback(progress.min(total_duration.max(progress)), total_duration);

                if underrun_requested.swap(false, Ordering::SeqCst) {
                    underrun_handler();
                }

                if Instant::now() >= next_metrics_report_at {
                    report_callback_metrics(&callback_metrics, &metrics_handler);
                    next_metrics_report_at = Instant::now() + Duration::from_secs(1);
                }

                if finish_fired.load(Ordering::SeqCst) {
                    report_callback_metrics(&callback_metrics, &metrics_handler);
                    finish_callback();
                    break;
                }

                thread::sleep(Duration::from_millis(20));
            }
        });
}

fn report_callback_metrics(
    callback_metrics: &CallbackMetricCounters,
    metrics_handler: &AudioMetricsHandler,
) {
    let metrics = callback_metrics.drain();
    // 只要有任意非零观测就上报：过去是"仅告警"（silence/underrun），P1-6 之后加入
    // 回调计数与耗时；无回调时（stop/未开始）保持沉默。
    if metrics.callback_count > 0
        || metrics.silence_due_to_lock > 0
        || metrics.underrun_frames > 0
        || metrics.stream_start_timeout_count > 0
        || metrics.stream_start_failure_count > 0
        || metrics.output_xrun_count > 0
        || metrics.callback_metrics_dropped_count > 0
        || metrics.output_sample_count > 0
    {
        metrics_handler(metrics);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        choose_exact_output_config_from_default, choose_exact_output_config_from_ranges,
        choose_output_config_from_ranges, output_dither_lsb, stream_error_disposition,
        wait_for_stream_start, write_output_data, write_output_data_with_metrics,
        CallbackMetricCounters, CallbackObservation, OutputSampleObservation, OutputSmoother,
        PendingSampleBufferGuard, StreamErrorDisposition, StreamStartWaitOutcome, TpdfDither,
        STREAM_START_FAILED, STREAM_START_PENDING, STREAM_START_READY,
    };
    use crate::player::backend::{OutputFormat, OutputSampleFormat};
    use crate::player::stream::{AudioFormat, PlaybackErrorHandler, SampleBuffer};
    use cpal::{SampleFormat, StreamError, SupportedBufferSize};
    use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
    use std::sync::Arc;

    fn config(
        channels: u16,
        min_rate: u32,
        max_rate: u32,
        sample_format: SampleFormat,
    ) -> cpal::SupportedStreamConfigRange {
        cpal::SupportedStreamConfigRange::new(
            channels,
            min_rate,
            max_rate,
            SupportedBufferSize::Range {
                min: 128,
                max: 4096,
            },
            sample_format,
        )
    }

    fn exact_config(
        channels: u16,
        sample_rate: u32,
        sample_format: SampleFormat,
    ) -> cpal::SupportedStreamConfig {
        cpal::SupportedStreamConfig::new(
            channels,
            sample_rate,
            SupportedBufferSize::Range {
                min: 128,
                max: 4096,
            },
            sample_format,
        )
    }

    #[test]
    fn output_config_prefers_exact_match_over_device_default_format() {
        let source = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 0.0,
            bits_per_sample: None,
        };
        let default = exact_config(2, 44_100, SampleFormat::F32);
        let configs = vec![config(2, 48_000, 48_000, SampleFormat::F32)];

        let selected =
            choose_output_config_from_ranges(Some(default), &configs, source).expect("config");

        assert_eq!(selected.sample_format(), SampleFormat::F32);
        assert_eq!(selected.channels(), 2);
        assert_eq!(selected.sample_rate(), 48_000);
    }

    #[test]
    fn output_config_uses_device_default_when_no_exact_match_exists() {
        let source = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 0.0,
            bits_per_sample: None,
        };
        let default = exact_config(2, 44_100, SampleFormat::F32);
        let configs = vec![config(2, 96_000, 96_000, SampleFormat::F32)];

        let selected =
            choose_output_config_from_ranges(Some(default), &configs, source).expect("config");

        assert_eq!(selected.sample_format(), SampleFormat::F32);
        assert_eq!(selected.channels(), 2);
        assert_eq!(selected.sample_rate(), 44_100);
    }

    #[test]
    fn output_config_ignores_unsupported_device_default_format() {
        let source = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 0.0,
            bits_per_sample: None,
        };
        let default = exact_config(2, 44_100, SampleFormat::DsdU8);
        let configs = vec![config(2, 48_000, 48_000, SampleFormat::F32)];

        let selected =
            choose_output_config_from_ranges(Some(default), &configs, source).expect("config");

        assert_eq!(selected.sample_format(), SampleFormat::F32);
        assert_eq!(selected.channels(), 2);
        assert_eq!(selected.sample_rate(), 48_000);
    }

    #[test]
    fn output_config_accepts_device_default_24bit_format() {
        let source = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 0.0,
            bits_per_sample: None,
        };
        let default = exact_config(2, 48_000, SampleFormat::I24);
        let configs = vec![config(2, 96_000, 96_000, SampleFormat::F32)];

        let selected =
            choose_output_config_from_ranges(Some(default), &configs, source).expect("config");

        assert_eq!(selected.sample_format(), SampleFormat::I24);
        assert_eq!(selected.channels(), 2);
        assert_eq!(selected.sample_rate(), 48_000);
    }

    #[test]
    fn exact_output_config_requires_previously_negotiated_format() {
        let negotiated = AudioFormat {
            channels: 2,
            sample_rate: 44_100,
            duration_secs: 0.0,
            bits_per_sample: None,
        };
        let configs = vec![config(2, 48_000, 48_000, SampleFormat::F32)];

        assert!(choose_exact_output_config_from_ranges(&configs, negotiated).is_none());
    }

    #[test]
    fn exact_output_config_requires_previously_negotiated_sample_format() {
        let negotiated = OutputFormat {
            audio_format: AudioFormat {
                channels: 2,
                sample_rate: 48_000,
                duration_secs: 0.0,
                bits_per_sample: None,
            },
            sample_format: OutputSampleFormat::F32,
            device_identity: "test-device".into(),
        };
        let configs = vec![config(2, 48_000, 48_000, SampleFormat::I16)];

        assert!(choose_exact_output_config_from_ranges(&configs, negotiated).is_none());
    }

    #[test]
    fn exact_output_config_accepts_previously_negotiated_24bit_format() {
        let negotiated = OutputFormat {
            audio_format: AudioFormat {
                channels: 2,
                sample_rate: 48_000,
                duration_secs: 0.0,
                bits_per_sample: None,
            },
            sample_format: OutputSampleFormat::I24,
            device_identity: "test-device".into(),
        };
        let configs = vec![
            config(2, 48_000, 48_000, SampleFormat::I16),
            config(2, 48_000, 48_000, SampleFormat::I24),
        ];

        let selected =
            choose_exact_output_config_from_ranges(&configs, negotiated).expect("config");

        assert_eq!(selected.sample_format(), SampleFormat::I24);
    }

    #[test]
    fn exact_output_config_accepts_previously_negotiated_device_default() {
        let negotiated = OutputFormat {
            audio_format: AudioFormat {
                channels: 2,
                sample_rate: 48_000,
                duration_secs: 0.0,
                bits_per_sample: None,
            },
            sample_format: OutputSampleFormat::F32,
            device_identity: "test-device".into(),
        };
        let default = exact_config(2, 48_000, SampleFormat::F32);

        let selected =
            choose_exact_output_config_from_default(Some(default), &negotiated).expect("config");

        assert_eq!(selected.sample_format(), SampleFormat::F32);
        assert_eq!(selected.channels(), 2);
        assert_eq!(selected.sample_rate(), 48_000);
    }

    #[test]
    fn exact_output_config_rejects_device_default_sample_format_drift() {
        let negotiated = OutputFormat {
            audio_format: AudioFormat {
                channels: 2,
                sample_rate: 48_000,
                duration_secs: 0.0,
                bits_per_sample: None,
            },
            sample_format: OutputSampleFormat::F32,
            device_identity: "test-device".into(),
        };
        let default = exact_config(2, 48_000, SampleFormat::I16);

        assert!(choose_exact_output_config_from_default(Some(default), &negotiated).is_none());
    }

    #[test]
    fn integer_output_formats_enable_tpdf_dither() {
        assert_eq!(output_dither_lsb(SampleFormat::F32), 0.0);
        assert_eq!(output_dither_lsb(SampleFormat::F64), 0.0);
        assert!(output_dither_lsb(SampleFormat::I16) > 0.0);
        assert!(output_dither_lsb(SampleFormat::I24) < output_dither_lsb(SampleFormat::I16));

        let lsb = output_dither_lsb(SampleFormat::I16);
        let mut dither = TpdfDither::new(lsb);
        for _ in 0..128 {
            let sample = dither.next();
            assert!((-lsb..=lsb).contains(&sample));
        }
    }

    #[test]
    fn integer_output_does_not_dither_empty_initial_buffer() {
        let samples = SampleBuffer::new();
        let stop_flag = AtomicBool::new(false);
        let volume = AtomicU64::new(1.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let callback_metrics = CallbackMetricCounters::default();
        let underrun_requested = AtomicBool::new(false);
        let error_handler: PlaybackErrorHandler = Arc::new(|_| {});
        let mut scratch = Vec::new();
        let mut smoother = OutputSmoother::new(2);
        let mut dither = TpdfDither::new(output_dither_lsb(SampleFormat::I16));
        let mut output = [1_i16; 256];

        write_output_data_with_metrics(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            &callback_metrics,
            &underrun_requested,
            2,
            &mut scratch,
            &mut smoother,
            &mut dither,
        );

        assert!(output.iter().all(|sample| *sample == 0));
        assert_eq!(frames_rendered.load(Ordering::Relaxed), 0);
        assert!(underrun_requested.load(Ordering::SeqCst));
    }

    #[test]
    fn integer_output_keeps_stopped_transition_at_equilibrium() {
        let samples = SampleBuffer::new();
        samples.push(&[0.25; 256]);
        let stop_flag = AtomicBool::new(true);
        let volume = AtomicU64::new(1.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let callback_metrics = CallbackMetricCounters::default();
        let underrun_requested = AtomicBool::new(false);
        let error_handler: PlaybackErrorHandler = Arc::new(|_| {});
        let mut scratch = Vec::new();
        let mut smoother = OutputSmoother::primed(2);
        let mut dither = TpdfDither::new(output_dither_lsb(SampleFormat::I16));
        let mut output = [1_i16; 256];

        write_output_data_with_metrics(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            &callback_metrics,
            &underrun_requested,
            2,
            &mut scratch,
            &mut smoother,
            &mut dither,
        );

        assert!(output.iter().all(|sample| *sample == 0));
        assert_eq!(frames_rendered.load(Ordering::Relaxed), 0);
        assert!(!finish_fired.load(Ordering::SeqCst));
        assert!(!underrun_requested.load(Ordering::SeqCst));
        assert_eq!(callback_metrics.drain().callback_count, 0);
    }

    #[test]
    fn integer_output_keeps_muted_audio_at_equilibrium() {
        let samples = SampleBuffer::new();
        samples.push(&[0.25; 256]);
        let stop_flag = AtomicBool::new(false);
        let volume = AtomicU64::new(0.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let callback_metrics = CallbackMetricCounters::default();
        let underrun_requested = AtomicBool::new(false);
        let error_handler: PlaybackErrorHandler = Arc::new(|_| {});
        let mut scratch = Vec::new();
        let mut smoother = OutputSmoother::new(2);
        let mut dither = TpdfDither::new(output_dither_lsb(SampleFormat::I16));
        let mut output = [1_i16; 256];

        write_output_data_with_metrics(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            &callback_metrics,
            &underrun_requested,
            2,
            &mut scratch,
            &mut smoother,
            &mut dither,
        );

        assert!(output.iter().all(|sample| *sample == 0));
        assert_eq!(frames_rendered.load(Ordering::Relaxed), 128);
        assert!(!underrun_requested.load(Ordering::SeqCst));
    }

    #[test]
    fn output_config_prefers_f32_for_exact_matches() {
        let source = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 0.0,
            bits_per_sample: None,
        };
        let configs = vec![
            config(2, 44_100, 96_000, SampleFormat::I32),
            config(2, 44_100, 96_000, SampleFormat::F32),
        ];

        let selected = choose_output_config_from_ranges(None, &configs, source).expect("config");

        assert_eq!(selected.sample_format(), SampleFormat::F32);
        assert_eq!(selected.channels(), 2);
        assert_eq!(selected.sample_rate(), 48_000);
    }

    #[test]
    fn output_config_prefers_highest_precision_integer_exact_match() {
        let source = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 0.0,
            bits_per_sample: None,
        };
        let configs = vec![
            config(2, 44_100, 96_000, SampleFormat::I32),
            config(2, 44_100, 96_000, SampleFormat::I16),
        ];

        let selected = choose_output_config_from_ranges(None, &configs, source).expect("config");

        assert_eq!(selected.sample_format(), SampleFormat::I32);
        assert_eq!(selected.channels(), 2);
        assert_eq!(selected.sample_rate(), 48_000);
    }

    #[test]
    fn output_config_prefers_24bit_over_16bit_exact_match() {
        let source = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 0.0,
            bits_per_sample: Some(24),
        };
        let configs = vec![
            config(2, 48_000, 48_000, SampleFormat::I16),
            config(2, 48_000, 48_000, SampleFormat::I24),
        ];

        let selected = choose_output_config_from_ranges(None, &configs, source).expect("config");

        assert_eq!(selected.sample_format(), SampleFormat::I24);
    }

    #[test]
    fn output_config_reports_missing_supported_configs() {
        let source = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 0.0,
            bits_per_sample: None,
        };

        let error = choose_output_config_from_ranges(None, &[], source).expect_err("config");

        assert!(
            format!("{error:#}").contains("No supported output configuration found"),
            "{error:#}"
        );
    }

    #[test]
    fn output_config_fallback_uses_nearest_supported_sample_rate() {
        let source = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 0.0,
            bits_per_sample: None,
        };
        let configs = vec![config(2, 96_000, 192_000, SampleFormat::F32)];

        let selected = choose_output_config_from_ranges(None, &configs, source).expect("config");

        assert_eq!(selected.channels(), 2);
        assert_eq!(selected.sample_rate(), 96_000);
    }

    #[test]
    fn output_config_fallback_prefers_matching_channels_before_rate() {
        let source = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 0.0,
            bits_per_sample: None,
        };
        let configs = vec![
            config(1, 48_000, 48_000, SampleFormat::F32),
            config(2, 96_000, 96_000, SampleFormat::F32),
        ];

        let selected = choose_output_config_from_ranges(None, &configs, source).expect("config");

        assert_eq!(selected.channels(), 2);
        assert_eq!(selected.sample_rate(), 96_000);
    }

    #[test]
    fn f32_output_preserves_buffered_samples() {
        let samples = SampleBuffer::new();
        samples.push(&[0.25, -0.5, 0.75, -1.25]);
        let stop_flag = AtomicBool::new(false);
        let volume = AtomicU64::new(1.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let errors = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let error_sink = Arc::clone(&errors);
        let error_handler: PlaybackErrorHandler = Arc::new(move |message: String| {
            error_sink.lock().unwrap().push(message);
        });
        let mut scratch = Vec::new();
        let mut output = [0.0_f32; 4];

        write_output_data(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            2,
            &mut scratch,
        );

        assert_eq!(output, [0.25, -0.5, 0.75, -1.0]);
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            2
        );
        assert!(errors.lock().unwrap().is_empty());
    }

    #[test]
    fn f32_output_silences_underfilled_callback_without_consuming_samples() {
        let samples = SampleBuffer::new();
        samples.push(&[0.25, -0.5]);
        let stop_flag = AtomicBool::new(false);
        let volume = AtomicU64::new(1.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let errors = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let error_sink = Arc::clone(&errors);
        let error_handler: PlaybackErrorHandler = Arc::new(move |message: String| {
            error_sink.lock().unwrap().push(message);
        });
        let mut scratch = Vec::new();
        let mut output = [1.0_f32; 4];

        write_output_data(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            2,
            &mut scratch,
        );

        assert_eq!(output, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            0
        );

        samples.push(&[0.75, -0.75]);
        write_output_data(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            2,
            &mut scratch,
        );

        assert_eq!(output, [0.25, -0.5, 0.75, -0.75]);
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            2
        );
        assert!(errors.lock().unwrap().is_empty());
    }

    #[test]
    fn f32_output_silences_incomplete_trailing_frame_without_consuming_it() {
        let samples = SampleBuffer::new();
        samples.push(&[0.25]);
        let stop_flag = AtomicBool::new(false);
        let volume = AtomicU64::new(1.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let errors = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let error_sink = Arc::clone(&errors);
        let error_handler: PlaybackErrorHandler = Arc::new(move |message: String| {
            error_sink.lock().unwrap().push(message);
        });
        let mut scratch = Vec::new();
        let mut output = [1.0_f32; 2];

        write_output_data(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            2,
            &mut scratch,
        );

        assert_eq!(output, [0.0, 0.0]);
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert!(errors.lock().unwrap().is_empty());

        samples.push(&[-0.25]);
        write_output_data(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            2,
            &mut scratch,
        );

        assert_eq!(output, [0.25, -0.25]);
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            1
        );
    }

    #[test]
    fn f32_output_silences_stopped_sessions_without_advancing_progress() {
        let samples = SampleBuffer::new();
        samples.push(&[0.25, -0.5, 0.75, -1.0]);
        let stop_flag = AtomicBool::new(true);
        let volume = AtomicU64::new(1.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let errors = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let error_sink = Arc::clone(&errors);
        let error_handler: PlaybackErrorHandler = Arc::new(move |message: String| {
            error_sink.lock().unwrap().push(message);
        });
        let mut scratch = Vec::new();
        let mut output = [1.0_f32; 4];

        write_output_data(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            2,
            &mut scratch,
        );

        assert_eq!(output, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert!(!finish_fired.load(std::sync::atomic::Ordering::SeqCst));
        assert!(errors.lock().unwrap().is_empty());
    }

    #[test]
    fn f32_output_sanitizes_non_finite_samples_and_volume() {
        let samples = SampleBuffer::new();
        samples.push(&[f32::NAN, f32::INFINITY, -0.5, 0.5]);
        let stop_flag = AtomicBool::new(false);
        let volume = AtomicU64::new(1.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let errors = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let error_sink = Arc::clone(&errors);
        let error_handler: PlaybackErrorHandler = Arc::new(move |message: String| {
            error_sink.lock().unwrap().push(message);
        });
        let mut scratch = Vec::new();
        let mut output = [1.0_f32; 4];

        write_output_data(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            2,
            &mut scratch,
        );

        assert_eq!(output, [0.0, 0.0, -0.5, 0.5]);
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            2
        );
        assert!(errors.lock().unwrap().is_empty());
    }

    #[test]
    fn f32_output_treats_non_finite_volume_as_silence() {
        let samples = SampleBuffer::new();
        samples.push(&[0.25, -0.5, 0.75, -1.0]);
        let stop_flag = AtomicBool::new(false);
        let volume = AtomicU64::new(f64::NAN.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let errors = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let error_sink = Arc::clone(&errors);
        let error_handler: PlaybackErrorHandler = Arc::new(move |message: String| {
            error_sink.lock().unwrap().push(message);
        });
        let mut scratch = Vec::new();
        let mut output = [1.0_f32; 4];

        write_output_data(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            2,
            &mut scratch,
        );

        assert_eq!(output, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            2
        );
        assert!(errors.lock().unwrap().is_empty());
    }

    #[test]
    fn f32_output_silences_buffer_errors_without_advancing_progress() {
        let samples = SampleBuffer::new();
        samples.push(&[0.25, -0.5]);
        samples.fail("decode failed");
        let stop_flag = AtomicBool::new(false);
        let volume = AtomicU64::new(1.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let errors = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let error_sink = Arc::clone(&errors);
        let error_handler: PlaybackErrorHandler = Arc::new(move |message: String| {
            error_sink.lock().unwrap().push(message);
        });
        let mut scratch = Vec::new();
        let mut output = [1.0_f32; 4];

        write_output_data(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            2,
            &mut scratch,
        );

        assert_eq!(output, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        assert_eq!(errors.lock().unwrap().as_slice(), ["decode failed"]);

        output = [1.0_f32; 4];
        write_output_data(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            2,
            &mut scratch,
        );

        assert_eq!(output, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(errors.lock().unwrap().len(), 1);
    }

    #[test]
    fn f32_output_counts_underrun_frames_when_buffer_underfills_callback() {
        let samples = SampleBuffer::new();
        samples.push(&[0.25, -0.5]);
        let stop_flag = AtomicBool::new(false);
        let volume = AtomicU64::new(1.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let callback_metrics = CallbackMetricCounters::default();
        let underrun_requested = AtomicBool::new(false);
        let errors = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let error_sink = Arc::clone(&errors);
        let error_handler: PlaybackErrorHandler = Arc::new(move |message: String| {
            error_sink.lock().unwrap().push(message);
        });
        let mut scratch = Vec::new();
        let mut smoother = OutputSmoother::new(2);
        let mut dither = TpdfDither::disabled();
        let mut output = [1.0_f32; 4];

        write_output_data_with_metrics(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            &callback_metrics,
            &underrun_requested,
            2,
            &mut scratch,
            &mut smoother,
            &mut dither,
        );

        assert_eq!(output, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        let drained = callback_metrics.drain();
        assert_eq!(drained.silence_due_to_lock, 0);
        assert_eq!(drained.underrun_frames, 2);
        assert_eq!(drained.callback_count, 1);
        assert!(underrun_requested.load(std::sync::atomic::Ordering::SeqCst));
        assert!(errors.lock().unwrap().is_empty());
    }

    #[test]
    fn f32_output_fades_in_new_stream_from_silence() {
        let samples = SampleBuffer::new();
        samples.push(&[1.0, -1.0, 1.0, -1.0]);
        let stop_flag = AtomicBool::new(false);
        let volume = AtomicU64::new(1.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let callback_metrics = CallbackMetricCounters::default();
        let underrun_requested = AtomicBool::new(false);
        let error_handler: PlaybackErrorHandler = Arc::new(|_: String| {});
        let mut scratch = Vec::new();
        let mut smoother = OutputSmoother::new(2);
        let mut dither = TpdfDither::disabled();
        let mut output = [0.0_f32; 4];

        write_output_data_with_metrics(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            &callback_metrics,
            &underrun_requested,
            2,
            &mut scratch,
            &mut smoother,
            &mut dither,
        );

        assert_eq!(output, [1.0 / 64.0, -1.0 / 64.0, 2.0 / 64.0, -2.0 / 64.0]);
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            2
        );
    }

    #[test]
    fn f32_output_reads_ring_buffer_without_lock_silence() {
        let samples = SampleBuffer::new();
        samples.push(&[0.25, -0.5, 0.75, -1.0]);
        let locked_samples = samples.clone();
        let handle = std::thread::spawn(move || {
            locked_samples.hold_lock_for_test(std::time::Duration::from_millis(100));
        });
        std::thread::sleep(std::time::Duration::from_millis(10));

        let stop_flag = AtomicBool::new(false);
        let volume = AtomicU64::new(1.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let callback_metrics = CallbackMetricCounters::default();
        let underrun_requested = AtomicBool::new(false);
        let errors = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let error_sink = Arc::clone(&errors);
        let error_handler: PlaybackErrorHandler = Arc::new(move |message: String| {
            error_sink.lock().unwrap().push(message);
        });
        let mut scratch = Vec::new();
        let mut smoother = OutputSmoother::primed(2);
        let mut dither = TpdfDither::disabled();
        let mut output = [1.0_f32; 4];

        write_output_data_with_metrics(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            &callback_metrics,
            &underrun_requested,
            2,
            &mut scratch,
            &mut smoother,
            &mut dither,
        );

        assert_eq!(output, [0.25, -0.5, 0.75, -1.0]);
        let drained = callback_metrics.drain();
        assert_eq!(drained.silence_due_to_lock, 0);
        assert_eq!(drained.underrun_frames, 0);
        assert_eq!(drained.callback_count, 1);
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            2
        );
        assert!(!finish_fired.load(std::sync::atomic::Ordering::SeqCst));
        assert!(!underrun_requested.load(std::sync::atomic::Ordering::SeqCst));
        assert!(errors.lock().unwrap().is_empty());

        handle.join().expect("lock holder should finish");
    }

    #[test]
    fn callback_duration_bucket_partitions_log2_microseconds() {
        use super::callback_duration_bucket;
        // < 1μs → 桶 0
        assert_eq!(callback_duration_bucket(500), 0);
        // 1μs → 桶 0（log2(1)=0）
        assert_eq!(callback_duration_bucket(1_000), 0);
        // 2μs → 桶 1
        assert_eq!(callback_duration_bucket(2_000), 1);
        // 4μs → 桶 2
        assert_eq!(callback_duration_bucket(4_000), 2);
        // 8μs → 桶 3
        assert_eq!(callback_duration_bucket(8_000), 3);
        // 1024μs → 桶 10（log2(1024)=10）
        assert_eq!(callback_duration_bucket(1_024_000), 10);
        // 32768μs → 溢出到最后一桶（15）
        assert_eq!(callback_duration_bucket(32_768_000), 15);
        // 100ms → 溢出到最后一桶
        assert_eq!(callback_duration_bucket(100_000_000), 15);
    }

    #[test]
    fn callback_metric_counters_record_and_drain() {
        let counters = CallbackMetricCounters::default();
        counters.record_callback(CallbackObservation {
            silence_due_to_lock: 1,
            underrun_frames: 10,
            elapsed_ns: 500,
            callback_frames: 512,
            ..CallbackObservation::default()
        });
        counters.record_callback(CallbackObservation {
            elapsed_ns: 2_000,
            callback_frames: 512,
            ..CallbackObservation::default()
        });
        counters.record_callback(CallbackObservation {
            elapsed_ns: 4_000,
            callback_frames: 512,
            ..CallbackObservation::default()
        });
        counters.record_nonfatal_stream_error(StreamErrorDisposition::Xrun);

        let drained = counters.drain();
        assert_eq!(drained.callback_count, 3);
        assert_eq!(drained.callback_elapsed_ns_total, 6_500);
        assert_eq!(drained.callback_elapsed_ns_max, 4_000);
        assert_eq!(drained.callback_over_period_count, 0);
        assert_eq!(drained.callback_metrics_dropped_count, 0);
        assert_eq!(drained.callback_frames_total, 1_536);
        assert_eq!(drained.callback_frames_min, 512);
        assert_eq!(drained.callback_frames_max, 512);
        assert_eq!(drained.underrun_frames, 10);
        assert_eq!(drained.silence_due_to_lock, 1);
        assert_eq!(drained.callback_duration_buckets[0], 1);
        assert_eq!(drained.callback_duration_buckets[1], 1);
        assert_eq!(drained.callback_duration_buckets[2], 1);
        assert_eq!(drained.callback_duration_buckets.iter().sum::<u64>(), 3);
        assert_eq!(drained.output_xrun_count, 1);
        // 二次 drain 应清零
        let empty = counters.drain();
        assert_eq!(empty.callback_count, 0);
        assert_eq!(empty.callback_frames_min, 0);
        assert_eq!(empty.callback_duration_buckets[0], 0);
    }

    #[test]
    fn concurrent_callback_metric_drain_keeps_each_observation_in_one_window() {
        const OBSERVATIONS: u64 = 5_000;
        let counters = Arc::new(CallbackMetricCounters::default());
        let producer_counters = Arc::clone(&counters);
        let producer_done = Arc::new(AtomicBool::new(false));
        let producer_done_flag = Arc::clone(&producer_done);
        let producer = std::thread::spawn(move || {
            for _ in 0..OBSERVATIONS {
                producer_counters.record_callback(CallbackObservation {
                    elapsed_ns: 2_000,
                    callback_frames: 512,
                    output: OutputSampleObservation {
                        sample_count: 2,
                        square_sum: 1.0,
                        peak_abs_bits: u64::from(1.0_f32.to_bits()),
                        ..OutputSampleObservation::default()
                    },
                    ..CallbackObservation::default()
                });
            }
            producer_done_flag.store(true, Ordering::Release);
        });

        let mut observed_callbacks = 0_u64;
        let mut observed_samples = 0_u64;
        while !producer_done.load(Ordering::Acquire) || !counters.callback_observations.is_empty() {
            let metrics = counters.drain();
            assert_eq!(
                metrics.callback_duration_buckets.iter().sum::<u64>(),
                metrics.callback_count
            );
            assert_eq!(metrics.output_sample_count, metrics.callback_count * 2);
            if metrics.output_sample_count > 0 {
                assert!((metrics.output_rms() - (0.5_f64).sqrt()).abs() < 1e-9);
                assert_eq!(metrics.output_peak_abs(), 1.0);
            }
            observed_callbacks += metrics.callback_count;
            observed_samples += metrics.output_sample_count;
            std::thread::yield_now();
        }
        producer.join().expect("metric producer should finish");

        assert_eq!(observed_callbacks, OBSERVATIONS);
        assert_eq!(observed_samples, OBSERVATIONS * 2);
        assert_eq!(
            counters
                .callback_metrics_dropped_count
                .load(Ordering::Relaxed),
            0
        );
    }

    #[test]
    fn stream_errors_preserve_cpal_nonfatal_semantics() {
        assert_eq!(
            stream_error_disposition(&StreamError::StreamInvalidated),
            StreamErrorDisposition::Fatal
        );
        assert_eq!(
            stream_error_disposition(&StreamError::BufferUnderrun),
            StreamErrorDisposition::Xrun
        );
    }

    #[test]
    fn pending_sample_buffer_guard_cancels_only_undelivered_buffers() {
        let failed_samples = SampleBuffer::new();
        drop(PendingSampleBufferGuard::new(failed_samples.clone()));
        let mut output = [0.0_f32; 2];
        let cancelled = failed_samples.pop_complete_frames_into(&mut output, 2);
        assert_eq!(cancelled.written, 0);
        assert!(!cancelled.finished);
        assert!(cancelled.error.is_none());

        let delivered_samples = SampleBuffer::new();
        let mut guard = PendingSampleBufferGuard::new(delivered_samples.clone());
        guard.disarm();
        drop(guard);
        delivered_samples.push(&[0.25, -0.25]);
        let status = delivered_samples.pop_complete_frames_into(&mut output, 2);
        assert_eq!(status.written, 2);
        assert!(!status.finished);
    }

    #[test]
    fn stream_start_wait_observes_ready_failed_timeout_and_cancellation() {
        let stop_flag = AtomicBool::new(false);

        let ready = AtomicU8::new(STREAM_START_READY);
        assert_eq!(
            wait_for_stream_start(
                &ready,
                &stop_flag,
                std::time::Duration::ZERO,
                std::time::Duration::ZERO,
            )
            .0,
            StreamStartWaitOutcome::Ready
        );

        let failed = AtomicU8::new(STREAM_START_FAILED);
        assert_eq!(
            wait_for_stream_start(
                &failed,
                &stop_flag,
                std::time::Duration::ZERO,
                std::time::Duration::ZERO,
            )
            .0,
            StreamStartWaitOutcome::Failed
        );

        let pending = AtomicU8::new(STREAM_START_PENDING);
        assert_eq!(
            wait_for_stream_start(
                &pending,
                &stop_flag,
                std::time::Duration::ZERO,
                std::time::Duration::ZERO,
            )
            .0,
            StreamStartWaitOutcome::TimedOut
        );

        stop_flag.store(true, Ordering::SeqCst);
        assert_eq!(
            wait_for_stream_start(
                &ready,
                &stop_flag,
                std::time::Duration::ZERO,
                std::time::Duration::ZERO,
            )
            .0,
            StreamStartWaitOutcome::Cancelled
        );
    }

    #[test]
    fn stream_start_wait_observes_release_from_callback_thread() {
        let state = Arc::new(AtomicU8::new(STREAM_START_PENDING));
        let state_for_callback = Arc::clone(&state);
        let callback = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(5));
            state_for_callback.store(STREAM_START_READY, Ordering::Release);
        });
        let stop_flag = AtomicBool::new(false);

        let outcome = wait_for_stream_start(
            &state,
            &stop_flag,
            std::time::Duration::from_millis(100),
            std::time::Duration::from_millis(1),
        )
        .0;

        callback.join().expect("callback thread should finish");
        assert_eq!(outcome, StreamStartWaitOutcome::Ready);
    }

    #[test]
    fn output_metrics_capture_peak_and_rms_after_sample_buffer_sanitization() {
        let samples = SampleBuffer::new();
        samples.push(&[1.25, f32::NAN, -0.5, 0.5]);
        let stop_flag = AtomicBool::new(false);
        let volume = AtomicU64::new(1.0_f64.to_bits());
        let frames_rendered = AtomicU64::new(0);
        let finish_fired = AtomicBool::new(false);
        let buffer_error_reported = AtomicBool::new(false);
        let callback_metrics = CallbackMetricCounters::new(48_000);
        let underrun_requested = AtomicBool::new(false);
        let error_handler: PlaybackErrorHandler = Arc::new(|_| {});
        let mut scratch = Vec::new();
        let mut smoother = OutputSmoother::primed(2);
        let mut dither = TpdfDither::disabled();
        let mut output = [0.0_f32; 4];

        write_output_data_with_metrics(
            &mut output,
            &samples,
            &stop_flag,
            &volume,
            &frames_rendered,
            &finish_fired,
            &buffer_error_reported,
            &error_handler,
            &callback_metrics,
            &underrun_requested,
            2,
            &mut scratch,
            &mut smoother,
            &mut dither,
        );

        assert_eq!(output, [1.0, 0.0, -0.5, 0.5]);
        let drained = callback_metrics.drain();
        assert_eq!(drained.callback_count, 1);
        assert_eq!(drained.callback_frames_total, 2);
        assert_eq!(drained.callback_frames_min, 2);
        assert_eq!(drained.callback_frames_max, 2);
        assert_eq!(drained.output_sample_count, 4);
        assert_eq!(drained.output_clipped_samples, 0);
        assert_eq!(drained.output_nonfinite_samples, 0);
        assert_eq!(drained.output_peak_abs(), 1.0);
        assert!((drained.output_rms() - (0.375_f64).sqrt()).abs() < 1e-9);
    }

    #[test]
    fn output_observation_counts_final_clipping_and_nonfinite_samples() {
        let mut observation = OutputSampleObservation::default();
        let sanitized = [1.25, f32::NAN, -0.5, 0.5].map(|sample| observation.observe(sample));

        assert_eq!(sanitized, [1.0, 0.0, -0.5, 0.5]);
        let callback_metrics = CallbackMetricCounters::default();
        callback_metrics.record_callback(CallbackObservation {
            callback_frames: 2,
            output: observation,
            ..CallbackObservation::default()
        });
        let drained = callback_metrics.drain();
        assert_eq!(drained.output_sample_count, 4);
        assert_eq!(drained.output_clipped_samples, 1);
        assert_eq!(drained.output_nonfinite_samples, 1);
        assert_eq!(drained.output_peak_abs(), 1.0);
        assert!((drained.output_rms() - (0.375_f64).sqrt()).abs() < 1e-9);
    }

    #[test]
    fn output_observation_does_not_treat_dither_overshoot_as_source_clipping() {
        let mut observation = OutputSampleObservation::default();
        let sanitized = observation.observe_with_source(1.000_01, 1.0);

        assert_eq!(sanitized, 1.0);
        assert_eq!(observation.clipped_samples, 0);
        assert_eq!(observation.nonfinite_samples, 0);
    }
}
