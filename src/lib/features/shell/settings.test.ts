import { describe, expect, it, vi } from 'vitest';
import { DEFAULT_THEME_PREFERENCES } from '$lib/themePresets';
import type { AppPreferences } from '$lib/types';
import {
  createSettingsController,
  type SettingsState,
} from './settings.svelte';

function createState(): SettingsState {
  return {
    format: 'flac',
    outputDir: '/tmp',
    downloadLyrics: true,
    notifyOnDownloadComplete: true,
    notifyOnPlaybackChange: true,
    logLevel: 'error',
    locale: 'zh-CN',
    volume: 1,
    themePresetId: DEFAULT_THEME_PREFERENCES.presetId,
    themeCustomColors: {},
    colorScheme: 'auto',
    dynamicAlbumAccent: true,
    settingsLogRefreshToken: 0,
    prefsReady: true,
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
      theme: true,
    },
    suspendDirtyTracking: 0,
  };
}

function createPreferences(
  overrides: Partial<AppPreferences> = {}
): AppPreferences {
  return {
    outputFormat: 'flac',
    outputDir: '/tmp',
    downloadLyrics: true,
    notifyOnDownloadComplete: true,
    notifyOnPlaybackChange: true,
    logLevel: 'error',
    locale: 'zh-CN',
    volume: 1,
    ...overrides,
  };
}

describe('createSettingsController theme preferences', () => {
  it('hydrates missing theme preferences to the default theme state', async () => {
    const state = createState();
    state.dirty.theme = false;
    state.themePresetId = 'night-console';
    state.themeCustomColors = { accent: '#111111' };
    const controller = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });

    await controller.hydratePreferences(state);

    expect(state.themePresetId).toBe(DEFAULT_THEME_PREFERENCES.presetId);
    expect(state.themeCustomColors).toEqual({});
    expect(state.prefsReady).toBe(true);
  });

  it('preserves dirty theme edits when hydrating legacy preferences without theme', async () => {
    const state = createState();
    state.themePresetId = 'night-console';
    state.themeCustomColors = { accent: '#111111' };
    const controller = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });

    await controller.hydratePreferences(state);

    expect(state.themePresetId).toBe('night-console');
    expect(state.themeCustomColors).toEqual({ accent: '#111111' });
    expect(state.dirty.theme).toBe(true);
    expect(state.prefsReady).toBe(true);
  });

  it('saves theme preferences in the full preference payload', async () => {
    const state = createState();
    state.themePresetId = 'clear-aqua';
    state.themeCustomColors = { accent: '#123ABC', danger: '#AA3300' };
    const setPreferences = vi.fn(
      async (preferences: AppPreferences) => preferences
    );
    const controller = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences,
      notifyError: vi.fn(),
    });

    await controller.savePreferences(state);

    expect(setPreferences).toHaveBeenCalledWith(
      expect.objectContaining({
        theme: {
          presetId: 'clear-aqua',
          customColors: {
            accent: '#123ABC',
            danger: '#AA3300',
          },
          colorScheme: 'auto',
          dynamicAlbumAccent: true,
        },
      })
    );
    expect(state.dirty.theme).toBe(false);
  });

  it('delegates volume preservation to the backend on save', async () => {
    // 后端 set_preferences 命令负责保留当前 volume；前端只是把本地缓存的值一并
    // 传上去做兜底。这里断言 saved payload 里的 volume 就是当前 state.volume，
    // 后端应对 volume 覆盖负责（在集成/命令测试里覆盖）。
    const state = createState();
    state.volume = 1;
    const setPreferences = vi.fn(
      async (preferences: AppPreferences) => preferences
    );
    const controller = createSettingsController({
      getPreferences: async () => createPreferences({ volume: 0.4 }),
      setPreferences,
      notifyError: vi.fn(),
    });

    await controller.savePreferences(state);

    expect(setPreferences).toHaveBeenCalledWith(
      expect.objectContaining({ volume: 1 })
    );
  });
});
