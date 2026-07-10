//! API 客户端分组，聚合应用各链路使用的 HTTP 客户端实例。
//!
//! 将原本分散在 `AppState` 顶层的四个 `ApiClient` 字段归并为一个具名子结构，
//! 便于统一执行缓存清理与 HTTP 客户端重置等跨客户端操作。

use harubble_core::ApiClient;
use std::sync::Arc;

/// 应用全局 API 客户端集合。
///
/// 各字段对应不同流量域的 HTTP 客户端：通用 API、播放专用、图片资源与下载链路。
/// 独立客户端使连接池、缓存与限速策略可按域隔离，互不干扰。
#[derive(Clone)]
pub(crate) struct ApiClients {
    /// 通用业务 API（专辑、歌曲详情、搜索等）
    pub(crate) api: Arc<ApiClient>,
    /// 播放链路专用客户端，用于拉取音频源 URL 与播放详情
    pub(crate) playback_api: Arc<ApiClient>,
    /// 图片资源专用客户端，用于封面等视觉素材下载
    pub(crate) image_api: Arc<ApiClient>,
    /// 下载链路专用客户端，用于音频文件的批量下载
    pub(crate) download_api: Arc<ApiClient>,
}
