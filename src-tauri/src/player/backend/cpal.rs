//! 基于 CPAL 的默认音频播放后端实现。
//!
//! 该模块负责选择可用输出设备与格式、创建音频输出流，并把播放器解码后的样本缓冲
//! 推送到系统音频设备，供桌面端实际发声使用。

use crate::player::backend::PlaybackBackend;
use crate::player::stream::{AudioFormat, PlaybackErrorHandler, SampleBuffer};
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream, SupportedStreamConfig};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn choose_output_config(
    device: &cpal::Device,
    audio_format: AudioFormat,
) -> Result<SupportedStreamConfig> {
    let configs = device
        .supported_output_configs()
        .context("Failed to query supported output configs")?
        .collect::<Vec<_>>();

    let exact = configs.iter().find(|config| {
        is_supported_sample_format(config.sample_format())
            && config.channels() == audio_format.channels
            && config.min_sample_rate() <= audio_format.sample_rate
            && config.max_sample_rate() >= audio_format.sample_rate
    });

    if let Some(config) = exact {
        return Ok(config.with_sample_rate(audio_format.sample_rate));
    }

    let fallback = configs
        .into_iter()
        .find(|config| is_supported_sample_format(config.sample_format()))
        .context("No supported output configuration found")?;

    Ok(fallback.with_max_sample_rate())
}

