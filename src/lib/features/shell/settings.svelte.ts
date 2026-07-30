import * as m from '$lib/paraglide/messages.js';
import type { Locale } from '$lib/i18n/types';
import type { AppPreferences, LogLevel, OutputFormat } from '$lib/types';
import {
  DEFAULT_THEME_PREFERENCES,
  type ColorScheme,
  type ThemeColorSlots,
} from '$lib/themePresets';
import { formatPreferencesError } from '$lib/features/shell/domainErrors';

interface SettingsControllerDeps {
  getPreferences: () => Promise<AppPreferences>;
  setPreferences: (
    preferences: AppPreferences,
    expectedRevision: number
  ) => Promise<AppPreferences>;
  notifyError: (message: string) => void;
  onLocaleChanged?: (locale: Locale) => void;
}

interface HydrateSettingsOptions {
  shouldDispose?: () => boolean;
}

interface ApplyPreferencesSnapshotOptions {
  /** 导入文件等用户明确要求覆盖当前编辑内容的场景。 */
  force?: boolean;
}

export interface SettingsState {
  schemaVersion: number;
  format: OutputFormat;
  outputDir: string;
  downloadLyrics: boolean;
  notifyOnDownloadComplete: boolean;
  notifyOnPlaybackChange: boolean;
  logLevel: LogLevel;
  locale: Locale;
  volume: number;
  themeRevision: number;
  themePresetId: string;
  themeCustomColors: Partial<ThemeColorSlots>;
  colorScheme: ColorScheme;
  dynamicAlbumAccent: boolean;
  settingsLogRefreshToken: number;
  prefsReady: boolean;
  isSaving: boolean;
  persistedSnapshot: string;
  lastSaveFailedSnapshot: string;
  dirty: {
    format: boolean;
    outputDir: boolean;
    downloadLyrics: boolean;
    notifyOnDownloadComplete: boolean;
    notifyOnPlaybackChange: boolean;
    logLevel: boolean;
    locale: boolean;
    theme: boolean;
  };
  suspendDirtyTracking: number;
}

const SCALAR_DIRTY_FIELDS = [
  'format',
  'outputDir',
  'downloadLyrics',
  'notifyOnDownloadComplete',
  'notifyOnPlaybackChange',
  'logLevel',
  'locale',
] as const;

type ScalarDirtyField = (typeof SCALAR_DIRTY_FIELDS)[number];

interface SnapshotMergeBaseline {
  revision: number;
  scalar: Record<ScalarDirtyField, unknown>;
  theme: string;
}

function createInitialState(): SettingsState {
  return {
    schemaVersion: 2,
    format: 'flac',
    outputDir: '',
    downloadLyrics: true,
    notifyOnDownloadComplete: true,
    notifyOnPlaybackChange: true,
    logLevel: 'error',
    locale: 'zh-CN',
    volume: 1,
    themeRevision: -1,
    themePresetId: DEFAULT_THEME_PREFERENCES.presetId,
    themeCustomColors: {},
    colorScheme: DEFAULT_THEME_PREFERENCES.colorScheme ?? 'auto',
    dynamicAlbumAccent: DEFAULT_THEME_PREFERENCES.dynamicAlbumAccent ?? true,
    settingsLogRefreshToken: 0,
    prefsReady: false,
    isSaving: false,
    persistedSnapshot: '',
    lastSaveFailedSnapshot: '',
    dirty: {
      format: false,
      outputDir: false,
      downloadLyrics: false,
      notifyOnDownloadComplete: false,
      notifyOnPlaybackChange: false,
      logLevel: false,
      locale: false,
      theme: false,
    },
    suspendDirtyTracking: 0,
  };
}

function normalizeThemePreferences(preferences: AppPreferences) {
  return preferences.theme ?? DEFAULT_THEME_PREFERENCES;
}

function isRevisionMismatchError(error: unknown): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    error.code === 'revisionMismatch'
  );
}

