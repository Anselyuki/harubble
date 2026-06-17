import type { DownloadJobSnapshot, DownloadTaskSnapshot } from '$lib/types';
import { formatByteSize, formatSpeed } from './formatters';
import * as m from '$lib/paraglide/messages.js';

export function getTaskProgressLabel(
  task: DownloadTaskSnapshot,
  speedLookup: (taskId: string) => number | undefined
): string | null {
  if (task.status !== 'downloading' && task.status !== 'writing') {
    return null;
  }

  if (task.status === 'downloading' && task.bytesTotal && task.bytesTotal > 0) {
    const percent = Math.min(
      Math.round((task.bytesDone / task.bytesTotal) * 100),
      100
    );
    const speed = speedLookup(task.id);
    const speedText = speed && speed > 0 ? ` · ${formatSpeed(speed)}` : '';
    return `${formatByteSize(task.bytesDone)} / ${formatByteSize(task.bytesTotal)} · ${percent}%${speedText}`;
  }

  if (task.bytesDone > 0) {
    return m.download_progress_bytes_processed({
      size: formatByteSize(task.bytesDone),
    });
  }

  return task.status === 'writing'
    ? m.download_progress_writing_file()
    : m.download_progress_receiving_data();
}

export function getTaskErrorLabel(task: DownloadTaskSnapshot): string | null {
  if (!task.error) return null;

  if (task.error.details && task.error.details !== task.error.message) {
    return `${task.error.message} · ${task.error.details}`;
  }

  return task.error.message;
}

export function getJobErrorSummary(job: DownloadJobSnapshot): string | null {
  const firstFailedTask = job.tasks.find(
    (task) => task.status === 'failed' && task.error
  );
  if (firstFailedTask) {
    return getTaskErrorLabel(firstFailedTask);
  }

  const firstCancelledTask = job.tasks.find(
    (task) => task.status === 'cancelled' && task.error
  );
  if (firstCancelledTask) {
    return getTaskErrorLabel(firstCancelledTask);
  }

  if (!job.error) return null;

  if (job.error.details && job.error.details !== job.error.message) {
    return `${job.error.message} · ${job.error.details}`;
  }

  return job.error.message;
}

export function getJobProgressText(
  job: DownloadJobSnapshot,
  speedLookup: (taskId: string) => number | undefined
): string {
  const terminalCount =
    job.completedTaskCount + job.failedTaskCount + job.cancelledTaskCount;
  const activeTask = job.tasks.find(
    (task) =>
      task.status === 'preparing' ||
      task.status === 'downloading' ||
      task.status === 'writing'
  );

  const base = m.download_progress_terminal_count({
    done: terminalCount,
    total: job.taskCount,
  });
  if (!activeTask) {
    return base;
  }

  const progressLabel = getTaskProgressLabel(activeTask, speedLookup);
  if (!progressLabel) {
    return `${base} · ${m.download_progress_processing({ name: activeTask.songName })}`;
  }

  return `${base} · ${activeTask.songName} · ${progressLabel}`;
}

export function getJobProgress(job: DownloadJobSnapshot): number {
  if (job.taskCount === 0) return 0;

  const terminalCount =
    job.completedTaskCount + job.failedTaskCount + job.cancelledTaskCount;
  const activeTask = job.tasks.find(
    (task) =>
      task.status === 'preparing' ||
      task.status === 'downloading' ||
      task.status === 'writing'
  );

  if (!activeTask) {
    return terminalCount / job.taskCount;
  }

  const activeTaskProgress =
    activeTask.status === 'downloading' && activeTask.bytesTotal
      ? activeTask.bytesDone / activeTask.bytesTotal
      : activeTask.status === 'writing'
        ? 1
        : 0;

  return Math.min((terminalCount + activeTaskProgress) / job.taskCount, 1);
}

