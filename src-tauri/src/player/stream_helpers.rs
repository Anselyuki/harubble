//! 音频流解码相关的纯计算辅助函数。
//!
//! 本模块收录音频格式探测、采样转换与 PCM 数值处理的纯计算辅助函数，
//! 供 `stream` 模块的解码路径与 `SampleConverter` 复用，同时使单元测试无需依赖 I/O 即可独立覆盖。

use crate::player::stream::AudioFormat;
use anyhow::{Context, Result};
use rubato::{SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};
use symphonia::core::audio::SignalSpec;
use symphonia::core::codecs::CodecParameters;
use symphonia::core::units::TimeBase;

/// sinc 重采样器每次处理的块大小（以源帧为单位）。
pub(crate) const SINC_RESAMPLE_CHUNK_FRAMES: usize = 1024;

/// 将解码后帧格式与已探测的源格式比对，若通道数或采样率偏移则返回错误。
///
/// 用于在解码循环中检测编解码器中途改变音频规格的异常情况，防止声道边界错位。
pub(crate) fn ensure_decoded_format_matches_source(
    spec: &SignalSpec,
    source_format: AudioFormat,
) -> Result<()> {
    let source_format = source_format.normalized();
    let decoded_channels = spec.channels.count() as u16;
    anyhow::ensure!(
        decoded_channels == source_format.channels,
        "Decoded audio channel count changed from {} to {decoded_channels}",
        source_format.channels
    );
    anyhow::ensure!(
        spec.rate == source_format.sample_rate,
        "Decoded audio sample rate changed from {} to {}",
        source_format.sample_rate,
        spec.rate
    );
    Ok(())
}

/// 将 Symphonia seek 结果中的时间戳差值转换为音频帧数。
///
/// `timestamp_delta` 为 `required_ts - actual_ts`，表示解码器在实际落点后仍需跳过的帧数。
/// 返回值用于在解码循环中丢弃不足 seek 目标的多余采样。
pub(crate) fn timestamp_delta_to_frames(
    timestamp_delta: u64,
    time_base: TimeBase,
    sample_rate: u32,
) -> u64 {
    let delta = time_base.calc_time(timestamp_delta);
    let seconds = delta.seconds as f64 + delta.frac;
    (seconds * f64::from(sample_rate.max(1))).round() as u64
}

/// 从 codec 参数中提取音频总时长（秒）。
///
/// 若缺少帧数或时基信息则返回 `0.0`。
pub(crate) fn codec_duration_secs(codec_params: &CodecParameters) -> f64 {
    match (codec_params.n_frames, codec_params.time_base) {
        (Some(n_frames), Some(time_base)) => {
            let duration = time_base.calc_time(n_frames);
            duration.seconds as f64 + duration.frac
        }
        _ => 0.0,
    }
}

/// 从 Symphonia codec 参数构造 `AudioFormat`。
///
/// 若参数中缺少通道布局或采样率信息则返回错误。
pub(crate) fn audio_format_from_codec_params(
    codec_params: &CodecParameters,
) -> Result<AudioFormat> {
    let channels = codec_params
        .channels
        .context("Missing audio channel layout")?
        .count() as u16;
    let sample_rate = codec_params
        .sample_rate
        .context("Missing audio sample rate")?;

    Ok(AudioFormat::with_bits_per_sample(
        channels,
        sample_rate,
        codec_duration_secs(codec_params),
        codec_params
            .bits_per_sample
            .and_then(|value| u16::try_from(value).ok()),
    ))
}

/// 创建基于 sinc 插值的重采样器。
///
/// 使用高质量 Blackman-Harris 窗与三次插值，适用于播放链路中的样本率转换。
/// 若重采样器初始化失败（如参数超出范围）则返回 `None`，调用方应回退到线性插值。
pub(crate) fn create_sinc_resampler(
    source_rate: u32,
    target_rate: u32,
    channels: usize,
) -> Option<SincFixedIn<f32>> {
    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Cubic,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };
    SincFixedIn::<f32>::new(
        target_rate as f64 / source_rate as f64,
        2.0,
        params,
        SINC_RESAMPLE_CHUNK_FRAMES,
        channels,
    )
    .ok()
}

/// 将平面格式（每声道独立 `Vec<f32>`）转换为交错格式。
///
/// 输出顺序为帧优先：`[ch0_frame0, ch1_frame0, ch0_frame1, ch1_frame1, ...]`。
/// 写入前对每个样本调用 `sanitize_pcm_sample`。
pub(crate) fn interleave_planar(planar: &[Vec<f32>]) -> Vec<f32> {
    let channels = planar.len();
    if channels == 0 {
        return Vec::new();
    }
    let frames = planar.iter().map(Vec::len).min().unwrap_or(0);
    let mut output = Vec::with_capacity(frames * channels);
    for frame in 0..frames {
        for channel in planar {
            output.push(sanitize_pcm_sample(channel[frame]));
        }
    }
    output
}

/// 将浮点 PCM 样本裁剪到 `[-1.0, 1.0]`，非有限值归零。
///
/// 用于保证进入采样缓冲区的所有样本处于合法范围，防止下游输出驱动收到越界值。
pub(crate) fn sanitize_pcm_sample(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}
