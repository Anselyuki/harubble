//! 首页数据模型。
//!
//! 定义首页各区块所需的聚合数据结构，作为前后端共享契约的核心库侧来源：
//! 按系列分组的专辑（[`SeriesGroup`]）、收听历史与事件（[`HistoryEntry`] /
//! [`ListeningEvent`]）、状态仪表盘（[`HomepageStatus`]）以及按 Tag 分组的
//! 专辑集合（[`TagGroup`]）。
//!
//! 本模块只承载序列化数据形状，不包含持久化或业务编排逻辑；首页数据的
//! 装配、收听历史读写等行为由上层服务与 Tauri command 层负责。所有结构均以
//! `camelCase` 序列化，与前端 `src/lib/types.ts` 中对应类型保持形状一致。

use serde::{Deserialize, Serialize};

use crate::api::Album;

/// 按系列分组的专辑集合，用于首页"按系列浏览"区块。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeriesGroup {
    pub series: String,
    pub albums: Vec<Album>,
}

/// 歌曲热度条目，对应 SQLite `song_heat` 表的一行。
///
/// `heat` 为有效收听次数（播放进度达到阈值时累加），`last_played_at` 为最近播放时间（ISO8601）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub song_cid: String,
    pub song_name: String,
    pub album_cid: String,
    pub album_name: String,
    pub cover_url: Option<String>,
    pub artists: Vec<String>,
    pub heat: u32,
    pub played_at: String,
}

/// 收听事件，由 `play_song` 内部自动构造并写入 SQLite。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListeningEvent {
    pub song_cid: String,
    pub song_name: String,
    pub album_cid: String,
    pub album_name: String,
    pub cover_url: Option<String>,
    pub artists: Vec<String>,
}

/// 首页状态仪表盘聚合数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomepageStatus {
    pub platform_album_count: u32,
    pub platform_song_count: u32,
    pub local_downloaded_count: u32,
    pub local_storage_bytes: u64,
    pub active_download_count: u32,
    pub completed_download_count: u32,
}

/// 按单个 tag 值聚合的专辑分组，用于首页"按维度浏览"区块。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagGroup {
    /// 维度 key（程序标识符，非展示名）。
    pub dimension_key: String,
    /// 当前分组的 tag 值（已本地化）。
    pub value: String,
    /// 命中该 tag 值的专辑列表。
    pub albums: Vec<Album>,
}

/// 从专辑名称中派生系列标签。
///
/// 对名称做大小写不敏感的单词边界匹配，识别 OST、EP 等关键词。
/// 返回匹配到的标签列表；未匹配到任何关键词时返回空列表。
pub fn derive_series_tags(name: &str) -> Vec<&'static str> {
    let upper = name.to_uppercase();
    let bytes = upper.as_bytes();
    let mut tags = Vec::new();

    if let Some(pos) = upper.find("OST") {
        let before_ok = pos == 0 || !bytes[pos - 1].is_ascii_alphanumeric();
        let after_ok = pos + 3 >= bytes.len() || !bytes[pos + 3].is_ascii_alphanumeric();
        if before_ok && after_ok {
            tags.push("OST");
        }
    }

    if let Some(pos) = upper.find("EP") {
        let before_ok = pos == 0 || !bytes[pos - 1].is_ascii_alphanumeric();
        let after_ok = pos + 2 >= bytes.len() || !bytes[pos + 2].is_ascii_alphanumeric();
        if before_ok && after_ok {
            tags.push("EP");
        }
    }

    tags
}
