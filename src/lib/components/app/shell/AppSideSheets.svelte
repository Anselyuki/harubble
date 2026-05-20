<script lang="ts">
  import type SettingsSheet from '$lib/components/app/shell/SettingsSheet.svelte';
  import type DownloadTasksSheet from '$lib/components/app/shell/DownloadTasksSheet.svelte';
  import type { Locale } from '$lib/i18n/types';
  import type {
    DownloadHistoryKindFilter,
    DownloadHistoryScopeFilter,
    DownloadHistoryStatusFilter,
    DownloadJobSnapshot,
    DownloadTaskSnapshot,
    LogLevel,
    OutputFormat,
  } from '$lib/types';

  type SettingsSheetComponent = typeof SettingsSheet;
  type DownloadTasksSheetComponent = typeof DownloadTasksSheet;

  interface Props {
    SettingsSheetView?: SettingsSheetComponent | null;
    DownloadTasksSheetView?: DownloadTasksSheetComponent | null;
    settingsOpen?: boolean;
    downloadPanelOpen?: boolean;
    format?: OutputFormat;
    outputDir?: string;
    downloadLyrics?: boolean;
    notifyOnDownloadComplete?: boolean;
    notifyOnPlaybackChange?: boolean;
    logLevel?: LogLevel;
    locale?: Locale;
    settingsLogRefreshToken: number;
    notifyInfo: (message: string) => void;
    notifyError: (message: string) => void;
    onOutputDirChange: (outputDir: string) => boolean | Promise<boolean>;
    jobs: DownloadJobSnapshot[];
    hasDownloadHistory: boolean;
    searchQuery?: string;
    scopeFilter?: DownloadHistoryScopeFilter;
    statusFilter?: DownloadHistoryStatusFilter;
    kindFilter?: DownloadHistoryKindFilter;
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
    onClearDownloadHistory: () => void | Promise<void>;
    onCancelDownloadJob: (jobId: string) => void | Promise<void>;
    onRetryDownloadJob: (jobId: string) => void | Promise<void>;
    onCancelDownloadTask: (
      jobId: string,
      taskId: string
    ) => void | Promise<void>;
    onRetryDownloadTask: (
      jobId: string,
      taskId: string
    ) => void | Promise<void>;
  }

  let {
    SettingsSheetView = null,
    DownloadTasksSheetView = null,
    settingsOpen = $bindable(false),
    downloadPanelOpen = $bindable(false),
    format = $bindable<OutputFormat>('flac'),
    outputDir = $bindable(''),
    downloadLyrics = $bindable(true),
    notifyOnDownloadComplete = $bindable(true),
    notifyOnPlaybackChange = $bindable(true),
    logLevel = $bindable<LogLevel>('error'),
    locale = $bindable<Locale>('zh-CN'),
    settingsLogRefreshToken,
    notifyInfo,
    notifyError,
    onOutputDirChange,
    jobs,
    hasDownloadHistory,
    searchQuery = $bindable(''),
    scopeFilter = $bindable<DownloadHistoryScopeFilter>('all'),
    statusFilter = $bindable<DownloadHistoryStatusFilter>('all'),
    kindFilter = $bindable<DownloadHistoryKindFilter>('all'),
    canClearDownloadHistory,
    getJobProgress,
    getJobProgressText,
    getJobStatusLabel,
    getJobKindLabel,
    getJobSummaryLabel,
    getJobDisplayTitle,
    getJobErrorSummary,
    isJobActive,
    canCancelTask,
    canRetryTask,
    getTaskErrorLabel,
    getTaskStatusLabel,
    onClearDownloadHistory,
    onCancelDownloadJob,
    onRetryDownloadJob,
    onCancelDownloadTask,
    onRetryDownloadTask,
  }: Props = $props();
</script>

{#if SettingsSheetView}
  <SettingsSheetView
    bind:open={settingsOpen}
    bind:format
    bind:outputDir
    bind:downloadLyrics
    bind:notifyOnDownloadComplete
    bind:notifyOnPlaybackChange
    bind:logLevel
    bind:locale
    logRefreshToken={settingsLogRefreshToken}
    {notifyInfo}
    {notifyError}
    {onOutputDirChange}
  />
{/if}

{#if DownloadTasksSheetView}
  <DownloadTasksSheetView
    bind:open={downloadPanelOpen}
    {jobs}
    {hasDownloadHistory}
    bind:searchQuery
    bind:scopeFilter
    bind:statusFilter
    bind:kindFilter
    {canClearDownloadHistory}
    {getJobProgress}
    {getJobProgressText}
    {getJobStatusLabel}
    {getJobKindLabel}
    {getJobSummaryLabel}
    {getJobDisplayTitle}
    {getJobErrorSummary}
    {isJobActive}
    {canCancelTask}
    {canRetryTask}
    {getTaskErrorLabel}
    {getTaskStatusLabel}
    {onClearDownloadHistory}
    {onCancelDownloadJob}
    {onRetryDownloadJob}
    {onCancelDownloadTask}
    {onRetryDownloadTask}
  />
{/if}
