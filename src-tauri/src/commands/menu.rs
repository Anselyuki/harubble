//! 与应用菜单栏同步相关的 Tauri command。
//!
//! 目前只提供播放器状态到勾选态的同步入口：前端在 `player.repeatMode` 或
//! `player.shuffleEnabled` 变化时调用此命令，让循环模式与随机播放的勾选态
//! 与 UI 保持一致。命令通过内部的 `crate::menu::sync_playback_state` 操作已挂载
//! 菜单中的 `CheckMenuItem` 引用，因此在菜单尚未装载时是空操作。

/// 把播放器当前状态同步到菜单勾选态。
///
/// - `repeat_mode`：形如 `"off"` / `"all"` / `"one"` 的字符串（前端 `RepeatMode` union）。
/// - `shuffle_enabled`：随机播放是否启用。
///
/// 该命令**不返回业务错误**：菜单同步失败会向调用方返回一个说明字符串，
/// 但前端 UI 侧应把它视作可忽略的软失败（例如启动初期菜单尚未构建）。
#[tauri::command]
pub async fn sync_playback_menu_state(
    repeat_mode: String,
    shuffle_enabled: bool,
) -> Result<(), String> {
    crate::menu::sync_playback_state(&repeat_mode, shuffle_enabled)
        .map_err(|error| error.to_string())
}
