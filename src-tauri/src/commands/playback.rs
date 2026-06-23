//! 播放控制与播放器状态读取相关的 Tauri command。
//!
//! 当前暴露的接口覆盖点播、暂停、恢复、跳转、上下曲切换与状态查询，
//! 主要服务于前端播放器、系统媒体控制和播放进度交互。

use crate::app_state::AppState;
use crate::player::{PlaybackContext, PlayerState};
use tauri::State;

/// 播放指定歌曲，并可携带封面地址与播放队列上下文。
///
/// 适用于用户主动点播、从队列切歌，或在上一首/下一首导航后启动新的播放会话。
/// 入参 `song_cid` 为要播放的歌曲唯一标识，`cover_url` 为可选封面地址，`playback_context` 为可选队列上下文；返回值为本次启动后可见的总时长（秒）。
/// 该接口会中断当前播放并创建新会话；若传入队列上下文，调用方应保证当前歌曲确实属于该队列，否则导航状态可能退化为单曲上下文。
#[tauri::command]
pub async fn play_song(
    state: State<'_, AppState>,
    song_cid: String,
    cover_url: Option<String>,
    playback_context: Option<PlaybackContext>,
) -> Result<f64, String> {
    state
        .play_song_internal(song_cid, cover_url, playback_context)
        .await
}

/// 暂停当前播放。
///
/// 适用于用户希望保留当前歌曲上下文与进度，但暂时停止声音输出的场景。
/// 返回值在成功时为空；若当前并未处于可暂停的播放态，该接口会无副作用地返回成功。
/// 调用方不需要先自行判断播放器状态，但应把“成功返回”理解为“已达到暂停目标”，而不是一定发生了状态切换。
#[tauri::command]
pub fn pause_playback(state: State<'_, AppState>) -> Result<(), String> {
    state.player.pause().map_err(|e| e.to_string())
}

/// 恢复当前已暂停的播放。
///
/// 适用于暂停后继续播放当前歌曲的场景。
/// 返回值在成功时为空；若当前没有可恢复的歌曲上下文，该接口会直接返回成功而不启动新播放。
/// 若调用方需要“没有上下文时自动开始播放”，应显式改用 `play_song`，不要把该接口当作通用播放入口。
#[tauri::command]
pub fn resume_playback(state: State<'_, AppState>) -> Result<(), String> {
    state.player.resume().map_err(|e| e.to_string())
}

/// 将当前播放进度跳转到指定秒数，并返回更新后的总时长。
///
/// 适用于进度条拖拽、系统媒体控制 seek，或程序化跳转到指定位置的场景。
/// 入参 `position_secs` 为目标秒数；返回值为重建播放会话后确认的总时长（秒）。
/// 该接口要求当前存在活跃歌曲且不处于加载中；调用时会重建播放会话，因此在高频拖拽场景中应由前端自行做节流或仅在拖拽结束时提交。
#[tauri::command]
pub async fn seek_current_playback(
    state: State<'_, AppState>,
    position_secs: f64,
) -> Result<f64, String> {
    state.seek_current_internal(position_secs).await
}

/// 播放队列中的下一首歌曲。
///
/// 适用于播放器“下一首”操作或系统媒体会话的 next 控制。
/// 返回值为切换后歌曲的总时长（秒）。
/// 该接口依赖当前会话已建立可导航的队列上下文；若当前不是队列播放或已位于末尾，将返回错误。
#[tauri::command]
pub async fn play_next(state: State<'_, AppState>) -> Result<f64, String> {
    state.play_next_internal().await
}

/// 播放队列中的上一首歌曲。
///
/// 适用于播放器“上一首”操作或系统媒体会话的 previous 控制。
/// 返回值为切换后歌曲的总时长（秒）。
/// 该接口依赖当前会话已建立可导航的队列上下文；若当前不是队列播放或已位于开头，将返回错误。
#[tauri::command]
pub async fn play_previous(state: State<'_, AppState>) -> Result<f64, String> {
    state.play_previous_internal().await
}

/// 获取当前播放器状态快照。
///
/// 适用于前端初始化播放器 UI、页面重连后同步状态，或在事件丢失后主动兜底拉取状态。
/// 返回值为当前 `PlayerState` 快照，不会触发任何播放副作用。
/// 该接口返回的是读取瞬间的状态视图；实时更新仍应以播放器事件流为主，而不是高频轮询此接口。
#[tauri::command]
pub fn get_player_state(state: State<'_, AppState>) -> Result<PlayerState, String> {
    Ok(state.player.get_state())
}

/// 设置播放器音量，并返回经过约束后的实际音量值。
///
/// 适用于音量滑杆提交、恢复用户偏好音量，或外部媒体控制同步音量场景。
/// 入参 `volume` 预期为 `0.0..=1.0` 范围内的浮点值；返回值为经过裁剪后的实际音量。
/// 该接口具备幂等性：传入相同有效音量时结果稳定；若传入越界值会被自动裁剪，调用方不应假设返回值一定等于原始输入。
/// 音量变化会自动持久化到偏好文件，下次启动时恢复。
#[tauri::command]
pub async fn set_playback_volume(state: State<'_, AppState>, volume: f64) -> Result<f64, String> {
    let actual = state.player.set_volume(volume);
    if (state.preferences().volume - actual).abs() > 0.001 {
        state
            .update_preferences(|prefs| {
                prefs.volume = actual;
            })
            .await?;
    }
    Ok(actual)
}
