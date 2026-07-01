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
    ticket: u64,
}

#[derive(Debug)]
struct GateState {
    active: bool,
    ticket: u64,
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
/// Drop 时只会释放自己创建的那一代 gate；如果已经有更新的播放启动进入，旧 ticket
/// 的释放会被忽略，避免旧任务错误地放开新任务的退让门。
#[derive(Debug)]
pub(crate) struct PlaybackLoadTicket {
    gate: PlaybackLoadGate,
    ticket: u64,
}

impl PlaybackLoadGate {
    pub(crate) fn new() -> Self {
        let snapshot = GateSnapshot {
            active: false,
            ticket: 0,
        };
        let (changes, _) = watch::channel(snapshot);
        Self {
            inner: Arc::new(GateInner {
                state: Mutex::new(GateState {
                    active: false,
                    ticket: 0,
                }),
                changes,
            }),
        }
    }

    pub(crate) fn enter(&self) -> PlaybackLoadTicket {
        let ticket = {
            let mut state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
            state.ticket = state.ticket.saturating_add(1);
            state.active = true;
            state.ticket
        };
        self.publish(GateSnapshot {
            active: true,
            ticket,
        });
        PlaybackLoadTicket {
            gate: self.clone(),
            ticket,
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        self.inner
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .active
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

    fn release(&self, ticket: u64) {
        let should_publish = {
            let mut state = self.inner.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.ticket == ticket && state.active {
                state.active = false;
                true
            } else {
                false
            }
        };

        if should_publish {
            self.publish(GateSnapshot {
                active: false,
                ticket,
            });
        }
    }

    fn publish(&self, snapshot: GateSnapshot) {
        self.inner.changes.send_replace(snapshot);
    }
}

impl Drop for PlaybackLoadTicket {
    fn drop(&mut self) {
        self.gate.release(self.ticket);
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
}
