//! 播放 transition actor。
//!
//! 该模块为 play / seek / next / previous 提供统一 inbox。actor 负责创建播放请求、
//! 激活播放加载 gate、记录队列等待与执行耗时；具体播放启动流程仍由 `AppState` 的播放
//! helper 执行，并继续依赖 request/session/stop flag 完成 supersede 与取消保护。

use crate::app_state::AppState;
use crate::command_scheduling::{self, CommandDomain};
use crate::logging::{LogCenter, LogLevel, LogPayload};
use crate::playback_load_gate::PlaybackLoadTicket;
use crate::player::{PlaybackError, PlaybackErrorCode};
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

type PlaybackActorJob = Box<
    dyn FnOnce(AppState, u64) -> Pin<Box<dyn Future<Output = PlaybackActorJobOutcome> + Send>>
        + Send
        + 'static,
>;

#[derive(Debug, Default)]
struct PlaybackActorJobOutcome {
    ticket_superseded: bool,
}

struct PlaybackActorMessage {
    state: AppState,
    command_name: &'static str,
    request_id: u64,
    load_ticket: PlaybackLoadTicket,
    submitted_at: Instant,
    job: PlaybackActorJob,
}

/// 播放 transition actor 的投递句柄。
#[derive(Clone)]
pub(crate) struct PlaybackActor {
    sender: mpsc::UnboundedSender<PlaybackActorMessage>,
}

/// 播放 transition actor 的接收端。
///
/// 该类型只在 `AppState::new` 中用于完成 actor 启动，启动后不再暴露给其他模块。
pub(crate) struct PlaybackActorInbox {
    receiver: mpsc::UnboundedReceiver<PlaybackActorMessage>,
}

impl PlaybackActor {
    /// 创建 actor 投递句柄与待启动 inbox。
    pub(crate) fn new() -> (Self, PlaybackActorInbox) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self { sender }, PlaybackActorInbox { receiver })
    }

    /// 向 actor 提交需要结果回传的播放 transition。
    pub(crate) async fn dispatch<F, Fut, T>(
        &self,
        state: AppState,
        command_name: &'static str,
        task: F,
    ) -> Result<T, PlaybackError>
    where
        F: FnOnce(AppState, u64) -> Fut + Send + 'static,
        Fut: Future<Output = Result<T, PlaybackError>> + Send + 'static,
        T: Send + 'static,
    {
        let (result_tx, result_rx) = oneshot::channel::<Result<T, PlaybackError>>();
        let job: PlaybackActorJob = Box::new(move |state, request_id| {
            Box::pin(async move {
                let result = task(state, request_id).await;
                let ticket_superseded = result
                    .as_ref()
                    .err()
                    .is_some_and(|error| error.code == PlaybackErrorCode::Superseded);
                let _ = result_tx.send(result);
                PlaybackActorJobOutcome { ticket_superseded }
            })
        });

        self.send(state, command_name, job)?;
        result_rx
            .await
            .map_err(|_| playback_actor_result_dropped_error(command_name))?
    }

    /// 向 actor 提交无需结果回传的播放 transition。
    pub(crate) fn spawn<F, Fut>(
        &self,
        state: AppState,
        command_name: &'static str,
        task: F,
    ) -> Result<(), PlaybackError>
    where
        F: FnOnce(AppState, u64) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let job: PlaybackActorJob = Box::new(move |state, request_id| {
            Box::pin(async move {
                task(state, request_id).await;
                PlaybackActorJobOutcome::default()
            })
        });
        self.send(state, command_name, job)
    }

    fn send(
        &self,
        state: AppState,
        command_name: &'static str,
        job: PlaybackActorJob,
    ) -> Result<(), PlaybackError> {
        command_scheduling::debug_assert_command_domain(
            command_name,
            CommandDomain::PlaybackTransition,
        );
        let request_id = state.begin_playback_transition(command_name);
        let load_ticket = state.playback_load_gate.enter();
        self.sender
            .send(PlaybackActorMessage {
                state,
                command_name,
                request_id,
                load_ticket,
                submitted_at: Instant::now(),
                job,
            })
            .map_err(|_| playback_actor_closed_error(command_name))
    }
}

/// 在播放 runtime 上启动 actor loop。
pub(crate) fn start_playback_actor(
    runtime: Arc<tokio::runtime::Runtime>,
    inbox: PlaybackActorInbox,
) {
    runtime.spawn(run_playback_actor(inbox.receiver));
}

async fn run_playback_actor(mut receiver: mpsc::UnboundedReceiver<PlaybackActorMessage>) {
    while let Some(message) = receiver.recv().await {
        let queue_depth = receiver.len();
        let queue_wait_ms = elapsed_ms(message.submitted_at);
        let request_id = message.request_id;
        let load_ticket = message.load_ticket;
        let run_state = message.state;
        let log_center = Arc::clone(&run_state.log_center);
        let command_name = message.command_name;
        let job = message.job;

        let run_started_at = Instant::now();
        let outcome = job(run_state, request_id).await;
        drop(load_ticket);
        record_playback_transition_metrics(
            log_center,
            command_name,
            request_id,
            queue_depth,
            queue_wait_ms,
            elapsed_ms(run_started_at),
            outcome.ticket_superseded,
        );
    }
}

fn record_playback_transition_metrics(
    log_center: Arc<LogCenter>,
    command_name: &'static str,
    request_id: u64,
    queue_depth: usize,
    queue_wait_ms: u128,
    run_ms: u128,
    ticket_superseded: bool,
) {
    let spec = command_scheduling::command_spec(command_name);
    let domain = spec.map(|spec| spec.domain.as_label()).unwrap_or("unknown");
    let priority = spec
        .map(|spec| spec.priority.as_label())
        .unwrap_or("unknown");

    log_center.record(
        LogPayload::new(
            LogLevel::Debug,
            "playback",
            "playback.transition_completed",
            "Playback transition command completed",
        )
        .context(json!({
            "command.name": command_name,
            "command.domain": domain,
            "command.priority": priority,
            "command.queue_wait_ms": queue_wait_ms,
            "command.run_ms": run_ms,
            "command.queue_depth": queue_depth,
            "playback.request_id": request_id,
            "playback.ticket_superseded": ticket_superseded,
        })),
    );
}

fn playback_actor_closed_error(command_name: &'static str) -> PlaybackError {
    PlaybackError::new(
        PlaybackErrorCode::Internal,
        format!("playback actor is closed while scheduling {command_name}"),
        false,
        None,
    )
}

fn playback_actor_result_dropped_error(command_name: &'static str) -> PlaybackError {
    PlaybackError::new(
        PlaybackErrorCode::Internal,
        format!("playback actor task finished without returning a result for {command_name}"),
        false,
        None,
    )
}

fn elapsed_ms(started_at: Instant) -> u128 {
    started_at.elapsed().as_millis()
}
