<script lang="ts">
  // MiniPlayerWindow 是 secondary window 的受控 IPC 例外 — 详见 miniPlayerBridge.ts。
  import { flushSync, onDestroy, onMount } from 'svelte';
  import {
    getPlayerState,
    getPreferences,
    inspectThemePackage,
    pausePlayback,
    playNext,
    playPrevious,
    resumePlayback,
    seekCurrentPlayback,
    showMainWindow,
    listenPlayerStateChanged,
    listenPlayerProgress,
    listenPreferencesSnapshot,
  } from '$lib/features/player/miniPlayerBridge';
  import type {
    AppPreferences,
    ColorScheme,
    PlayerState,
    ThemePackageDocument,
  } from '$lib/types';
  import {
    applyAppThemeTokenSet,
    applyContextThemePalette,
    deriveGlobalTokensFromSlots,
    resolveAppThemeTokenSet,
  } from '$lib/themeTokens';
  import { resolveThemeColors } from '$lib/themePresets';
  import { resolveThemePackageColors } from '$lib/features/shell/themePackageRuntime.svelte';
  import { applyThemePackageDocument } from '$lib/features/shell/themePackageManager.svelte';
  import { runThemePackageTransition } from '$lib/features/shell/themePackageTransition';
  import { shouldAcceptThemeSnapshot } from '$lib/features/player/themeSnapshotGuard';
  import { formatTime } from '$lib/features/player/formatUtils';
  import {
    hasPlaybackCompleted,
    shouldApplyPlaybackProgress,
  } from '$lib/features/player/playback-contract';
  import PlayToggleGlyph from '$lib/components/app/player/PlayToggleGlyph.svelte';
  import { ExternalLink, Music2, SkipBack, SkipForward } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { imageDataSrc } from '$lib/imageDataSrc';

  type PendingAction = 'play' | 'previous' | 'next' | 'seek';
  type PlayToggleTarget = 'playing' | 'paused';
  type MiniThemeScheme = 'light' | 'dark';

  interface MiniThemeRequest {
    revision: number;
    activePackageId: string | null;
    scheme: MiniThemeScheme;
  }

  const MINI_THEME_INSPECT_RETRY_MS = 250;

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

  let playerState = $state<PlayerState>(EMPTY_PLAYER_STATE);
  let pendingAction = $state<PendingAction | null>(null);
  let pendingPlayTarget = $state<PlayToggleTarget | null>(null);
  let playToggleTransitionKey = $state(0);
  let seekPreview = $state<number | null>(null);
  let prefersReducedMotion = $state(false);
  let mediaQuery: MediaQueryList | null = null;
  let reducedMotionQuery: MediaQueryList | null = null;
  // 本窗口自持的 theme.revision，供 preferences_snapshot 事件 reducer 单调筛选
  let miniThemeRevision = -1;
  let miniThemeActivePackageId: string | null = null;
  let miniThemeApplySeq = 0;
  let hasCommittedThemeSnapshot = false;
  let committedThemePackageId: string | null = null;
  let lastThemePreferences: AppPreferences | null = null;
  let pendingMiniThemeRequest: MiniThemeRequest | null = null;
  let retriedMiniThemeRequest: MiniThemeRequest | null = null;
  let miniThemeRetryTimer: ReturnType<typeof setTimeout> | null = null;

  const hasSong = $derived(Boolean(playerState.songCid));
  const title = $derived(playerState.songName || 'Harubble');
  const artists = $derived(
    playerState.artists.length
      ? playerState.artists.join(' / ')
      : m.mini_player_no_track()
  );
  const duration = $derived(Math.max(0, playerState.duration || 0));
  const progress = $derived(Math.max(0, playerState.progress || 0));
  const shownProgress = $derived(seekPreview ?? progress);
  const progressRatio = $derived(
    duration > 0 ? Math.min(1, Math.max(0, shownProgress / duration)) : 0
  );
  const canSeek = $derived(hasSong && duration > 0 && !playerState.isLoading);
  const playButtonLoading = $derived(
    playerState.isLoading || pendingPlayTarget !== null
  );
  const playLabel = $derived(
    playButtonLoading
      ? m.player_status_loading()
      : playerState.isPlaying
        ? m.player_aria_pause()
        : m.player_aria_play()
  );

  function applySystemTheme() {
    const dark = mediaQuery?.matches ?? false;
    const colorScheme = lastThemePreferences?.theme?.colorScheme;
    const followsSystem = colorScheme === undefined || colorScheme === 'auto';
    if (lastThemePreferences && !followsSystem) {
      return;
    }
    if (lastThemePreferences) {
      void applyThemePreferences(lastThemePreferences);
      return;
    }
    document.documentElement.classList.toggle('dark', dark);
    document.documentElement.classList.toggle('light', !dark);
    document.documentElement.style.colorScheme = dark ? 'dark' : 'light';
  }

  function isSameMiniThemeRequest(
    left: MiniThemeRequest | null,
    right: MiniThemeRequest
  ): boolean {
    return (
      left?.revision === right.revision &&
      left.activePackageId === right.activePackageId &&
      left.scheme === right.scheme
    );
  }

  function scheduleMiniThemeInspectRetry(
    snapshot: AppPreferences,
    request: MiniThemeRequest,
    applySeq: number
  ): void {
    if (isSameMiniThemeRequest(retriedMiniThemeRequest, request)) return;
    retriedMiniThemeRequest = request;
    if (miniThemeRetryTimer !== null) clearTimeout(miniThemeRetryTimer);
    miniThemeRetryTimer = setTimeout(() => {
      miniThemeRetryTimer = null;
      if (applySeq !== miniThemeApplySeq) return;
      void applyThemePreferences(snapshot);
    }, MINI_THEME_INSPECT_RETRY_MS);
  }

  function clearMiniThemeInspectRetry(request: MiniThemeRequest): void {
    if (!isSameMiniThemeRequest(retriedMiniThemeRequest, request)) return;
    retriedMiniThemeRequest = null;
    if (miniThemeRetryTimer !== null) {
      clearTimeout(miniThemeRetryTimer);
      miniThemeRetryTimer = null;
    }
  }

  /**
   * 单调 reducer：接收来自主窗口的 preferences_snapshot 广播并同步 Mini Player DOM 令牌。
   *
   * 严格按 `theme.revision` 单调递增接受快照：老事件直接丢弃。
   * 解析 scheme（跟随系统时依赖 mediaQuery）后调用 `resolveAppThemeTokenSet` 派生
   * 23 个全局 token；首次快照和同包重算同步提交，跨包切换在遮罩中点原子提交。
   */
  async function applyThemePreferences(
    snapshot: AppPreferences
  ): Promise<void> {
    const theme = snapshot.theme;
    if (!theme) return;
    const incomingRevision = theme.revision ?? 0;
    const incomingActivePackageId = theme.activePackageId ?? null;
    if (
      !shouldAcceptThemeSnapshot(
        miniThemeRevision,
        miniThemeActivePackageId,
        incomingRevision,
        incomingActivePackageId
      )
    ) {
      return;
    }
    const scheme = resolveScheme(theme.colorScheme);
    const request: MiniThemeRequest = {
      revision: incomingRevision,
      activePackageId: incomingActivePackageId,
      scheme,
    };
    if (isSameMiniThemeRequest(pendingMiniThemeRequest, request)) {
      // The bridge may replay the same snapshot while its package inspect or
      // reveal is still pending. The existing request owns that transition;
      // restarting it would make a same-target wipe visibly jump backwards.
      return;
    }
    miniThemeRevision = incomingRevision;
    miniThemeActivePackageId = incomingActivePackageId;
    lastThemePreferences = snapshot;
    const applySeq = ++miniThemeApplySeq;
    if (
      retriedMiniThemeRequest !== null &&
      !isSameMiniThemeRequest(retriedMiniThemeRequest, request)
    ) {
      clearMiniThemeInspectRetry(retriedMiniThemeRequest);
    }
    pendingMiniThemeRequest = request;

    try {
      let packageDocument: ThemePackageDocument | null = null;
      let packageInspectFailed = false;
      if (incomingActivePackageId) {
        try {
          const inspected = await inspectThemePackage(incomingActivePackageId);
          if (inspected?.manifest.id === incomingActivePackageId) {
            packageDocument = inspected;
          } else {
            packageInspectFailed = true;
          }
        } catch {
          packageInspectFailed = true;
        }
      }
      if (
        applySeq !== miniThemeApplySeq ||
        incomingRevision !== miniThemeRevision ||
        incomingActivePackageId !== miniThemeActivePackageId
      ) {
        return;
      }

      const themeColors = packageDocument
        ? resolveThemePackageColors(theme, packageDocument, scheme)
        : resolveThemeColors({
            presetId: theme.presetId,
            customColors: theme.customColors,
          });
      const tokens = packageDocument
        ? deriveGlobalTokensFromSlots(themeColors, scheme)
        : resolveAppThemeTokenSet(themeColors, scheme);
      const animate =
        !packageInspectFailed &&
        hasCommittedThemeSnapshot &&
        incomingActivePackageId !== committedThemePackageId;

      await runThemePackageTransition(
        () => {
          if (
            applySeq !== miniThemeApplySeq ||
            incomingRevision !== miniThemeRevision ||
            incomingActivePackageId !== miniThemeActivePackageId
          ) {
            return;
          }
          document.documentElement.classList.toggle('dark', scheme === 'dark');
          document.documentElement.classList.toggle('light', scheme !== 'dark');
          document.documentElement.style.colorScheme = scheme;
          applyThemePackageDocument(packageDocument, scheme);
          applyAppThemeTokenSet(tokens, { animate: false });
          applyContextThemePalette(null, tokens, scheme, { animate: false });
          hasCommittedThemeSnapshot = true;
          committedThemePackageId = packageDocument?.manifest.id ?? null;
        },
        {
          animate,
          reason: 'activate',
          targetPackageId: packageDocument?.manifest.id ?? null,
        }
      );
      if (packageInspectFailed && applySeq === miniThemeApplySeq) {
        scheduleMiniThemeInspectRetry(snapshot, request, applySeq);
      } else if (!packageInspectFailed) {
        clearMiniThemeInspectRetry(request);
      }
    } finally {
      if (pendingMiniThemeRequest === request) {
        pendingMiniThemeRequest = null;
      }
    }
  }

  function resolveScheme(preference: ColorScheme | undefined): MiniThemeScheme {
    if (preference === 'dark') return 'dark';
    if (preference === 'light') return 'light';
    return mediaQuery?.matches ? 'dark' : 'light';
  }

  function applyReducedMotion() {
    prefersReducedMotion = reducedMotionQuery?.matches ?? false;
  }

  function hasTauriRuntime(): boolean {
    return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
  }

  function openMainWindow() {
    if (!hasTauriRuntime()) return;
    void showMainWindow().catch((_error: unknown) => {});
  }

  async function runAction<T>(key: PendingAction, action: () => Promise<T>) {
    if (pendingAction) return;
    pendingAction = key;
    let succeeded = false;
    try {
      await action();
      succeeded = true;
    } catch {
      // Keep the tray surface quiet; the main window handles user-facing errors.
    } finally {
      if (key !== 'play' || !succeeded) {
        pendingAction = null;
        if (key === 'play') {
          pendingPlayTarget = null;
        }
      }
    }
  }

  async function refreshPlayerStateAfterPlaybackCommand() {
    const state = await getPlayerState();
    playerState = state;
    clearPlayPendingIfSettled(state);
  }

  function clearPlayPendingIfSettled(state: PlayerState) {
    if (!pendingPlayTarget) return;
    if (state.isLoading) return;
    if (!state.songCid || (!state.isPlaying && !state.isPaused)) {
      pendingPlayTarget = null;
      pendingAction = null;
      return;
    }
    if (
      (pendingPlayTarget === 'playing' && state.isPlaying) ||
      (pendingPlayTarget === 'paused' && state.isPaused)
    ) {
      pendingPlayTarget = null;
      pendingAction = null;
    }
  }

  function togglePlayback() {
    if (!hasSong || playerState.isLoading || pendingAction || pendingPlayTarget)
      return;
    playToggleTransitionKey += 1;
    flushSync();
    pendingPlayTarget = playerState.isPlaying ? 'paused' : 'playing';
    void runAction('play', () =>
      (playerState.isPlaying
        ? pausePlayback()
        : hasPlaybackCompleted(playerState)
          ? seekCurrentPlayback(0)
          : resumePlayback()
      ).then(refreshPlayerStateAfterPlaybackCommand)
    );
  }

  function handlePrevious() {
    if (!playerState.hasPrevious) return;
    void runAction('previous', playPrevious);
  }

  function handleNext() {
    if (!playerState.hasNext) return;
    void runAction('next', playNext);
  }

  function handleSeekInput(event: Event) {
    seekPreview = Number((event.currentTarget as HTMLInputElement).value);
  }

  function handleSeekCommit(event: Event) {
    if (!canSeek) return;
    const nextProgress = Number(
      (event.currentTarget as HTMLInputElement).value
    );
    seekPreview = null;
    void runAction('seek', () => seekCurrentPlayback(nextProgress));
  }

  onMount(() => {
    const lifecycle = { disposed: false };
    const unlisteners: (() => void)[] = [];

    mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    applySystemTheme();
    mediaQuery.addEventListener('change', applySystemTheme);

    reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    applyReducedMotion();
    reducedMotionQuery.addEventListener('change', applyReducedMotion);

    if (hasTauriRuntime()) {
      void getPlayerState()
        .then((state) => {
          if (!lifecycle.disposed) playerState = state;
        })
        .catch((_error: unknown) => {});

      // Playback subscriptions are independent of theme hydration. A slow or
      // failed preferences/inspect request must not create a startup event gap.
      void listenPlayerStateChanged((state) => {
        if (lifecycle.disposed) return;
        playerState = state;
        clearPlayPendingIfSettled(state);
      })
        .then((stateUnlisten) => {
          if (lifecycle.disposed) {
            stateUnlisten();
            return;
          }
          unlisteners.push(stateUnlisten);
        })
        .catch((_error: unknown) => {});
      void listenPlayerProgress((state) => {
        if (
          lifecycle.disposed ||
          !shouldApplyPlaybackProgress(state, playerState)
        ) {
          return;
        }
        playerState = {
          ...playerState,
          sessionId: state.sessionId,
          progress: state.progress,
          duration: state.duration,
        };
      })
        .then((progressUnlisten) => {
          if (lifecycle.disposed) {
            progressUnlisten();
            return;
          }
          unlisteners.push(progressUnlisten);
        })
        .catch((_error: unknown) => {});

      void (async () => {
        // 先订阅主题事件，再读取初始快照，避免两步之间发生的切换丢失。
        const prefsUnlisten = await listenPreferencesSnapshot((snapshot) => {
          if (lifecycle.disposed) return;
          void applyThemePreferences(snapshot);
        });
        if (lifecycle.disposed) {
          prefsUnlisten();
          return;
        }
        unlisteners.push(prefsUnlisten);
        await getPreferences()
          .then((prefs) => {
            if (!lifecycle.disposed) return applyThemePreferences(prefs);
          })
          .catch((_error: unknown) => {});
      })().catch((_error: unknown) => {});
    }

    return () => {
      lifecycle.disposed = true;
      mediaQuery?.removeEventListener('change', applySystemTheme);
      mediaQuery = null;
      reducedMotionQuery?.removeEventListener('change', applyReducedMotion);
      reducedMotionQuery = null;
      while (unlisteners.length) {
        unlisteners.pop()?.();
      }
    };
  });

  onDestroy(() => {
    miniThemeApplySeq += 1;
    if (miniThemeRetryTimer !== null) {
      clearTimeout(miniThemeRetryTimer);
      miniThemeRetryTimer = null;
    }
    document.documentElement.classList.remove('mini-player-document');
  });

  $effect(() => {
    document.documentElement.classList.add('mini-player-document');
  });
