//! 下载任务系统相关的 Tauri command。
//!
//! 当前暴露的接口覆盖下载任务的创建、查询、取消、重试与历史清理，实际
//! 下载执行与进度事件由桥接层驱动。

use crate::app_state::AppState;
use crate::audio_cache;
use crate::downloads::events::{emit_download_job_updated, emit_download_manager_state_changed};
use harubble_core::download::model::{
    CreateDownloadJobRequest, DownloadJobSnapshot, DownloadManagerSnapshot,
};
use harubble_core::prepare_job;
use tauri::{AppHandle, State};

fn emit_download_state(app: &AppHandle, manager_snapshot: &DownloadManagerSnapshot) {
    emit_download_manager_state_changed(app, manager_snapshot);
    for job in &manager_snapshot.jobs {
        emit_download_job_updated(app, job);
    }
}

/// 清空播放器使用的音频缓存，并在清理前停止当前播放。
///
/// 适用于用户手动清理缓存、排查缓存损坏，或需要强制释放本地音频缓存空间的场景。
/// 返回值为本次实际移除的缓存条目数量。
/// 该接口会先终止当前播放会话，再删除缓存文件；如果调用方只想结束播放而不清理缓存，应改用播放控制接口。
#[tauri::command]
pub fn clear_audio_cache(state: State<'_, AppState>) -> Result<u64, String> {
    state.player.stop().map_err(|e| e.to_string())?;
    audio_cache::clear_audio_cache().map_err(|e| e.to_string())
}

/// 清空后端 API 响应缓存。
///
/// 适用于希望强制下次请求重新命中上游接口、排查缓存脏数据，或在调试时手动刷新后端响应缓存的场景。
/// 成功时返回空值。
/// 该接口只影响内存中的 API 响应缓存，不会删除已完成下载的文件，也不会影响下载任务历史。
#[tauri::command]
pub fn clear_response_cache(state: State<'_, AppState>) -> Result<(), String> {
    state.api.clear_response_cache();
    Ok(())
}

/// 重建 HTTP 客户端以适应网络环境变化。
///
/// 当系统代理设置变更、网络接口切换或网络恢复后，已有的 HTTP 客户端可能持有过期的
/// 连接池与代理配置，导致后续请求失败。调用此接口会以当前系统环境重新构建客户端。
/// 成功时返回空值；若重建失败则保留原有客户端不变并返回错误。
#[tauri::command]
pub fn reset_http_client(state: State<'_, AppState>) -> Result<(), String> {
    state.api.reset_http_client().map_err(|e| e.to_string())
}

/// 创建新的下载批次，并按当前偏好覆盖输出目录。
///
/// 适用于用户从专辑页或多选歌曲列表发起新的下载请求。
/// 入参 `request` 描述待下载任务与下载选项；返回值为新建批次的快照，且其输出目录会被当前应用偏好中的下载目录覆盖。
/// 调用方不应依赖 `request.options.output_dir` 生效；若需要改变下载目录，应先更新偏好再调用该接口。
#[tauri::command]
pub async fn create_download_job(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CreateDownloadJobRequest,
) -> Result<DownloadJobSnapshot, String> {
    let api = state.api.clone();
    let preferences = state.preferences();
    let normalized_request = CreateDownloadJobRequest {
        options: harubble_core::download::model::DownloadOptions {
            output_dir: preferences.output_dir,
            ..request.options
        },
        ..request
    };
    let _creation_guard = state.download_job_creation_lock.lock().await;
    let id_generator = {
        let service = state.download_service.lock().await;
        service.id_generator()
    };
    let prepared = prepare_job(&id_generator, &api, normalized_request)
        .await
        .map_err(|e| e.to_string())?;
    let (job_snapshot, manager_snapshot) = {
        let mut service = state.download_service.lock().await;
        let job_snapshot = service.register_prepared_job(prepared);
        let manager_snapshot = service.manager_snapshot();
        (job_snapshot, manager_snapshot)
    };
    state
        .persist_download_snapshot_async(manager_snapshot.clone())
        .await;

    emit_download_job_updated(&app, &job_snapshot);
    emit_download_manager_state_changed(&app, &manager_snapshot);

    Ok(job_snapshot)
}

/// 获取当前下载管理器的完整快照。
///
/// 适用于下载面板初始化、应用重连后恢复状态，或在事件流丢失后做一次整体状态兜底同步。
/// 返回值包含所有批次的当前快照。
/// 该接口返回的是读取瞬间的整体视图；实时更新仍应优先消费下载事件，而不是高频轮询。
#[tauri::command]
pub async fn list_download_jobs(
    state: State<'_, AppState>,
) -> Result<DownloadManagerSnapshot, String> {
    let service = state.download_service.lock().await;
    Ok(service.snapshot())
}

