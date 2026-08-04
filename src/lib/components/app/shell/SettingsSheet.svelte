<script lang="ts">
  import { untrack } from 'svelte';
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import ChevronsLeftIcon from '@lucide/svelte/icons/chevrons-left';
  import ChevronsRightIcon from '@lucide/svelte/icons/chevrons-right';
  import PreferencesSettingsSection from '$lib/components/app/shell/settings/PreferencesSettingsSection.svelte';
  import ThemeSettingsSection from '$lib/components/app/shell/settings/ThemeSettingsSection.svelte';
  import ThemePackageLibrarySection from '$lib/components/app/shell/settings/ThemePackageLibrarySection.svelte';
  import NotificationsSettingsSection from '$lib/components/app/shell/settings/NotificationsSettingsSection.svelte';
  import CacheSettingsSection from '$lib/components/app/shell/settings/CacheSettingsSection.svelte';
  import LogsSettingsSection from '$lib/components/app/shell/settings/LogsSettingsSection.svelte';
  import {
    clearAudioCache,
    getLogFileStatus,
    getNotificationPermissionState,
    listLogRecords,
    requestNotificationPermission,
    selectDirectory,
    sendTestNotification,
    type NotificationPermissionState,
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
  import {
    getThemePackageManager,
    type ThemePackageManager,
  } from '$lib/features/shell/themePackageManager.svelte';

  type SettingsThemePackageManager = Pick<
    ThemePackageManager,
    | 'activePackageId'
    | 'previewingId'
    | 'installedPackages'
    | 'latestError'
    | 'hydrate'
    | 'setActive'
    | 'preview'
    | 'dismissPreview'
    | 'importFromFile'
    | 'importFromUrl'
    | 'uninstall'
  >;

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
    themePackageManager?: SettingsThemePackageManager;
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
    themePackageManager = getThemePackageManager(),
    notifyInfo,
    notifyError,
    onOutputDirChange,
  }: Props = $props();

  let sheetBodyEl = $state<HTMLDivElement | null>(null);
  let sectionNavEl = $state<HTMLElement | null>(null);
  let activeSection = $state<SettingsSection>('preferences');
  let navOverflowStart = $state(false);
  let navOverflowEnd = $state(false);
  let sectionScrollLock: SettingsSection | null = null;
  let sectionScrollLockTimer: ReturnType<typeof setTimeout> | null = null;
  let previewBackdropRetracted = $state(false);
  const previewingThemePackage = $derived(
    Boolean(
      themePackageManager.previewingId &&
      themePackageManager.previewingId !== themePackageManager.activePackageId
    )
  );
  const packageColorsLocked = $derived(
    Boolean(
      themePackageManager.activePackageId || themePackageManager.previewingId
    )
  );

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
        lockActiveSection(target);
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
  let isRequestingNotificationPermission = $state(false);
  let notificationPermissionState = $state<NotificationPermissionState | null>(
    null
  );
  let notificationPermissionRequestSeq = 0;
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
      notificationPermissionLabel: m.settings_notification_permission_label(),
      notificationPermissionGranted:
        m.settings_notification_permission_granted(),
      notificationPermissionDenied: m.settings_notification_permission_denied(),
      notificationPermissionPrompt: m.settings_notification_permission_prompt(),
      notificationPermissionLoading:
        m.settings_notification_permission_loading(),
      notificationPermissionRequest:
        m.settings_notification_permission_request(),
      notificationPermissionRequesting:
        m.settings_notification_permission_requesting(),
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
      retractPreviewOverlay:
        m.settings_theme_packages_retract_preview_overlay(),
      restorePreviewOverlay:
        m.settings_theme_packages_restore_preview_overlay(),
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
  const sectionLinks = $derived([
    { id: 'preferences' as SettingsSection, label: labels.sectionPreferences },
    { id: 'theme' as SettingsSection, label: labels.sectionTheme },
    {
      id: 'theme-packages' as SettingsSection,
      label: labels.sectionThemePackages,
    },
    {
      id: 'notifications' as SettingsSection,
      label: labels.sectionNotifications,
    },
    { id: 'cache' as SettingsSection, label: labels.sectionCache },
    { id: 'logs' as SettingsSection, label: labels.sectionLogs },
  ]);
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

  function togglePreviewBackdrop() {
    if (!previewingThemePackage) return;
    previewBackdropRetracted = !previewBackdropRetracted;
  }

  function scrollToSection(section: SettingsSection) {
    lockActiveSection(section);
    sheetBodyEl
      ?.querySelector<HTMLElement>(`[data-settings-section="${section}"]`)
      ?.scrollIntoView({ behavior: 'smooth', block: 'start' });
    sectionNavEl
      ?.querySelector<HTMLElement>(`[data-section-link="${section}"]`)
      ?.scrollIntoView({
        behavior: 'smooth',
        block: 'nearest',
        inline: 'nearest',
      });
  }

  function lockActiveSection(section: SettingsSection) {
    activeSection = section;
    sectionScrollLock = section;
    if (sectionScrollLockTimer !== null) {
      clearTimeout(sectionScrollLockTimer);
    }
    sectionScrollLockTimer = setTimeout(() => {
      sectionScrollLock = null;
      sectionScrollLockTimer = null;
    }, 800);
  }

  function updateNavOverflow() {
    if (!sectionNavEl) return;
    navOverflowStart = sectionNavEl.scrollLeft > 2;
    navOverflowEnd =
      sectionNavEl.scrollLeft + sectionNavEl.clientWidth <
      sectionNavEl.scrollWidth - 2;
  }

  function syncActiveSectionFromScroll() {
    if (!sheetBodyEl) return;
    if (sectionScrollLock) {
      activeSection = sectionScrollLock;
      return;
    }
    const sections = Array.from(
      sheetBodyEl.querySelectorAll<HTMLElement>('[data-settings-section]')
    );
    if (sections.length === 0) return;

    const bodyTop = sheetBodyEl.getBoundingClientRect().top;
    const atBottom =
      sheetBodyEl.scrollTop + sheetBodyEl.clientHeight >=
      sheetBodyEl.scrollHeight - 2;
    const current = atBottom
      ? sections.at(-1)
      : ([...sections]
          .reverse()
          .find(
            (section) => section.getBoundingClientRect().top <= bodyTop + 24
          ) ?? sections[0]);
    const next = current?.dataset.settingsSection as
      | SettingsSection
      | undefined;
    if (!next || next === activeSection) return;
    activeSection = next;
    sectionNavEl
      ?.querySelector<HTMLElement>(`[data-section-link="${next}"]`)
      ?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
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
    if (isSendingTestNotification || notificationPermissionState !== 'granted')
      return;
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
  async function handleRequestNotificationPermission() {
    if (isRequestingNotificationPermission) return;
    isRequestingNotificationPermission = true;
    try {
      notificationPermissionState = await requestNotificationPermission();
    } catch (error) {
      notifyError(
        m.settings_toast_notification_failed({
          error: error instanceof Error ? error.message : String(error),
        })
      );
    } finally {
      isRequestingNotificationPermission = false;
    }
  }
  function getNotificationPermissionLabel() {
    switch (notificationPermissionState) {
      case 'granted':
        return labels.notificationPermissionGranted;
      case 'denied':
        return labels.notificationPermissionDenied;
      case 'prompt':
      case 'prompt-with-rationale':
        return labels.notificationPermissionPrompt;
      default:
        return labels.notificationPermissionLoading;
    }
  }
  $effect(() => {
    if (!open) {
      lastLoadedWhileOpen = false;
      logRequestSeq += 1;
      notificationPermissionRequestSeq += 1;
      logViewerLoading = false;
      return;
    }
    if (lastLoadedWhileOpen) return;
    lastLoadedWhileOpen = true;
    void refreshLogs(logFileKind);
    const requestSeq = ++notificationPermissionRequestSeq;
    void getNotificationPermissionState()
      .then((state) => {
        if (requestSeq === notificationPermissionRequestSeq) {
          notificationPermissionState = state;
        }
      })
      .catch(() => {
        if (requestSeq === notificationPermissionRequestSeq) {
          notificationPermissionState = null;
        }
      });
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
  $effect(() => {
    if ((!open || !previewingThemePackage) && previewBackdropRetracted) {
      previewBackdropRetracted = false;
    }
  });
  $effect(() => {
    if (!open || !sectionNavEl) return;
    updateNavOverflow();
    const observer = new ResizeObserver(updateNavOverflow);
    observer.observe(sectionNavEl);
    return () => observer.disconnect();
  });
  $effect(() => {
    return () => {
      if (sectionScrollLockTimer !== null) {
        clearTimeout(sectionScrollLockTimer);
      }
    };
  });
</script>

<Sheet.Root bind:open>
  <Sheet.Content
    class="app-side-sheet settings-sheet gap-0 border-[var(--sheet-border)] bg-[var(--surface-sheet)] p-0 text-[var(--text-primary)] shadow-[0_24px_64px_rgba(15,23,42,0.18)] backdrop-blur-xl"
    data-testid="settings-sheet"
    data-previewing={previewingThemePackage}
    data-preview-backdrop-retracted={previewBackdropRetracted}
    overlayProps={{
      class: previewBackdropRetracted
        ? 'settings-preview-overlay settings-preview-overlay--retracted'
        : 'settings-preview-overlay',
      'data-testid': 'settings-preview-backdrop',
      'data-previewing': previewingThemePackage,
      'data-retracted': previewBackdropRetracted,
    }}
  >
    <Sheet.Header class="sheet-header settings-sheet-header">
      <Sheet.Title>{labels.title}</Sheet.Title>
      <Sheet.Description>{labels.description}</Sheet.Description>
    </Sheet.Header>
    <Tooltip.Provider>
      {#if previewingThemePackage}
        <Tooltip.Root>
          <Tooltip.Trigger>
            {#snippet child({ props })}
              <Button
                {...props}
                variant="outline"
                size="icon-lg"
                class="preview-backdrop-toggle"
                aria-label={previewBackdropRetracted
                  ? labels.restorePreviewOverlay
                  : labels.retractPreviewOverlay}
                aria-pressed={previewBackdropRetracted}
                data-testid="theme-preview-backdrop-toggle"
                onclick={togglePreviewBackdrop}
              >
                {#if previewBackdropRetracted}
                  <ChevronsRightIcon aria-hidden="true" />
                {:else}
                  <ChevronsLeftIcon aria-hidden="true" />
                {/if}
              </Button>
            {/snippet}
          </Tooltip.Trigger>
          <Tooltip.Content
            class="preview-backdrop-tooltip"
            side="left"
            sideOffset={8}
          >
            {previewBackdropRetracted
              ? labels.restorePreviewOverlay
              : labels.retractPreviewOverlay}
          </Tooltip.Content>
        </Tooltip.Root>
      {/if}
      <div
        class="settings-section-nav-shell"
        class:has-overflow-start={navOverflowStart}
        class:has-overflow-end={navOverflowEnd}
      >
        <nav
          class="settings-section-nav"
          aria-label={labels.title}
          bind:this={sectionNavEl}
          onscroll={updateNavOverflow}
        >
          {#each sectionLinks as section (section.id)}
            <button
              type="button"
              class:active={activeSection === section.id}
              aria-current={activeSection === section.id
                ? 'location'
                : undefined}
              data-section-link={section.id}
              onclick={() => scrollToSection(section.id)}
            >
              {section.label}
            </button>
          {/each}
        </nav>
      </div>
      <div
        class="sheet-body"
        bind:this={sheetBodyEl}
        onscroll={syncActiveSectionFromScroll}
      >
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
            {packageColorsLocked}
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
            manager={themePackageManager}
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
            {isRequestingNotificationPermission}
            {notificationPermissionState}
            sectionTitle={labels.sectionNotifications}
            notificationTestLabel={labels.notificationTest}
            notificationTestSendingLabel={labels.notificationTestSending}
            notificationPermissionLabel={labels.notificationPermissionLabel}
            notificationPermissionStatus={getNotificationPermissionLabel()}
            notificationPermissionRequestLabel={labels.notificationPermissionRequest}
            notificationPermissionRequestingLabel={labels.notificationPermissionRequesting}
            lyricsTitle={labels.lyricsTitle}
            lyricsDescription={labels.lyricsDescription}
            notifyDownloadTitle={labels.notifyDownloadTitle}
            notifyDownloadDescription={labels.notifyDownloadDescription}
            notifyPlaybackTitle={labels.notifyPlaybackTitle}
            notifyPlaybackDescription={labels.notifyPlaybackDescription}
            onSendTestNotification={() => void handleSendTestNotification()}
            onRequestNotificationPermission={() =>
              void handleRequestNotificationPermission()}
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
  .settings-section-nav {
    display: flex;
    gap: 4px;
    overflow-x: auto;
    padding: 8px 16px;
    border-block: 1px solid var(--sheet-border, var(--border));
    background: var(--sheet-control-bg, var(--bg-primary));
    scrollbar-width: none;
  }
  .settings-section-nav::-webkit-scrollbar {
    display: none;
  }
  .sheet-body::-webkit-scrollbar {
    display: none;
  }
  .settings-section-nav-shell {
    position: relative;
  }
  .settings-section-nav-shell::before,
  .settings-section-nav-shell::after {
    content: '';
    position: absolute;
    z-index: 3;
    top: 0;
    bottom: 0;
    width: 18px;
    pointer-events: none;
    opacity: 0;
    transition: opacity var(--motion-fast);
  }
  .settings-section-nav-shell::before {
    left: 0;
    background: linear-gradient(90deg, var(--surface-sheet), transparent);
  }
  .settings-section-nav-shell::after {
    right: 0;
    background: linear-gradient(270deg, var(--surface-sheet), transparent);
  }
  .settings-section-nav-shell.has-overflow-start::before,
  .settings-section-nav-shell.has-overflow-end::after {
    opacity: 1;
  }
  .sheet-body {
    overflow-x: hidden;
    scrollbar-width: none;
  }
  .settings-section-nav button {
    min-height: 40px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--shape-sm);
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    white-space: nowrap;
    cursor: pointer;
  }
  .settings-section-nav button:hover,
  .settings-section-nav button.active {
    background: var(--hover-bg-elevated);
    color: var(--text-primary);
  }
  .settings-section-nav button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .sheet-body > [data-settings-section] {
    scroll-margin-top: 12px;
  }
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
  :global(.settings-sheet .preview-backdrop-toggle) {
    position: absolute;
    top: 12px;
    left: -48px;
    z-index: 1;
    width: 40px;
    height: 40px;
    border-color: var(--sheet-border, var(--border));
    border-radius: var(--shape-md);
    background: var(--sheet-control-bg, var(--bg-primary));
    color: var(--text-primary);
    box-shadow: 0 8px 22px
      color-mix(in srgb, var(--text-primary) 12%, transparent);
  }
  :global(.settings-sheet .preview-backdrop-toggle:hover) {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-readable-foreground);
  }
  :global(.settings-sheet .preview-backdrop-toggle:focus-visible) {
    border-color: var(--accent);
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    box-shadow: none;
  }
  :global(.settings-preview-overlay) {
    clip-path: inset(0);
    transition: clip-path var(--motion-slow) var(--ease-ios);
    will-change: clip-path;
  }
  :global(.settings-preview-overlay--retracted) {
    clip-path: inset(0 100% 0 0);
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
    color: var(--accent-readable-foreground);
  }
  @media (max-width: 420px) {
    :global(.settings-sheet .settings-sheet-header) {
      padding-right: 108px;
    }
    :global(.settings-sheet .preview-backdrop-toggle) {
      right: 60px;
      left: auto;
    }
    :global(.preview-backdrop-tooltip) {
      display: none;
    }
    :global(.settings-section-heading) {
      display: grid;
      grid-template-columns: 1fr;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    :global(.settings-preview-overlay) {
      transition-duration: 0ms;
    }
  }
</style>
