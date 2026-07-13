//! 日志查询与日志文件状态读取相关的 Tauri command。
//!
//! 当前暴露的接口覆盖日志分页查询与日志文件状态检查，
//! 主要用于前端日志面板浏览、筛选与导出前的可用性判断。

use crate::app_state::AppState;
use crate::logging::{LogFileStatus, LogViewerPage, LogViewerQuery};
use tauri::State;

/// 按查询条件读取日志记录分页结果。
///
/// 适用于日志面板分页加载、按级别或关键字过滤日志，以及导出前预览当前查询结果。
/// 入参 `query` 描述分页、级别与筛选条件；返回值为对应页的日志记录结果。
/// 该接口返回的是查询瞬间的日志视图；若会话中仍有新日志持续写入，调用方需要重新查询后续页或刷新当前条件。
#[tauri::command]
pub fn list_log_records(
    state: State<'_, AppState>,
    query: LogViewerQuery,
) -> Result<LogViewerPage, String> {
    state.log_center().list_records(query)
}

/// 获取当前会话日志与持久化日志文件的存在状态。
///
/// 适用于日志面板判断日志来源是否可读，或在导出/打开日志文件前先检查文件状态。
/// 返回值为日志文件状态摘要。
/// 该接口只报告当前状态，不会主动创建日志文件；调用方应根据返回值决定是否展示“文件不存在”或“稍后再试”等提示。
#[tauri::command]
pub fn get_log_file_status(state: State<'_, AppState>) -> Result<LogFileStatus, String> {
    Ok(state.log_center().file_status())
}
