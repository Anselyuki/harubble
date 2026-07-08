//! 播放器状态快照模型。
//!
//! 该模块定义向前端界面与系统媒体会话广播的播放器状态结构，是播放控制器、事件层
//! 与前端播放器之间共享的稳定状态契约。

use serde::Serialize;

/// 当前播放链路的输入与输出音频格式摘要。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackFormatState {
    /// 输入音频采样率，单位为 Hz。
    pub source_sample_rate: u32,
    /// 输入音频通道数。
    pub source_channels: u16,
    /// 输入 PCM 位深；未知时为 `None`。
    pub source_bits_per_sample: Option<u16>,
    /// 输出音频采样率，单位为 Hz。
    pub output_sample_rate: u32,
    /// 输出音频通道数。
    pub output_channels: u16,
    /// 输出 PCM 位深；未知时为 `None`。
    pub output_bits_per_sample: Option<u16>,
    /// 输出设备样本格式。
    pub output_sample_format: String,
    /// 是否启用了采样率转换。
    pub resampling: bool,
    /// 是否启用了声道转换。
    pub channel_remix: bool,
}

/// 向前端与系统媒体会话广播的播放器状态快照。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    /// 当前播放器会话 ID；每次开始、停止或失败都会推进。
    pub session_id: u64,
    /// 当前已加载歌曲的 CID；空闲时为 `None`。
    pub song_cid: Option<String>,
    /// 当前已加载歌曲名；空闲时为 `None`。
    pub song_name: Option<String>,
    /// 当前歌曲的艺术家列表。
    pub artists: Vec<String>,
    /// 供 UI 与系统媒体会话使用的当前封面地址。
    pub cover_url: Option<String>,
    /// 是否正在主动播放音频。
    pub is_playing: bool,
    /// 是否在保留当前歌曲上下文的前提下处于暂停态。
    pub is_paused: bool,
    /// 后端是否仍在为当前歌曲准备可播放音频。
    pub is_loading: bool,
    /// 当前队列是否可以切换到上一项。
    pub has_previous: bool,
    /// 当前队列是否可以切换到下一项。
    pub has_next: bool,
    /// 当前播放进度，单位为秒。
    pub progress: f64,
    /// 当前歌曲总时长，已知时单位为秒。
    pub duration: f64,
    /// 当前播放音量，范围固定为 `0.0..=1.0`。
    pub volume: f64,
    /// 当前播放链路的输入与输出音频格式摘要。
    pub playback_format: Option<PlaybackFormatState>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            session_id: 0,
            song_cid: None,
            song_name: None,
            artists: Vec::new(),
            cover_url: None,
            is_playing: false,
            is_paused: false,
            is_loading: false,
            has_previous: false,
            has_next: false,
            progress: 0.0,
            duration: 0.0,
            volume: 1.0,
            playback_format: None,
        }
    }
}
