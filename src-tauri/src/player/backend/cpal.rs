//! 基于 CPAL 的默认音频播放后端实现。
//!
//! 该模块负责选择可用输出设备与格式、创建音频输出流，并把播放器解码后的样本缓冲
//! 推送到系统音频设备，供桌面端实际发声使用。

use crate::player::backend::{
    AudioCallbackMetrics, AudioMetricsHandler, AudioUnderrunHandler, OutputFormat,
    OutputSampleFormat, PlaybackBackend,
};
use crate::player::stream::{AudioFormat, PlaybackErrorHandler, SampleBuffer};
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    FromSample, Sample, SampleFormat, SizedSample, Stream, SupportedStreamConfig,
    SupportedStreamConfigRange, I24, U24,
};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const OUTPUT_SMOOTHING_FRAMES: usize = 64;

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

    Ok(fallback
        .clone()
        .with_sample_rate(clamp_sample_rate(fallback, audio_format.sample_rate)))
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
        .map(|config| config.clone().with_sample_rate(audio_format.sample_rate))
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

fn is_supported_sample_format(format: SampleFormat) -> bool {
    sample_format_priority(format).is_some()
}

fn is_supported_output_config(config: &SupportedStreamConfig) -> bool {
    is_supported_sample_format(config.sample_format())
        && config.channels() > 0
        && config.sample_rate() > 0
}

fn sample_format_priority(format: SampleFormat) -> Option<usize> {
    match format {
        SampleFormat::F32 => Some(0),
        SampleFormat::F64 => Some(1),
        SampleFormat::I16 => Some(2),
        SampleFormat::U16 => Some(3),
        SampleFormat::I24 => Some(4),
        SampleFormat::U24 => Some(5),
        SampleFormat::I32 => Some(6),
        SampleFormat::U32 => Some(7),
        SampleFormat::I8 => Some(8),
        SampleFormat::U8 => Some(9),
        SampleFormat::I64 => Some(10),
        SampleFormat::U64 => Some(11),
        _ => None,
    }
}

fn output_sample_format(format: SampleFormat) -> Option<OutputSampleFormat> {
    match format {
        SampleFormat::F32 => Some(OutputSampleFormat::F32),
        SampleFormat::F64 => Some(OutputSampleFormat::F64),
        SampleFormat::I8 => Some(OutputSampleFormat::I8),
        SampleFormat::I16 => Some(OutputSampleFormat::I16),
        SampleFormat::I24 => Some(OutputSampleFormat::I24),
        SampleFormat::I32 => Some(OutputSampleFormat::I32),
        SampleFormat::I64 => Some(OutputSampleFormat::I64),
        SampleFormat::U8 => Some(OutputSampleFormat::U8),
        SampleFormat::U16 => Some(OutputSampleFormat::U16),
        SampleFormat::U24 => Some(OutputSampleFormat::U24),
        SampleFormat::U32 => Some(OutputSampleFormat::U32),
        SampleFormat::U64 => Some(OutputSampleFormat::U64),
        _ => None,
    }
}

fn output_dither_lsb(format: SampleFormat) -> f32 {
    match format {
        SampleFormat::F32 | SampleFormat::F64 => 0.0,
        SampleFormat::I8 | SampleFormat::U8 => 1.0 / 128.0,
        SampleFormat::I16 | SampleFormat::U16 => 1.0 / 32_768.0,
        SampleFormat::I24 | SampleFormat::U24 => 1.0 / 8_388_608.0,
        SampleFormat::I32 | SampleFormat::U32 => 1.0 / 2_147_483_648.0,
        SampleFormat::I64 | SampleFormat::U64 => 1.0 / 9_223_372_036_854_775_808.0,
        _ => 0.0,
    }
}

fn cpal_sample_format(format: OutputSampleFormat) -> SampleFormat {
    match format {
        OutputSampleFormat::F32 => SampleFormat::F32,
        OutputSampleFormat::F64 => SampleFormat::F64,
        OutputSampleFormat::I8 => SampleFormat::I8,
        OutputSampleFormat::I16 => SampleFormat::I16,
        OutputSampleFormat::I24 => SampleFormat::I24,
        OutputSampleFormat::I32 => SampleFormat::I32,
        OutputSampleFormat::I64 => SampleFormat::I64,
        OutputSampleFormat::U8 => SampleFormat::U8,
        OutputSampleFormat::U16 => SampleFormat::U16,
        OutputSampleFormat::U24 => SampleFormat::U24,
        OutputSampleFormat::U32 => SampleFormat::U32,
        OutputSampleFormat::U64 => SampleFormat::U64,
    }
}

fn clamp_sample_rate(config: &SupportedStreamConfigRange, source_rate: u32) -> u32 {
    source_rate
        .max(config.min_sample_rate())
        .min(config.max_sample_rate())
}

fn sample_rate_distance(config: &SupportedStreamConfigRange, source_rate: u32) -> u32 {
    clamp_sample_rate(config, source_rate).abs_diff(source_rate)
}

#[derive(Default)]
struct CallbackMetricCounters {
    silence_due_to_lock: AtomicU64,
    underrun_frames: AtomicU64,
}

