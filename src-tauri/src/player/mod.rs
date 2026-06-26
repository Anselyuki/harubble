//! 播放器宿主层公共模块。
//!
//! 该模块聚合播放器后端、控制器、媒体会话、状态快照、事件与解码流等子模块，
//! 为 Tauri command、系统媒体控制与前端状态同步提供统一的播放器公共入口。

pub mod api;
pub mod backend;
pub mod controller;
pub mod events;
pub mod media;
pub mod state;
pub mod stream;

/// 播放命令返回给前端的稳定结果与错误类型。
pub use api::{PlaybackError, PlaybackErrorCode, PlaybackStartResult};
/// 播放控制器与前后端共享的播放队列上下文类型。
pub use controller::{AudioPlayer, PlaybackContext, PlaybackQueueEntry};
/// 提供给前端与媒体会话同步使用的播放器状态快照。
pub use state::PlayerState;
