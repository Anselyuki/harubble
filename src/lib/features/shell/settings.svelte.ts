import * as m from '$lib/paraglide/messages.js';
import type { Locale } from '$lib/i18n/types';
import type { AppPreferences, LogLevel, OutputFormat } from '$lib/types';
import {
  DEFAULT_THEME_PREFERENCES,
  type ColorScheme,
  type ThemeColorSlots,
} from '$lib/themePresets';

interface SettingsControllerDeps {
  getPreferences: () => Promise<AppPreferences>;
  setPreferences: (preferences: AppPreferences) => Promise<AppPreferences>;
  notifyError: (message: string) => void;
  onLocaleChanged?: (locale: Locale) => void;
}

interface HydrateSettingsOptions {
  shouldDispose?: () => boolean;
}

export interface SettingsState {
  format: OutputFormat;
  outputDir: string;
  downloadLyrics: boolean;
  notifyOnDownloadComplete: boolean;
  notifyOnPlaybackChange: boolean;
  logLevel: LogLevel;
  locale: Locale;
  volume: number;
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

function createInitialState(): SettingsState {
  return {
    format: 'flac',
    outputDir: '',
    downloadLyrics: true,
    notifyOnDownloadComplete: true,
    notifyOnPlaybackChange: true,
    logLevel: 'error',
    locale: 'zh-CN',
    volume: 1,
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

function getPreferencesSnapshot(preferences: AppPreferences): string {
  return JSON.stringify({
    format: preferences.outputFormat,
    outputDir: preferences.outputDir,
    downloadLyrics: preferences.downloadLyrics,
    notifyOnDownloadComplete: preferences.notifyOnDownloadComplete,
    notifyOnPlaybackChange: preferences.notifyOnPlaybackChange,
    logLevel: preferences.logLevel,
    locale: preferences.locale,
    theme: preferences.theme ?? DEFAULT_THEME_PREFERENCES,
  });
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

  function applyNormalizedTheme(
    theme: ReturnType<typeof normalizeThemePreferences>
  ) {
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
  }

  async function hydratePreferences(options: HydrateSettingsOptions = {}) {
    try {
      const prefs = await deps.getPreferences();
      if (options.shouldDispose?.()) {
        return;
      }
      state.suspendDirtyTracking += 1;
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
      }
      state.volume = prefs.volume;
      deps.onLocaleChanged?.(prefs.locale);
      state.persistedSnapshot = getPreferencesSnapshot(prefs);
      state.lastSaveFailedSnapshot = '';
      state.prefsReady = true;
      setTimeout(() => {
        state.suspendDirtyTracking = Math.max(
          0,
          state.suspendDirtyTracking - 1
        );
      }, 0);
    } catch {
      if (!options.shouldDispose?.()) {
        state.persistedSnapshot = getStateSnapshot(state);
        state.lastSaveFailedSnapshot = '';
        state.prefsReady = true;
      }
    }
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

    const requestSnapshot = getStateSnapshot(state);
    // volume 字段由播放器子系统（set_playback_volume）单独持久化，也不在设置面板
    // UI 里编辑。后端 `set_preferences` 命令会忽略请求里的 volume 字段并保留当前
    // 后端值，因此这里只把本地缓存的 `state.volume` 一并传上去做兜底，防止未来
    // 后端语义改变时导致必填字段缺失。
    const prefs: AppPreferences = {
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
      },
    };

    state.isSaving = true;
    currentSavePromise = (async () => {
      try {
        const updated = await deps.setPreferences(prefs);
        const currentSnapshot = getStateSnapshot(state);
        if (currentSnapshot === requestSnapshot) {
          state.suspendDirtyTracking += 1;
          state.format = updated.outputFormat;
          state.outputDir = updated.outputDir;
          state.downloadLyrics = updated.downloadLyrics;
          state.notifyOnDownloadComplete = updated.notifyOnDownloadComplete;
          state.notifyOnPlaybackChange = updated.notifyOnPlaybackChange;
          state.logLevel = updated.logLevel;
          state.locale = updated.locale;
          const updatedTheme = normalizeThemePreferences(updated);
          applyNormalizedTheme(updatedTheme);
          state.persistedSnapshot = getStateSnapshot(state);
          setTimeout(() => {
            state.suspendDirtyTracking = Math.max(
              0,
              state.suspendDirtyTracking - 1
            );
          }, 0);
        } else {
          state.persistedSnapshot = requestSnapshot;
        }
        deps.onLocaleChanged?.(updated.locale);
        state.dirty.format = false;
        state.dirty.outputDir = false;
        state.dirty.downloadLyrics = false;
        state.dirty.notifyOnDownloadComplete = false;
        state.dirty.notifyOnPlaybackChange = false;
        state.dirty.logLevel = false;
        state.dirty.locale = false;
        state.dirty.theme = false;
        state.lastSaveFailedSnapshot = '';
        return true;
      } catch (error) {
        state.lastSaveFailedSnapshot = requestSnapshot;
        deps.notifyError(
          m.shell_error_save_settings_failed({
            error: error instanceof Error ? error.message : String(error),
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
    applyDefaultOutputDir,
    savePreferences,
    handleAppError,
  };
}

export type SettingsController = ReturnType<typeof createSettingsController>;