impl CallbackMetricCounters {
    fn record_silence_due_to_lock(&self) {
        self.silence_due_to_lock.fetch_add(1, Ordering::Relaxed);
    }

    fn record_underrun_frames(&self, frames: u64) {
        if frames > 0 {
            self.underrun_frames.fetch_add(frames, Ordering::Relaxed);
        }
    }

    fn drain(&self) -> AudioCallbackMetrics {
        AudioCallbackMetrics {
            silence_due_to_lock: self.silence_due_to_lock.swap(0, Ordering::Relaxed),
            underrun_frames: self.underrun_frames.swap(0, Ordering::Relaxed),
        }
    }
}

pub struct CpalBackend {
    stream: Option<Stream>,
    samples: Option<SampleBuffer>,
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
        let callback_metrics = Arc::new(CallbackMetricCounters::default());
        let underrun_requested = Arc::new(AtomicBool::new(false));

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

        stream.play().context("Failed to start output stream")?;
        self.stream = Some(stream);
        self.samples = Some(samples);
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if let Some(samples) = self.samples.take() {
            samples.finish();
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
    output_channels: u16,
    dither_lsb: f32,
) -> Result<Stream>
where
    T: Sample + SizedSample + FromSample<f32>,
{
    let channels = usize::from(output_channels.max(1));
    let mut scratch = Vec::<f32>::new();
    let mut smoother = OutputSmoother::new(channels);
    let mut dither = TpdfDither::new(dither_lsb);
    let error_handler_for_stream = Arc::clone(&error_handler);

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
            },
            move |err| {
                error_handler_for_stream(format!("output stream error: {err}"));
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

fn step_toward(current: f32, target: f32) -> f32 {
    let step = 1.0 / OUTPUT_SMOOTHING_FRAMES as f32;
    if current < target {
        (current + step).min(target)
    } else {
        (current - step).max(target)
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
) where
    T: Sample + FromSample<f32>,
{
    smoother.smooth_silence(output, channels);
    for (target, sample) in data.iter_mut().zip(output.iter().copied()) {
        *target = T::from_sample(sanitize_output_sample(sample * output_gain));
    }
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
    if stop_flag.load(Ordering::SeqCst) {
        data.fill(T::EQUILIBRIUM);
        return;
    }

    if scratch.len() < data.len() {
        scratch.resize(data.len(), 0.0);
    }
    let output = &mut scratch[..data.len()];
    output.fill(0.0);

    let gain = output_gain(volume);

    let Some(status) = samples.try_pop_realtime_frames_into(output, channels) else {
        write_smoothed_silence(data, output, channels, smoother, gain);
        callback_metrics.record_silence_due_to_lock();
        return;
    };
    if let Some(error) = status.error {
        write_smoothed_silence(data, output, channels, smoother, gain);
        if !buffer_error_reported.swap(true, Ordering::SeqCst) {
            error_handler(error);
        }
        return;
    }

    smoother.smooth_audio(output, status.written, channels);

    for (target, sample) in data.iter_mut().zip(output.iter().copied()) {
        let dithered = sample * gain + dither.next();
        *target = T::from_sample(sanitize_output_sample(dithered));
    }

    let channels = channels.max(1);
    frames_rendered.fetch_add((status.written / channels) as u64, Ordering::Relaxed);

    let writable_samples = data.len() - (data.len() % channels);
    if status.written < writable_samples && !status.finished {
        callback_metrics
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

fn sanitize_output_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

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
    if metrics.silence_due_to_lock > 0 || metrics.underrun_frames > 0 {
        metrics_handler(metrics);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        choose_exact_output_config_from_default, choose_exact_output_config_from_ranges,
        choose_output_config_from_ranges, output_dither_lsb, write_output_data,
        write_output_data_with_metrics, CallbackMetricCounters, OutputSmoother, TpdfDither,
    };
    use crate::player::backend::{AudioCallbackMetrics, OutputFormat, OutputSampleFormat};
    use crate::player::stream::{AudioFormat, PlaybackErrorHandler, SampleBuffer};
    use cpal::{SampleFormat, SupportedBufferSize};
    use std::sync::atomic::{AtomicBool, AtomicU64};
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
    fn output_config_accepts_non_f32_exact_matches() {
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

        assert_eq!(selected.sample_format(), SampleFormat::I16);
        assert_eq!(selected.channels(), 2);
        assert_eq!(selected.sample_rate(), 48_000);
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
        assert_eq!(
            callback_metrics.drain(),
            AudioCallbackMetrics {
                silence_due_to_lock: 0,
                underrun_frames: 2,
            }
        );
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
        assert_eq!(
            callback_metrics.drain(),
            AudioCallbackMetrics {
                silence_due_to_lock: 0,
                underrun_frames: 0,
            }
        );
        assert_eq!(
            frames_rendered.load(std::sync::atomic::Ordering::Relaxed),
            2
        );
        assert!(!finish_fired.load(std::sync::atomic::Ordering::SeqCst));
        assert!(!underrun_requested.load(std::sync::atomic::Ordering::SeqCst));
        assert!(errors.lock().unwrap().is_empty());

        handle.join().expect("lock holder should finish");
    }
}
