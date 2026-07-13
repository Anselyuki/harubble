import { describe, expect, it, vi } from 'vitest';
import { DEFAULT_THEME_PREFERENCES } from '$lib/themePresets';
import type { AppPreferences } from '$lib/types';
import { createSettingsController } from './settings.svelte';

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

function flushMicroAndTimers() {
  return new Promise<void>((resolve) => setTimeout(resolve, 0));
}

describe('createSettingsController - hydratePreferences', () => {
  it('populates state from backend and marks prefs ready', async () => {
    const ctrl = createSettingsController({
      getPreferences: async () =>
        createPreferences({
          outputFormat: 'wav',
          outputDir: '/mnt/music',
          locale: 'en-US',
        }),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    await ctrl.hydratePreferences();
    expect(ctrl.state.format).toBe('wav');
    expect(ctrl.state.outputDir).toBe('/mnt/music');
    expect(ctrl.state.locale).toBe('en-US');
    expect(ctrl.state.prefsReady).toBe(true);
  });

  it('hydrates missing theme preferences to the default theme state', async () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    ctrl.state.themePresetId = 'night-console';
    ctrl.state.themeCustomColors = { accent: '#111111' };
    ctrl.state.dirty.theme = false;

    await ctrl.hydratePreferences();

    expect(ctrl.state.themePresetId).toBe(DEFAULT_THEME_PREFERENCES.presetId);
    expect(ctrl.state.themeCustomColors).toEqual({});
    expect(ctrl.state.prefsReady).toBe(true);
  });

  it('preserves dirty theme edits when hydrating legacy preferences without theme', async () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    ctrl.state.themePresetId = 'night-console';
    ctrl.state.themeCustomColors = { accent: '#111111' };
    ctrl.state.dirty.theme = true;

    await ctrl.hydratePreferences();

    expect(ctrl.state.themePresetId).toBe('night-console');
    expect(ctrl.state.themeCustomColors).toEqual({ accent: '#111111' });
    expect(ctrl.state.dirty.theme).toBe(true);
    expect(ctrl.state.prefsReady).toBe(true);
  });

  it('preserves dirty scalar edits and overrides non-dirty scalars', async () => {
    const ctrl = createSettingsController({
      getPreferences: async () =>
        createPreferences({ outputFormat: 'mp3', locale: 'en-US' }),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    // 用户已经改过 format 但没保存
    ctrl.state.format = 'wav';
    ctrl.state.dirty.format = true;

    await ctrl.hydratePreferences();

    expect(ctrl.state.format).toBe('wav');
    expect(ctrl.state.locale).toBe('en-US');
  });

  it('handles hydration cancellation via shouldDispose', async () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences({ outputFormat: 'wav' }),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    await ctrl.hydratePreferences({ shouldDispose: () => true });
    expect(ctrl.state.format).toBe('flac'); // 未被覆盖
    expect(ctrl.state.prefsReady).toBe(false);
  });

  it('marks prefsReady true even when getPreferences rejects', async () => {
    const ctrl = createSettingsController({
      getPreferences: async () => {
        throw new Error('unavailable');
      },
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    await ctrl.hydratePreferences();
    expect(ctrl.state.prefsReady).toBe(true);
    expect(ctrl.state.persistedSnapshot).not.toBe('');
  });
});

describe('createSettingsController - savePreferences', () => {
  it('saves theme preferences in the full preference payload', async () => {
    const setPreferences = vi.fn(
      async (preferences: AppPreferences) => preferences
    );
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences,
      notifyError: vi.fn(),
    });
    ctrl.state.themePresetId = 'clear-aqua';
    ctrl.state.themeCustomColors = { accent: '#123ABC', danger: '#AA3300' };

    await ctrl.savePreferences();

    expect(setPreferences).toHaveBeenCalledWith(
      expect.objectContaining({
        theme: {
          presetId: 'clear-aqua',
          customColors: { accent: '#123ABC', danger: '#AA3300' },
          colorScheme: 'auto',
          dynamicAlbumAccent: true,
        },
      })
    );
    expect(ctrl.state.dirty.theme).toBe(false);
  });

  it('delegates volume preservation to the backend on save', async () => {
    const setPreferences = vi.fn(
      async (preferences: AppPreferences) => preferences
    );
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences({ volume: 0.4 }),
      setPreferences,
      notifyError: vi.fn(),
    });
    ctrl.state.volume = 1;
    await ctrl.savePreferences();
    expect(setPreferences).toHaveBeenCalledWith(
      expect.objectContaining({ volume: 1 })
    );
  });

  it('clears all dirty flags and records persisted snapshot on success', async () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: async (prefs) => prefs,
      notifyError: vi.fn(),
    });
    ctrl.state.dirty.format = true;
    ctrl.state.dirty.locale = true;
    ctrl.state.dirty.theme = true;

    await ctrl.savePreferences();

    expect(ctrl.state.dirty.format).toBe(false);
    expect(ctrl.state.dirty.locale).toBe(false);
    expect(ctrl.state.dirty.theme).toBe(false);
    expect(ctrl.state.persistedSnapshot).not.toBe('');
    expect(ctrl.state.lastSaveFailedSnapshot).toBe('');
    expect(ctrl.state.isSaving).toBe(false);
  });

  it('records lastSaveFailedSnapshot and notifies on failure', async () => {
    const notifyError = vi.fn();
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: async () => {
        throw new Error('disk full');
      },
      notifyError,
    });
    ctrl.state.dirty.format = true;

    const ok = await ctrl.savePreferences();

    expect(ok).toBe(false);
    expect(ctrl.state.lastSaveFailedSnapshot).not.toBe('');
    expect(notifyError).toHaveBeenCalledTimes(1);
    // Dirty flags 未被清除，允许后续重试。
    expect(ctrl.state.dirty.format).toBe(true);
  });

  it('when an in-flight save with unchanged snapshot exists, later save resolves without extra IPC', async () => {
    let resolve!: (v: AppPreferences) => void;
    const setPreferences = vi
      .fn<(prefs: AppPreferences) => Promise<AppPreferences>>()
      .mockImplementationOnce(
        (prefs) =>
          new Promise((r) => {
            resolve = () => r(prefs);
          })
      );
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences,
      notifyError: vi.fn(),
    });
    ctrl.state.format = 'wav';

    const p1 = ctrl.savePreferences();
    const p2 = ctrl.savePreferences();
    resolve(createPreferences({ outputFormat: 'wav' }));

    const [r1, r2] = await Promise.all([p1, p2]);
    expect(r1).toBe(true);
    expect(r2).toBe(true);
    // 第二个 save 走的是「等前一个 save 完成后 snapshot 与 persisted 相同」的短路，
    // 因此不会再次触发 setPreferences。
    expect(setPreferences).toHaveBeenCalledTimes(1);
  });

  it('when snapshot changes during in-flight save, the follow-up save is queued and sent', async () => {
    const setPreferences =
      vi.fn<(prefs: AppPreferences) => Promise<AppPreferences>>();
    let resolveFirst!: (v: AppPreferences) => void;
    setPreferences
      .mockImplementationOnce(
        () =>
          new Promise((r) => {
            resolveFirst = r;
          })
      )
      .mockImplementationOnce(async (prefs) => prefs);

    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences,
      notifyError: vi.fn(),
    });

    ctrl.state.format = 'wav';
    const p1 = ctrl.savePreferences();

    // 保存进行中时用户又改了字段
    ctrl.state.format = 'mp3';
    const p2 = ctrl.savePreferences();

    resolveFirst(createPreferences({ outputFormat: 'wav' }));

    await Promise.all([p1, p2]);
    expect(setPreferences).toHaveBeenCalledTimes(2);
    expect(setPreferences.mock.calls[1]?.[0]?.outputFormat).toBe('mp3');
  });

  it('when the queued snapshot equals lastSaveFailedSnapshot, follow-up short-circuits to false', async () => {
    const setPreferences =
      vi.fn<(prefs: AppPreferences) => Promise<AppPreferences>>();
    let rejectFirst!: (e: Error) => void;
    setPreferences.mockImplementationOnce(
      () =>
        new Promise((_r, rej) => {
          rejectFirst = rej;
        })
    );

    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences,
      notifyError: vi.fn(),
    });
    ctrl.state.format = 'wav';
    const p1 = ctrl.savePreferences();
    const p2 = ctrl.savePreferences();

    rejectFirst(new Error('io'));
    const [r1, r2] = await Promise.all([p1, p2]);
    expect(r1).toBe(false);
    expect(r2).toBe(false);
    // 第二个 save 命中 lastSaveFailedSnapshot 短路，无需再次发起 IPC
    expect(setPreferences).toHaveBeenCalledTimes(1);
  });
});

