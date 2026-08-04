// @vitest-environment jsdom

import { cleanup, render, waitFor } from '@testing-library/svelte';
import '@testing-library/jest-dom/vitest';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  AppPreferences,
  ThemePackageDocument,
  PlayerState,
} from '$lib/types';

const bridge = vi.hoisted(() => {
  let preferencesHandler: ((snapshot: unknown) => void) | null = null;

  return {
    getPlayerState: vi.fn(),
    getPreferences: vi.fn(),
    inspectThemePackage: vi.fn(),
    pausePlayback: vi.fn(),
    playNext: vi.fn(),
    playPrevious: vi.fn(),
    resumePlayback: vi.fn(),
    seekCurrentPlayback: vi.fn(),
    showMainWindow: vi.fn(),
    listenPlayerStateChanged: vi.fn(async () => vi.fn()),
    listenPlayerProgress: vi.fn(async () => vi.fn()),
    listenPreferencesSnapshot: vi.fn(
      async (handler: (snapshot: unknown) => void) => {
        preferencesHandler = handler;
        return vi.fn();
      }
    ),
    emitPreferences(snapshot: unknown) {
      preferencesHandler?.(snapshot);
    },
    resetHandler() {
      preferencesHandler = null;
    },
  };
});

const themeHarness = vi.hoisted(() => {
  type TransitionOptions = {
    animate?: boolean;
    reason?: string;
    targetPackageId?: string | null;
  };

  const harness = {
    autoCommit: true,
    inCommit: false,
    pendingCommits: [] as (() => void)[],
    writerCommitStates: [] as boolean[],
    domCommitStates: [] as {
      beforeDark: boolean;
      beforeLight: boolean;
      beforeColorScheme: string;
      afterDark: boolean;
      afterLight: boolean;
      afterColorScheme: string;
    }[],
    runTransition: vi.fn(
      async (commit: () => void, _options: TransitionOptions) => {
        const guardedCommit = () => {
          const state = {
            beforeDark: document.documentElement.classList.contains('dark'),
            beforeLight: document.documentElement.classList.contains('light'),
            beforeColorScheme: document.documentElement.style.colorScheme,
            afterDark: false,
            afterLight: false,
            afterColorScheme: '',
          };
          harness.inCommit = true;
          try {
            commit();
          } finally {
            harness.inCommit = false;
          }
          state.afterDark = document.documentElement.classList.contains('dark');
          state.afterLight =
            document.documentElement.classList.contains('light');
          state.afterColorScheme = document.documentElement.style.colorScheme;
          harness.domCommitStates.push(state);
        };
        if (harness.autoCommit) guardedCommit();
        else harness.pendingCommits.push(guardedCommit);
      }
    ),
    applyDocument: vi.fn(() => {
      harness.writerCommitStates.push(harness.inCommit);
    }),
    applyAppTokens: vi.fn(() => {
      harness.writerCommitStates.push(harness.inCommit);
    }),
    applyContextTokens: vi.fn(() => {
      harness.writerCommitStates.push(harness.inCommit);
    }),
    reset() {
      harness.autoCommit = true;
      harness.inCommit = false;
      harness.pendingCommits = [];
      harness.writerCommitStates = [];
      harness.domCommitStates = [];
    },
  };

  return harness;
});

vi.mock('$lib/features/player/miniPlayerBridge', () => bridge);

vi.mock('$lib/features/shell/themePackageTransition', () => ({
  runThemePackageTransition: themeHarness.runTransition,
}));

vi.mock('$lib/features/shell/themePackageManager.svelte', () => ({
  applyThemePackageDocument: themeHarness.applyDocument,
}));

vi.mock('$lib/features/shell/themePackageRuntime.svelte', () => ({
  resolveThemePackageColors: vi.fn(() => ({ accent: '#112233' })),
}));

vi.mock('$lib/themePresets', () => ({
  resolveThemeColors: vi.fn(() => ({ accent: '#445566' })),
}));