function getStateSnapshot(state: SettingsState): string {
  return JSON.stringify({
    format: state.format,
    outputDir: state.outputDir,
    downloadLyrics: state.downloadLyrics,
    notifyOnDownloadComplete: state.notifyOnDownloadComplete,
    notifyOnPlaybackChange: state.notifyOnPlaybackChange,
    logLevel: state.logLevel,
    locale: state.locale,
    theme: {
      presetId: state.themePresetId,
      customColors: state.themeCustomColors,
      colorScheme: state.colorScheme,
      dynamicAlbumAccent: state.dynamicAlbumAccent,
    },
  });
}

function getThemeSnapshotForState(state: SettingsState): string {
  return JSON.stringify({
    presetId: state.themePresetId,
    customColors: state.themeCustomColors,
    colorScheme: state.colorScheme,
    dynamicAlbumAccent: state.dynamicAlbumAccent,
  });
}

function getThemeSnapshotForPreferences(preferences: AppPreferences): string {
  const theme = normalizeThemePreferences(preferences);
  return JSON.stringify({
    presetId: theme.presetId,
    customColors: theme.customColors,
    colorScheme: theme.colorScheme ?? 'auto',
    dynamicAlbumAccent: theme.dynamicAlbumAccent ?? true,
  });
}

function getPreferencesSnapshot(preferences: AppPreferences): string {
  const theme = normalizeThemePreferences(preferences);
  return JSON.stringify({
    format: preferences.outputFormat,
    outputDir: preferences.outputDir,
    downloadLyrics: preferences.downloadLyrics,
    notifyOnDownloadComplete: preferences.notifyOnDownloadComplete,
    notifyOnPlaybackChange: preferences.notifyOnPlaybackChange,
    logLevel: preferences.logLevel,
    locale: preferences.locale,
    theme: {
      presetId: theme.presetId,
      customColors: theme.customColors,
      colorScheme: theme.colorScheme ?? 'auto',
      dynamicAlbumAccent: theme.dynamicAlbumAccent ?? true,
    },
  });
}

function buildPreferencesPayload(state: SettingsState): AppPreferences {
  return {
    schemaVersion: state.schemaVersion,
    outputFormat: state.format,
    outputDir: state.outputDir,
    downloadLyrics: state.downloadLyrics,
    notifyOnDownloadComplete: state.notifyOnDownloadComplete,
    notifyOnPlaybackChange: state.notifyOnPlaybackChange,
    logLevel: state.logLevel,
    locale: state.locale,
    volume: state.volume,
    theme: {
      presetId: state.themePresetId,
      customColors: { ...state.themeCustomColors },
      colorScheme: state.colorScheme,
      dynamicAlbumAccent: state.dynamicAlbumAccent,
      // The backend owns and rewrites this field, while echo-style test or
      // browser adapters still need a monotonic snapshot-shaped payload.
      revision: Math.max(0, state.themeRevision),
    },
  };
}

function clearDirtyFieldsMatchingPreferences(
  state: SettingsState,
  preferences: AppPreferences
): void {
  const incoming = JSON.parse(getPreferencesSnapshot(preferences)) as Record<
    string,
    unknown
  >;
  const stateRef = state as unknown as Record<string, unknown>;
  for (const field of SCALAR_DIRTY_FIELDS) {
    if (stateRef[field] === incoming[field]) state.dirty[field] = false;
  }
  if (
    getThemeSnapshotForState(state) ===
    getThemeSnapshotForPreferences(preferences)
  ) {
    state.dirty.theme = false;
  }
}

/**
 * 创建应用设置状态控制器。
 *
 * 该控制器负责持有偏好设置的响应式状态、脏字段追踪、
 * 快照差异驱动的自动保存以及后端 hydration。调用方通过
 * `state` getter 拿到 `$state` 代理用于双向绑定，
 * 通过 `init()` 建立 dirty tracking / auto-save 副作用，
 * 通过 `dispose()` 拆除。所有状态变更规则（脏标记、保存快照、
 * 失败重试抑制）都在这里聚合，`appRuntime` 只做装配。
 */
