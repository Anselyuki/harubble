//! 播放启动资源退让门。
//!
//! 该 gate 不是互斥锁，而是跨 command domain 的轻量信号：播放启动、切歌或 seek
//! 重建会话期间，视觉辅助与后台 I/O 可以主动退让，避免与音频下载、probe、初始缓冲争抢资源。

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::watch;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GateSnapshot {
    active: bool,
}

#[derive(Debug)]
struct GateState {
    active_tickets: usize,
}

#[derive(Debug)]
struct GateInner {
    state: Mutex<GateState>,
    changes: watch::Sender<GateSnapshot>,
}

/// 播放启动退让门。
#[derive(Debug, Clone)]
pub(crate) struct PlaybackLoadGate {
    inner: Arc<GateInner>,
}

/// 一次播放启动窗口的 ticket。
///
/// 多个并发启动窗口会分别计数；只有最后一个 ticket Drop 后 gate 才会重新变为空闲。
#[derive(Debug)]
pub(crate) struct PlaybackLoadTicket {
    gate: PlaybackLoadGate,
}

impl PlaybackLoadGate {
    pub(crate) fn new() -> Self {
        let snapshot = GateSnapshot { active: false };
        let (changes, _) = watch::channel(snapshot);
        Self {
            inner: Arc::new(GateInner {
                state: Mutex::new(GateState { active_tickets: 0 }),
                changes,
            }),
        }
    }

    pub(crate) fn enter(&self) -> PlaybackLoadTicket {
        let mut state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
        let was_inactive = state.active_tickets == 0;
        state.active_tickets = state
            .active_tickets
            .checked_add(1)
            .expect("playback load gate ticket count overflow");
        if was_inactive {
            self.inner
                .changes
                .send_replace(GateSnapshot { active: true });
        }
        drop(state);
        PlaybackLoadTicket { gate: self.clone() }
    }

    pub(crate) fn is_active(&self) -> bool {
        self.inner
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .active_tickets
            > 0
    }

    /// 仅在当前没有播放启动任务时执行操作。
    ///
    /// 操作执行期间会保留 gate 状态锁，使新的启动 ticket 无法与清理操作交错；
    /// 调用方应只在控制路径使用，并避免在闭包中再次访问该 gate。
    pub(crate) fn run_if_inactive<T>(&self, action: impl FnOnce() -> T) -> Option<T> {
        let state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.active_tickets > 0 {
            return None;
        }
        let result = action();
        drop(state);
        Some(result)
    }

    pub(crate) async fn wait_until_inactive(&self) {
        let mut receiver = self.inner.changes.subscribe();
        loop {
            if !receiver.borrow().active {
                return;
            }
            if receiver.changed().await.is_err() {
                return;
            }
        }
    }

    pub(crate) async fn wait_until_inactive_with_settle(&self, settle_delay: Duration) {
        let mut saw_active = false;

        loop {
            if self.is_active() {
                saw_active = true;
                self.wait_until_inactive().await;
                continue;
            }

            if !saw_active || settle_delay.is_zero() {
                return;
            }

            tokio::time::sleep(settle_delay).await;
            if !self.is_active() {
                return;
            }
        }
    }

    fn release(&self) {
        let mut state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
        debug_assert!(state.active_tickets > 0, "playback load gate underflow");
        state.active_tickets = state.active_tickets.saturating_sub(1);
        if state.active_tickets == 0 {
            self.inner
                .changes
                .send_replace(GateSnapshot { active: false });
        }
    }
}

impl Drop for PlaybackLoadTicket {
    fn drop(&mut self) {
        self.gate.release();
    }
}

#[cfg(test)]
mod tests {
    use super::PlaybackLoadGate;
    use std::time::Duration;

    #[tokio::test]
    async fn old_ticket_cannot_release_newer_loading_window() {
        let gate = PlaybackLoadGate::new();
        let first = gate.enter();
        let second = gate.enter();

        drop(first);
        assert!(
            gate.is_active(),
            "old playback ticket must not open the newer loading gate"
        );

        drop(second);
        assert!(!gate.is_active());
    }

    #[test]
    fn newer_ticket_cannot_release_older_loading_window() {
        let gate = PlaybackLoadGate::new();
        let first = gate.enter();
        let second = gate.enter();

        drop(second);
        assert!(
            gate.is_active(),
            "newer playback ticket must not hide an older pending load"
        );

        drop(first);
        assert!(!gate.is_active());
    }

    #[tokio::test]
    async fn wait_until_inactive_unblocks_when_current_ticket_drops() {
        let gate = PlaybackLoadGate::new();
        let ticket = gate.enter();
        let waiter_gate = gate.clone();
        let waiter = tokio::spawn(async move {
            waiter_gate
                .wait_until_inactive_with_settle(Duration::ZERO)
                .await;
        });

        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());

        drop(ticket);
        waiter.await.expect("waiter should finish");
    }

    #[tokio::test]
    async fn wait_until_inactive_waits_for_all_overlapping_tickets() {
        let gate = PlaybackLoadGate::new();
        let first = gate.enter();
        let second = gate.enter();
        let waiter_gate = gate.clone();
        let waiter = tokio::spawn(async move {
            waiter_gate.wait_until_inactive().await;
        });

        tokio::task::yield_now().await;
        drop(second);
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());

        drop(first);
        waiter.await.expect("waiter should finish");
    }

    #[test]
    fn inactive_action_is_deferred_until_current_ticket_drops() {
        let gate = PlaybackLoadGate::new();
        let ticket = gate.enter();

        assert_eq!(gate.run_if_inactive(|| 1), None);

        drop(ticket);
        assert_eq!(gate.run_if_inactive(|| 2), Some(2));
    }
}