vi.mock('$lib/themeTokens', () => ({
  applyAppThemeTokenSet: themeHarness.applyAppTokens,
  applyContextThemePalette: themeHarness.applyContextTokens,
  deriveGlobalTokensFromSlots: vi.fn(() => ({ accent: '#112233' })),
  resolveAppThemeTokenSet: vi.fn(() => ({ accent: '#445566' })),
}));

import MiniPlayerWindow from './MiniPlayerWindow.svelte';

const EMPTY_PLAYER_STATE: PlayerState = {
  sessionId: 0,
  songCid: null,
  songName: null,
  artists: [],
  coverUrl: null,
  isPlaying: false,
  isPaused: false,
  isLoading: false,
  hasPrevious: false,
  hasNext: false,
  progress: 0,
  duration: 0,
  volume: 1,
  playbackFormat: null,
};

function preferences(
  revision: number,
  activePackageId: string | null,
  colorScheme: 'auto' | 'light' | 'dark' = 'light'
): AppPreferences {
  return {
    schemaVersion: 2,
    outputFormat: 'flac',
    outputDir: '',
    downloadLyrics: true,
    notifyOnDownloadComplete: true,
    notifyOnPlaybackChange: true,
    logLevel: 'info',
    locale: 'zh-CN',
    volume: 1,
    theme: {
      presetId: 'harubble-classic',
      customColors: {},
      colorScheme,
      activePackageId,
      revision,
    },
  };
}

function themePackage(id: string): ThemePackageDocument {
  return {
    schemaVersion: 1,
    manifest: { id, name: id, version: '1.0.0' },
    slots: {},
  };
}

async function renderWithInitialTheme(
  snapshot: AppPreferences = preferences(1, 'alpha', 'dark')
) {
  bridge.getPreferences.mockResolvedValue(snapshot);
  const view = render(MiniPlayerWindow);
  await waitFor(() => expect(themeHarness.runTransition).toHaveBeenCalled());
  return view;
}

