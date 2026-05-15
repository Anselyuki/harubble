//! 播放输出后端抽象与默认实现选择入口。
//!
//! 该模块定义播放器与具体音频输出实现之间的抽象边界，并负责创建默认播放后端；
//! 当前默认后端为基于 CPAL 的桌面音频输出实现。

use crate::player::stream::{AudioFormat, PlaybackErrorHandler, SampleBuffer};
use anyhow::Result;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// 音频播放后端抽象。
pub trait PlaybackBackend: Send {
    /// 根据源格式协商后端实际接受的输出格式。
    fn negotiate_output_format(&self, source_format: AudioFormat) -> Result<AudioFormat>;

    /// 启动音频播放流。
    #[allow(clippy::too_many_arguments)]
    fn play_stream(
        &mut self,
        format: AudioFormat,
        samples: SampleBuffer,
        stop_flag: Arc<AtomicBool>,
        volume: Arc<Mutex<f64>>,
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
