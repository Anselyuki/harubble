//! Harubble 后端库。
//!
//! 这个 crate 是桌面应用后端的宿主层入口，负责把 `harubble_core` 提供的领域能力接入
//! Tauri 运行时，并组合成可供 `main.rs`、命令注册、事件桥接与集成测试复用的最小公共边界。
//! 如果 `harubble_core` 关注的是“平台无关的核心能力”，那么这里关注的就是“应用进程内如何把
//! 这些能力接线成一个真正可运行的桌面后端”。
//!
//! # 首页导航
//!
//! - [`AppState`]：聚合 API 客户端、播放器、下载服务、偏好与日志中心的应用级共享状态。
//! - [`commands`]：所有对前端暴露的 Tauri command 包装层，是 UI 调用后端的主要入口。
//! - [`initialize_download_bridge`]：后台下载批次消费循环与事件广播桥接的启动入口。
//! - [`spawn_inventory_scan`]：本地库存扫描的公开触发入口。
//! - [`LogLevel`]、[`LogPayload`]：供后端内部与宿主层复用的结构化日志类型。
//!
//! # 什么时候看这个 crate
//!
//! - 当你要理解某个 Tauri command 如何访问后端状态，应该先看 [`commands`] 与 [`AppState`]。
//! - 当你要排查下载任务为什么会在前端持续收到事件，应该看 [`initialize_download_bridge`]。
//! - 当你要补集成测试或启动 wiring，而不想深入所有私有模块时，这里提供了最小必要入口。
//!
//! # 设计边界
//!
//! - 只暴露 `main.rs` 启动、命令注册和测试复用真正需要的公共入口。
//! - 内部实现细节（helper、normalization、worker pipeline）保持私有，避免把临时结构扩散成契约。
//! - 不为测试便利扩大可见性；若某项能力需要被外部使用，应通过明确公共入口暴露，而不是直接泄漏内部模块。
//!
//! # 与 `harubble_core` 的关系
//!
//! 该 crate 主要负责宿主层编排：状态装配、Tauri command、事件发射、系统通知、播放器后端和偏好持久化。
//! 业务语义本身仍尽量下沉到 `harubble_core`，因此当你需要修改下载模型、搜索结果或音频写盘契约时，
//! 通常应该优先检查 `harubble_core` 的公开 API，再回到这里看宿主层如何接入。

mod album_metadata_cache;
mod app_state;
mod audio_cache;
mod background_tasks;
pub mod collection;
mod command_registry;
mod command_scheduling;
pub mod commands;
pub mod desktop_lifecycle;
mod download_session;
mod downloads;
mod i18n;
mod listening_history;
mod local_inventory;
mod local_inventory_provenance;
mod local_inventory_scan;
mod logging;
mod migration;
mod network_monitor;
mod notification;
mod playback_actor;
mod playback_load_gate;
mod player;
mod preferences;
mod search;
mod startup_recovery;
mod tag_editor;
mod tag_registry;
mod tag_registry_index;
mod theme;

/// 启动 belong 预热后台任务。
///
/// 适用于应用启动阶段在后台异步预热 belong 缓存，以便首页"按系列浏览"功能在用户打开时
/// 能够立即展示分组数据。
pub use app_state::spawn_belong_warmup;
/// 启动 tag registry 远程同步后台任务。
///
/// 在应用启动后异步从远程拉取最新 tag JSON，与本地版本比对后按需替换。
pub use app_state::spawn_tag_registry_sync;
/// 应用级共享状态入口。
///
/// 适用于 `main.rs` 启动 wiring、Tauri command 注入，以及需要访问聚合后端能力的集成测试入口。
pub use app_state::AppState;
/// 后台任务目录 —— 跨领域任务生命周期协调层。
///
/// 适用于搜索 / 下载 / 库扫描 / tag registry sync 等跨领域后台任务的统一登记、
/// 取消和 shutdown 协调；业务状态仍由各领域自持。
pub use background_tasks::{spawn_tracked, TaskDirectory, TaskEntry, TaskId, TaskState};
/// 初始化下载桥接执行循环。
///
/// 适用于应用启动后拉起下载任务消费与事件广播主循环。
pub use downloads::bridge::initialize as initialize_download_bridge;
/// 启动一次本地库存扫描。
///
/// 适用于启动后首次扫描、偏好变更后的重扫，或外部调用方主动触发库存刷新。
pub use local_inventory::spawn_inventory_scan;
/// 日志等级与日志载荷公共类型。
///
/// 适用于构造结构化日志并通过公共后端入口统一记录。
pub use logging::{LogLevel, LogPayload};
/// 启动网络配置变更监听后台任务。
///
/// 在 macOS 上监听系统代理与网络路由变化，自动重建 HTTP 客户端。
pub use network_monitor::spawn_network_monitor;
