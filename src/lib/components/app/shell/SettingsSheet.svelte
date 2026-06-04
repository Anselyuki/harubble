<script lang="ts">
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import BellIcon from '@lucide/svelte/icons/bell';
  import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
  import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
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
  import {
    DEFAULT_THEME_PRESET_ID,
    THEME_COLOR_SLOTS,
    THEME_PRESETS,
    getThemePreset,
    isValidThemeHex,
    normalizeThemeHex,
    resolveThemeColors,
    type ThemeColorSlot,
    type ThemeColorSlots,
  } from '$lib/themePresets';
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
    themePresetId?: string;
    themeCustomColors?: Partial<ThemeColorSlots>;
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
    themePresetId = $bindable(DEFAULT_THEME_PRESET_ID),
    themeCustomColors = $bindable<Partial<ThemeColorSlots>>({}),
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
  let themeColorDrafts = $state<Partial<Record<ThemeColorSlot, string>>>({});
  let lastSyncedThemeColors = $state<ThemeColorSlots>(
    resolveThemeColors({
      presetId: themePresetId,
      customColors: themeCustomColors,
    })
  );
  const labels = $derived.by(() => {
    void localeState.current;
    return {
      title: m.settings_title(),
      description: m.settings_description(),
      sectionPreferences: m.settings_section_preferences(),
      sectionTheme: m.settings_section_theme(),
      sectionNotifications: m.settings_section_notifications(),
      sectionCache: m.settings_section_cache(),
      sectionLogs: m.settings_section_logs(),
      themePreset: m.settings_theme_preset_label(),
      themeReset: m.settings_theme_reset(),
      themeResetTitle: m.settings_theme_reset_title(),
      themeHexInvalid: m.settings_theme_hex_invalid(),
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
  const resolvedThemeColors = $derived(
    resolveThemeColors({
      presetId: themePresetId,
      customColors: themeCustomColors,
    })
  );
  const themePresetOptions = $derived.by(() => {
    void localeState.current;
    return THEME_PRESETS.map((preset) => ({
      ...preset,
      label: getPresetLabel(preset.id),
      description: getPresetDescription(preset.id),
    }));
  });
  const currentThemePresetLabel = $derived(
    themePresetOptions.find((preset) => preset.id === themePresetId)?.label ??
      getPresetLabel(DEFAULT_THEME_PRESET_ID)
  );

  function getPresetLabel(presetId: string): string {
    switch (presetId) {
      case 'clear-aqua':
        return m.settings_theme_preset_clear_aqua_name();
      case 'night-console':
        return m.settings_theme_preset_night_console_name();
      case 'harubble-classic':
      default:
        return m.settings_theme_preset_harubble_classic_name();
    }
  }

  function getPresetDescription(presetId: string): string {
    switch (presetId) {
      case 'clear-aqua':
        return m.settings_theme_preset_clear_aqua_description();
      case 'night-console':
        return m.settings_theme_preset_night_console_description();
      case 'harubble-classic':
      default:
        return m.settings_theme_preset_harubble_classic_description();
    }
  }

  function getSlotLabel(slot: ThemeColorSlot): string {
    switch (slot) {
      case 'accent':
        return m.settings_theme_slot_accent();
      case 'surface':
        return m.settings_theme_slot_surface();
      case 'textPrimary':
        return m.settings_theme_slot_text_primary();
      case 'textSecondary':
        return m.settings_theme_slot_text_secondary();
      case 'tint':
        return m.settings_theme_slot_tint();
      case 'danger':
        return m.settings_theme_slot_danger();
    }
  }

  function getThemeDraft(slot: ThemeColorSlot): string {
    return themeColorDrafts[slot] ?? resolvedThemeColors[slot];
  }

  function syncThemeDraftsToResolvedColors(
    nextResolvedColors = resolvedThemeColors,
    forceAll = false
  ) {
    themeColorDrafts = Object.fromEntries(
      THEME_COLOR_SLOTS.map((slot) => {
        const currentDraft = themeColorDrafts[slot];
        const shouldSyncSlot =
          forceAll ||
          currentDraft === undefined ||
          currentDraft === lastSyncedThemeColors[slot];

        return [slot, shouldSyncSlot ? nextResolvedColors[slot] : currentDraft];
      })
    ) as Partial<Record<ThemeColorSlot, string>>;
    lastSyncedThemeColors = { ...nextResolvedColors };
  }

  function handleThemePresetChange(nextPresetId: string) {
    const presetId = getThemePreset(nextPresetId).id;
    const nextResolvedColors = resolveThemeColors({
      presetId,
      customColors: {},
    });
    themePresetId = presetId;
    themeCustomColors = {};
    syncThemeDraftsToResolvedColors(nextResolvedColors, true);
  }

  function handleThemeTextInput(slot: ThemeColorSlot, value: string) {
    themeColorDrafts = { ...themeColorDrafts, [slot]: value };
    const normalized = normalizeThemeHex(value);
    if (!normalized) return;
    themeCustomColors = {
      ...themeCustomColors,
      [slot]: normalized,
    };
  }

  function handleThemeColorInput(slot: ThemeColorSlot, value: string) {
    const normalized = normalizeThemeHex(value);
    if (!normalized) return;
    themeColorDrafts = { ...themeColorDrafts, [slot]: normalized };
    themeCustomColors = {
      ...themeCustomColors,
      [slot]: normalized,
    };
  }

  function resetThemeCustomColors() {
    const nextResolvedColors = resolveThemeColors({
      presetId: themePresetId,
      customColors: {},
    });
    themeCustomColors = {};
    syncThemeDraftsToResolvedColors(nextResolvedColors, true);
  }

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
  $effect(() => {
    void themePresetId;
    void themeCustomColors;
    syncThemeDraftsToResolvedColors(resolvedThemeColors);
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
        <div class="settings-section-heading settings-theme-heading">
          <h3>{labels.sectionTheme}</h3>
          <Button
            variant="secondary"
            class="h-8"
            title={labels.themeResetTitle}
            onclick={resetThemeCustomColors}
          >
            <RotateCcwIcon data-icon="inline-start" />{labels.themeReset}
          </Button>
        </div>
        <label class="settings-field" for="theme-preset-select">
          <span>{labels.themePreset}</span>
          <Select.Root
            type="single"
            value={themePresetId}
            onValueChange={handleThemePresetChange}
          >
            <Select.Trigger
              id="theme-preset-select"
              class="sheet-select-trigger h-9 w-full border-[var(--sheet-border)]"
            >
              {currentThemePresetLabel}
            </Select.Trigger>
            <Select.Content class="sheet-select-content">
              {#each themePresetOptions as preset (preset.id)}
                <Select.Item value={preset.id} label={preset.label}>
                  <div class="settings-theme-preset-option">
                    <div class="settings-theme-preset-copy">
                      <strong>{preset.label}</strong>
                      <small>{preset.description}</small>
                    </div>
                    <div class="settings-theme-swatch-strip" aria-hidden="true">
                      {#each THEME_COLOR_SLOTS as slot (slot)}
                        <span style={`--swatch-color: ${preset.colors[slot]}`}
                        ></span>
                      {/each}
                    </div>
                  </div>
                </Select.Item>
              {/each}
            </Select.Content>
          </Select.Root>
        </label>
        <div class="settings-theme-color-list">
          {#each THEME_COLOR_SLOTS as slot (slot)}
            {@const draft = getThemeDraft(slot)}
            {@const invalid = draft.length > 0 && !isValidThemeHex(draft)}
            {@const invalidHelpId = `theme-color-${slot}-error`}
            <label class="settings-theme-color-row" for={`theme-color-${slot}`}>
              <span
                class="settings-theme-color-swatch"
                style={`--swatch-color: ${resolvedThemeColors[slot]}`}
              ></span>
              <span class="settings-theme-color-label"
                >{getSlotLabel(slot)}</span
              >
              <Input
                id={`theme-color-${slot}`}
                class="settings-theme-hex-input h-8 border-[var(--sheet-border)] bg-[var(--sheet-control-bg)]"
                value={draft}
                aria-invalid={invalid}
                aria-describedby={invalid ? invalidHelpId : undefined}
                oninput={(event) =>
                  handleThemeTextInput(slot, event.currentTarget.value)}
              />
              <input
                class="settings-theme-native-color"
                type="color"
                value={resolvedThemeColors[slot]}
                aria-label={getSlotLabel(slot)}
                oninput={(event) =>
                  handleThemeColorInput(slot, event.currentTarget.value)}
              />
              {#if invalid}
                <small id={invalidHelpId} class="settings-theme-invalid"
                  >{labels.themeHexInvalid}</small
                >
              {/if}
            </label>
          {/each}
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
  .settings-theme-heading {
    align-items: center;
  }
  .settings-theme-preset-option {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .settings-theme-preset-copy {
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .settings-theme-preset-copy strong {
    overflow: hidden;
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 700;
    line-height: 1.25;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .settings-theme-preset-copy small {
    overflow: hidden;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.3;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .settings-theme-swatch-strip {
    display: grid;
    grid-template-columns: repeat(6, 10px);
    overflow: hidden;
    border: 1px solid var(--sheet-border);
    border-radius: 999px;
  }
  .settings-theme-swatch-strip span,
  .settings-theme-color-swatch {
    background: var(--swatch-color);
  }
  .settings-theme-swatch-strip span {
    width: 10px;
    height: 18px;
  }
  .settings-theme-color-list {
    display: grid;
    gap: 7px;
  }
  .settings-theme-color-row {
    display: grid;
    grid-template-columns: 18px minmax(5.5rem, 1fr) minmax(6.5rem, 8rem) 34px;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .settings-theme-color-swatch {
    width: 18px;
    height: 18px;
    border: 1px solid var(--sheet-border);
    border-radius: 999px;
    box-shadow: inset 0 1px 0 color-mix(in srgb, white 36%, transparent);
  }
  .settings-theme-color-label {
    min-width: 0;
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 600;
    line-height: 1.3;
  }
  :global(.settings-theme-hex-input) {
    font-family: var(--font-mono);
    font-size: 12px;
    text-transform: uppercase;
  }
  :global(.settings-theme-hex-input[aria-invalid='true']) {
    border-color: color-mix(
      in srgb,
      var(--destructive) 55%,
      var(--sheet-border)
    );
    color: var(--destructive);
  }
  .settings-theme-native-color {
    width: 34px;
    height: 32px;
    border: 1px solid var(--sheet-border);
    border-radius: 7px;
    background: var(--sheet-control-bg);
    padding: 3px;
  }
  .settings-theme-invalid {
    grid-column: 3 / 5;
    margin-top: -3px;
    color: var(--destructive);
    font-size: 11px;
    font-weight: 500;
    line-height: 1.35;
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
    .settings-theme-color-row {
      grid-template-columns: 18px minmax(0, 1fr) 34px;
    }
    :global(.settings-theme-hex-input) {
      grid-column: 2 / 4;
    }
    .settings-theme-invalid {
      grid-column: 2 / 4;
    }
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