</script>

<svelte:head>
  <title>Harubble</title>
</svelte:head>

<main class="mini-player">
  <section class="track-row" aria-label={m.mini_player_track_section_aria()}>
    <div class="cover-shell" aria-hidden="true" data-tauri-drag-region="deep">
      {#if playerState.coverUrl}
        <img class="cover-art" use:imageDataSrc={playerState.coverUrl} alt="" />
      {:else}
        <Music2 size={24} strokeWidth={1.8} />
      {/if}
    </div>

    <div class="track-meta" data-tauri-drag-region="deep">
      <p class="track-title" {title}>{title}</p>
      <p class="track-artists" title={artists}>{artists}</p>
    </div>

    <button
      type="button"
      class="icon-button open-button"
      aria-label={m.mini_player_open_aria()}
      title={m.mini_player_open_aria()}
      onclick={openMainWindow}
    >
      <ExternalLink size={16} strokeWidth={1.8} />
    </button>
  </section>

  <section class="progress-row" aria-label={m.player_aria_timeline()}>
    <span class="time-label">{formatTime(shownProgress)}</span>
    <input
      class="progress-slider"
      type="range"
      min="0"
      max={duration || 1}
      step="0.1"
      value={shownProgress}
      disabled={!canSeek || pendingAction === 'seek'}
      aria-label={m.player_aria_seek()}
      aria-valuetext={`${formatTime(shownProgress)} of ${formatTime(duration)}`}
      oninput={handleSeekInput}
      onchange={handleSeekCommit}
      style="--progress-ratio:{progressRatio * 100}%"
    />
    <span class="time-label">{formatTime(duration)}</span>
  </section>

  <section class="controls-row" aria-label={m.player_aria_transport()}>
    <button
      type="button"
      class="icon-button"
      aria-label={m.mini_player_previous_aria()}
      title={m.mini_player_previous_aria()}
      disabled={!playerState.hasPrevious || pendingAction !== null}
      onclick={handlePrevious}
    >
      <SkipBack size={17} strokeWidth={1.9} />
    </button>

    <button
      type="button"
      class="play-button"
      aria-label={playLabel}
      title={playLabel}
      disabled={!hasSong || playerState.isLoading || pendingAction !== null}
      aria-busy={playButtonLoading}
      onclick={togglePlayback}
    >
      <PlayToggleGlyph
        isPlaying={playerState.isPlaying}
        isLoading={playerState.isLoading}
        isPending={pendingPlayTarget !== null}
        transitionKey={playToggleTransitionKey}
        reducedMotion={prefersReducedMotion}
        size="19px"
      />
    </button>

    <button
      type="button"
      class="icon-button"
      aria-label={m.mini_player_next_aria()}
      title={m.mini_player_next_aria()}
      disabled={!playerState.hasNext || pendingAction !== null}
      onclick={handleNext}
    >
      <SkipForward size={17} strokeWidth={1.9} />
    </button>
  </section>
</main>

<style>
  :global(.mini-player-document),
  :global(.mini-player-document body) {
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: var(--mini-player-window-bg);
  }

  :global(.mini-player-document) {
    --mini-player-window-bg: #f5f5f7;
  }

  :global(.mini-player-document.dark) {
    --mini-player-window-bg: #1c1c1e;
  }

  .mini-player {
    --mini-surface: color-mix(in srgb, var(--bg-secondary) 86%, white 14%);
    --mini-border: color-mix(in srgb, var(--border) 80%, white 20%);
    --mini-control-bg: color-mix(in srgb, var(--bg-primary) 62%, transparent);
    --mini-control-hover: color-mix(in srgb, var(--accent) 11%, transparent);
    width: 100vw;
    height: 100vh;
    display: grid;
    grid-template-rows: minmax(62px, auto) 28px 44px;
    gap: 8px;
    padding: 12px;
    background:
      linear-gradient(180deg, var(--player-shell-highlight), transparent 52%),
      var(--mini-surface);
    border: 1px solid var(--mini-border);
    box-shadow:
      inset 0 1px 0 color-mix(in srgb, white 36%, transparent),
      0 18px 42px rgba(0, 0, 0, 0.18);
    color: var(--text-primary);
    font-family: var(--font-body);
    user-select: none;
  }

  :global(.dark) .mini-player {
    --mini-surface: color-mix(in srgb, var(--bg-secondary) 92%, white 8%);
    --mini-border: color-mix(in srgb, var(--border) 86%, white 14%);
    --mini-control-bg: color-mix(in srgb, var(--bg-tertiary) 76%, transparent);
    --mini-control-hover: color-mix(in srgb, var(--accent) 18%, transparent);
    box-shadow:
      inset 0 1px 0 color-mix(in srgb, white 12%, transparent),
      0 20px 46px rgba(0, 0, 0, 0.42);
  }

  .track-row {
    min-width: 0;
    display: grid;
    grid-template-columns: 54px minmax(0, 1fr) 40px;
    align-items: center;
    gap: 10px;
  }

  .cover-shell {
    width: 54px;
    height: 54px;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: var(--shape-md);
    background:
      linear-gradient(135deg, rgba(var(--accent-rgb), 0.18), transparent),
      var(--bg-tertiary);
    color: var(--text-secondary);
    box-shadow: inset 0 1px 0 color-mix(in srgb, white 26%, transparent);
  }

  .cover-art {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .track-meta {
    min-width: 0;
    display: grid;
    gap: 4px;
  }

  .track-title,
  .track-artists {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    letter-spacing: 0;
  }

  .track-title {
    font-size: 13px;
    line-height: 1.25;
    font-weight: 700;
    color: var(--text-primary);
  }

  .track-artists {
    font-size: 11px;
    line-height: 1.2;
    color: var(--text-secondary);
  }

  .progress-row {
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr) 34px;
    align-items: center;
    gap: 8px;
  }

  .time-label {
    font-family: var(--font-mono);
    font-size: 10px;
    line-height: 1;
    color: var(--text-tertiary);
    text-align: center;
  }

  .progress-slider {
    width: 100%;
    height: 18px;
    appearance: none;
    background: transparent;
    cursor: pointer;
  }

  .progress-slider:disabled {
    cursor: default;
    opacity: 0.56;
  }

  .progress-slider::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: var(--shape-pill);
    background: linear-gradient(
      90deg,
      var(--album-accent) var(--progress-ratio, 0%),
      var(--player-track-bg) var(--progress-ratio, 0%)
    );
  }

  .progress-slider::-webkit-slider-thumb {
    appearance: none;
    width: 12px;
    height: 12px;
    margin-top: -4px;
    border-radius: 50%;
    border: 2px solid var(--player-thumb-border);
    background: var(--album-accent);
    box-shadow: var(--player-thumb-shadow);
  }

  .controls-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
  }

  .icon-button,
  .play-button {
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border: 1px solid transparent;
    border-radius: var(--shape-md);
    background: var(--mini-control-bg);
    color: var(--player-control-color);
    transition: var(--motion-hover);
  }

  .play-button {
    width: 44px;
    height: 44px;
    background: linear-gradient(
      135deg,
      var(--album-accent),
      var(--album-accent-hover)
    );
    color: var(--player-play-text);
    box-shadow: var(--player-play-shadow);
  }

  .icon-button:hover:not(:disabled),
  .play-button:hover:not(:disabled) {
    background: var(--mini-control-hover);
  }

  .play-button:hover:not(:disabled) {
    background: linear-gradient(
      135deg,
      var(--album-accent-hover),
      var(--album-accent)
    );
    box-shadow: var(--player-play-shadow-hover);
  }

  .icon-button:focus-visible,
  .play-button:focus-visible,
  .progress-slider:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--accent) 48%, transparent);
    outline-offset: 2px;
  }

  .icon-button:disabled,
  .play-button:disabled {
    cursor: default;
    opacity: 0.42;
  }

  .play-button[aria-busy='true']:disabled {
    opacity: 1;
  }

  .open-button {
    align-self: start;
  }
</style>
