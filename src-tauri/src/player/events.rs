//! 播放器事件常量与事件发射辅助函数。
//!
//! 该模块定义播放器状态变化与播放进度推进相关的 Tauri 事件名，并提供统一的快照
//! 广播入口，供播放器控制器在状态变化后通知前端。

use crate::player::state::PlayerState;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

/// 播放状态、队列导航标记或音量发生变化时发出的 Tauri 事件名。
///
/// 事件载荷为完整的 [`PlayerState`] 快照。
pub const PLAYER_STATE_CHANGED: &str = "player-state-changed";
/// 播放进度推进时发出的 Tauri 事件名。
///
/// 事件载荷同样是完整的 [`PlayerState`] 快照，便于前端用统一结构同步时间、时长
/// 与当前歌曲元数据。
pub const PLAYER_PROGRESS: &str = "player-progress";

/// 以当前播放器快照发出 [`PLAYER_STATE_CHANGED`] 事件。
pub fn emit_state(app: &AppHandle, state: &Arc<Mutex<PlayerState>>) {
    let _ = app.emit(PLAYER_STATE_CHANGED, state.lock().unwrap().clone());
}

/// 以当前播放器快照发出 [`PLAYER_PROGRESS`] 事件。
pub fn emit_progress(app: &AppHandle, state: &Arc<Mutex<PlayerState>>) {
    let _ = app.emit(PLAYER_PROGRESS, state.lock().unwrap().clone());
}

/// 以已有的播放器状态快照发出 [`PLAYER_PROGRESS`] 事件。
///
/// 适用于调用方已经持有状态快照（例如节流后的进度发射线程），可避免重复加锁与克隆。
pub fn emit_progress_snapshot(app: &AppHandle, snapshot: &PlayerState) {
    let _ = app.emit(PLAYER_PROGRESS, snapshot.clone());
}
