<script lang="ts">
  import { untrack } from 'svelte';
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import PreferencesSettingsSection from '$lib/components/app/shell/settings/PreferencesSettingsSection.svelte';
  import ThemeSettingsSection from '$lib/components/app/shell/settings/ThemeSettingsSection.svelte';
  import ThemePackageLibrarySection from '$lib/components/app/shell/settings/ThemePackageLibrarySection.svelte';
  import NotificationsSettingsSection from '$lib/components/app/shell/settings/NotificationsSettingsSection.svelte';
  import CacheSettingsSection from '$lib/components/app/shell/settings/CacheSettingsSection.svelte';
  import LogsSettingsSection from '$lib/components/app/shell/settings/LogsSettingsSection.svelte';
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
    type ColorScheme,
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
  import type { SettingsSection } from '$lib/components/app/shell/settingsSection';

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
    colorScheme?: ColorScheme;
    dynamicAlbumAccent?: boolean;
    logRefreshToken?: number;
    initialSection?: SettingsSection | null;
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
    colorScheme = $bindable<ColorScheme>('auto'),
    dynamicAlbumAccent = $bindable<boolean>(true),
    logRefreshToken = 0,
    initialSection = null,
    notifyInfo,
    notifyError,
    onOutputDirChange,
  }: Props = $props();

  let sheetBodyEl = $state<HTMLDivElement | null>(null);

  // 打开设置抽屉时若外部指定了 initialSection，等 Sheet 内容挂载后再滚动到锚点。
  // Sheet 使用 GSAP 缓动进场（约 260ms），因此这里用两次 rAF 让它稳定后再滚。
  $effect(() => {
    if (!open || !initialSection) return;
    const target = initialSection;
    const body = sheetBodyEl;
    if (!body) return;
    let cancelled = false;
    const scheduleId = requestAnimationFrame(() => {
      if (cancelled) return;
      requestAnimationFrame(() => {
        if (cancelled) return;
        const anchor = body.querySelector<HTMLElement>(
          `[data-settings-section="${target}"]`
        );
        anchor?.scrollIntoView({ behavior: 'smooth', block: 'start' });
      });
    });
    return () => {
      cancelled = true;
      cancelAnimationFrame(scheduleId);
    };
  });
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
      sectionThemePackages: m.settings_section_theme_packages(),
      sectionThemePackagesDescription: m.settings_theme_packages_description(),
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
      appearanceLabel: m.settings_appearance_label(),
      appearanceAuto: m.settings_appearance_auto(),
      appearanceLight: m.settings_appearance_light(),
      appearanceDark: m.settings_appearance_dark(),
      appearanceSegmentAria: m.settings_appearance_segment_aria(),
      dynamicAlbumLabel: m.settings_theme_dynamic_album_label(),
      dynamicAlbumOn: m.settings_theme_dynamic_album_on(),
      dynamicAlbumOff: m.settings_theme_dynamic_album_off(),
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
    untrack(() => {
      themeColorDrafts = Object.fromEntries(
        THEME_COLOR_SLOTS.map((slot) => {
          const currentDraft = themeColorDrafts[slot];
          const shouldSyncSlot =
            forceAll ||
            currentDraft === undefined ||
            currentDraft === lastSyncedThemeColors[slot];

          return [
            slot,
            shouldSyncSlot ? nextResolvedColors[slot] : currentDraft,
          ];
        })
      ) as Partial<Record<ThemeColorSlot, string>>;
      lastSyncedThemeColors = { ...nextResolvedColors };
    });
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
    <Tooltip.Provider>
      <div class="sheet-body" bind:this={sheetBodyEl}>
        <div data-settings-section="preferences">
          <PreferencesSettingsSection
            bind:locale
            bind:format
            bind:logLevel
            {outputDir}
            {localeOptions}
            {formatOptions}
            {logLevelOptions}
            {currentLocaleLabel}
            {currentFormatLabel}
            {currentLogLevelLabel}
            sectionTitle={labels.sectionPreferences}
            languageLabel={labels.languageLabel}
            outputFormatLabel={labels.outputFormat}
            logLevelLabel={labels.logLevel}
            outputDirLabel={labels.outputDir}
            outputDirSelectLabel={labels.outputDirSelect}
            onSelectDirectory={() => void handleSelectDirectory()}
          />
        </div>
        <div data-settings-section="theme">
          <ThemeSettingsSection
            bind:colorScheme
            bind:dynamicAlbumAccent
            {themePresetId}
            {resolvedThemeColors}
            {themePresetOptions}
            {currentThemePresetLabel}
            {getThemeDraft}
            {getSlotLabel}
            {isValidThemeHex}
            sectionTitle={labels.sectionTheme}
            themePresetLabel={labels.themePreset}
            themeResetLabel={labels.themeReset}
            themeResetTitle={labels.themeResetTitle}
            themeHexInvalidLabel={labels.themeHexInvalid}
            appearanceLabel={labels.appearanceLabel}
            appearanceAutoLabel={labels.appearanceAuto}
            appearanceLightLabel={labels.appearanceLight}
            appearanceDarkLabel={labels.appearanceDark}
            appearanceSegmentAria={labels.appearanceSegmentAria}
            dynamicAlbumLabel={labels.dynamicAlbumLabel}
            dynamicAlbumOnLabel={labels.dynamicAlbumOn}
            dynamicAlbumOffLabel={labels.dynamicAlbumOff}
            onThemePresetChange={handleThemePresetChange}
            onThemeTextInput={handleThemeTextInput}
            onThemeColorInput={handleThemeColorInput}
            onResetThemeCustomColors={resetThemeCustomColors}
          />
        </div>
        <div data-settings-section="theme-packages">
          <ThemePackageLibrarySection
            sectionTitle={labels.sectionThemePackages}
            sectionDescription={labels.sectionThemePackagesDescription}
          />
        </div>
        <div data-settings-section="notifications">
          <NotificationsSettingsSection
            bind:downloadLyrics
            bind:notifyOnDownloadComplete
            bind:notifyOnPlaybackChange
            {isSendingTestNotification}
            sectionTitle={labels.sectionNotifications}
            notificationTestLabel={labels.notificationTest}
            notificationTestSendingLabel={labels.notificationTestSending}
            lyricsTitle={labels.lyricsTitle}
            lyricsDescription={labels.lyricsDescription}
            notifyDownloadTitle={labels.notifyDownloadTitle}
            notifyDownloadDescription={labels.notifyDownloadDescription}
            notifyPlaybackTitle={labels.notifyPlaybackTitle}
            notifyPlaybackDescription={labels.notifyPlaybackDescription}
            onSendTestNotification={() => void handleSendTestNotification()}
          />
        </div>
        <div data-settings-section="cache">
          <CacheSettingsSection
            {isClearingAudioCache}
            sectionTitle={labels.sectionCache}
            cacheDescription={labels.cacheDescription}
            cacheClearLabel={labels.cacheClear}
            cacheClearingLabel={labels.cacheClearing}
            onClearAudioCache={() => void handleClearAudioCache()}
          />
        </div>
        <div data-settings-section="logs">
          <LogsSettingsSection
            {logFileKind}
            {logRecords}
            {logFileStatus}
            {logViewerLoading}
            {logViewerError}
            sectionTitle={labels.sectionLogs}
            logsDescription={labels.logsDescription}
            logSegmentAria={labels.logSegmentAria}
            logSessionLabel={labels.logSession}
            logPersistentLabel={labels.logPersistent}
            logStatusAvailableLabel={labels.logStatusAvailable}
            logStatusNoneLabel={labels.logStatusNone}
            logLoadingLabel={labels.logLoading}
            logEmptyLabel={labels.logEmpty}
            onRefreshLogs={(kind) => void refreshLogs(kind)}
          />
        </div>
      </div>
    </Tooltip.Provider>
  </Sheet.Content>
</Sheet.Root>

<style>
  :global(.settings-section) {
    gap: 12px;
  }
  :global(.settings-section-heading) {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  :global(.settings-section-heading > :first-child) {
    min-width: 0;
  }
  :global(.settings-section-heading > [data-slot='button']) {
    flex-shrink: 0;
  }
  :global(.settings-section-heading h3) {
    margin: 0;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0;
  }
  :global(.settings-section-heading p) {
    margin: 3px 0 0;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.45;
  }
  :global(.settings-field) {
    display: grid;
    gap: 6px;
    min-width: 0;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
  }
  :global(.settings-segment) {
    display: inline-grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(0, 1fr);
    overflow: hidden;
    border: 1px solid var(--sheet-border);
    border-radius: var(--shape-md);
    background: var(--sheet-row-bg);
    padding: 2px;
    flex-shrink: 0;
  }
  :global(.settings-segment button) {
    height: 26px;
    padding-inline: 10px;
    border: 0;
    border-radius: var(--shape-sm);
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
  }
  :global(.settings-segment button.active) {
    background: var(--accent);
    color: white;
  }
  @media (max-width: 420px) {
    :global(.settings-section-heading) {
      display: grid;
      grid-template-columns: 1fr;
    }
  }
</style>
