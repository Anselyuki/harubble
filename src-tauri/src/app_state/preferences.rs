//! 偏好设置子系统，聚合持久化存储、内存镜像与写锁三个协同字段。
//!
//! 将原本分散在 `AppState` 顶层的偏好相关字段归并为一个具名子结构，
//! 以明确其协作边界：写操作必须先持有 `preferences_write_lock`，
//! 再通过 `preferences_store` 落盘，最后更新 `preferences` 内存镜像。

use crate::preferences::{AppPreferences, PreferencesStore};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex;

/// 偏好设置子系统，持有偏好的持久化存储、运行时内存镜像与写序列化锁。
///
/// 三个字段协同工作：
/// - `preferences_store`：负责偏好的序列化与磁盘读写
/// - `preferences`：当前生效的内存镜像，读多写少，使用标准库互斥锁以避免异步开销
/// - `preferences_write_lock`：异步写锁，确保并发写操作串行化，防止丢失更新
#[derive(Clone)]
pub(crate) struct PreferencesSubsystem {
    /// 偏好持久化存储，负责序列化/反序列化与磁盘 I/O
    pub(crate) preferences_store: Arc<PreferencesStore>,
    /// 运行时内存镜像，提供低延迟读访问
    pub(crate) preferences: Arc<StdMutex<AppPreferences>>,
    /// 异步写序列化锁，防止并发偏好写操作产生竞争
    pub(crate) preferences_write_lock: Arc<Mutex<()>>,
}
