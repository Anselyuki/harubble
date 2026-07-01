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
    ) -> Result<()>;

    /// 暂停播放。
    fn pause(&mut self) -> Result<()>;

    /// 恢复播放。
    fn resume(&mut self) -> Result<()>;

    /// 停止播放并释放当前流。
    fn stop(&mut self) -> Result<()>;
}

pub mod cpal;

/// 创建默认播放后端实现。
///
/// 当前默认返回基于 CPAL 的后端；若底层音频设备或输出流初始化失败，会直接返回
/// 错误给上层调用方。
pub fn create_backend() -> Result<Box<dyn PlaybackBackend>> {
    Ok(Box::new(cpal::CpalBackend::new()?))
}
