//! 下载子系统，聚合下载服务实例、会话持久化存储与任务创建锁。
//!
//! 将原本分散在 `AppState` 顶层的下载相关字段归并为一个具名子结构，
//! 以明确协作约束：新建任务时需先持有 `download_job_creation_lock`，
//! 任务完成或状态变更后通过 `download_session_store` 持久化快照。

use crate::download_session::DownloadSessionStore;
use harubble_core::DownloadService;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 下载子系统，持有下载服务实例、会话快照存储与任务创建互斥锁。
///
/// 三个字段协同工作：
/// - `download_service`：核心下载管理器，管理批次生命周期与任务调度
/// - `download_job_creation_lock`：防止并发批次创建产生 ID 冲突或重复任务
/// - `download_session_store`：将下载会话快照持久化到磁盘，支持重启后恢复
#[derive(Clone)]
pub(crate) struct DownloadSubsystem {
    /// 核心下载服务，管理批次创建、任务调度与状态迁移
    pub(crate) download_service: Arc<Mutex<DownloadService>>,
    /// 批次创建互斥锁，确保 ID 生成与批次注册的原子性
    pub(crate) download_job_creation_lock: Arc<Mutex<()>>,
    /// 下载会话快照持久化存储，用于应用重启后的状态恢复
    pub(crate) download_session_store: Arc<DownloadSessionStore>,
}