fn is_supported_sample_format(format: SampleFormat) -> bool {
    matches!(
        format,
        SampleFormat::F32
            | SampleFormat::F64
            | SampleFormat::I8
            | SampleFormat::I16
            | SampleFormat::I32
            | SampleFormat::I64
            | SampleFormat::U8
            | SampleFormat::U16
            | SampleFormat::U32
            | SampleFormat::U64
    )
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
    fn negotiate_output_format(&self, source_format: AudioFormat) -> Result<AudioFormat> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No default output device available")?;
        let config = choose_output_config(&device, source_format)?;
        Ok(AudioFormat {
            channels: config.channels(),
            sample_rate: config.sample_rate(),
            duration_secs: source_format.duration_secs,
        })
    }

    fn play_stream(
        &mut self,
        format: AudioFormat,
        samples: SampleBuffer,
        stop_flag: Arc<AtomicBool>,
        volume: Arc<AtomicU64>,
        progress_callback: Arc<dyn Fn(f64, f64) + Send + Sync>,
        finish_callback: Arc<dyn Fn() + Send + Sync>,
        error_handler: PlaybackErrorHandler,
    ) -> Result<()> {
        self.stop()?;

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No default output device available")?;
        let config = choose_output_config(&device, format)?;
        let stream_config: cpal::StreamConfig = config.clone().into();

        let total_duration = format.duration_secs;
        let output_rate = config.sample_rate().max(1);
        let output_channels = config.channels().max(1);
        let frames_rendered = Arc::new(AtomicU64::new(0));
        let finish_fired = Arc::new(AtomicBool::new(false));
        let buffer_error_reported = Arc::new(AtomicBool::new(false));

        let stream = match config.sample_format() {
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                &stream_config,
                samples.clone(),
                Arc::clone(&stop_flag),
                Arc::clone(&volume),
                Arc::clone(&frames_rendered),
                Arc::clone(&finish_fired),
                Arc::clone(&buffer_error_reported),
                Arc::clone(&error_handler),
                output_channels,
            )?,
            SampleFormat::F64 => build_stream::<f64>(
                &device,
                &stream_config,
                samples.clone(),
                Arc::clone(&stop_flag),
                Arc::clone(&volume),
                Arc::clone(&frames_rendered),
                Arc::clone(&finish_fired),
                Arc::clone(&buffer_error_reported),
                Arc::clone(&error_handler),
                output_channels,
            )?,
            SampleFormat::I8 => build_stream::<i8>(
                &device,
                &stream_config,
                samples.clone(),
                Arc::clone(&stop_flag),
                Arc::clone(&volume),
                Arc::clone(&frames_rendered),
                Arc::clone(&finish_fired),
                Arc::clone(&buffer_error_reported),
                Arc::clone(&error_handler),
                output_channels,
            )?,
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                &stream_config,
                samples.clone(),
                Arc::clone(&stop_flag),
                Arc::clone(&volume),
                Arc::clone(&frames_rendered),
                Arc::clone(&finish_fired),
                Arc::clone(&buffer_error_reported),
                Arc::clone(&error_handler),
                output_channels,
            )?,
            SampleFormat::I32 => build_stream::<i32>(
                &device,
                &stream_config,
                samples.clone(),
                Arc::clone(&stop_flag),
                Arc::clone(&volume),
                Arc::clone(&frames_rendered),
                Arc::clone(&finish_fired),
                Arc::clone(&buffer_error_reported),
                Arc::clone(&error_handler),
                output_channels,
            )?,
            SampleFormat::I64 => build_stream::<i64>(
                &device,
                &stream_config,
                samples.clone(),
                Arc::clone(&stop_flag),
                Arc::clone(&volume),
                Arc::clone(&frames_rendered),
                Arc::clone(&finish_fired),
                Arc::clone(&buffer_error_reported),
                Arc::clone(&error_handler),
                output_channels,
            )?,
            SampleFormat::U8 => build_stream::<u8>(
                &device,
                &stream_config,
                samples.clone(),
                Arc::clone(&stop_flag),
                Arc::clone(&volume),
                Arc::clone(&frames_rendered),
                Arc::clone(&finish_fired),
                Arc::clone(&buffer_error_reported),
                Arc::clone(&error_handler),
                output_channels,
            )?,
            SampleFormat::U16 => build_stream::<u16>(
                &device,
                &stream_config,
                samples.clone(),
                Arc::clone(&stop_flag),
                Arc::clone(&volume),
                Arc::clone(&frames_rendered),
                Arc::clone(&finish_fired),
                Arc::clone(&buffer_error_reported),
                Arc::clone(&error_handler),
                output_channels,
            )?,
            SampleFormat::U32 => build_stream::<u32>(
                &device,
                &stream_config,
                samples.clone(),
                Arc::clone(&stop_flag),
                Arc::clone(&volume),
                Arc::clone(&frames_rendered),
                Arc::clone(&finish_fired),
                Arc::clone(&buffer_error_reported),
                Arc::clone(&error_handler),
                output_channels,
            )?,
            SampleFormat::U64 => build_stream::<u64>(
                &device,
                &stream_config,
                samples.clone(),
                Arc::clone(&stop_flag),
                Arc::clone(&volume),
                Arc::clone(&frames_rendered),
                Arc::clone(&finish_fired),
                Arc::clone(&buffer_error_reported),
                Arc::clone(&error_handler),
                output_channels,
            )?,
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
    output_channels: u16,
) -> Result<Stream>
where
    T: Sample + SizedSample + FromSample<f32>,
{
    let channels = usize::from(output_channels.max(1));
    let mut scratch = Vec::<f32>::new();
    let error_handler_for_stream = Arc::clone(&error_handler);

    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                write_output_data(
                    data,
                    &samples,
                    &stop_flag,
                    &volume,
                    &frames_rendered,
                    &finish_fired,
                    &buffer_error_reported,
                    &error_handler,
                    channels,
                    &mut scratch,
                );
            },
            move |err| {
                error_handler_for_stream(err.to_string());
            },
            None,
        )
        .context("Failed to build output stream")
}

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
    if stop_flag.load(Ordering::SeqCst) {
        data.fill(T::EQUILIBRIUM);
        return;
    }

    if scratch.len() < data.len() {
        scratch.resize(data.len(), 0.0);
    }
    let output = &mut scratch[..data.len()];
    output.fill(0.0);

    let status = samples.pop_into(output);
    let gain = f64::from_bits(volume.load(Ordering::Relaxed)).clamp(0.0, 1.0) as f32;
    for (target, sample) in data.iter_mut().zip(output.iter().copied()) {
        *target = T::from_sample((sample * gain).clamp(-1.0, 1.0));
    }

    if let Some(error) = status.error {
        if !buffer_error_reported.swap(true, Ordering::SeqCst) {
            error_handler(error);
        }
    }

    frames_rendered.fetch_add((status.written / channels.max(1)) as u64, Ordering::Relaxed);

    if status.finished {
        finish_fired.store(true, Ordering::SeqCst);
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
) {
    let _ = thread::Builder::new()
        .name("player-output-monitor".into())
        .spawn(move || {
            while !stop_flag.load(Ordering::SeqCst) {
                let progress =
                    frames_rendered.load(Ordering::Relaxed) as f64 / f64::from(output_rate.max(1));
                progress_callback(progress.min(total_duration.max(progress)), total_duration);

                if finish_fired.load(Ordering::SeqCst) {
                    finish_callback();
                    break;
                }

                thread::sleep(Duration::from_millis(20));
            }
        });
}
