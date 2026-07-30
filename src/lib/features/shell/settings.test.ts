// @vitest-environment jsdom

import { describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import { DEFAULT_THEME_PREFERENCES } from '$lib/themePresets';
import type { AppPreferences } from '$lib/types';
import { createSettingsController } from './settings.svelte';

function createPreferences(
  overrides: Partial<AppPreferences> = {}
): AppPreferences {
  return {
    schemaVersion: 2,
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

  it('normalizes package-owned theme fields out of the persisted UI snapshot', async () => {
    const setPreferences = vi.fn(
      async (preferences: AppPreferences) => preferences
    );
    const ctrl = createSettingsController({
      getPreferences: async () =>
        createPreferences({
          theme: {
            ...DEFAULT_THEME_PREFERENCES,
            activePackageId: 'ark-ui-endfield',
            revision: 7,
          },
        }),
      setPreferences,
      notifyError: vi.fn(),
    });
    ctrl.init();

    await ctrl.hydratePreferences();
    await flushMicroAndTimers();

    expect(setPreferences).not.toHaveBeenCalled();
    expect(ctrl.state.persistedSnapshot).not.toContain('activePackageId');
    expect(ctrl.state.persistedSnapshot).not.toContain('revision');
    ctrl.dispose();
  });

  it('applies an authoritative imported snapshot without writing it back', async () => {
    const setPreferences = vi.fn(
      async (preferences: AppPreferences) => preferences
    );
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences,
      notifyError: vi.fn(),
    });
    ctrl.init();

    ctrl.applyPreferencesSnapshot(
      createPreferences({
        outputFormat: 'wav',
        locale: 'en-US',
        theme: {
          ...DEFAULT_THEME_PREFERENCES,
          presetId: 'clear-aqua',
          activePackageId: 'ark-ui-ark',
          revision: 9,
        },
      })
    );
    await flushMicroAndTimers();

    expect(ctrl.state.format).toBe('wav');
    expect(ctrl.state.locale).toBe('en-US');
    expect(ctrl.state.themePresetId).toBe('clear-aqua');
    expect(setPreferences).not.toHaveBeenCalled();
    ctrl.dispose();
  });

  it('rejects an older theme revision before it can roll back the window state', () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });

    expect(
      ctrl.applyPreferencesSnapshot(
        createPreferences({
          outputFormat: 'wav',
          theme: {
            ...DEFAULT_THEME_PREFERENCES,
            presetId: 'clear-aqua',
            revision: 5,
          },
        })
      )
    ).toBe(true);
    expect(
      ctrl.applyPreferencesSnapshot(
        createPreferences({
          outputFormat: 'mp3',
          theme: {
            ...DEFAULT_THEME_PREFERENCES,
            presetId: 'night-console',
            revision: 4,
          },
        })
      )
    ).toBe(false);

    expect(ctrl.state.format).toBe('wav');
    expect(ctrl.state.themePresetId).toBe('clear-aqua');
    expect(ctrl.state.themeRevision).toBe(5);
  });

  it('merges cross-window snapshots without overwriting dirty fields', () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    ctrl.state.format = 'wav';
    ctrl.state.dirty.format = true;
    ctrl.state.persistedSnapshot = JSON.stringify({
      format: 'flac',
      outputDir: '/old',
      downloadLyrics: true,
      notifyOnDownloadComplete: true,
      notifyOnPlaybackChange: true,
      logLevel: 'error',
      locale: 'zh-CN',
      theme: {
        presetId: DEFAULT_THEME_PREFERENCES.presetId,
        customColors: {},
        colorScheme: 'auto',
        dynamicAlbumAccent: true,
      },
    });

    expect(
      ctrl.applyPreferencesSnapshot(
        createPreferences({ outputFormat: 'mp3', outputDir: '/new' })
      )
    ).toBe(true);
    expect(ctrl.state.format).toBe('wav');
    expect(ctrl.state.outputDir).toBe('/new');
    expect(ctrl.state.dirty.format).toBe(true);
    expect(ctrl.state.dirty.outputDir).toBe(false);
    expect(JSON.parse(ctrl.state.persistedSnapshot).format).toBe('mp3');
    expect(JSON.parse(ctrl.state.persistedSnapshot).outputDir).toBe('/new');
  });

  it('keeps a dirty value pending when it equals the old baseline but differs from a newer event', () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    ctrl.state.persistedSnapshot = JSON.stringify({
      format: 'flac',
      outputDir: '/old',
      downloadLyrics: true,
      notifyOnDownloadComplete: true,
      notifyOnPlaybackChange: true,
      logLevel: 'error',
      locale: 'zh-CN',
      theme: {
        presetId: DEFAULT_THEME_PREFERENCES.presetId,
        customColors: {},
        colorScheme: 'auto',
        dynamicAlbumAccent: true,
      },
    });
    // The user changed away and back, so this dirty bit still represents a
    // local intent even though the value equals the old persisted baseline.
    ctrl.state.dirty.format = true;

    ctrl.applyPreferencesSnapshot(
      createPreferences({ outputFormat: 'mp3', outputDir: '/new' })
    );

    expect(ctrl.state.format).toBe('flac');
    expect(ctrl.state.dirty.format).toBe(true);
    expect(JSON.parse(ctrl.state.persistedSnapshot).format).toBe('mp3');
  });

  it('allows an explicit import to replace dirty fields and clear dirty state', () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    ctrl.state.format = 'wav';
    ctrl.state.dirty.format = true;

    ctrl.applyPreferencesSnapshot(createPreferences({ outputFormat: 'mp3' }), {
      force: true,
    });

    expect(ctrl.state.format).toBe('mp3');
    expect(ctrl.state.dirty.format).toBe(false);
  });

  it('does not let an older forced import response roll back a newer event', () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    ctrl.applyPreferencesSnapshot(
      createPreferences({
        outputFormat: 'wav',
        theme: {
          ...DEFAULT_THEME_PREFERENCES,
          presetId: 'night-console',
          revision: 3,
        },
      })
    );
    ctrl.state.dirty.format = true;
    ctrl.state.dirty.theme = true;

    expect(
      ctrl.applyPreferencesSnapshot(
        createPreferences({
          outputFormat: 'mp3',
          theme: {
            ...DEFAULT_THEME_PREFERENCES,
            presetId: 'clear-aqua',
            revision: 2,
          },
        }),
        { force: true }
      )
    ).toBe(false);

    expect(ctrl.state.themeRevision).toBe(3);
    expect(ctrl.state.format).toBe('wav');
    expect(ctrl.state.themePresetId).toBe('night-console');
    expect(ctrl.state.dirty.format).toBe(true);
    expect(ctrl.state.dirty.theme).toBe(true);
  });

  it('does not let a forced import replay erase edits made after its event', () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    const imported = createPreferences({
      outputFormat: 'wav',
      theme: {
        ...DEFAULT_THEME_PREFERENCES,
        presetId: 'clear-aqua',
        revision: 2,
      },
    });
    // The import event is delivered before invoke resolves.
    ctrl.applyPreferencesSnapshot(imported);
    ctrl.state.format = 'mp3';
    ctrl.state.themePresetId = 'night-console';
    ctrl.state.dirty.format = true;
    ctrl.state.dirty.theme = true;

    expect(ctrl.applyPreferencesSnapshot(imported, { force: true })).toBe(true);

    expect(ctrl.state.themeRevision).toBe(2);
    expect(ctrl.state.format).toBe('mp3');
    expect(ctrl.state.themePresetId).toBe('night-console');
    expect(ctrl.state.dirty.format).toBe(true);
    expect(ctrl.state.dirty.theme).toBe(true);
  });

  it('lets a forced import replay replace edits that predate its event', () => {
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });
    const imported = createPreferences({
      outputFormat: 'wav',
      theme: {
        ...DEFAULT_THEME_PREFERENCES,
        presetId: 'clear-aqua',
        revision: 2,
      },
    });
    ctrl.state.format = 'mp3';
    ctrl.state.themePresetId = 'night-console';
    ctrl.state.dirty.format = true;
    ctrl.state.dirty.theme = true;

    ctrl.applyPreferencesSnapshot(imported);
    expect(ctrl.state.format).toBe('mp3');
    expect(ctrl.state.themePresetId).toBe('night-console');

    ctrl.applyPreferencesSnapshot(imported, { force: true });

    expect(ctrl.state.format).toBe('wav');
    expect(ctrl.state.themePresetId).toBe('clear-aqua');
    expect(ctrl.state.dirty.format).toBe(false);
    expect(ctrl.state.dirty.theme).toBe(false);
  });

  it('ends remote-snapshot dirty suppression before the next browser task', async () => {
    const setPreferences = vi.fn(
      async (preferences: AppPreferences) => preferences
    );
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences,
      notifyError: vi.fn(),
    });
    ctrl.init();
    await flushMicroAndTimers();

    ctrl.applyPreferencesSnapshot(
      createPreferences({
        outputFormat: 'wav',
        theme: { ...DEFAULT_THEME_PREFERENCES, revision: 2 },
      })
    );
    // Svelte first observes the controller-owned writes while suppression is
    // active, then releases it in the same microtask checkpoint. A setTimeout
    // release leaves the next user-interaction task able to race it.
    await tick();
    await Promise.resolve();
    expect(ctrl.state.suspendDirtyTracking).toBe(0);
    expect(JSON.parse(ctrl.state.persistedSnapshot).format).toBe('wav');
    expect(setPreferences).not.toHaveBeenCalled();

    ctrl.state.format = 'mp3';
    await tick();
    expect(ctrl.state.dirty.format).toBe(true);
    await vi.waitFor(() =>
      expect(setPreferences).toHaveBeenCalledWith(
        expect.objectContaining({ outputFormat: 'mp3' }),
        2
      )
    );
    ctrl.dispose();
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

  it('does not publish a hydrated locale that was superseded by a dirty local edit', async () => {
    const onLocaleChanged = vi.fn();
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences({ locale: 'zh-CN' }),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
      onLocaleChanged,
    });
    ctrl.state.locale = 'en-US';
    ctrl.state.dirty.locale = true;

    await ctrl.hydratePreferences();

    expect(ctrl.state.locale).toBe('en-US');
    expect(ctrl.state.dirty.locale).toBe(true);
    expect(onLocaleChanged).not.toHaveBeenCalled();
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

  it('does not let an older hydration response roll back a newer preferences event', async () => {
    let resolveHydration!: (value: AppPreferences) => void;
    const ctrl = createSettingsController({
      getPreferences: () =>
        new Promise((resolve) => {
          resolveHydration = resolve;
        }),
      setPreferences: vi.fn(),
      notifyError: vi.fn(),
    });

    const hydration = ctrl.hydratePreferences();
    ctrl.applyPreferencesSnapshot(
      createPreferences({
        outputFormat: 'wav',
        theme: {
          ...DEFAULT_THEME_PREFERENCES,
          presetId: 'clear-aqua',
          revision: 3,
        },
      })
    );
    resolveHydration(
      createPreferences({
        outputFormat: 'mp3',
        theme: { ...DEFAULT_THEME_PREFERENCES, revision: 2 },
      })
    );

    await hydration;

    expect(ctrl.state.prefsReady).toBe(true);
    expect(ctrl.state.format).toBe('wav');
    expect(ctrl.state.themePresetId).toBe('clear-aqua');
    expect(ctrl.state.themeRevision).toBe(3);
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
        schemaVersion: 2,
        theme: {
          presetId: 'clear-aqua',
          customColors: { accent: '#123ABC', danger: '#AA3300' },
          colorScheme: 'auto',
          dynamicAlbumAccent: true,
          revision: 0,
        },
      }),
      0
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
      expect.objectContaining({ volume: 1 }),
      0
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

  it('rebases dirty local fields onto a newer cross-window snapshot and retries once', async () => {
    const mismatch = {
      code: 'revisionMismatch',
      detail: { currentRevision: 2, expectedRevision: 1 },
    };
    const remote = createPreferences({
      outputDir: '/remote/music',
      theme: {
        ...DEFAULT_THEME_PREFERENCES,
        activePackageId: 'remote-package',
        revision: 2,
      },
    });
    const setPreferences = vi
      .fn<
        (
          preferences: AppPreferences,
          expectedRevision: number
        ) => Promise<AppPreferences>
      >()
      .mockRejectedValueOnce(mismatch)
      .mockImplementationOnce(async (preferences, expectedRevision) =>
        createPreferences({
          ...preferences,
          theme: {
            ...DEFAULT_THEME_PREFERENCES,
            ...preferences.theme,
            activePackageId: 'remote-package',
            revision: expectedRevision + 1,
          },
        })
      );
    const getPreferences = vi.fn(async () => remote);
    const ctrl = createSettingsController({
      getPreferences,
      setPreferences,
      notifyError: vi.fn(),
    });
    ctrl.applyPreferencesSnapshot(
      createPreferences({
        theme: { ...DEFAULT_THEME_PREFERENCES, revision: 1 },
      })
    );
    ctrl.state.format = 'wav';
    ctrl.state.dirty.format = true;

    await expect(ctrl.savePreferences()).resolves.toBe(true);

    expect(getPreferences).toHaveBeenCalledOnce();
    expect(setPreferences).toHaveBeenCalledTimes(2);
    expect(setPreferences).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({ outputFormat: 'wav', outputDir: '/tmp' }),
      1
    );
    expect(setPreferences).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({
        outputFormat: 'wav',
        outputDir: '/remote/music',
      }),
      2
    );
    expect(ctrl.state.format).toBe('wav');
    expect(ctrl.state.outputDir).toBe('/remote/music');
    expect(ctrl.state.themeRevision).toBe(3);
    expect(ctrl.state.dirty.format).toBe(false);
  });

  it('does not retry an ordinary settings revision conflict more than once', async () => {
    const notifyError = vi.fn();
    const firstMismatch = {
      code: 'revisionMismatch',
      detail: { currentRevision: 2, expectedRevision: 1 },
    };
    const retryMismatch = {
      code: 'revisionMismatch',
      detail: { currentRevision: 3, expectedRevision: 2 },
    };
    const setPreferences = vi
      .fn<
        (
          preferences: AppPreferences,
          expectedRevision: number
        ) => Promise<AppPreferences>
      >()
      .mockRejectedValueOnce(firstMismatch)
      .mockRejectedValueOnce(retryMismatch);
    const ctrl = createSettingsController({
      getPreferences: vi.fn(async () =>
        createPreferences({
          theme: { ...DEFAULT_THEME_PREFERENCES, revision: 2 },
        })
      ),
      setPreferences,
      notifyError,
    });
    ctrl.applyPreferencesSnapshot(
      createPreferences({
        theme: { ...DEFAULT_THEME_PREFERENCES, revision: 1 },
      })
    );
    ctrl.state.format = 'wav';
    ctrl.state.dirty.format = true;

    await expect(ctrl.savePreferences()).resolves.toBe(false);

    expect(setPreferences).toHaveBeenCalledTimes(2);
    expect(notifyError).toHaveBeenCalledOnce();
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

  it('keeps a local theme dirty when a newer cross-window revision supersedes the in-flight save response', async () => {
    let resolveFirst!: (value: AppPreferences) => void;
    const setPreferences = vi
      .fn<(prefs: AppPreferences) => Promise<AppPreferences>>()
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveFirst = resolve;
          })
      )
      .mockImplementationOnce(async (prefs) =>
        createPreferences({
          ...prefs,
          theme: {
            ...DEFAULT_THEME_PREFERENCES,
            ...prefs.theme,
            activePackageId: 'remote-package',
            revision: 4,
          },
        })
      );
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences,
      notifyError: vi.fn(),
    });
    ctrl.applyPreferencesSnapshot(
      createPreferences({
        theme: { ...DEFAULT_THEME_PREFERENCES, revision: 1 },
      })
    );
    ctrl.state.themePresetId = 'clear-aqua';
    ctrl.state.dirty.theme = true;

    const firstSave = ctrl.savePreferences();
    // A second local intent is now pending behind the request for clear-aqua.
    ctrl.state.themePresetId = 'night-console';
    ctrl.applyPreferencesSnapshot(
      createPreferences({
        theme: {
          ...DEFAULT_THEME_PREFERENCES,
          activePackageId: 'remote-package',
          revision: 3,
        },
      })
    );
    resolveFirst(
      createPreferences({
        theme: {
          ...DEFAULT_THEME_PREFERENCES,
          presetId: 'clear-aqua',
          revision: 2,
        },
      })
    );

    await firstSave;

    expect(ctrl.state.themeRevision).toBe(3);
    expect(ctrl.state.themePresetId).toBe('night-console');
    expect(ctrl.state.dirty.theme).toBe(true);
    expect(setPreferences).toHaveBeenCalledTimes(1);

    await ctrl.savePreferences();

    expect(setPreferences).toHaveBeenCalledTimes(2);
    expect(setPreferences.mock.calls[1]?.[0]?.theme?.presetId).toBe(
      'night-console'
    );
    expect(ctrl.state.themeRevision).toBe(4);
    expect(ctrl.state.dirty.theme).toBe(false);
  });

  it('keeps a post-event theme edit when the matching save response replays the same revision', async () => {
    let resolveFirst!: (value: AppPreferences) => void;
    const setPreferences = vi
      .fn<(prefs: AppPreferences) => Promise<AppPreferences>>()
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveFirst = resolve;
          })
      )
      .mockImplementationOnce(async (prefs) =>
        createPreferences({
          ...prefs,
          theme: {
            ...DEFAULT_THEME_PREFERENCES,
            ...prefs.theme,
            revision: 3,
          },
        })
      );
    const ctrl = createSettingsController({
      getPreferences: async () => createPreferences(),
      setPreferences,
      notifyError: vi.fn(),
    });
    ctrl.applyPreferencesSnapshot(
      createPreferences({
        theme: { ...DEFAULT_THEME_PREFERENCES, revision: 1 },
      })
    );
    ctrl.state.themePresetId = 'clear-aqua';
    ctrl.state.dirty.theme = true;

    const firstSave = ctrl.savePreferences();
    const savedSnapshot = createPreferences({
      theme: {
        ...DEFAULT_THEME_PREFERENCES,
        presetId: 'clear-aqua',
        revision: 2,
      },
    });
    // The backend broadcasts the successful write before invoke resolves.
    ctrl.applyPreferencesSnapshot(savedSnapshot);
    ctrl.state.themePresetId = 'night-console';
    ctrl.state.dirty.theme = true;
    resolveFirst(savedSnapshot);

    await firstSave;

    expect(ctrl.state.themeRevision).toBe(2);
    expect(ctrl.state.themePresetId).toBe('night-console');
    expect(ctrl.state.dirty.theme).toBe(true);

    await ctrl.savePreferences();

    expect(setPreferences).toHaveBeenCalledTimes(2);
    expect(setPreferences.mock.calls[1]?.[0]?.theme?.presetId).toBe(
      'night-console'
    );
    expect(ctrl.state.themeRevision).toBe(3);
    expect(ctrl.state.dirty.theme).toBe(false);
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
