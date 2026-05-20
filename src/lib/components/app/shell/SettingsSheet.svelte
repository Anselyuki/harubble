<script lang="ts">
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import BellIcon from '@lucide/svelte/icons/bell';
  import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import {
    clearAudioCache,
    getLogFileStatus,
    listLogRecords,
    selectDirectory,
    sendTestNotification,
  } from '$lib/settingsApi';
  import type { Locale } from '$lib/i18n/types';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import type {
    LogFileKind,
    LogFileStatus,
    LogLevel,
    LogViewerRecord,
    OutputFormat,
  } from '$lib/types';
  interface Props {
    open?: boolean;
    format?: OutputFormat;
    outputDir?: string;
    downloadLyrics?: boolean;
    notifyOnDownloadComplete?: boolean;
    notifyOnPlaybackChange?: boolean;
    logLevel?: LogLevel;
    locale?: Locale;
    logRefreshToken?: number;
    notifyInfo: (message: string) => void;
    notifyError: (message: string) => void;
    onOutputDirChange: (outputDir: string) => boolean | Promise<boolean>;
  }
  let {
    open = $bindable(false),
    format = $bindable<OutputFormat>('flac'),
    outputDir = $bindable(''),
    downloadLyrics = $bindable(true),
    notifyOnDownloadComplete = $bindable(true),
    notifyOnPlaybackChange = $bindable(true),
    logLevel = $bindable<LogLevel>('error'),
    locale = $bindable<Locale>('zh-CN'),
    logRefreshToken = 0,
    notifyInfo,
    notifyError,
    onOutputDirChange,
  }: Props = $props();
  let logFileKind = $state<LogFileKind>('session');
  let logRecords = $state<LogViewerRecord[]>([]);
  let logFileStatus = $state<LogFileStatus | null>(null);
  let logViewerLoading = $state(false);
  let logViewerError = $state('');
  let logRequestSeq = 0;
  let isSendingTestNotification = $state(false);
  let isClearingAudioCache = $state(false);
  let lastLoadedWhileOpen = $state(false);
  const labels = $derived.by(() => {
    void localeState.current;
    return {
      title: m.settings_title(),
      description: m.settings_description(),
      sectionPreferences: m.settings_section_preferences(),
      sectionNotifications: m.settings_section_notifications(),
      sectionCache: m.settings_section_cache(),
      sectionLogs: m.settings_section_logs(),
      languageLabel: m.settings_language_label(),
      zhCN: m.settings_language_zh_cn(),
      enUS: m.settings_language_en_us(),
      outputFormat: m.settings_output_format(),
      logLevel: m.settings_log_level(),
      outputDir: m.settings_output_dir(),
      outputDirSelect: m.settings_output_dir_select(),
      notificationTest: m.settings_notification_test(),
      notificationTestSending: m.settings_notification_test_sending(),
      lyricsTitle: m.settings_lyrics_title(),
      lyricsDescription: m.settings_lyrics_description(),
      notifyDownloadTitle: m.settings_notify_download_title(),
      notifyDownloadDescription: m.settings_notify_download_description(),
      notifyPlaybackTitle: m.settings_notify_playback_title(),
      notifyPlaybackDescription: m.settings_notify_playback_description(),
      cacheDescription: m.settings_cache_description(),
      cacheClear: m.settings_cache_clear(),
      cacheClearing: m.settings_cache_clearing(),
      logsDescription: m.settings_logs_description(),
      logSegmentAria: m.settings_log_segment_aria(),
      logSession: m.settings_log_session(),
      logPersistent: m.settings_log_persistent(),
      logStatusAvailable: m.settings_log_status_available(),
      logStatusNone: m.settings_log_status_none(),
      logLoading: m.settings_log_loading(),
      logEmpty: m.settings_log_empty(),
    };
  });
  const formatOptions = $derived.by(() => {
    void localeState.current;
    return [
      { value: 'flac' as OutputFormat, label: m.settings_format_flac() },
      { value: 'wav' as OutputFormat, label: m.settings_format_wav() },
      { value: 'mp3' as OutputFormat, label: m.settings_format_mp3() },
    ];
  });
  const logLevelOptions = $derived.by(() => {
    void localeState.current;
    return [
      { value: 'error' as LogLevel, label: m.settings_loglevel_error() },
      { value: 'warn' as LogLevel, label: m.settings_loglevel_warn() },
      { value: 'info' as LogLevel, label: m.settings_loglevel_info() },
      { value: 'debug' as LogLevel, label: m.settings_loglevel_debug() },
    ];
  });
  const localeOptions = $derived([
    { value: 'zh-CN' as Locale, label: labels.zhCN },
    { value: 'en-US' as Locale, label: labels.enUS },
  ]);
  const currentLocaleLabel = $derived(
    localeOptions.find((o) => o.value === locale)?.label ?? labels.zhCN
  );
  const currentFormatLabel = $derived(
    formatOptions.find((o) => o.value === format)?.label ?? 'FLAC'
  );
  const currentLogLevelLabel = $derived(
    logLevelOptions.find((o) => o.value === logLevel)?.label ?? 'Error'
  );
  async function refreshLogs(kind = logFileKind) {
    const requestSeq = ++logRequestSeq;
    logViewerLoading = true;
    logViewerError = '';
    try {
      const [page, status] = await Promise.all([
        listLogRecords({ kind, limit: 100 }),
        getLogFileStatus(),
      ]);
      if (requestSeq !== logRequestSeq || !open) return;
      logRecords = page.records;
      logFileStatus = status;
      logFileKind = kind;
    } catch (error) {
      if (requestSeq !== logRequestSeq || !open) return;
      logViewerError = error instanceof Error ? error.message : String(error);
    } finally {
      if (requestSeq === logRequestSeq) logViewerLoading = false;
    }
  }
  async function handleSelectDirectory() {
    const currentOutputDir = outputDir;
    const dir = await selectDirectory(currentOutputDir);
    if (!dir || dir === currentOutputDir) return;
    outputDir = dir;
    const saved = await onOutputDirChange(dir);
    if (!saved) {
      outputDir = currentOutputDir;
      notifyError(m.settings_toast_dir_save_failed());
    }
  }
  async function handleClearAudioCache() {
    if (isClearingAudioCache) return;
    isClearingAudioCache = true;
    try {
      const removed = await clearAudioCache();
      notifyInfo(
        removed > 0
          ? m.settings_toast_cache_cleared({ count: removed })
          : m.settings_toast_cache_empty()
      );
    } catch (error) {
      notifyError(
        m.settings_toast_cache_failed({
          error: error instanceof Error ? error.message : String(error),
        })
      );
    } finally {
      isClearingAudioCache = false;
    }
  }
  async function handleSendTestNotification() {
    if (isSendingTestNotification) return;
    isSendingTestNotification = true;
    try {
      await sendTestNotification();
      notifyInfo(m.settings_toast_notification_sent());
    } catch (error) {
      notifyError(
        m.settings_toast_notification_failed({
          error: error instanceof Error ? error.message : String(error),
        })
      );
    } finally {
      isSendingTestNotification = false;
    }
  }
  $effect(() => {
    if (!open) {
      lastLoadedWhileOpen = false;
      logRequestSeq += 1;
      logViewerLoading = false;
      return;
    }
    if (lastLoadedWhileOpen) return;
    lastLoadedWhileOpen = true;
    void refreshLogs(logFileKind);
  });
  $effect(() => {
    const refreshToken = logRefreshToken;
    if (!open || !lastLoadedWhileOpen || refreshToken === 0) return;
    void refreshLogs(logFileKind);
  });
