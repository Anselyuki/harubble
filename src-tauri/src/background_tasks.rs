//! 后台任务目录 —— 跨领域生命周期协调层。
//!
//! P1-4 目标：为搜索 / 下载 / 库扫描 / tag registry sync 等跨领域后台任务提供
//! 统一的可追踪、可取消、可回收基础设施；任务业务逻辑仍归各自领域，中央层
//! 只保留任务目录条目 + shutdown 协调 + 取消令牌。
//!
//! # 设计约束
//! - 不持有任务业务状态；条目仅记录 id / domain / state / cancel_token / started_at
//! - spawn_tracked 返回 JoinHandle 让调用方仍可 await
//! - shutdown_all 在应用退出时统一 cancel + 等待所有条目回收
//! - 目录用 tokio::sync::Mutex，避免在 await 期间持有 std::sync::Mutex

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// 后台任务的稳定标识。
///
/// 由领域名 + 逻辑任务名 + 单调递增代次组成，用于日志、指标与调试关联。
/// 代次允许同名任务多次 spawn 时区分（例如重复的 tag_registry_sync）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaskId {
    pub domain: &'static str,
    pub name: &'static str,
    pub generation: u64,
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}#{}", self.domain, self.name, self.generation)
    }
}

/// 任务生命周期状态。
///
/// 状态转移只允许：Pending → Running → { Completed | Failed | Cancelled }。
/// Cancelling 是显式取消请求后到 Cancelled 之间的过渡态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Running,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

/// 目录中的任务条目。
///
/// 只记录元数据，业务对象由领域各自持有。cancel_token 是与目录内条目共享的
/// 取消令牌；调用方在 spawn 内部应显式检查它以便协作式取消。
#[derive(Clone)]
pub struct TaskEntry {
    pub id: TaskId,
    pub state: TaskState,
    pub cancel_token: CancellationToken,
    pub started_at: Instant,
}

/// 跨领域后台任务目录。
///
/// 是应用范围的共享单例（通过 AppState 分发），仅承担登记 / 查询 /
/// 取消 / 全体 shutdown 协调职责，不承载任何业务状态。
#[derive(Clone)]
pub struct TaskDirectory {
    entries: Arc<Mutex<HashMap<TaskId, TaskEntry>>>,
    next_generation: Arc<Mutex<HashMap<(&'static str, &'static str), u64>>>,
}

impl TaskDirectory {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            next_generation: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 分配下一代次并返回构造好的 TaskId。
    ///
    /// 同一 (domain, name) 的代次严格单调递增，用于区分重复触发的同名任务。
    pub async fn next_task_id(&self, domain: &'static str, name: &'static str) -> TaskId {
        let mut gens = self.next_generation.lock().await;
        let generation = gens
            .entry((domain, name))
            .and_modify(|g| *g += 1)
            .or_insert(0);
        TaskId {
            domain,
            name,
            generation: *generation,
        }
    }

    /// 登记一个即将 spawn 的任务，返回其 cancel_token 以便传给业务逻辑。
    pub async fn register(&self, id: TaskId) -> CancellationToken {
        let token = CancellationToken::new();
        let entry = TaskEntry {
            id: id.clone(),
            state: TaskState::Pending,
            cancel_token: token.clone(),
            started_at: Instant::now(),
        };
        self.entries.lock().await.insert(id, entry);
        token
    }

    /// 更新任务状态。
    pub async fn set_state(&self, id: &TaskId, state: TaskState) {
        if let Some(entry) = self.entries.lock().await.get_mut(id) {
            entry.state = state;
        }
    }

    /// 从目录移除任务条目（任务终态回收后调用）。
    pub async fn remove(&self, id: &TaskId) {
        self.entries.lock().await.remove(id);
    }

    /// 请求取消指定任务，返回是否成功找到条目。
    pub async fn cancel(&self, id: &TaskId) -> bool {
        let mut entries = self.entries.lock().await;
        if let Some(entry) = entries.get_mut(id) {
            entry.cancel_token.cancel();
            entry.state = TaskState::Cancelling;
            true
        } else {
            false
        }
    }

    /// 请求取消给定 (domain, name) 名下的所有代次任务。
    ///
    /// 适用于替代型语义：新任务启动时先取消老任务。
    pub async fn cancel_all_named(&self, domain: &str, name: &str) -> usize {
        let mut count = 0;
        for entry in self.entries.lock().await.values_mut() {
            if entry.id.domain == domain && entry.id.name == name {
                entry.cancel_token.cancel();
                entry.state = TaskState::Cancelling;
                count += 1;
            }
        }
        count
    }

