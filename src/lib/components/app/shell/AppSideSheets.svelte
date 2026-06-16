<script lang="ts">
  import type SettingsSheet from '$lib/components/app/shell/SettingsSheet.svelte';
  import type DownloadTasksSheet from '$lib/components/app/shell/DownloadTasksSheet.svelte';
  import type { Locale } from '$lib/i18n/types';
  import {
    DEFAULT_THEME_PRESET_ID,
    type ColorScheme,
    type ThemeColorSlots,
  } from '$lib/themePresets';
  import type { LogLevel, OutputFormat } from '$lib/types';
  import { getDownloadContext } from '$lib/contexts';

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
    themePresetId?: string;
    themeCustomColors?: Partial<ThemeColorSlots>;
    colorScheme?: ColorScheme;
    settingsLogRefreshToken: number;
    notifyInfo: (message: string) => void;
    notifyError: (message: string) => void;
    onOutputDirChange: (outputDir: string) => boolean | Promise<boolean>;
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
    themePresetId = $bindable(DEFAULT_THEME_PRESET_ID),
    themeCustomColors = $bindable<Partial<ThemeColorSlots>>({}),
    colorScheme = $bindable<ColorScheme>('auto'),
    settingsLogRefreshToken,
    notifyInfo,
    notifyError,
    onOutputDirChange,
  }: Props = $props();

  const download = getDownloadContext();
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
    bind:themePresetId
    bind:themeCustomColors
    bind:colorScheme
    logRefreshToken={settingsLogRefreshToken}
    {notifyInfo}
    {notifyError}
    {onOutputDirChange}
  />
{/if}

{#if DownloadTasksSheetView}
  <DownloadTasksSheetView
    bind:open={downloadPanelOpen}
    jobs={download.filteredJobs}
    hasDownloadHistory={download.hasDownloadHistory}
    bind:searchQuery={download.searchQuery}
    bind:scopeFilter={download.scopeFilter}
    bind:statusFilter={download.statusFilter}
    bind:kindFilter={download.kindFilter}
    canClearDownloadHistory={download.canClearDownloadHistory}
    getJobProgress={download.getJobProgress}
    getJobProgressText={download.getJobProgressText}
    getJobStatusLabel={download.getJobStatusLabel}
    getJobKindLabel={download.getJobKindLabel}
    getJobSummaryLabel={download.getJobSummaryLabel}
    getJobDisplayTitle={download.getJobDisplayTitle}
    getJobErrorSummary={download.getJobErrorSummary}
    isJobActive={download.isJobActive}
    canCancelTask={download.canCancelTask}
    canRetryTask={download.canRetryTask}
    getTaskErrorLabel={download.getTaskErrorLabel}
    getTaskStatusLabel={download.getTaskStatusLabel}
    onClearDownloadHistory={download.handleClearDownloadHistory}
    onCancelDownloadJob={download.handleCancelDownloadJob}
    onRetryDownloadJob={download.handleRetryDownloadJob}
    onCancelDownloadTask={download.handleCancelDownloadTask}
    onRetryDownloadTask={download.handleRetryDownloadTask}
  />
{/if}
