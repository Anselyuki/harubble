//! 面向前端暴露的 Tauri command 模块集合。
//!
//! 该模块按领域拆分出媒体库、播放控制、下载任务、偏好、本地库存、日志与搜索等
//! command 子模块，是前端通过 Tauri bridge 调用后端能力的主要入口目录。

pub mod album_catalog;
pub mod collection;
pub mod downloads;
pub mod homepage;
pub mod library;
pub mod local_inventory;
pub mod logging;
pub mod menu;
pub mod playback;
pub mod preferences;
pub mod search;
pub mod tag_editor;
pub mod tag_registry;
pub mod window;