</script>

<Sheet.Root bind:open>
  <Sheet.Content
    class="app-side-sheet settings-sheet gap-0 overflow-hidden border-[var(--sheet-border)] bg-[var(--surface-sheet)] p-0 text-[var(--text-primary)] shadow-[0_24px_64px_rgba(15,23,42,0.18)] backdrop-blur-xl"
  >
    <Sheet.Header class="sheet-header settings-sheet-header">
      <Sheet.Title>{labels.title}</Sheet.Title>
      <Sheet.Description>{labels.description}</Sheet.Description>
    </Sheet.Header>
    <div class="sheet-body">
      <section class="sheet-section settings-section">
        <div class="settings-section-heading">
          <h3>{labels.sectionPreferences}</h3>
        </div>
        <div class="settings-field-grid">
          <label class="settings-field" for="locale-select">
            <span>{labels.languageLabel}</span>
            <Select.Root type="single" bind:value={locale}
              ><Select.Trigger
                id="locale-select"
                class="sheet-select-trigger h-9 w-full border-[var(--sheet-border)]"
                >{currentLocaleLabel}</Select.Trigger
              ><Select.Content class="sheet-select-content"
                >{#each localeOptions as option (option.value)}<Select.Item
                    value={option.value}
                    label={option.label}
                  />{/each}</Select.Content
              ></Select.Root
            >
          </label>
          <label class="settings-field" for="format-select">
            <span>{labels.outputFormat}</span>
            <Select.Root type="single" bind:value={format}
              ><Select.Trigger
                id="format-select"
                class="sheet-select-trigger h-9 w-full border-[var(--sheet-border)]"
                >{currentFormatLabel}</Select.Trigger
              ><Select.Content class="sheet-select-content"
                >{#each formatOptions as option (option.value)}<Select.Item
                    value={option.value}
                    label={option.label}
                  />{/each}</Select.Content
              ></Select.Root
            >
          </label>
          <label class="settings-field" for="log-level-select">
            <span>{labels.logLevel}</span>
            <Select.Root type="single" bind:value={logLevel}
              ><Select.Trigger
                id="log-level-select"
                class="sheet-select-trigger h-9 w-full border-[var(--sheet-border)]"
                >{currentLogLevelLabel}</Select.Trigger
              ><Select.Content class="sheet-select-content"
                >{#each logLevelOptions as option (option.value)}<Select.Item
                    value={option.value}
                    label={option.label}
                  />{/each}</Select.Content
              ></Select.Root
            >
          </label>
          <div class="settings-field">
            <label for="output-dir">{labels.outputDir}</label>
            <div class="settings-path-row">
              <Input
                id="output-dir"
                class="h-9 border-[var(--sheet-border)] bg-[var(--sheet-control-bg)]"
                readonly
                value={outputDir}
              />
              <Button
                class="h-9 shrink-0"
                onclick={() => void handleSelectDirectory()}
                ><FolderOpenIcon
                  data-icon="inline-start"
                />{labels.outputDirSelect}</Button
              >
            </div>
          </div>
        </div>
      </section>
      <section class="sheet-section settings-section">
        <div class="settings-section-heading">
          <h3>{labels.sectionNotifications}</h3>
          <Button
            variant="secondary"
            disabled={isSendingTestNotification}
            onclick={() => void handleSendTestNotification()}
            ><BellIcon data-icon="inline-start" />{isSendingTestNotification
              ? labels.notificationTestSending
              : labels.notificationTest}</Button
          >
        </div>
        <div class="settings-toggle-list">
          <label class="settings-toggle"
            ><span
              ><strong>{labels.lyricsTitle}</strong><small
                >{labels.lyricsDescription}</small
              ></span
            ><Switch bind:checked={downloadLyrics} /></label
          >
          <label class="settings-toggle"
            ><span
              ><strong>{labels.notifyDownloadTitle}</strong><small
                >{labels.notifyDownloadDescription}</small
              ></span
            ><Switch bind:checked={notifyOnDownloadComplete} /></label
          >
          <label class="settings-toggle"
            ><span
              ><strong>{labels.notifyPlaybackTitle}</strong><small
                >{labels.notifyPlaybackDescription}</small
              ></span
            ><Switch bind:checked={notifyOnPlaybackChange} /></label
          >
        </div>
      </section>
      <section class="settings-section settings-action-section">
        <div class="settings-section-heading">
          <div>
            <h3>{labels.sectionCache}</h3>
            <p>{labels.cacheDescription}</p>
          </div>
          <Button
            variant="secondary"
            disabled={isClearingAudioCache}
            onclick={() => void handleClearAudioCache()}
            ><Trash2Icon data-icon="inline-start" />{isClearingAudioCache
              ? labels.cacheClearing
              : labels.cacheClear}</Button
          >
        </div>
      </section>
      <section class="sheet-section settings-section">
        <div class="settings-section-heading settings-log-heading">
          <div>
            <h3>{labels.sectionLogs}</h3>
            <p>{labels.logsDescription}</p>
          </div>
          <div class="settings-segment" aria-label={labels.logSegmentAria}>
            <button
              type="button"
              class:active={logFileKind === 'session'}
              aria-pressed={logFileKind === 'session'}
              onclick={() => void refreshLogs('session')}
              >{labels.logSession}</button
            >
            <button
              type="button"
              class:active={logFileKind === 'persistent'}
              aria-pressed={logFileKind === 'persistent'}
              onclick={() => void refreshLogs('persistent')}
              >{labels.logPersistent}</button
            >
          </div>
        </div>
        <p class="settings-log-status">
          {labels.logSession}: {logFileStatus?.hasSessionLog
            ? labels.logStatusAvailable
            : labels.logStatusNone} · {labels.logPersistent}: {logFileStatus?.hasPersistentLog
            ? labels.logStatusAvailable
            : labels.logStatusNone}
        </p>
        {#if logViewerLoading}
          <div class="settings-empty-state">{labels.logLoading}</div>
        {:else if logViewerError}
          <div class="settings-error-state">{logViewerError}</div>
        {:else if logRecords.length > 0}
          <div class="settings-log-list">
            {#each logRecords as record (record.id)}
              <article class="settings-log-record">
                <div class="settings-log-meta">
                  <span>{record.level}</span><time>{record.ts}</time>
                </div>
                <p class="settings-log-message">{record.message}</p>
                <p class="settings-log-source">
                  {record.domain} · {record.code}
                </p>
                {#if record.details}<p class="settings-log-details">
                    {record.details}
                  </p>{/if}
              </article>
            {/each}
          </div>
        {:else}
          <div class="settings-empty-state">{labels.logEmpty}</div>
        {/if}
      </section>
    </div>
  </Sheet.Content>
</Sheet.Root>

<style>
  .settings-section {
    gap: 12px;
  }
  .settings-section-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  .settings-section-heading h3 {
    margin: 0;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0;
  }
  .settings-section-heading p {
    margin: 3px 0 0;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.45;
  }
  .settings-field-grid {
    display: grid;
    gap: 10px;
  }
  .settings-field {
    display: grid;
    gap: 6px;
    min-width: 0;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
  }
  .settings-path-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
  }
  .settings-toggle-list {
    display: grid;
    overflow: hidden;
    border: 1px solid var(--sheet-border);
    border-radius: 8px;
  }
  .settings-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 58px;
    padding: 10px 12px;
    background: var(--sheet-row-bg);
    cursor: pointer;
    transition: background var(--motion-fast) var(--ease-standard);
  }
  .settings-toggle + .settings-toggle {
    border-top: 1px solid var(--sheet-border);
  }
  .settings-toggle:hover {
    background: var(--sheet-row-hover-bg);
  }
  .settings-toggle span {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .settings-toggle strong {
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 600;
  }
  .settings-toggle small {
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.35;
  }
  .settings-action-section {
    padding-block: 13px;
  }
  .settings-log-heading {
    align-items: center;
  }
  .settings-segment {
    display: inline-grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    overflow: hidden;
    border: 1px solid var(--sheet-border);
    border-radius: 8px;
    background: var(--sheet-row-bg);
    padding: 2px;
  }
  .settings-segment button {
    height: 26px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition:
      background var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard);
  }
  .settings-segment button.active {
    background: var(--accent);
    color: white;
  }
  .settings-log-status {
    margin: -4px 0 0;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.4;
  }
  .settings-log-list {
    display: grid;
    gap: 8px;
    max-height: 240px;
    overflow-y: auto;
    border: 1px solid var(--sheet-border);
    border-radius: 8px;
    background: var(--sheet-row-bg);
    padding: 8px;
  }
  .settings-log-record {
    display: grid;
    gap: 4px;
    border: 1px solid var(--sheet-border);
    border-radius: 7px;
    background: color-mix(in srgb, var(--bg-primary) 52%, transparent);
    padding: 8px 10px;
  }
  .settings-log-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.35;
  }
  .settings-log-meta span {
    font-weight: 700;
    text-transform: uppercase;
  }
  .settings-log-message {
    margin: 0;
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 600;
    line-height: 1.45;
  }
  .settings-log-source,
  .settings-log-details {
    margin: 0;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.4;
  }
  .settings-log-details {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .settings-empty-state,
  .settings-error-state {
    border: 1px solid var(--sheet-border);
    border-radius: 8px;
    background: var(--sheet-row-bg);
    padding: 14px 12px;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.45;
  }
  .settings-error-state {
    border-color: color-mix(in srgb, var(--destructive) 40%, transparent);
    background: color-mix(in srgb, var(--destructive) 10%, transparent);
    color: var(--destructive);
  }
  @media (max-width: 420px) {
    .settings-path-row,
    .settings-section-heading {
      grid-template-columns: 1fr;
    }
    .settings-section-heading {
      display: grid;
    }
    .settings-log-heading {
      align-items: stretch;
    }
  }
</style>