export function getJobStatusLabel(job: DownloadJobSnapshot): string {
  switch (job.status) {
    case 'queued':
      return m.download_job_status_queued();
    case 'running': {
      const activeTask = job.tasks.find(
        (task) =>
          task.status === 'preparing' ||
          task.status === 'downloading' ||
          task.status === 'writing'
      );
      const currentIndex = activeTask
        ? activeTask.songIndex + 1
        : job.completedTaskCount;
      return m.download_job_status_running({
        current: currentIndex,
        total: job.taskCount,
      });
    }
    case 'completed':
      return m.download_job_status_completed();
    case 'partiallyFailed':
      return m.download_job_status_partially_failed({
        failed: job.failedTaskCount,
        total: job.taskCount,
      });
    case 'failed':
      return m.download_job_status_failed();
    case 'cancelled':
      return m.download_job_status_cancelled();
    default:
      return job.status;
  }
}

export function getTaskStatusLabel(
  task: DownloadTaskSnapshot,
  speedLookup: (taskId: string) => number | undefined
): string {
  switch (task.status) {
    case 'queued':
      return m.download_job_task_queued();
    case 'preparing':
      return m.download_job_task_preparing();
    case 'downloading': {
      const progressLabel = getTaskProgressLabel(task, speedLookup);
      return progressLabel ?? m.download_job_task_downloading();
    }
    case 'writing': {
      const progressLabel = getTaskProgressLabel(task, speedLookup);
      return progressLabel
        ? m.download_job_task_writing_with_progress({
            progress: progressLabel,
          })
        : m.download_job_task_writing();
    }
    case 'completed':
      return m.download_job_task_completed();
    case 'failed':
      return m.download_job_task_failed();
    case 'cancelled':
      return m.download_job_task_cancelled();
    default:
      return task.status;
  }
}

export function getJobKindLabel(job: DownloadJobSnapshot): string {
  switch (job.kind) {
    case 'song':
      return m.download_job_kind_song();
    case 'album':
      return m.download_job_kind_album();
    case 'selection':
      return m.download_job_kind_selection();
    default:
      return job.kind;
  }
}

export function getSelectionJobAlbumCount(job: DownloadJobSnapshot): number {
  return new Set(job.tasks.map((task) => task.albumCid)).size;
}

export function getSelectionJobScopeLabel(job: DownloadJobSnapshot): string {
  const albumCount = getSelectionJobAlbumCount(job);
  if (albumCount <= 1) {
    const albumName = job.tasks[0]?.albumName;
    return albumName
      ? m.download_job_scope_from_album({ album: albumName })
      : m.download_job_scope_same_album();
  }

  return m.download_job_scope_cross_albums({ count: albumCount });
}

export function getJobSummaryLabel(job: DownloadJobSnapshot): string {
  switch (job.kind) {
    case 'song': {
      const albumName = job.tasks[0]?.albumName;
      return albumName
        ? m.download_job_scope_from_album({ album: albumName })
        : m.download_job_summary_single_task();
    }
    case 'album':
      return m.download_job_summary_song_count({ count: job.taskCount });
    case 'selection': {
      if (job.taskCount <= 1) {
        return getSelectionJobScopeLabel(job);
      }

      const albumCount = getSelectionJobAlbumCount(job);
      if (albumCount <= 1) {
        return m.download_job_summary_song_count({ count: job.taskCount });
      }

      return m.download_job_summary_song_count_cross_albums({
        count: job.taskCount,
        albumCount,
      });
    }
    default:
      return m.download_job_summary_song_count({ count: job.taskCount });
  }
}

export function getJobDisplayTitle(job: DownloadJobSnapshot): string {
  if (job.kind !== 'selection') {
    return job.title;
  }
  const albumCount = getSelectionJobAlbumCount(job);
  if (albumCount > 1) {
    return m.download_job_selection_title_cross_albums({
      count: job.taskCount,
      albumCount,
    });
  }
  return m.download_job_selection_title({ count: job.taskCount });
}