    /// 快照当前登记的全部任务条目。
    pub async fn snapshot(&self) -> Vec<TaskEntry> {
        self.entries.lock().await.values().cloned().collect()
    }

    /// 请求全部任务取消。
    ///
    /// 应用退出流程调用；调用方随后应通过等待 JoinHandle 完成回收（由各领域负责）。
    pub async fn cancel_all(&self) {
        for entry in self.entries.lock().await.values_mut() {
            entry.cancel_token.cancel();
            entry.state = TaskState::Cancelling;
        }
    }
}

impl Default for TaskDirectory {
    fn default() -> Self {
        Self::new()
    }
}

/// spawn 一个由目录追踪的后台任务。
///
/// 内部登记条目、构造 cancel_token 传入 body，任务终态时自动从目录移除并转换状态。
/// 返回 JoinHandle 供调用方按需 await 或忽略。
pub fn spawn_tracked<F, Fut, T>(
    directory: TaskDirectory,
    id: TaskId,
    body: F,
) -> tauri::async_runtime::JoinHandle<Option<T>>
where
    F: FnOnce(CancellationToken) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn(async move {
        let token = directory.register(id.clone()).await;
        directory.set_state(&id, TaskState::Running).await;

        let result = if token.is_cancelled() {
            None
        } else {
            let output = body(token.clone()).await;
            Some(output)
        };

        // 终态归档：Cancelled 优先于 Completed
        let final_state = if token.is_cancelled() {
            TaskState::Cancelled
        } else if result.is_some() {
            TaskState::Completed
        } else {
            TaskState::Cancelled
        };
        directory.set_state(&id, final_state).await;
        directory.remove(&id).await;
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn assigns_monotonic_generations() {
        let dir = TaskDirectory::new();
        let id1 = dir.next_task_id("test", "job").await;
        let id2 = dir.next_task_id("test", "job").await;
        let id3 = dir.next_task_id("test", "other").await;
        assert_eq!(id1.generation, 0);
        assert_eq!(id2.generation, 1);
        assert_eq!(id3.generation, 0);
    }

    #[tokio::test]
    async fn register_and_snapshot() {
        let dir = TaskDirectory::new();
        let id = dir.next_task_id("test", "job").await;
        let _token = dir.register(id.clone()).await;
        let snapshot = dir.snapshot().await;
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].id, id);
        assert_eq!(snapshot[0].state, TaskState::Pending);
    }

    #[tokio::test]
    async fn cancel_propagates_to_token() {
        let dir = TaskDirectory::new();
        let id = dir.next_task_id("test", "cancellable").await;
        let token = dir.register(id.clone()).await;
        assert!(!token.is_cancelled());
        dir.cancel(&id).await;
        assert!(token.is_cancelled());
        let snapshot = dir.snapshot().await;
        assert_eq!(snapshot[0].state, TaskState::Cancelling);
    }

    #[tokio::test]
    async fn cancel_all_named_targets_only_matching() {
        let dir = TaskDirectory::new();
        let a_id = dir.next_task_id("dom_a", "same_name").await;
        let b_id = dir.next_task_id("dom_a", "same_name").await;
        let c_id = dir.next_task_id("dom_b", "other").await;
        let a_tok = dir.register(a_id.clone()).await;
        let b_tok = dir.register(b_id.clone()).await;
        let c_tok = dir.register(c_id.clone()).await;

        let count = dir.cancel_all_named("dom_a", "same_name").await;
        assert_eq!(count, 2);
        assert!(a_tok.is_cancelled());
        assert!(b_tok.is_cancelled());
        assert!(!c_tok.is_cancelled());
    }

    #[tokio::test]
    async fn spawn_tracked_removes_entry_on_completion() {
        let dir = TaskDirectory::new();
        let id = dir.next_task_id("test", "quick").await;
        let handle = spawn_tracked(dir.clone(), id.clone(), |_token| async move { 42 });
        let result = handle.await.unwrap();
        assert_eq!(result, Some(42));
        // 等待条目从目录移除
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(dir.snapshot().await.is_empty());
    }

    #[tokio::test]
    async fn cancel_all_cancels_every_task() {
        let dir = TaskDirectory::new();
        let ids: Vec<TaskId> = {
            let mut v = Vec::new();
            for _ in 0..3 {
                v.push(dir.next_task_id("test", "worker").await);
            }
            v
        };
        let tokens: Vec<CancellationToken> = {
            let mut tokens = Vec::new();
            for id in &ids {
                tokens.push(dir.register(id.clone()).await);
            }
            tokens
        };
        dir.cancel_all().await;
        for token in &tokens {
            assert!(token.is_cancelled());
        }
    }
}
