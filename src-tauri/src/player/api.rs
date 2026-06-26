//! 播放命令对外返回的稳定 API 数据结构。

use serde::{Deserialize, Serialize};

/// 播放命令成功返回的会话摘要。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackStartResult {
    /// 后端确认的音频总时长，单位为秒。
    pub duration: f64,
    /// 本次启动对应的播放器会话 ID。
    pub session_id: u64,
}

impl PlaybackStartResult {
    pub fn new(duration: f64, session_id: u64) -> Self {
        Self {
            duration,
            session_id,
        }
    }
}

/// 播放错误的稳定分类。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PlaybackErrorCode {
    /// 当前请求已被更新的播放会话取代。
    Superseded,
    /// 当前没有可播放的歌曲上下文。
    NoActiveTrack,
    /// 当前操作需要队列里的下一首，但当前队列没有可导航的目标。
    NoNextTrack,
    /// 当前操作需要队列里的上一首，但当前队列没有可导航的目标。
    NoPreviousTrack,
    /// 播放器仍在加载，暂不接受该操作。
    Loading,
    /// 网络或远端接口相关失败。
    Network,
    /// 音频探测、解码、重采样或输出失败。
    Audio,
    /// 本地缓存或文件 IO 失败。
    Io,
    /// 其他内部错误。
    Internal,
}

impl PlaybackErrorCode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Superseded => "superseded",
            Self::NoActiveTrack => "noActiveTrack",
            Self::NoNextTrack => "noNextTrack",
            Self::NoPreviousTrack => "noPreviousTrack",
            Self::Loading => "loading",
            Self::Network => "network",
            Self::Audio => "audio",
            Self::Io => "io",
            Self::Internal => "internal",
        }
    }
}

/// 播放命令失败时返回给前端的结构化错误。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackError {
    pub code: PlaybackErrorCode,
    pub message: String,
    pub retryable: bool,
    pub session_id: Option<u64>,
}

impl PlaybackError {
    pub fn new(
        code: PlaybackErrorCode,
        message: impl Into<String>,
        retryable: bool,
        session_id: Option<u64>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
            session_id,
        }
    }

    pub fn superseded(session_id: u64) -> Self {
        Self::new(
            PlaybackErrorCode::Superseded,
            "Playback request was superseded by a newer session",
            false,
            Some(session_id),
        )
    }
}

impl std::fmt::Display for PlaybackError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for PlaybackError {}
