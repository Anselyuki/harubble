//! 下载批次状态管理服务。
//!
//! 该模块负责创建下载批次、维护任务队列、处理取消与重试、生成管理器快照，并为
//! 执行桥接层提供稳定的下载状态更新与恢复入口。

use crate::api::ApiClient;
use crate::download::error::DownloadServiceError;
use crate::download::model::{
    CreateDownloadJobRequest, DownloadErrorCode, DownloadErrorInfo, DownloadJob, DownloadJobKind,
    DownloadJobSnapshot, DownloadJobStatus, DownloadManagerSnapshot, DownloadTaskSnapshot,
    DownloadTaskStatus, InternalDownloadTask,
};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

/// 为下载批次与子任务生成进程内唯一 ID 的简单生成器。
#[derive(Default)]
pub struct IdGenerator {
    counter: AtomicU64,
}

impl IdGenerator {
    /// 生成新的下载批次 ID。
    pub fn next_job_id(&self) -> String {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        format!("job-{}-{n}", unix_millis())
    }

    /// 生成新的下载任务 ID。
    pub fn next_task_id(&self) -> String {
        let n = self.counter.fetch_add(1, Ordering::Relaxed);
        format!("task-{}-{n}", unix_millis())
    }
}

fn unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn iso_timestamp_now() -> String {
    OffsetDateTime::now_utc()
        .format(&Iso8601::DEFAULT)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn selection_job_title(
    song_count: usize,
    album_count: usize,
    first_song_name: Option<&str>,
    first_album_name: Option<&str>,
) -> String {
    let album_context = first_album_name
        .filter(|name| !name.is_empty())
        .map(|name| format!("{name} · 已选 {song_count} 首"));

    if song_count == 1 {
        return first_song_name
            .filter(|name| !name.is_empty())
            .map(|name| name.to_string())
            .or(album_context)
            .unwrap_or_else(|| "已选 1 首".to_string());
    }

    if album_count <= 1 {
        return album_context.unwrap_or_else(|| format!("已选 {song_count} 首"));
    }

    format!("已选 {song_count} 首 · {album_count} 张专辑")
}

#[derive(Default)]
pub struct DownloadServiceState {
    jobs: Vec<DownloadJob>,
    active_job_id: Option<String>,
    active_task_id: Option<String>,
    active_task_cancel_flag: Option<Arc<AtomicBool>>,
}

impl DownloadServiceState {
    fn snapshot(&self) -> DownloadManagerSnapshot {
        let queued_job_ids = self
            .jobs
            .iter()
            .filter(|job| job.status == DownloadJobStatus::Queued)
            .map(|job| job.id.clone())
            .collect();

        DownloadManagerSnapshot {
            jobs: self.jobs.iter().map(DownloadJob::to_snapshot).collect(),
            active_job_id: self.active_job_id.clone(),
            queued_job_ids,
        }
    }
}

/// 一次任务状态更新后的批次回写结果。
///
/// 适用于执行桥接层在更新任务状态后，同时决定是否需要立即持久化管理器快照。
pub struct TaskStateUpdate {
    /// 更新后的批次快照。
    pub snapshot: DownloadJobSnapshot,
    /// 当前变更是否值得立即持久化到磁盘。
    pub should_persist: bool,
}

/// 下载批次管理服务。
///
/// 负责创建批次、维护任务队列、处理取消/重试与生成前端可消费的管理器快照；
/// 不直接执行下载 I/O，而是为桥接层和执行器提供状态管理能力。
#[derive(Default)]
pub struct DownloadService {
    state: DownloadServiceState,
    id_generator: IdGenerator,
}

impl DownloadService {
    /// 创建一个空的下载服务实例。
    ///
    /// 适用于应用冷启动或测试初始化；返回值中不包含任何已恢复批次。
    pub fn new() -> Self {
        Self::default()
    }

    /// 从已持久化的管理器快照恢复下载服务状态。
    ///
    /// 适用于应用重启后的下载恢复；返回值会重建内部任务实体，并根据快照重新推导
    /// 批次状态，但不会自动恢复正在执行中的活动任务标记。
    pub fn from_manager_snapshot(snapshot: DownloadManagerSnapshot) -> Self {
        Self {
            state: DownloadServiceState {
                jobs: snapshot.jobs.into_iter().map(restore_job).collect(),
                active_job_id: None,
                active_task_id: None,
                active_task_cancel_flag: None,
            },
            id_generator: IdGenerator::default(),
        }
    }

    /// 获取当前下载管理器快照。
    ///
    /// 适用于前端全量同步、持久化落盘或测试断言；返回值为当前内存状态的只读拷贝。
    pub fn snapshot(&self) -> DownloadManagerSnapshot {
        self.state.snapshot()
    }

    /// 根据批次 ID 获取单个批次快照。
    ///
    /// 入参 `job_id` 为目标批次标识；若批次不存在则返回 `None`。
    pub fn get_job(&self, job_id: &str) -> Option<DownloadJobSnapshot> {
        self.state
            .jobs
            .iter()
            .find(|job| job.id == job_id)
            .map(DownloadJob::to_snapshot)
    }

    /// 根据请求构建一个新的下载批次并加入队列。
    ///
    /// 入参 `request` 描述批次类型、目标歌曲与下载选项；该方法会查询上游 API 补齐
    /// 任务标题与专辑上下文，成功时返回已入队的批次快照。
    pub async fn create_job(
        &mut self,
        api: &ApiClient,
        request: CreateDownloadJobRequest,
    ) -> Result<DownloadJobSnapshot, DownloadServiceError> {
        validate_request(&request)?;

        let job_id = self.id_generator.next_job_id();
        let (title, tasks) = self.build_job_tasks(api, &job_id, &request).await?;

        // Acquire the lock only for writing the job — brief, no await inside.
        let job = DownloadJob {
            id: job_id,
            kind: request.kind,
            status: DownloadJobStatus::Queued,
            created_at: iso_timestamp_now(),
            started_at: None,
            finished_at: None,
            options: request.options,
            title,
            tasks,
            error: None,
        };

        let snapshot = job.to_snapshot();
        self.state.jobs.push(job);
        Ok(snapshot)
    }

    async fn build_job_tasks(
        &self,
        api: &ApiClient,
        job_id: &str,
        request: &CreateDownloadJobRequest,
    ) -> Result<(String, Vec<InternalDownloadTask>), DownloadServiceError> {
        match request.kind {
            DownloadJobKind::Song => self.build_song_job_tasks(api, job_id, request).await,
            DownloadJobKind::Album => self.build_album_job_tasks(api, job_id, request).await,
            DownloadJobKind::Selection => {
                self.build_selection_job_tasks(api, job_id, request).await
            }
        }
    }

    async fn build_song_job_tasks(
        &self,
        api: &ApiClient,
        job_id: &str,
        request: &CreateDownloadJobRequest,
    ) -> Result<(String, Vec<InternalDownloadTask>), DownloadServiceError> {
        let song_cid = request.song_cids.first().ok_or_else(|| {
            DownloadServiceError::new("invalidRequest", "song job requires one song cid")
        })?;
        let song = api.get_song_detail(song_cid).await.map_err(api_error)?;
        let album = api
            .get_album_detail(&song.album_cid)
            .await
            .map_err(api_error)?;
        let tasks = vec![make_task(
            &self.id_generator,
            job_id,
            &song.cid,
            &song.name,
            &song.artists,
            &song.album_cid,
            &album.name,
            0,
            1,
            request.options.format,
            request.options.download_lyrics,
        )];

        Ok((song.name, tasks))
    }

    async fn build_album_job_tasks(
        &self,
        api: &ApiClient,
        job_id: &str,
        request: &CreateDownloadJobRequest,
    ) -> Result<(String, Vec<InternalDownloadTask>), DownloadServiceError> {
        let album_cid = request.album_cid.as_ref().ok_or_else(|| {
            DownloadServiceError::new("invalidRequest", "album job requires albumCid")
        })?;
        let album = api.get_album_detail(album_cid).await.map_err(api_error)?;
        let song_count = album.songs.len();
        let tasks = album
            .songs
            .iter()
            .enumerate()
            .map(|(index, song)| {
                make_task(
                    &self.id_generator,
                    job_id,
                    &song.cid,
                    &song.name,
                    &song.artists,
                    album_cid,
                    &album.name,
                    index,
                    song_count,
                    request.options.format,
                    request.options.download_lyrics,
                )
            })
            .collect();

        Ok((album.name, tasks))
    }

    async fn build_selection_job_tasks(
        &self,
        api: &ApiClient,
        job_id: &str,
        request: &CreateDownloadJobRequest,
    ) -> Result<(String, Vec<InternalDownloadTask>), DownloadServiceError> {
        let mut album_names = HashMap::<String, String>::new();
        let mut resolved = Vec::with_capacity(request.song_cids.len());

        for song_cid in &request.song_cids {
            let song = api.get_song_detail(song_cid).await.map_err(api_error)?;
            let album_name = if let Some(name) = album_names.get(&song.album_cid) {
                name.clone()
            } else {
                let album = api
                    .get_album_detail(&song.album_cid)
                    .await
                    .map_err(api_error)?;
                let name = album.name.clone();
                album_names.insert(song.album_cid.clone(), name.clone());
                name
            };
            resolved.push((song, album_name));
        }

        let song_count = resolved.len();
        let album_count = resolved
            .iter()
            .map(|(song, _)| song.album_cid.as_str())
            .collect::<HashSet<_>>()
            .len();
        let title = selection_job_title(
            song_count,
            album_count,
            resolved.first().map(|(song, _)| song.name.as_str()),
            resolved.first().map(|(_, album_name)| album_name.as_str()),
        );
        let tasks = resolved
            .into_iter()
            .enumerate()
            .map(|(index, (song, album_name))| {
                make_task(
                    &self.id_generator,
                    job_id,
                    &song.cid,
                    &song.name,
                    &song.artists,
                    &song.album_cid,
                    &album_name,
                    index,
                    song_count,
                    request.options.format,
                    request.options.download_lyrics,
                )
            })
            .collect();

        Ok((title, tasks))
    }

    /// 取消整个下载批次。
    ///
    /// 如果该批次正在执行，会同时触发当前活跃任务的取消信号；返回值为取消后的批次
    /// 快照。若批次不存在则返回 `None`。
    pub fn cancel_job(&mut self, job_id: &str) -> Option<DownloadJobSnapshot> {
        let is_active_job = self.state.active_job_id.as_deref() == Some(job_id);
        if is_active_job {
            self.cancel_active_task_execution();
        }
        let job = self.state.jobs.iter_mut().find(|job| job.id == job_id)?;
        for task in &mut job.tasks {
            cancel_task_if_active(task);
        }
        job.status = DownloadJobStatus::Cancelled;
        job.finished_at = if is_active_job {
            None
        } else {
            Some(iso_timestamp_now())
        };
        Some(job.to_snapshot())
    }

    /// 取消指定批次中的单个子任务。
    pub fn cancel_task(&mut self, job_id: &str, task_id: &str) -> Option<DownloadJobSnapshot> {
        let is_active_job = self.state.active_job_id.as_deref() == Some(job_id);
        let is_active_task = self.state.active_task_id.as_deref() == Some(task_id);
        if is_active_job && is_active_task {
            self.cancel_active_task_execution();
        }
        let job = self.state.jobs.iter_mut().find(|job| job.id == job_id)?;
        let task = job.tasks.iter_mut().find(|task| task.id == task_id)?;
        cancel_task_if_active(task);

        if !is_active_job && !job.tasks.iter().any(is_task_pending) {
            job.status = job.job_status();
            job.finished_at = Some(iso_timestamp_now());
        }

        Some(job.to_snapshot())
    }

    /// 重试整个下载批次中可重试的子任务。
    pub fn retry_job(&mut self, job_id: &str) -> Option<DownloadJobSnapshot> {
        let is_active_job = self.state.active_job_id.as_deref() == Some(job_id);
        if is_active_job {
            return self
                .state
                .jobs
                .iter()
                .find(|job| job.id == job_id)
                .map(DownloadJob::to_snapshot);
        }

        let job = self.state.jobs.iter_mut().find(|job| job.id == job_id)?;
        for task in &mut job.tasks {
            reset_task_if_retryable(task);
        }
        job.status = DownloadJobStatus::Queued;
        job.finished_at = None;
        job.error = None;

        Some(job.to_snapshot())
    }

    /// 重试单个子任务；若批次已处于终态，会一并重新排队该批次。
    pub fn retry_task(&mut self, job_id: &str, task_id: &str) -> Option<DownloadJobSnapshot> {
        let is_active_task = self.state.active_task_id.as_deref() == Some(task_id)
            && self.state.active_job_id.as_deref() == Some(job_id);
        if is_active_task {
            return self
                .state
                .jobs
                .iter()
                .find(|job| job.id == job_id)
                .map(DownloadJob::to_snapshot);
        }

        let job = self.state.jobs.iter_mut().find(|job| job.id == job_id)?;
        let task = job.tasks.iter_mut().find(|task| task.id == task_id)?;
        reset_task_if_retryable(task);
        if matches!(
            job.status,
            DownloadJobStatus::Failed
                | DownloadJobStatus::Cancelled
                | DownloadJobStatus::PartiallyFailed
        ) {
            job.status = DownloadJobStatus::Queued;
            job.finished_at = None;
            job.error = None;
        }

        Some(job.to_snapshot())
    }

    /// 清理已结束的历史批次，返回被移除的数量。
    pub fn clear_history(&mut self) -> usize {
        let before = self.state.jobs.len();
        self.state.jobs.retain(|job| {
            !matches!(
                job.status,
                DownloadJobStatus::Completed
                    | DownloadJobStatus::Failed
                    | DownloadJobStatus::Cancelled
                    | DownloadJobStatus::PartiallyFailed
            )
        });
        before - self.state.jobs.len()
    }

    // -------------------------------------------------------------------------
    // Execution state management (used by the bridge to drive the execution
    // loop)
    // -------------------------------------------------------------------------

    /// 原子地领取下一个排队中的批次并将其标记为运行中。
    ///
    /// 适用于执行桥接循环准备启动新批次时调用；若当前没有可启动批次，返回 `None`。
    /// 返回值为已切换到运行态后的批次快照。
    pub fn start_next_queued_job(&mut self) -> Option<DownloadJobSnapshot> {
        let job = self.state.jobs.iter_mut().find(|job| {
            job.status == DownloadJobStatus::Queued && self.state.active_job_id.is_none()
        })?;

        job.status = DownloadJobStatus::Running;
        job.started_at = Some(iso_timestamp_now());
        self.state.active_job_id = Some(job.id.clone());
        Some(job.to_snapshot())
    }

    /// 从指定批次中取出下一个排队任务，并立即标记为准备中。
    ///
    /// 适用于工作线程准备执行单个任务前领取任务；返回值同时包含任务副本与更新后的
    /// 批次快照，便于调用方同步广播状态。
    pub fn pop_next_task(
        &mut self,
        job_id: &str,
    ) -> Option<(InternalDownloadTask, DownloadJobSnapshot)> {
        let job = self.state.jobs.iter_mut().find(|job| job.id == job_id)?;
        let task = job
            .tasks
            .iter_mut()
            .find(|task| task.status == DownloadTaskStatus::Queued)?;
        task.status = DownloadTaskStatus::Preparing;
        self.state.active_task_id = Some(task.id.clone());
        self.state.active_task_cancel_flag = Some(Arc::new(AtomicBool::new(false)));
        Some((task.clone(), job.to_snapshot()))
    }

    /// 更新指定批次内某个任务的状态与附带产物信息。
    ///
    /// 调用方可按需传入进度、输出路径或错误信息；对于已经进入终态的任务，若尝试
    /// 切换到其他状态会返回 `None`，以避免状态回退破坏不变量。
    #[allow(clippy::too_many_arguments)]
    pub fn update_task_state(
        &mut self,
        job_id: &str,
        task_id: &str,
        status: DownloadTaskStatus,
        bytes_done: Option<u64>,
        bytes_total: Option<u64>,
        output_path: Option<&str>,
        error: Option<DownloadErrorInfo>,
    ) -> Option<TaskStateUpdate> {
        let job = self.state.jobs.iter_mut().find(|job| job.id == job_id)?;
        let task = job.tasks.iter_mut().find(|task| task.id == task_id)?;

        if is_terminal_task_status(task.status) && status != task.status {
            return None;
        }
        if task.status == DownloadTaskStatus::Cancelled && status != DownloadTaskStatus::Cancelled {
            return None;
        }
        if job.status == DownloadJobStatus::Cancelled && status != DownloadTaskStatus::Cancelled {
            return None;
        }

        let previous_status = task.status;
        let previous_output_path = task.output_path.clone();
        let had_error = task.error.is_some();

        if let Some(done) = bytes_done {
            task.bytes_done = done;
        }
        if bytes_total.is_some() {
            task.bytes_total = bytes_total;
        }
        if output_path.is_some() {
            task.output_path = output_path.map(String::from);
        }
        if error.is_some() {
            task.error = error;
        }
        task.status = status;

        let should_persist = task.status != previous_status
            || task.output_path != previous_output_path
            || (!had_error && task.error.is_some())
            || is_terminal_task_status(task.status);

        Some(TaskStateUpdate {
            snapshot: job.to_snapshot(),
            should_persist,
        })
    }

    /// 在当前批次所有任务处理完成后收尾，并推导最终批次状态。
    ///
    /// 若用户此前已显式取消批次，会保留取消态而不是被任务聚合结果覆盖；返回值为
    /// 收尾后的最终批次快照。
    pub fn finish_job(&mut self, job_id: &str) -> Option<DownloadJobSnapshot> {
        let job = self.state.jobs.iter_mut().find(|job| job.id == job_id)?;

        // Respect explicit user cancellation when the queue drains after an active stop.
        let derived_status = job.job_status();
        job.status = if job.status == DownloadJobStatus::Cancelled {
            DownloadJobStatus::Cancelled
        } else {
            derived_status
        };
        if is_terminal_job_status(job.status) {
            job.finished_at = Some(iso_timestamp_now());
        }
        self.state.active_job_id = None;
        self.state.active_task_id = None;
        self.state.active_task_cancel_flag = None;

        Some(job.to_snapshot())
    }

    /// 返回当前活动任务对应的取消标志。
    ///
    /// 只有当传入的批次 ID 与任务 ID 正好命中当前执行中的任务时才会返回标志；
    /// 否则返回 `None`，表示当前没有可用于中断执行中的活跃任务的取消标志。
    pub fn active_task_cancel_flag(&self, job_id: &str, task_id: &str) -> Option<Arc<AtomicBool>> {
        if self.state.active_job_id.as_deref() != Some(job_id) {
            return None;
        }
        if self.state.active_task_id.as_deref() != Some(task_id) {
            return None;
        }

        self.state.active_task_cancel_flag.clone()
    }

    fn cancel_active_task_execution(&mut self) {
        if let Some(cancel_flag) = &self.state.active_task_cancel_flag {
            cancel_flag.store(true, Ordering::SeqCst);
        }
    }

    /// 返回当前是否仍有等待执行的排队批次。
    pub fn has_queued_jobs(&self) -> bool {
        self.state
            .jobs
            .iter()
            .any(|job| job.status == DownloadJobStatus::Queued)
    }

    /// 返回当前下载管理器的整体快照。
    pub fn manager_snapshot(&self) -> DownloadManagerSnapshot {
        self.state.snapshot()
    }

    /// 返回指定批次当前配置的输出目录。
    pub fn job_output_dir(&self, job_id: &str) -> Option<String> {
        let job = self.state.jobs.iter().find(|job| job.id == job_id)?;
        Some(job.options.output_dir.clone())
    }
}

fn validate_request(request: &CreateDownloadJobRequest) -> Result<(), DownloadServiceError> {
    match request.kind {
        DownloadJobKind::Song if request.song_cids.is_empty() => Err(DownloadServiceError::new(
            "invalidRequest",
            "song job requires one song cid",
        )),
        DownloadJobKind::Album if request.album_cid.is_none() => Err(DownloadServiceError::new(
            "invalidRequest",
            "album job requires albumCid",
        )),
        DownloadJobKind::Selection if request.song_cids.is_empty() => {
            Err(DownloadServiceError::new(
                "invalidRequest",
                "selection job requires at least one song cid",
            ))
        }
        _ => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
fn make_task(
    id_generator: &IdGenerator,
    job_id: &str,
    song_cid: &str,
    song_name: &str,
    artists: &[String],
    album_cid: &str,
    album_name: &str,
    song_index: usize,
    song_count: usize,
    format: crate::audio::OutputFormat,
    download_lyrics: bool,
) -> InternalDownloadTask {
    InternalDownloadTask {
        id: id_generator.next_task_id(),
        job_id: job_id.to_string(),
        song_cid: song_cid.to_string(),
        song_name: song_name.to_string(),
        artists: artists.to_vec(),
        album_cid: album_cid.to_string(),
        album_name: album_name.to_string(),
        status: DownloadTaskStatus::Queued,
        bytes_done: 0,
        bytes_total: None,
        output_path: None,
        error: None,
        attempt: 0,
        song_index,
        song_count,
        format,
        download_lyrics,
    }
}

fn restore_job(snapshot: DownloadJobSnapshot) -> DownloadJob {
    let mut job = DownloadJob {
        id: snapshot.id,
        kind: snapshot.kind,
        status: snapshot.status,
        created_at: snapshot.created_at,
        started_at: snapshot.started_at,
        finished_at: snapshot.finished_at,
        options: snapshot.options,
        title: snapshot.title,
        tasks: Vec::new(),
        error: snapshot.error,
    };

    job.tasks = snapshot
        .tasks
        .into_iter()
        .map(|task| restore_task(task, &job))
        .collect();
    job.status = job.job_status();

    job
}

fn restore_task(snapshot: DownloadTaskSnapshot, job: &DownloadJob) -> InternalDownloadTask {
    InternalDownloadTask {
        id: snapshot.id,
        job_id: snapshot.job_id,
        song_cid: snapshot.song_cid,
        song_name: snapshot.song_name,
        artists: snapshot.artists,
        album_cid: snapshot.album_cid,
        album_name: snapshot.album_name,
        status: snapshot.status,
        bytes_done: snapshot.bytes_done,
        bytes_total: snapshot.bytes_total,
        output_path: snapshot
            .output_path
            .map(|path| restore_output_path(&path, &job.options.output_dir)),
        error: snapshot.error,
        attempt: snapshot.attempt,
        song_index: snapshot.song_index,
        song_count: snapshot.song_count,
        format: job.options.format,
        download_lyrics: job.options.download_lyrics,
    }
}

fn restore_output_path(path: &str, root_output_dir: &str) -> String {
    let restored = Path::new(path);
    if restored.is_absolute() {
        return path.to_string();
    }

    Path::new(root_output_dir)
        .join(restored)
        .to_string_lossy()
        .to_string()
}

fn cancel_task_if_active(task: &mut InternalDownloadTask) {
    if matches!(
        task.status,
        DownloadTaskStatus::Queued
            | DownloadTaskStatus::Preparing
            | DownloadTaskStatus::Downloading
            | DownloadTaskStatus::Writing
    ) {
        task.status = DownloadTaskStatus::Cancelled;
        task.error = Some(DownloadErrorInfo {
            code: DownloadErrorCode::Cancelled,
            message: "Cancelled by user".to_string(),
            retryable: false,
            details: None,
        });
    }
}

fn is_task_pending(task: &InternalDownloadTask) -> bool {
    matches!(
        task.status,
        DownloadTaskStatus::Queued
            | DownloadTaskStatus::Preparing
            | DownloadTaskStatus::Downloading
            | DownloadTaskStatus::Writing
    )
}

fn is_terminal_job_status(status: DownloadJobStatus) -> bool {
    matches!(
        status,
        DownloadJobStatus::Completed
            | DownloadJobStatus::Failed
            | DownloadJobStatus::Cancelled
            | DownloadJobStatus::PartiallyFailed
    )
}

fn is_terminal_task_status(status: DownloadTaskStatus) -> bool {
    matches!(
        status,
        DownloadTaskStatus::Completed | DownloadTaskStatus::Failed | DownloadTaskStatus::Cancelled
    )
}

fn reset_task_if_retryable(task: &mut InternalDownloadTask) {
    if matches!(
        task.status,
        DownloadTaskStatus::Failed | DownloadTaskStatus::Cancelled
    ) {
        task.status = DownloadTaskStatus::Queued;
        task.bytes_done = 0;
        task.bytes_total = None;
        task.output_path = None;
        task.error = None;
        task.attempt += 1;
    }
}

fn api_error(error: anyhow::Error) -> DownloadServiceError {
    DownloadServiceError::new("api", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        iso_timestamp_now, selection_job_title, DownloadService, DownloadServiceState, IdGenerator,
    };
    use crate::audio::OutputFormat;
    use crate::download::model::{
        DownloadErrorCode, DownloadErrorInfo, DownloadJob, DownloadJobKind, DownloadJobStatus,
        DownloadManagerSnapshot, DownloadOptions, DownloadTaskSnapshot, DownloadTaskStatus,
        InternalDownloadTask,
    };
    use std::path::Path;
    use time::format_description::well_known::Iso8601;
    use time::OffsetDateTime;

    fn make_task(id: &str, status: DownloadTaskStatus) -> InternalDownloadTask {
        InternalDownloadTask {
            id: id.to_string(),
            job_id: "job-1".to_string(),
            song_cid: "song-1".to_string(),
            song_name: "Song".to_string(),
            artists: vec!["Artist".to_string()],
            album_cid: "album-1".to_string(),
            album_name: "Album".to_string(),
            status,
            bytes_done: 0,
            bytes_total: None,
            output_path: None,
            error: None,
            attempt: 0,
            song_index: 0,
            song_count: 1,
            format: OutputFormat::Flac,
            download_lyrics: true,
        }
    }

    fn make_job(status: DownloadJobStatus, tasks: Vec<InternalDownloadTask>) -> DownloadJob {
        DownloadJob {
            id: "job-1".to_string(),
            kind: DownloadJobKind::Album,
            status,
            created_at: "2026-04-15T00:00:00Z".to_string(),
            started_at: Some("2026-04-15T00:00:10Z".to_string()),
            finished_at: None,
            options: DownloadOptions {
                output_dir: "/tmp".to_string(),
                format: OutputFormat::Flac,
                download_lyrics: true,
            },
            title: "Album".to_string(),
            tasks,
            error: None,
        }
    }

    fn make_task_snapshot(status: DownloadTaskStatus) -> DownloadTaskSnapshot {
        DownloadTaskSnapshot {
            id: "task-1".to_string(),
            job_id: "job-1".to_string(),
            song_cid: "song-1".to_string(),
            song_name: "Song".to_string(),
            artists: vec!["Artist".to_string()],
            album_cid: "album-1".to_string(),
            album_name: "Album".to_string(),
            status,
            bytes_done: 128,
            bytes_total: Some(512),
            output_path: Some("Album/Song.flac".to_string()),
            error: Some(DownloadErrorInfo {
                code: DownloadErrorCode::Internal,
                message: "persisted".to_string(),
                retryable: true,
                details: None,
            }),
            attempt: 2,
            song_index: 0,
            song_count: 1,
        }
    }

    #[test]
    fn restores_task_runtime_fields_from_job_options() {
        let snapshot = DownloadManagerSnapshot {
            jobs: vec![crate::download::model::DownloadJobSnapshot {
                id: "job-1".to_string(),
                kind: DownloadJobKind::Album,
                status: DownloadJobStatus::Completed,
                created_at: "2026-04-15T00:00:00Z".to_string(),
                started_at: Some("2026-04-15T00:00:10Z".to_string()),
                finished_at: Some("2026-04-15T00:01:00Z".to_string()),
                options: DownloadOptions {
                    output_dir: "/tmp".to_string(),
                    format: OutputFormat::Mp3,
                    download_lyrics: false,
                },
                title: "Album".to_string(),
                task_count: 1,
                completed_task_count: 1,
                failed_task_count: 0,
                cancelled_task_count: 0,
                tasks: vec![make_task_snapshot(DownloadTaskStatus::Completed)],
                error: None,
            }],
            active_job_id: None,
            queued_job_ids: Vec::new(),
        };

        let service = DownloadService::from_manager_snapshot(snapshot);
        let task = &service.state.jobs[0].tasks[0];

        assert!(matches!(task.format, OutputFormat::Mp3));
        assert!(!task.download_lyrics);
        assert_eq!(
            task.output_path.as_ref().map(Path::new),
            Some(Path::new("/tmp/Album/Song.flac"))
        );
    }

    #[test]
    fn restores_relative_output_path_to_internal_absolute_form() {
        let snapshot = DownloadManagerSnapshot {
            jobs: vec![crate::download::model::DownloadJobSnapshot {
                id: "job-1".to_string(),
                kind: DownloadJobKind::Album,
                status: DownloadJobStatus::Completed,
                created_at: "2026-04-15T00:00:00Z".to_string(),
                started_at: Some("2026-04-15T00:00:10Z".to_string()),
                finished_at: Some("2026-04-15T00:01:00Z".to_string()),
                options: DownloadOptions {
                    output_dir: "/tmp/root".to_string(),
                    format: OutputFormat::Flac,
                    download_lyrics: true,
                },
                title: "Album".to_string(),
                task_count: 1,
                completed_task_count: 1,
                failed_task_count: 0,
                cancelled_task_count: 0,
                tasks: vec![make_task_snapshot(DownloadTaskStatus::Completed)],
                error: None,
            }],
            active_job_id: None,
            queued_job_ids: Vec::new(),
        };

        let service = DownloadService::from_manager_snapshot(snapshot);
        let restored = service.manager_snapshot();

        assert_eq!(
            restored.jobs[0].tasks[0].output_path.as_deref(),
            Some("Album/Song.flac")
        );
        assert_eq!(
            service.state.jobs[0].tasks[0]
                .output_path
                .as_ref()
                .map(Path::new),
            Some(Path::new("/tmp/root/Album/Song.flac"))
        );
    }

    #[test]
    fn keeps_cancelled_status_when_finishing_cancelled_job() {
        let mut service = DownloadService {
            state: DownloadServiceState {
                jobs: vec![make_job(
                    DownloadJobStatus::Cancelled,
                    vec![
                        make_task("task-1", DownloadTaskStatus::Failed),
                        make_task("task-2", DownloadTaskStatus::Cancelled),
                    ],
                )],
                active_job_id: Some("job-1".to_string()),
                active_task_id: Some("task-2".to_string()),
                active_task_cancel_flag: None,
            },
            id_generator: IdGenerator::default(),
        };

        let snapshot = service.finish_job("job-1").expect("job should exist");

        assert!(matches!(snapshot.status, DownloadJobStatus::Cancelled));
    }

    #[test]
    fn ignores_retry_for_active_task_until_worker_exits() {
        let mut service = DownloadService {
            state: DownloadServiceState {
                jobs: vec![make_job(
                    DownloadJobStatus::Running,
                    vec![make_task("task-1", DownloadTaskStatus::Cancelled)],
                )],
                active_job_id: Some("job-1".to_string()),
                active_task_id: Some("task-1".to_string()),
                active_task_cancel_flag: None,
            },
            id_generator: IdGenerator::default(),
        };

        let snapshot = service
            .retry_task("job-1", "task-1")
            .expect("job should exist");

        assert!(matches!(
            snapshot.tasks[0].status,
            DownloadTaskStatus::Cancelled
        ));
        assert_eq!(snapshot.tasks[0].attempt, 0);
    }
    #[test]
    fn generates_iso8601_utc_timestamp() {
        let timestamp = iso_timestamp_now();

        let parsed = OffsetDateTime::parse(&timestamp, &Iso8601::DEFAULT)
            .expect("timestamp should be valid ISO-8601");

        assert_eq!(parsed.offset().whole_seconds(), 0);
        assert!(timestamp.ends_with('Z'));
    }

    #[test]
    fn keeps_track_name_for_single_song_selection() {
        assert_eq!(
            selection_job_title(1, 1, Some("夜航星"), Some("前路未明")),
            "夜航星"
        );
    }

    #[test]
    fn adds_album_context_for_single_album_selection() {
        assert_eq!(
            selection_job_title(3, 1, Some("夜航星"), Some("前路未明")),
            "前路未明 · 已选 3 首"
        );
    }

    #[test]
    fn falls_back_to_album_context_when_single_song_name_is_missing() {
        assert_eq!(
            selection_job_title(1, 1, Some(""), Some("前路未明")),
            "前路未明 · 已选 1 首"
        );
    }

    #[test]
    fn shows_album_span_for_cross_album_selection() {
        assert_eq!(
            selection_job_title(5, 2, Some("夜航星"), Some("前路未明")),
            "已选 5 首 · 2 张专辑"
        );
    }
}