describe('mini player theme package transitions', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    bridge.resetHandler();
    themeHarness.reset();
    bridge.getPlayerState.mockResolvedValue(EMPTY_PLAYER_STATE);
    bridge.inspectThemePackage.mockImplementation(async (id: string) =>
      themePackage(id)
    );
    Object.defineProperty(window, '__TAURI_INTERNALS__', {
      configurable: true,
      value: {},
    });
    document.documentElement.classList.remove('dark', 'light');
    document.documentElement.style.colorScheme = '';
  });

  afterEach(() => {
    cleanup();
    Reflect.deleteProperty(window, '__TAURI_INTERNALS__');
    document.documentElement.classList.remove('dark', 'light');
    document.documentElement.style.colorScheme = '';
  });

  it('registers playback listeners without waiting for theme hydration', async () => {
    let resolvePreferences!: (snapshot: AppPreferences) => void;
    bridge.getPreferences.mockReturnValue(
      new Promise<AppPreferences>((resolve) => {
        resolvePreferences = resolve;
      })
    );

    render(MiniPlayerWindow);

    await waitFor(() => {
      expect(bridge.listenPlayerStateChanged).toHaveBeenCalledOnce();
      expect(bridge.listenPlayerProgress).toHaveBeenCalledOnce();
    });
    expect(themeHarness.runTransition).not.toHaveBeenCalled();

    resolvePreferences(preferences(1, 'alpha'));
    await waitFor(() => expect(themeHarness.runTransition).toHaveBeenCalled());
  });

  it('keeps playback listeners alive when theme subscription fails', async () => {
    bridge.listenPreferencesSnapshot.mockRejectedValueOnce(
      new Error('theme event unavailable')
    );

    render(MiniPlayerWindow);

    await waitFor(() => {
      expect(bridge.listenPlayerStateChanged).toHaveBeenCalledOnce();
      expect(bridge.listenPlayerProgress).toHaveBeenCalledOnce();
    });
  });

  it('animates only a package id change after the initial snapshot commits', async () => {
    await renderWithInitialTheme();

    expect(themeHarness.runTransition).toHaveBeenNthCalledWith(
      1,
      expect.any(Function),
      {
        animate: false,
        reason: 'activate',
        targetPackageId: 'alpha',
      }
    );
    expect(document.documentElement).toHaveClass('dark');
    expect(document.documentElement.style.colorScheme).toBe('dark');
    expect(themeHarness.domCommitStates[0]).toEqual({
      beforeDark: false,
      beforeLight: true,
      beforeColorScheme: 'light',
      afterDark: true,
      afterLight: false,
      afterColorScheme: 'dark',
    });

    bridge.emitPreferences(preferences(2, 'beta', 'light'));
    await waitFor(() =>
      expect(themeHarness.runTransition).toHaveBeenCalledTimes(2)
    );
    expect(themeHarness.runTransition).toHaveBeenNthCalledWith(
      2,
      expect.any(Function),
      {
        animate: true,
        reason: 'activate',
        targetPackageId: 'beta',
      }
    );

    bridge.emitPreferences(preferences(3, 'beta', 'dark'));
    await waitFor(() =>
      expect(themeHarness.runTransition).toHaveBeenCalledTimes(3)
    );
    expect(themeHarness.runTransition).toHaveBeenNthCalledWith(
      3,
      expect.any(Function),
      {
        animate: false,
        reason: 'activate',
        targetPackageId: 'beta',
      }
    );
    expect(themeHarness.writerCommitStates).toHaveLength(9);
    expect(themeHarness.writerCommitStates.every(Boolean)).toBe(true);
    expect(themeHarness.domCommitStates[1]).toMatchObject({
      beforeDark: true,
      beforeColorScheme: 'dark',
      afterLight: true,
      afterColorScheme: 'light',
    });
  });

  it('identifies the actually rendered document as the transition target', async () => {
    bridge.inspectThemePackage.mockRejectedValue(new Error('unavailable'));

    await renderWithInitialTheme();

    expect(themeHarness.runTransition).toHaveBeenCalledWith(
      expect.any(Function),
      {
        animate: false,
        reason: 'activate',
        targetPackageId: null,
      }
    );
    expect(themeHarness.applyDocument).toHaveBeenCalledWith(null, 'dark');
  });

  it('recovers a transient inspect failure without another snapshot', async () => {
    bridge.inspectThemePackage.mockRejectedValueOnce(new Error('unavailable'));

    await renderWithInitialTheme();

    await waitFor(
      () => expect(bridge.inspectThemePackage).toHaveBeenCalledTimes(2),
      { timeout: 1_500 }
    );
    await waitFor(() =>
      expect(themeHarness.applyDocument).toHaveBeenLastCalledWith(
        expect.objectContaining({
          manifest: expect.objectContaining({ id: 'alpha' }),
        }),
        'dark'
      )
    );
    expect(themeHarness.runTransition).toHaveBeenNthCalledWith(
      1,
      expect.any(Function),
      {
        animate: false,
        reason: 'activate',
        targetPackageId: null,
      }
    );
    expect(themeHarness.runTransition).toHaveBeenNthCalledWith(
      2,
      expect.any(Function),
      {
        animate: true,
        reason: 'activate',
        targetPackageId: 'alpha',
      }
    );
  });

  it('coalesces an identical snapshot while its transition is pending', async () => {
    await renderWithInitialTheme();
    let resolveBeta!: (document: ThemePackageDocument) => void;
    const betaInspection = new Promise<ThemePackageDocument>((resolve) => {
      resolveBeta = resolve;
    });
    bridge.inspectThemePackage.mockImplementation(async (id: string) => {
      if (id === 'beta') return betaInspection;
      return themePackage(id);
    });

    bridge.emitPreferences(preferences(2, 'beta'));
    await waitFor(() =>
      expect(bridge.inspectThemePackage).toHaveBeenCalledWith('beta')
    );
    bridge.emitPreferences(preferences(2, 'beta'));

    expect(themeHarness.runTransition).toHaveBeenCalledTimes(1);
    resolveBeta(themePackage('beta'));
    await waitFor(() =>
      expect(themeHarness.runTransition).toHaveBeenCalledTimes(2)
    );
  });

  it('does not start a transition for a snapshot superseded during package inspection', async () => {
    await renderWithInitialTheme();
    let resolveBeta!: (document: ThemePackageDocument) => void;
    const betaInspection = new Promise<ThemePackageDocument>((resolve) => {
      resolveBeta = resolve;
    });
    bridge.inspectThemePackage.mockImplementation(async (id: string) => {
      if (id === 'beta') return betaInspection;
      return themePackage(id);
    });

    bridge.emitPreferences(preferences(2, 'beta'));
    await waitFor(() =>
      expect(bridge.inspectThemePackage).toHaveBeenCalledWith('beta')
    );
    bridge.emitPreferences(preferences(3, 'gamma'));
    await waitFor(() =>
      expect(themeHarness.runTransition).toHaveBeenCalledTimes(2)
    );

    resolveBeta(themePackage('beta'));
    await Promise.resolve();
    await Promise.resolve();

    expect(
      themeHarness.runTransition.mock.calls.map((call) => call[1])
    ).toEqual([
      expect.objectContaining({ targetPackageId: 'alpha' }),
      expect.objectContaining({ targetPackageId: 'gamma' }),
    ]);
    expect(themeHarness.applyDocument).not.toHaveBeenCalledWith(
      expect.objectContaining({
        manifest: expect.objectContaining({ id: 'beta' }),
      }),
      expect.anything()
    );
  });

  it('rejects an obsolete transition midpoint after a newer snapshot arrives', async () => {
    await renderWithInitialTheme();
    themeHarness.autoCommit = false;

    bridge.emitPreferences(preferences(2, 'beta'));
    await waitFor(() =>
      expect(themeHarness.runTransition).toHaveBeenCalledTimes(2)
    );
    bridge.emitPreferences(preferences(3, 'gamma'));
    await waitFor(() =>
      expect(themeHarness.runTransition).toHaveBeenCalledTimes(3)
    );

    const writesBeforeMidpoints = themeHarness.applyDocument.mock.calls.length;
    themeHarness.pendingCommits[0]?.();
    expect(themeHarness.applyDocument).toHaveBeenCalledTimes(
      writesBeforeMidpoints
    );

    themeHarness.pendingCommits[1]?.();
    expect(themeHarness.applyDocument).toHaveBeenLastCalledWith(
      expect.objectContaining({
        manifest: expect.objectContaining({ id: 'gamma' }),
      }),
      'light'
    );
  });
});