describe('createSettingsController - handleAppError', () => {
  it('increments log refresh token only when settings panel is open', () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    ctrl.handleAppError(false);
    expect(ctrl.state.settingsLogRefreshToken).toBe(0);
    ctrl.handleAppError(true);
    expect(ctrl.state.settingsLogRefreshToken).toBe(1);
    ctrl.handleAppError(true);
    expect(ctrl.state.settingsLogRefreshToken).toBe(2);
  });
});

describe('createSettingsController - applyDefaultOutputDir', () => {
  it('sets outputDir only when currently empty', () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    ctrl.applyDefaultOutputDir('/home/music');
    expect(ctrl.state.outputDir).toBe('/home/music');

    ctrl.applyDefaultOutputDir('/other');
    expect(ctrl.state.outputDir).toBe('/home/music');
  });

  it('is a no-op when the provided default is empty', () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    ctrl.applyDefaultOutputDir('');
    expect(ctrl.state.outputDir).toBe('');
  });
});

describe('createSettingsController - hydration does not surface as user dirty', () => {
  // 依赖 init() 建立的 $effect：hydrate 期间对 state 的赋值会被 suspendDirtyTracking
  // 计数器抑制，不允许翻转 dirty 标记。这里通过跑 init 后 hydrate、再等 timer 让 tracking
  // 恢复，然后确认 dirty 全 false 来验证。
  it('does not mark fields dirty after a fresh hydrate', async () => {
    const ctrl = createSettingsController({
      getPreferences: async () =>
        createPreferences({
          outputFormat: 'wav',
          outputDir: '/mnt/music',
          locale: 'en-US',
        }),
      setPreferences: async (prefs) => prefs,
      notifyError: vi.fn(),
    });
    ctrl.init();
    await ctrl.hydratePreferences();
    await flushMicroAndTimers();

    expect(ctrl.state.dirty.format).toBe(false);
    expect(ctrl.state.dirty.outputDir).toBe(false);
    expect(ctrl.state.dirty.locale).toBe(false);
    expect(ctrl.state.dirty.theme).toBe(false);
    ctrl.dispose();
  });
});