/// 根据批次 ID 获取单个下载批次。
///
/// 适用于详情面板按需查看单个批次、轮询某个特定批次状态，或在已知批次 ID 时执行精确查询。
/// 入参 `job_id` 为下载批次标识；返回值为可选批次快照，不存在时返回 `None`。
/// 调用方应显式处理 `None`，不要把它当作错误；这通常表示批次不存在、已被清理，或 ID 本身无效。
#[tauri::command]
pub async fn get_download_job(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Option<DownloadJobSnapshot>, String> {
    let service = state.download_service.lock().await;
    Ok(service.get_job(&job_id))
}

/// 取消整个下载批次，并在成功时广播最新状态。
///
/// 适用于用户主动终止整个批次，或在离开页面前放弃一组待下载任务的场景。
/// 入参 `job_id` 为目标批次标识；返回值为取消后的批次快照，不存在或无法取消时返回 `None`。
/// 该接口只对当前存在的批次生效；返回 `Some` 时会同步持久化最新状态并广播事件，调用方不应再额外手动触发状态刷新。
#[tauri::command]
pub async fn cancel_download_job(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Option<DownloadJobSnapshot>, String> {
    let (snapshot, manager_snapshot) = {
        let mut service = state.download_service.lock().await;
        let snapshot = service.cancel_job(&job_id);
        let manager_snapshot = service.manager_snapshot();
        (snapshot, manager_snapshot)
    };

    if let Some(job_snapshot) = &snapshot {
        state
            .persist_download_snapshot_async(manager_snapshot.clone())
            .await;
        emit_download_job_updated(&app, job_snapshot);
        emit_download_manager_state_changed(&app, &manager_snapshot);
    }

    Ok(snapshot)
}

/// 取消批次中的单个下载任务，并在成功时广播最新状态。
///
/// 适用于用户只想停止批次中的某一首歌，而保留同批次其他任务继续执行的场景。
/// 入参 `job_id` 与 `task_id` 分别标识所属批次和目标任务；返回值为更新后的批次快照，不存在时返回 `None`。
/// 若任务已结束或标识无效，可能不会产生变化；调用方应基于返回快照重新渲染批次状态，而不是自行推断局部状态。
#[tauri::command]
pub async fn cancel_download_task(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    task_id: String,
) -> Result<Option<DownloadJobSnapshot>, String> {
    let (snapshot, manager_snapshot) = {
        let mut service = state.download_service.lock().await;
        let snapshot = service.cancel_task(&job_id, &task_id);
        let manager_snapshot = service.manager_snapshot();
        (snapshot, manager_snapshot)
    };

    if let Some(job_snapshot) = &snapshot {
        state
            .persist_download_snapshot_async(manager_snapshot.clone())
            .await;
        emit_download_job_updated(&app, job_snapshot);
        emit_download_manager_state_changed(&app, &manager_snapshot);
    }

    Ok(snapshot)
}

/// 重试整个下载批次中可重试的任务。
///
/// 适用于批次中出现失败任务后，用户希望按当前配置重新拉起该批次的可重试部分。
/// 入参 `job_id` 为目标批次标识；返回值为更新后的批次快照，不存在或无可重试任务时返回 `None`。
/// 该接口不会强制重跑所有任务，只会重置服务判定为可重试的项目；调用方应基于返回快照判断实际重试范围。
#[tauri::command]
pub async fn retry_download_job(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Option<DownloadJobSnapshot>, String> {
    let (snapshot, manager_snapshot) = {
        let mut service = state.download_service.lock().await;
        let snapshot = service.retry_job(&job_id);
        let manager_snapshot = service.manager_snapshot();
        (snapshot, manager_snapshot)
    };

    if let Some(job_snapshot) = &snapshot {
        state
            .persist_download_snapshot_async(manager_snapshot.clone())
            .await;
        emit_download_job_updated(&app, job_snapshot);
        emit_download_manager_state_changed(&app, &manager_snapshot);
    }

    Ok(snapshot)
}

/// 重试批次中的单个下载任务。
///
/// 适用于只对单首失败歌曲执行定点重试，而不影响同批次其他任务的场景。
/// 入参 `job_id` 与 `task_id` 分别标识所属批次和目标任务；返回值为更新后的批次快照，不存在或不可重试时返回 `None`。
/// 该接口依赖任务处于允许重试的终态；调用方应把它当作有条件操作，而不是保证成功生效的强制指令。
#[tauri::command]
pub async fn retry_download_task(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    task_id: String,
) -> Result<Option<DownloadJobSnapshot>, String> {
    let (snapshot, manager_snapshot) = {
        let mut service = state.download_service.lock().await;
        let snapshot = service.retry_task(&job_id, &task_id);
        let manager_snapshot = service.manager_snapshot();
        (snapshot, manager_snapshot)
    };

    if let Some(job_snapshot) = &snapshot {
        state
            .persist_download_snapshot_async(manager_snapshot.clone())
            .await;
        emit_download_job_updated(&app, job_snapshot);
        emit_download_manager_state_changed(&app, &manager_snapshot);
    }

    Ok(snapshot)
}

/// 清理所有已结束的下载历史记录。
///
/// 适用于用户手动清空下载历史面板，或在控制历史体量时进行批量清理。
/// 返回值为本次被移除的历史批次数量。
/// 该接口只会移除已结束的批次，不会取消仍在运行中的任务，也不会删除已经下载到磁盘的文件。
#[tauri::command]
pub async fn clear_download_history(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let (removed_count, manager_snapshot) = {
        let mut service = state.download_service.lock().await;
        let removed_count = service.clear_history();
        let manager_snapshot = service.manager_snapshot();
        (removed_count, manager_snapshot)
    };
    state
        .persist_download_snapshot_async(manager_snapshot.clone())
        .await;

    emit_download_state(&app, &manager_snapshot);

    Ok(removed_count)
}