describe('mini player window entrypoint', () => {
  it('routes the secondary window without booting the full app runtime', async () => {
    // @ts-expect-error Vitest runs in Node and reads source files.
    const { readFileSync } = await import('node:fs');
    const entry = readFileSync('src/main.ts', 'utf8');
    const miniPlayer = readFileSync(
      'src/lib/components/app/player/MiniPlayerWindow.svelte',
      'utf8'
    );
    const bridge = readFileSync(
      'src/lib/features/player/miniPlayerBridge.ts',
      'utf8'
    );

    expect(entry).toContain('MiniPlayerWindow');
    expect(entry).toContain("get('window') === 'mini-player'");

    // Bridge centralises event subscriptions — component imports from bridge, not directly from Tauri.
    expect(miniPlayer).toContain('miniPlayerBridge');
    expect(miniPlayer).toContain('listenPlayerStateChanged');
    expect(miniPlayer).toContain('listenPlayerProgress');
    expect(miniPlayer).toContain('sessionId');
    expect(miniPlayer).toContain('showMainWindow');
    expect(miniPlayer).toContain('hasPlaybackCompleted(playerState)');
    expect(miniPlayer).toContain(
      'shouldApplyPlaybackProgress(state, playerState)'
    );
    expect(miniPlayer).toContain('seekCurrentPlayback(0)');
    expect(miniPlayer).not.toContain('createAppRuntime');

    // Bridge itself wraps the raw Tauri listen calls.
    expect(bridge).toMatch(/listen<PlayerState>\(\s*'player-state-changed'/);
    expect(bridge).toMatch(/listen<PlayerState>\(\s*'player-progress'/);
  });
});
