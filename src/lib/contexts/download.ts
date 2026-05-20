import { getContext, setContext } from 'svelte';
import type {
  DownloadHistoryKindFilter,
  DownloadHistoryScopeFilter,
  DownloadHistoryStatusFilter,
  DownloadJobSnapshot,
  DownloadTaskSnapshot,
} from '$lib/types';
import { DOWNLOAD_CONTEXT_KEY } from './keys';

export interface DownloadContext {
  readonly activeDownloadCount: number;
  readonly filteredJobs: DownloadJobSnapshot[];
  readonly hasDownloadHistory: boolean;
  searchQuery: string;
  scopeFilter: DownloadHistoryScopeFilter;
  statusFilter: DownloadHistoryStatusFilter;
  kindFilter: DownloadHistoryKindFilter;
  canClearDownloadHistory: () => boolean;
  getJobProgress: (job: DownloadJobSnapshot) => number;
  getJobProgressText: (job: DownloadJobSnapshot) => string;
  getJobStatusLabel: (job: DownloadJobSnapshot) => string;
  getJobKindLabel: (job: DownloadJobSnapshot) => string;
  getJobSummaryLabel: (job: DownloadJobSnapshot) => string;
  getJobDisplayTitle: (job: DownloadJobSnapshot) => string;
  getJobErrorSummary: (job: DownloadJobSnapshot) => string | null;
  isJobActive: (jobId: string) => boolean;
  canCancelTask: (task: DownloadTaskSnapshot) => boolean;
  canRetryTask: (task: DownloadTaskSnapshot) => boolean;
  getTaskErrorLabel: (task: DownloadTaskSnapshot) => string | null;
  getTaskStatusLabel: (task: DownloadTaskSnapshot) => string;
  handleClearDownloadHistory: () => void | Promise<void>;
  handleCancelDownloadJob: (jobId: string) => void | Promise<void>;
  handleRetryDownloadJob: (jobId: string) => void | Promise<void>;
  handleCancelDownloadTask: (
    jobId: string,
    taskId: string
  ) => void | Promise<void>;
  handleRetryDownloadTask: (
    jobId: string,
    taskId: string
  ) => void | Promise<void>;
  handleSongDownload: (songCid: string) => void | Promise<void>;
  getSongDownloadState: (songCid: string) => string;
  isSongDownloadInteractionBlocked: (songCid: string) => boolean;
}

export function setDownloadContext(ctx: DownloadContext): void {
  setContext(DOWNLOAD_CONTEXT_KEY, ctx);
}

export function getDownloadContext(): DownloadContext {
  return getContext<DownloadContext>(DOWNLOAD_CONTEXT_KEY);
}
