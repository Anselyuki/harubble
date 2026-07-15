<script lang="ts">
  // MiniPlayerWindow 是 secondary window 的受控 IPC 例外 — 详见 miniPlayerBridge.ts。
  import { onDestroy, onMount } from 'svelte';
  import {
    getPlayerState,
    pausePlayback,
    playNext,
    playPrevious,
    resumePlayback,
    seekCurrentPlayback,
    showMainWindow,
    listenPlayerStateChanged,
    listenPlayerProgress,
  } from '$lib/features/player/miniPlayerBridge';
  import type { PlayerState } from '$lib/types';
  import { formatTime } from '$lib/features/player/formatUtils';
  import {
    ExternalLink,
    Loader2,
    Music2,
    Pause,
    Play,
    SkipBack,
    SkipForward,
  } from '@lucide/svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { imageDataSrc } from '$lib/imageDataSrc';

  type PendingAction = 'play' | 'previous' | 'next' | 'seek';
  type PlayToggleTarget = 'playing' | 'paused';

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
  let seekPreview = $state<number | null>(null);
  let mediaQuery: MediaQueryList | null = null;

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
    document.documentElement.classList.toggle('dark', dark);
    document.documentElement.classList.toggle('light', !dark);
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
    pendingPlayTarget = playerState.isPlaying ? 'paused' : 'playing';
    void runAction('play', () =>
      (playerState.isPlaying ? pausePlayback() : resumePlayback()).then(
        refreshPlayerStateAfterPlaybackCommand
      )
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

    if (hasTauriRuntime()) {
      void getPlayerState()
        .then((state) => {
          if (!lifecycle.disposed) playerState = state;
        })
        .catch((_error: unknown) => {});

      void (async () => {
        const stateUnlisten = await listenPlayerStateChanged((state) => {
          playerState = state;
          clearPlayPendingIfSettled(state);
        });
        const progressUnlisten = await listenPlayerProgress((state) => {
          if (state.sessionId < playerState.sessionId) return;
          playerState = {
            ...playerState,
            sessionId: state.sessionId,
            progress: state.progress,
            duration: state.duration,
          };
        });

        if (lifecycle.disposed) {
          stateUnlisten();
          progressUnlisten();
          return;
        }

        unlisteners.push(stateUnlisten, progressUnlisten);
      })().catch((_error: unknown) => {});
    }

    return () => {
      lifecycle.disposed = true;
      mediaQuery?.removeEventListener('change', applySystemTheme);
      mediaQuery = null;
      while (unlisteners.length) {
        unlisteners.pop()?.();
      }
    };
  });

  onDestroy(() => {
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
    <div class="cover-shell" aria-hidden="true" data-tauri-drag-region>
      {#if playerState.coverUrl}
        <img class="cover-art" use:imageDataSrc={playerState.coverUrl} alt="" />
      {:else}
        <Music2 size={24} strokeWidth={1.8} />
      {/if}
    </div>

    <div class="track-meta" data-tauri-drag-region>
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
      onclick={togglePlayback}
    >
      {#if playerState.isLoading || pendingPlayTarget !== null}
        <span class="spin">
          <Loader2 size={19} strokeWidth={1.8} />
        </span>
      {:else if playerState.isPlaying}
        <Pause size={19} strokeWidth={2} />
      {:else}
        <Play size={19} strokeWidth={2} />
      {/if}
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
    grid-template-columns: 54px minmax(0, 1fr) 30px;
    align-items: center;
    gap: 10px;
  }

  .cover-shell {
    width: 54px;
    height: 54px;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: 8px;
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
    border-radius: 999px;
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
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border: 1px solid transparent;
    border-radius: 8px;
    background: var(--mini-control-bg);
    color: var(--player-control-color);
    transition: var(--motion-hover);
  }

  .play-button {
    width: 36px;
    height: 36px;
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

  .open-button {
    align-self: start;
  }

  .spin {
    display: grid;
    place-items: center;
    animation: motion-spin var(--motion-spinner) linear infinite;
  }

  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
</style>
