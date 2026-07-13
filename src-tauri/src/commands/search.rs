//! 库内搜索与索引查询相关的 Tauri command。
//!
//! 当前暴露的接口覆盖本地库存索引上的搜索请求执行，
//! 主要用于前端搜索框、筛选切换与结果页的本地召回。

use crate::app_state::AppState;
use harubble_core::{SearchLibraryRequest, SearchLibraryResponse};
use std::fmt;
use tauri::State;

// ─── 错误类型 ─────────────────────────────────────────────────────────────────

/// 搜索域操作错误。
///
/// 实现 `Serialize`，可直接通过 Tauri IPC 序列化返回前端。
/// 序列化格式：`{ "code": "<variant>", "detail": <payload> }`。
#[derive(Debug, serde::Serialize)]
#[serde(tag = "code", content = "detail", rename_all = "camelCase")]
pub enum SearchError {
    /// 索引尚未就绪或正在重建，调用方可稍后重试。
    NotReady,
    /// 其他内部错误，通常不可重试。
    Internal(String),
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchError::NotReady => write!(f, "搜索索引尚未就绪，请稍后重试"),
            SearchError::Internal(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SearchError {}

// ─── Command ─────────────────────────────────────────────────────────────────

/// 在本地索引中执行库内搜索。
///
/// 适用于搜索框提交、筛选条件切换，或需要在本地库存索引中按范围执行召回的场景。
/// 入参 `request` 描述查询词、分页与搜索范围；返回值为本次搜索结果与索引状态。
/// 该接口依赖本地索引状态；当索引尚未就绪或正在重建时，调用方应结合返回值中的状态字段决定展示空结果、占位态还是重试提示。
#[tauri::command]
pub async fn search_library(
    state: State<'_, AppState>,
    request: SearchLibraryRequest,
) -> Result<SearchLibraryResponse, SearchError> {
    state
        .library_search_service
        .search(request, state.preferences().locale)
        .await
        .map_err(|msg| {
            let lower = msg.to_lowercase();
            if lower.contains("not ready") || lower.contains("index not ready") {
                SearchError::NotReady
            } else {
                SearchError::Internal(msg)
            }
        })
}