export function createSettingsController(deps: SettingsControllerDeps) {
  const state = $state<SettingsState>(createInitialState());

  const lastObservedSettings = {
    format: state.format,
    outputDir: state.outputDir,
    downloadLyrics: state.downloadLyrics,
    notifyOnDownloadComplete: state.notifyOnDownloadComplete,
    notifyOnPlaybackChange: state.notifyOnPlaybackChange,
    logLevel: state.logLevel,
    locale: state.locale,
    theme: getThemeSnapshotForState(state),
  };

  let currentSavePromise: Promise<boolean> | null = null;
  let rootCleanup: (() => void) | null = null;
  let latestSnapshotMergeBaseline: SnapshotMergeBaseline | null = null;

  function captureSnapshotMergeBaseline(
    revision: number
  ): SnapshotMergeBaseline {
    const stateRef = state as unknown as Record<string, unknown>;
    return {
      revision,
      scalar: Object.fromEntries(
        SCALAR_DIRTY_FIELDS.map((field) => [field, stateRef[field]])
      ) as Record<ScalarDirtyField, unknown>,
      theme: getThemeSnapshotForState(state),
    };
  }

  function finishControlledStateUpdate(): void {
    const observed = lastObservedSettings as Record<string, unknown>;
    const stateRef = state as unknown as Record<string, unknown>;
    for (const field of SCALAR_DIRTY_FIELDS) {
      observed[field] = stateRef[field];
    }
    lastObservedSettings.theme = getThemeSnapshotForState(state);
    state.suspendDirtyTracking = Math.max(0, state.suspendDirtyTracking - 1);
  }

  function applyNormalizedTheme(
    theme: ReturnType<typeof normalizeThemePreferences>
  ): boolean {
    const incomingRevision = theme.revision ?? 0;
    if (incomingRevision < state.themeRevision) return false;
    state.themeRevision = incomingRevision;
    if (state.themePresetId !== theme.presetId) {
      state.themePresetId = theme.presetId;
    }
    const nextCustomColors = theme.customColors;
    if (
      JSON.stringify(state.themeCustomColors) !==
      JSON.stringify(nextCustomColors)
    ) {
      state.themeCustomColors = { ...nextCustomColors };
    }
    state.colorScheme = theme.colorScheme ?? 'auto';
    state.dynamicAlbumAccent = theme.dynamicAlbumAccent ?? true;
    return true;
  }

  async function hydratePreferences(options: HydrateSettingsOptions = {}) {
    try {
      const prefs = await deps.getPreferences();
      if (options.shouldDispose?.()) {
        return;
      }
      const incomingRevision = normalizeThemePreferences(prefs).revision ?? 0;
      if (incomingRevision < state.themeRevision) {
        // Subscription is established before hydration, so an event can win
        // this race. Never let the older command response roll it back.
        state.prefsReady = true;
        return;
      }
      state.suspendDirtyTracking += 1;
      state.schemaVersion = prefs.schemaVersion;
      if (!state.dirty.outputDir) {
        state.outputDir = prefs.outputDir || state.outputDir;
      }
      if (!state.dirty.format) {
        state.format = prefs.outputFormat;
      }
      if (!state.dirty.downloadLyrics) {
        state.downloadLyrics = prefs.downloadLyrics;
      }
      if (!state.dirty.notifyOnDownloadComplete) {
        state.notifyOnDownloadComplete = prefs.notifyOnDownloadComplete;
      }
      if (!state.dirty.notifyOnPlaybackChange) {
        state.notifyOnPlaybackChange = prefs.notifyOnPlaybackChange;
      }
      if (!state.dirty.logLevel) {
        state.logLevel = prefs.logLevel;
      }
      if (!state.dirty.locale) {
        state.locale = prefs.locale;
      }
      const theme = normalizeThemePreferences(prefs);
      if (!state.dirty.theme) {
        applyNormalizedTheme(theme);
      } else {
        state.themeRevision = Math.max(
          state.themeRevision,
          theme.revision ?? 0
        );
      }
      state.volume = prefs.volume;
      state.persistedSnapshot = getPreferencesSnapshot(prefs);
      clearDirtyFieldsMatchingPreferences(state, prefs);
      state.lastSaveFailedSnapshot = '';
      state.prefsReady = true;
      finishControlledStateUpdate();
      if (!state.dirty.locale) deps.onLocaleChanged?.(prefs.locale);
    } catch {
      if (!options.shouldDispose?.()) {
        state.persistedSnapshot = getStateSnapshot(state);
        state.lastSaveFailedSnapshot = '';
        state.prefsReady = true;
      }
    }
  }

  /** Apply an authoritative snapshot already returned by an IPC command. */
  function applyPreferencesSnapshot(
    prefs: AppPreferences,
    options: ApplyPreferencesSnapshotOptions = {}
  ): boolean {
    const incomingRevision = normalizeThemePreferences(prefs).revision ?? 0;
    if (incomingRevision < state.themeRevision) return false;
    const incomingSnapshot = getPreferencesSnapshot(prefs);
    const isIdempotentReplay =
      incomingRevision === state.themeRevision &&
      incomingSnapshot === state.persistedSnapshot;
    // Import events can beat their forced command response. The pre-merge
    // baseline lets the replay replace older dirty values field by field while
    // preserving edits made after that event.
    const replayBaseline =
      options.force === true &&
      isIdempotentReplay &&
      latestSnapshotMergeBaseline?.revision === incomingRevision
        ? latestSnapshotMergeBaseline
        : null;
    const stateRef = state as unknown as Record<string, unknown>;
    const shouldForceScalar = (field: ScalarDirtyField): boolean =>
      options.force === true &&
      (!replayBaseline || stateRef[field] === replayBaseline.scalar[field]);
    const shouldForceTheme =
      options.force === true &&
      (!replayBaseline ||
        getThemeSnapshotForState(state) === replayBaseline.theme);
    if (incomingRevision > state.themeRevision) {
      latestSnapshotMergeBaseline =
        captureSnapshotMergeBaseline(incomingRevision);
    }

    state.suspendDirtyTracking += 1;
    state.schemaVersion = prefs.schemaVersion;
    if (shouldForceScalar('outputDir') || !state.dirty.outputDir) {
      state.outputDir = prefs.outputDir;
    }
    if (shouldForceScalar('format') || !state.dirty.format) {
      state.format = prefs.outputFormat;
    }
    if (shouldForceScalar('downloadLyrics') || !state.dirty.downloadLyrics) {
      state.downloadLyrics = prefs.downloadLyrics;
    }
    if (
      shouldForceScalar('notifyOnDownloadComplete') ||
      !state.dirty.notifyOnDownloadComplete
    ) {
      state.notifyOnDownloadComplete = prefs.notifyOnDownloadComplete;
    }
    if (
      shouldForceScalar('notifyOnPlaybackChange') ||
      !state.dirty.notifyOnPlaybackChange
    ) {
      state.notifyOnPlaybackChange = prefs.notifyOnPlaybackChange;
    }
    if (shouldForceScalar('logLevel') || !state.dirty.logLevel) {
      state.logLevel = prefs.logLevel;
    }
    if (shouldForceScalar('locale') || !state.dirty.locale) {
      state.locale = prefs.locale;
    }
    if (shouldForceTheme || !state.dirty.theme) {
      applyNormalizedTheme(normalizeThemePreferences(prefs));
    } else {
      state.themeRevision = Math.max(state.themeRevision, incomingRevision);
    }
    state.volume = prefs.volume;
    state.persistedSnapshot = incomingSnapshot;
    state.lastSaveFailedSnapshot = '';
    state.prefsReady = true;
    clearDirtyFieldsMatchingPreferences(state, prefs);
    finishControlledStateUpdate();
    if (!state.dirty.locale) deps.onLocaleChanged?.(prefs.locale);
    return true;
  }

  function applyDefaultOutputDir(value: string) {
    if (value && !state.outputDir) {
      state.outputDir = value;
    }
  }

  async function savePreferences(): Promise<boolean> {
    if (state.isSaving && currentSavePromise) {
      await currentSavePromise;
      const nextSnapshot = getStateSnapshot(state);
      if (nextSnapshot === state.persistedSnapshot) {
        return true;
      }
      if (nextSnapshot === state.lastSaveFailedSnapshot) {
        return false;
      }
      return savePreferences();
    }

    let failedSnapshot = getStateSnapshot(state);
    // volume 字段由播放器子系统（set_playback_volume）单独持久化，也不在设置面板
    // UI 里编辑。后端 `set_preferences` 命令会忽略请求里的 volume 字段并保留当前
    // 后端值，因此这里只把本地缓存的 `state.volume` 一并传上去做兜底，防止未来
    // 后端语义改变时导致必填字段缺失。
    const prefs = buildPreferencesPayload(state);

    state.isSaving = true;
    currentSavePromise = (async () => {
      try {
        let submittedPrefs = prefs;
        let updated: AppPreferences;
        try {
          updated = await deps.setPreferences(
            submittedPrefs,
            Math.max(0, state.themeRevision)
          );
        } catch (error) {
          if (!isRevisionMismatchError(error)) throw error;

          // Another window committed first. Merge its authoritative snapshot
          // into non-dirty fields, keep this window's local intent, then retry
          // exactly once against the new revision.
          const authoritative = await deps.getPreferences();
          applyPreferencesSnapshot(authoritative);
          if (getStateSnapshot(state) === state.persistedSnapshot) {
            return true;
          }
          submittedPrefs = buildPreferencesPayload(state);
          failedSnapshot = getStateSnapshot(state);
          updated = await deps.setPreferences(
            submittedPrefs,
            Math.max(0, state.themeRevision)
          );
        }
        const updatedTheme = normalizeThemePreferences(updated);
        const updatedRevision = updatedTheme.revision ?? 0;
        if (updatedRevision < state.themeRevision) {
          // A newer cross-window snapshot already superseded this response.
          // Keep local dirty fields intact so auto-save can rebase them on the
          // latest revision instead of treating the old response as an ack.
          state.lastSaveFailedSnapshot = '';
          return true;
        }

        const unchangedSinceRequest = {
          format: state.format === submittedPrefs.outputFormat,
          outputDir: state.outputDir === submittedPrefs.outputDir,
          downloadLyrics:
            state.downloadLyrics === submittedPrefs.downloadLyrics,
          notifyOnDownloadComplete:
            state.notifyOnDownloadComplete ===
            submittedPrefs.notifyOnDownloadComplete,
          notifyOnPlaybackChange:
            state.notifyOnPlaybackChange ===
            submittedPrefs.notifyOnPlaybackChange,
          logLevel: state.logLevel === submittedPrefs.logLevel,
          locale: state.locale === submittedPrefs.locale,
          theme:
            getThemeSnapshotForState(state) ===
            getThemeSnapshotForPreferences(submittedPrefs),
        };

        state.suspendDirtyTracking += 1;
        state.schemaVersion = updated.schemaVersion;
        if (unchangedSinceRequest.format) {
          state.format = updated.outputFormat;
        }
        if (unchangedSinceRequest.outputDir) {
          state.outputDir = updated.outputDir;
        }
        if (unchangedSinceRequest.downloadLyrics) {
          state.downloadLyrics = updated.downloadLyrics;
        }
        if (unchangedSinceRequest.notifyOnDownloadComplete) {
          state.notifyOnDownloadComplete = updated.notifyOnDownloadComplete;
        }
        if (unchangedSinceRequest.notifyOnPlaybackChange) {
          state.notifyOnPlaybackChange = updated.notifyOnPlaybackChange;
        }
        if (unchangedSinceRequest.logLevel) {
          state.logLevel = updated.logLevel;
        }
        if (unchangedSinceRequest.locale) {
          state.locale = updated.locale;
        }
        if (unchangedSinceRequest.theme) {
          applyNormalizedTheme(updatedTheme);
        } else {
          state.themeRevision = Math.max(state.themeRevision, updatedRevision);
        }
        state.persistedSnapshot = getPreferencesSnapshot(updated);
        state.dirty.format = state.format !== updated.outputFormat;
        state.dirty.outputDir = state.outputDir !== updated.outputDir;
        state.dirty.downloadLyrics =
          state.downloadLyrics !== updated.downloadLyrics;
        state.dirty.notifyOnDownloadComplete =
          state.notifyOnDownloadComplete !== updated.notifyOnDownloadComplete;
        state.dirty.notifyOnPlaybackChange =
          state.notifyOnPlaybackChange !== updated.notifyOnPlaybackChange;
        state.dirty.logLevel = state.logLevel !== updated.logLevel;
        state.dirty.locale = state.locale !== updated.locale;
        state.dirty.theme =
          getThemeSnapshotForState(state) !==
          getThemeSnapshotForPreferences(updated);
        state.lastSaveFailedSnapshot = '';
        finishControlledStateUpdate();
        if (unchangedSinceRequest.locale) {
          deps.onLocaleChanged?.(updated.locale);
        }
        return true;
      } catch (error) {
        state.lastSaveFailedSnapshot = failedSnapshot;
        deps.notifyError(
          m.shell_error_save_settings_failed({
            error: formatPreferencesError(error),
          })
        );
        return false;
      } finally {
        state.isSaving = false;
        currentSavePromise = null;
      }
    })();

    return currentSavePromise;
  }

  function handleAppError(settingsOpen: boolean) {
    if (settingsOpen) {
      state.settingsLogRefreshToken += 1;
    }
  }

  /**
   * 建立 dirty tracking 与 auto-save 副作用。
   *
   * 必须在 Svelte 组件生命周期作用域内调用，会在 `dispose()` 时统一清理。
   * 多次调用只有第一次生效，后续调用为空操作。
   */
  function init() {
    if (rootCleanup) return;
    rootCleanup = $effect.root(() => {
      $effect(() => {
        const observed = lastObservedSettings as Record<string, unknown>;
        const stateRef = state as unknown as Record<string, unknown>;
        if (state.suspendDirtyTracking > 0) {
          for (const field of SCALAR_DIRTY_FIELDS) {
            observed[field] = stateRef[field];
          }
          return;
        }
        for (const field of SCALAR_DIRTY_FIELDS) {
          const value = stateRef[field];
          if (value !== observed[field]) {
            state.dirty[field] = true;
            observed[field] = value;
          }
        }
      });

      $effect(() => {
        const value = getThemeSnapshotForState(state);
        if (state.suspendDirtyTracking > 0) {
          lastObservedSettings.theme = value;
          return;
        }
        if (value !== lastObservedSettings.theme) {
          state.dirty.theme = true;
          lastObservedSettings.theme = value;
        }
      });

      $effect(() => {
        const {
          persistedSnapshot,
          isSaving,
          lastSaveFailedSnapshot,
          prefsReady,
        } = state;
        if (!prefsReady || isSaving) return;
        const currentSnapshot = getStateSnapshot(state);
        if (currentSnapshot === persistedSnapshot) return;
        if (currentSnapshot === lastSaveFailedSnapshot) return;
        void savePreferences();
      });
    });
  }

  function dispose() {
    rootCleanup?.();
    rootCleanup = null;
  }

  return {
    get state() {
      return state;
    },
    init,
    dispose,
    hydratePreferences,
    applyPreferencesSnapshot,
    applyDefaultOutputDir,
    savePreferences,
    handleAppError,
  };
}

export type SettingsController = ReturnType<typeof createSettingsController>;
