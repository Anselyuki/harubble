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

function normalizeThemePreferences(preferences: AppPreferences) {
  return preferences.theme ?? DEFAULT_THEME_PREFERENCES;
}

export function createSettingsController(deps: SettingsControllerDeps) {
  let initialized = false;
  let currentSavePromise: Promise<boolean> | null = null;

  function init() {
    if (initialized) return;
    initialized = true;
  }

  function getSnapshot(state: SettingsState): string {
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
      },
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

  async function hydratePreferences(
    state: SettingsState,
    options: HydrateSettingsOptions = {}
  ) {
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
        state.themePresetId = theme.presetId;
        state.themeCustomColors = { ...theme.customColors };
        state.colorScheme = theme.colorScheme ?? 'auto';
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
        state.persistedSnapshot = getSnapshot(state);
        state.lastSaveFailedSnapshot = '';
        state.prefsReady = true;
      }
    }
  }

  function applyDefaultOutputDir(state: SettingsState, value: string) {
    if (value && !state.outputDir) {
      state.outputDir = value;
    }
  }

  async function savePreferences(state: SettingsState): Promise<boolean> {
    if (state.isSaving && currentSavePromise) {
      await currentSavePromise;
      const nextSnapshot = getSnapshot(state);
      if (nextSnapshot === state.persistedSnapshot) {
        return true;
      }
      if (nextSnapshot === state.lastSaveFailedSnapshot) {
        return false;
      }
      return savePreferences(state);
    }

    const requestSnapshot = getSnapshot(state);
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
      },
    };

    state.isSaving = true;
    currentSavePromise = (async () => {
      try {
        const updated = await deps.setPreferences(prefs);
        const currentSnapshot = getSnapshot(state);
        if (currentSnapshot === requestSnapshot) {
          state.format = updated.outputFormat;
          state.outputDir = updated.outputDir;
          state.downloadLyrics = updated.downloadLyrics;
          state.notifyOnDownloadComplete = updated.notifyOnDownloadComplete;
          state.notifyOnPlaybackChange = updated.notifyOnPlaybackChange;
          state.logLevel = updated.logLevel;
          state.locale = updated.locale;
          const updatedTheme = normalizeThemePreferences(updated);
          state.themePresetId = updatedTheme.presetId;
          state.themeCustomColors = { ...updatedTheme.customColors };
          state.colorScheme = updatedTheme.colorScheme ?? 'auto';
          state.persistedSnapshot = getSnapshot(state);
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

  function handleAppError(state: SettingsState, settingsOpen: boolean) {
    if (settingsOpen) {
      state.settingsLogRefreshToken += 1;
    }
  }

  function dispose() {
    initialized = false;
  }

  return {
    init,
    dispose,
    hydratePreferences,
    applyDefaultOutputDir,
    savePreferences,
    handleAppError,
  };
}
