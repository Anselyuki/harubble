//! 播放输出后端抽象与默认实现选择入口。
//!
//! 该模块定义播放器与具体音频输出实现之间的抽象边界，并负责创建默认播放后端；
//! 当前默认后端为基于 CPAL 的桌面音频输出实现。

use crate::player::stream::{AudioFormat, PlaybackErrorHandler, SampleBuffer};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::Arc;

/// 播放后端最终打开输出设备时使用的样本格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputSampleFormat {
    F32,
    F64,
    I8,
    I16,
    I24,
    I32,
    I64,
    U8,
    U16,
    U24,
    U32,
    U64,
}

impl OutputSampleFormat {
    /// 返回该输出样本格式对应的容器位深。
    pub(crate) fn bits_per_sample(self) -> u16 {
        match self {
            Self::F32 | Self::I32 | Self::U32 => 32,
            Self::F64 | Self::I64 | Self::U64 => 64,
            Self::I8 | Self::U8 => 8,
            Self::I16 | Self::U16 => 16,
            Self::I24 | Self::U24 => 24,
        }
    }

    /// 返回用于日志与诊断展示的稳定样本格式名称。
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I24 => "i24",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U24 => "u24",
            Self::U32 => "u32",
            Self::U64 => "u64",
        }
    }
}

/// 播放后端协商出的完整输出格式。
#[derive(Debug, Clone, PartialEq)]
pub struct OutputFormat {
    /// 解码线程需要转换到的通道数、采样率和时长。
    pub audio_format: AudioFormat,
    /// 输出设备实际接受的样本格式。
    pub sample_format: OutputSampleFormat,
    /// 协商时的输出设备身份；用于开流前检测默认设备是否已切换。
    pub device_identity: String,
}

/// 回调运行时间直方图桶数（覆盖 1μs~65ms 的 log2 分布）。
///
/// 索引 i 对应 [2^i μs, 2^(i+1) μs) 区间；索引 15 为 ≥32768μs（约 33ms 及以上）的溢出桶。
pub const CALLBACK_DURATION_BUCKETS: usize = 16;

/// 音频实时回调路径的聚合指标（P1-6 基准测量）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioCallbackMetrics {
    /// 输出回调因为拿不到样本缓冲锁而静音的次数。
    pub silence_due_to_lock: u64,
    /// 输出回调因可播放样本不足而补静音的帧数。
    pub underrun_frames: u64,
    /// 上报窗口内回调调用总次数。
    pub callback_count: u64,
    /// 上报窗口内回调运行时间累加（纳秒），配合 `callback_count` 求平均。
    pub callback_elapsed_ns_total: u64,
    /// 上报窗口内单次回调最长运行时间（纳秒）。
    pub callback_elapsed_ns_max: u64,
    /// 上报窗口内超过 `frames / sample_rate` 理论回调周期的调用次数。
    pub callback_over_period_count: u64,
    /// 指标队列满时未能入队的完整回调观测数。
    pub callback_metrics_dropped_count: u64,
    /// 上报窗口内设备请求的输出帧数总和。
    pub callback_frames_total: u64,
    /// 上报窗口内单次回调请求的最小输出帧数。
    pub callback_frames_min: u64,
    /// 上报窗口内单次回调请求的最大输出帧数。
    pub callback_frames_max: u64,
    /// 上报窗口内回调运行时间 log2μs 直方图（索引即桶编号）。
    ///
    /// 供 monitor 线程近似求 P50/P95/P99 百分位；直方图桶在回调路径内以 Relaxed 原子写入。
    pub callback_duration_buckets: [u64; CALLBACK_DURATION_BUCKETS],
    /// 新流启动后等待首次设备回调的时间（纳秒）。
    pub stream_start_wait_ns: u64,
    /// 等待新流首次设备回调超时的次数。
    pub stream_start_timeout_count: u64,
    /// 新流在首次设备回调前收到后端错误的次数。
    pub stream_start_failure_count: u64,
    /// 输出流发生 underrun/overrun 后由后端继续恢复的次数。
    pub output_xrun_count: u64,
    /// 上报窗口内实际写入设备缓冲的样本数。
    pub output_sample_count: u64,
    /// 实际输出样本平方和，使用 Q32 定点表示。
    pub output_square_sum_q32: u64,
    /// 实际输出样本绝对峰值的 `f32` 位表示。
    pub output_peak_abs_bits: u64,
    /// 写入设备前被裁剪到 `[-1.0, 1.0]` 的样本数。
    pub output_clipped_samples: u64,
    /// 写入设备前被归零的 NaN/Inf 样本数。
    pub output_nonfinite_samples: u64,
}

impl AudioCallbackMetrics {
    /// 返回当前上报窗口内实际输出的绝对峰值。
    pub fn output_peak_abs(self) -> f64 {
        f64::from(f32::from_bits(self.output_peak_abs_bits as u32))
    }

    /// 返回当前完整回调上报窗口内实际输出的均方根值。
    pub fn output_rms(self) -> f64 {
        if self.output_sample_count == 0 {
            return 0.0;
        }
        let mean_square = self.output_square_sum_q32 as f64
            / (1_u64 << 32) as f64
            / self.output_sample_count as f64;
        mean_square.max(0.0).sqrt()
    }
}

/// 音频实时回调指标的上报回调。
pub type AudioMetricsHandler = Arc<dyn Fn(AudioCallbackMetrics) + Send + Sync>;

/// 输出回调检测到播放缓冲不足时的快速通知回调。
pub type AudioUnderrunHandler = Arc<dyn Fn() + Send + Sync>;

/// 音频播放后端抽象。
pub trait PlaybackBackend: Send {
    /// 根据源格式协商后端实际接受的输出格式。
    fn negotiate_output_format(&self, source_format: AudioFormat) -> Result<OutputFormat>;

    /// 启动音频播放流。
    #[allow(clippy::too_many_arguments)]
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
    ) -> Result<()>;

    /// 让当前输出进入会话切换期。
    ///
    /// 调用方会先停止旧会话；后端应停止消费旧样本，但尽可能保留设备输出流，持续
    /// 写入数字静音，直到新流准备完成。无法保留流的后端可以退化为普通停止。
    fn quiesce_for_transition(&mut self) -> Result<()> {
        self.stop()
    }

    /// 暂停播放。
    fn pause(&mut self) -> Result<()>;

    /// 恢复播放。
    fn resume(&mut self) -> Result<()>;

    /// 停止播放并释放当前流。
    fn stop(&mut self) -> Result<()>;
}

pub mod cpal;
pub(crate) mod cpal_helpers;

/// 创建默认播放后端实现。
///
/// 当前默认返回基于 CPAL 的后端；若底层音频设备或输出流初始化失败，会直接返回
/// 错误给上层调用方。
pub fn create_backend() -> Result<Box<dyn PlaybackBackend>> {
    Ok(Box::new(cpal::CpalBackend::new()?))
}
