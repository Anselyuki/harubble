import type {
  PlaybackContext,
  PlaybackQueueEntry,
  PlayerState,
  PlaybackEndedEvent,
} from '$lib/types';
import { parseLyricText } from './lyrics';
import { buildPlaybackContext } from './queue';
import {
  shouldApplyPlaybackEnded,
  shouldIgnorePlaybackError,
  type PlaybackSnapshot,
} from './playback-contract';
import * as m from '$lib/paraglide/messages.js';

interface PlayerControllerDeps {
  playSong: (
    songCid: string,
    coverUrl: string | null,
    context: PlaybackContext | null
  ) => Promise<void>;
  pausePlayback: () => Promise<void>;
  resumePlayback: () => Promise<void>;
  seekCurrentPlayback: (positionSecs: number) => Promise<void>;
  setPlaybackVolume: (volume: number) => Promise<number>;
  getPlayerState: () => Promise<PlayerState>;
  getSongLyrics: (songCid: string) => Promise<string | null>;
  notifyError: (message: string) => void;
}

type RepeatMode = 'all' | 'one';
type PlayToggleTarget = 'playing' | 'paused';

interface PlayerSong {
  cid: string;
  name: string;
  artists: string[];
  coverUrl: string | null;
}

interface LyricLine {
  id: string;
  time: number | null;
  text: string;
}

export function createPlayerController(deps: PlayerControllerDeps) {
  let initialized = false;
  let currentSong = $state<PlayerSong | null>(null);
  let isPlaying = $state(false);
  let isPaused = $state(false);
  let isLoading = $state(false);
  let pendingPlayToggleTarget = $state<PlayToggleTarget | null>(null);
  let hasPrevious = $state(false);
  let hasNext = $state(false);
  let progress = $state(0);
  let duration = $state(0);
  let shuffleEnabled = $state(false);
  let repeatMode = $state<RepeatMode>('all');
  let playbackEntries = $state<PlaybackQueueEntry[]>([]);
  let playbackOrder = $state<PlaybackQueueEntry[]>([]);
  let playbackIndex = $state(-1);
  let lyricsOpen = $state(false);
  let playlistOpen = $state(false);
  let fullscreenOpen = $state(false);
  let lyricsLoading = $state(false);
  let lyricsError = $state('');
  let lyricsLines = $state<LyricLine[]>([]);
  let lyricsSongCid = $state<string | null>(null);
  let playingCid = $state<string | null>(null);
  let volume = $state(1.0);
  let muted = $state(false);
  let volumeBeforeMute = 1.0;
  let playbackEndRequestSeq = 0;
  let playRequestSeq = 0;
  let lastPlaybackSnapshot: PlaybackSnapshot = {
    cid: null as string | null,
    active: false,
    sessionId: 0,
  };
  let lyricRequestSeq = 0;

  function init() {
    if (initialized) return;
    initialized = true;
  }

  function normalizePlayerSong(state: PlayerState): PlayerSong | null {
    if (!state.songCid) return null;

    return {
      cid: state.songCid,
      name: state.songName ?? '',
      artists: state.artists,
      coverUrl: state.coverUrl ?? null,
    };
  }

  function areStringArraysEqual(left: string[], right: string[]): boolean {
    if (left.length !== right.length) return false;
    return left.every((value, index) => value === right[index]);
  }

  function arePlayerSongsEqual(
    left: PlayerSong | null,
    right: PlayerSong | null
  ): boolean {
    if (left === right) return true;
    if (!left || !right) return false;

    return (
      left.cid === right.cid &&
      left.name === right.name &&
      left.coverUrl === right.coverUrl &&
      areStringArraysEqual(left.artists, right.artists)
    );
  }

  function assignPlayerStateFields(state: PlayerState) {
    if (state.sessionId >= lastPlaybackSnapshot.sessionId) {
      lastPlaybackSnapshot = {
        ...lastPlaybackSnapshot,
        sessionId: state.sessionId,
      };
    }
    if (isPlaying !== state.isPlaying) isPlaying = state.isPlaying;
    if (isPaused !== state.isPaused) isPaused = state.isPaused;
    if (isLoading !== state.isLoading) isLoading = state.isLoading;
    if (hasPrevious !== state.hasPrevious) hasPrevious = state.hasPrevious;
    if (hasNext !== state.hasNext) hasNext = state.hasNext;
    if (progress !== state.progress) progress = state.progress;
    if (duration !== state.duration) duration = state.duration;
    if (Math.abs(volume - state.volume) > 0.001) volume = state.volume;
    if (state.volume > 0 && muted) muted = false;
  }

  function clearPlayTogglePendingWhenSettled(state: PlayerState) {
    if (!pendingPlayToggleTarget) return;
    if (state.isLoading) return;
    if (!state.songCid || (!state.isPlaying && !state.isPaused)) {
      pendingPlayToggleTarget = null;
      return;
    }
    if (
      (pendingPlayToggleTarget === 'playing' && state.isPlaying) ||
      (pendingPlayToggleTarget === 'paused' && state.isPaused)
    ) {
      pendingPlayToggleTarget = null;
    }
  }

  function buildSinglePlaybackEntry(song: PlayerSong): PlaybackQueueEntry {
    return {
      cid: song.cid,
      name: song.name,
      artists: song.artists,
      coverUrl: song.coverUrl,
    };
  }

  function shufflePlaybackEntries(
    entries: PlaybackQueueEntry[],
    currentCid: string | null
  ): PlaybackQueueEntry[] {
    if (entries.length <= 1) return [...entries];

    const rest = [...entries];
    let pinnedEntry: PlaybackQueueEntry | null = null;

    if (currentCid) {
      const pinnedIndex = rest.findIndex((entry) => entry.cid === currentCid);
      if (pinnedIndex >= 0) {
        pinnedEntry = rest.splice(pinnedIndex, 1)[0];
      }
    }

    for (let index = rest.length - 1; index > 0; index -= 1) {
      const swapIndex = Math.floor(Math.random() * (index + 1));
      [rest[index], rest[swapIndex]] = [rest[swapIndex], rest[index]];
    }

    return pinnedEntry ? [pinnedEntry, ...rest] : rest;
  }

  function applyPlaybackQueue(
    entries: PlaybackQueueEntry[],
    currentCid: string | null
  ) {
    playbackEntries = [...entries];

    if (!entries.length) {
      playbackOrder = [];
      playbackIndex = -1;
      return;
    }

    playbackOrder = shuffleEnabled
      ? shufflePlaybackEntries(entries, currentCid)
      : [...entries];
    playbackIndex = currentCid
      ? playbackOrder.findIndex((entry) => entry.cid === currentCid)
      : 0;

    if (playbackIndex < 0) {
      playbackIndex = 0;
    }
  }

  function syncPlaybackQueueWithSong(song: PlayerSong | null) {
    if (!song) {
      if (playbackIndex !== -1) {
        playbackIndex = -1;
      }
      return;
    }

    const currentOrderIndex = playbackOrder.findIndex(
      (entry) => entry.cid === song.cid
    );
    if (currentOrderIndex >= 0) {
      if (playbackIndex !== currentOrderIndex) {
        playbackIndex = currentOrderIndex;
      }
      return;
    }

    const currentSourceIndex = playbackEntries.findIndex(
      (entry) => entry.cid === song.cid
    );
    if (currentSourceIndex >= 0) {
      applyPlaybackQueue(playbackEntries, song.cid);
      return;
    }

    applyPlaybackQueue([buildSinglePlaybackEntry(song)], song.cid);
  }

  async function loadLyrics(songCid: string) {
    const requestSeq = ++lyricRequestSeq;
    lyricsSongCid = songCid;
    lyricsLoading = true;
    lyricsError = '';
    lyricsLines = [];

    try {
      const lyricText = await deps.getSongLyrics(songCid);
      if (requestSeq !== lyricRequestSeq) return;

      if (!lyricText) {
        lyricsLoading = false;
        return;
      }

      lyricsLines = parseLyricText(lyricText);
    } catch (error) {
      if (requestSeq !== lyricRequestSeq) return;
      lyricsError = error instanceof Error ? error.message : String(error);
    } finally {
      if (requestSeq === lyricRequestSeq) {
        lyricsLoading = false;
      }
    }
  }

  function syncPlayerState(state: PlayerState) {
    const nextSong = normalizePlayerSong(state);
    if (!arePlayerSongsEqual(currentSong, nextSong)) {
      currentSong = nextSong;
    }
    assignPlayerStateFields(state);
    clearPlayTogglePendingWhenSettled(state);

    if (!state.isLoading && playingCid !== null) {
      playingCid = null;
    }

    syncPlaybackQueueWithSong(currentSong);
  }

  function syncPlayerProgress(state: PlayerState) {
    if (progress !== state.progress) progress = state.progress;
    if (duration !== state.duration) duration = state.duration;
  }

  function syncPlaybackLifecycle() {
    const songCid = currentSong?.cid ?? null;
    const isCurrentActive = Boolean(songCid) && (isPlaying || isPaused);
    const previousSnapshot = lastPlaybackSnapshot;

    if (!songCid) {
      lyricRequestSeq += 1;
      lyricsSongCid = null;
      lyricsLines = [];
      lyricsError = '';
      lyricsLoading = false;
      lyricsOpen = false;
      playlistOpen = false;
      if (previousSnapshot.cid !== null || previousSnapshot.active) {
        lastPlaybackSnapshot = { cid: null, active: false, sessionId: 0 };
      }
      return;
    }

    if (
      previousSnapshot.cid !== songCid ||
      previousSnapshot.active !== isCurrentActive
    ) {
      lastPlaybackSnapshot = {
        cid: songCid,
        active: isCurrentActive,
        sessionId: previousSnapshot.sessionId,
      };
    }

    if (isLoading || songCid === lyricsSongCid) {
      return;
    }

    void loadLyrics(songCid);
  }

  async function playQueueEntry(
    entry: PlaybackQueueEntry,
    order = playbackOrder,
    index = order.findIndex((candidate) => candidate.cid === entry.cid),
    options: { forceRestart?: boolean } = {}
  ) {
    if (index < 0) return;

    playbackOrder = [...order];
    playbackIndex = index;

    if (!options.forceRestart) {
      if (currentSong?.cid === entry.cid && isPaused) {
        await resume();
        return;
      }

      if (currentSong?.cid === entry.cid && (isPlaying || isLoading)) {
        return;
      }

      if (playingCid === entry.cid) {
        return;
      }
    }

    playingCid = entry.cid;
    const requestSeq = ++playRequestSeq;
    try {
      const context = buildPlaybackContext(playbackOrder, playbackIndex);
      await deps.playSong(entry.cid, entry.coverUrl ?? null, context ?? null);
    } catch (error) {
      if (shouldIgnorePlaybackError(error, requestSeq, playRequestSeq)) {
        return;
      }
      playingCid = null;
      deps.notifyError(
        m.player_error_play_failed({
          error: error instanceof Error ? error.message : String(error),
        })
      );
    }
  }

  function syncPlaybackEnded(event: PlaybackEndedEvent) {
    if (
      !shouldApplyPlaybackEnded(
        event,
        currentSong?.cid ?? null,
        lastPlaybackSnapshot
      )
    )
      return;
    lastPlaybackSnapshot = {
      cid: event.songCid,
      active: false,
      sessionId: event.sessionId,
    };
    progress = event.progress;
    duration = event.duration;
    void handlePlaybackEnded(event.songCid);
  }

  function resolveWrappedQueueIndex(step: 1 | -1): number {
    if (!playbackOrder.length) return -1;

    const base = playbackIndex >= 0 ? playbackIndex : 0;
    const next = base + step;

    if (next < 0) {
      return playbackOrder.length - 1;
    }
    if (next >= playbackOrder.length) {
      return 0;
    }
    return next;
  }

  function toggleShuffle(next: boolean) {
    const currentCid = currentSong?.cid ?? null;
    shuffleEnabled = next;
    applyPlaybackQueue(playbackEntries, currentCid);
  }

  function toggleRepeat(next: RepeatMode) {
    repeatMode = next;
  }

  function toggleLyrics() {
    if (!currentSong) return;
    if (
      !lyricsOpen &&
      !lyricsLoading &&
      lyricsLines.length === 0 &&
      !lyricsError
    )
      return;
    lyricsOpen = !lyricsOpen;
    if (lyricsOpen) {
      playlistOpen = false;
    }
  }

  function togglePlaylist() {
    if (!currentSong) return;
    playlistOpen = !playlistOpen;
    if (playlistOpen) {
      lyricsOpen = false;
    }
  }

  function toggleFullscreen() {
    if (!currentSong) return;
    fullscreenOpen = !fullscreenOpen;
    if (fullscreenOpen) {
      lyricsOpen = false;
    }
  }

  async function handlePlaybackEnded(songCid: string) {
    const requestSeq = ++playbackEndRequestSeq;

    if (repeatMode === 'one') {
      const entry = playbackOrder.find((e) => e.cid === songCid);
      if (entry) {
        const index = playbackOrder.indexOf(entry);
        await playQueueEntry(entry, playbackOrder, index, {
          forceRestart: true,
        });
      }
      return;
    }

    if (!playbackOrder.length) return;
    const currentIndex = playbackOrder.findIndex(
      (entry) => entry.cid === songCid
    );
    if (currentIndex < 0) return;

    const nextIndex =
      currentIndex + 1 >= playbackOrder.length ? 0 : currentIndex + 1;
    if (requestSeq !== playbackEndRequestSeq) return;
    await playQueueEntry(playbackOrder[nextIndex], playbackOrder, nextIndex, {
      forceRestart: true,
    });
  }

  async function pause() {
    if (pendingPlayToggleTarget || isLoading) return;
    pendingPlayToggleTarget = 'paused';
    try {
      await deps.pausePlayback();
    } catch (error) {
      pendingPlayToggleTarget = null;
      deps.notifyError(
        m.player_error_pause_failed({
          error: error instanceof Error ? error.message : String(error),
        })
      );
      return;
    }

    try {
      syncPlayerState(await deps.getPlayerState());
    } catch {
      pendingPlayToggleTarget = null;
    }
  }

  async function resume() {
    if (pendingPlayToggleTarget || isLoading) return;
    pendingPlayToggleTarget = 'playing';
    try {
      await deps.resumePlayback();
    } catch (error) {
      pendingPlayToggleTarget = null;
      deps.notifyError(
        m.player_error_resume_failed({
          error: error instanceof Error ? error.message : String(error),
        })
      );
      return;
    }

    try {
      syncPlayerState(await deps.getPlayerState());
    } catch {
      pendingPlayToggleTarget = null;
    }
  }

  async function seek(positionSecs: number) {
    if (!duration || duration <= 0 || isLoading) return;
    try {
      await deps.seekCurrentPlayback(positionSecs);
    } catch (error) {
      deps.notifyError(
        m.player_error_seek_failed({
          error: error instanceof Error ? error.message : String(error),
        })
      );
    }
  }

  async function playNext() {
    const nextIndex = resolveWrappedQueueIndex(1);
    if (nextIndex < 0) return;
    await playQueueEntry(playbackOrder[nextIndex], playbackOrder, nextIndex);
  }

  async function playPrevious() {
    if (!currentSong) return;

    if (progress > 3 && !isLoading) {
      await seek(0);
      return;
    }

    const previousIndex = resolveWrappedQueueIndex(-1);
    if (previousIndex < 0) return;
    await playQueueEntry(
      playbackOrder[previousIndex],
      playbackOrder,
      previousIndex
    );
  }

  async function setVolume(gain: number) {
    const clamped = Math.max(0, Math.min(1, gain));
    volume = clamped;
    if (clamped > 0) muted = false;
    try {
      await deps.setPlaybackVolume(clamped);
    } catch (error) {
      deps.notifyError(error instanceof Error ? error.message : String(error));
    }
  }

  function toggleMute() {
    if (muted) {
      const restored = volumeBeforeMute > 0.05 ? volumeBeforeMute : 0.5;
      void setVolume(restored);
      muted = false;
    } else {
      volumeBeforeMute = volume;
      void setVolume(0);
      muted = true;
    }
  }

  function dispose() {
    initialized = false;
    currentSong = null;
    isPlaying = false;
    isPaused = false;
    isLoading = false;
    pendingPlayToggleTarget = null;
    hasPrevious = false;
    hasNext = false;
    progress = 0;
    duration = 0;
    shuffleEnabled = false;
    repeatMode = 'all';
    playbackEntries = [];
    playbackOrder = [];
    playbackIndex = -1;
    lyricsOpen = false;
    playlistOpen = false;
    fullscreenOpen = false;
    lyricsLoading = false;
    lyricsError = '';
    lyricsLines = [];
    lyricsSongCid = null;
    playingCid = null;
    volume = 1.0;
    muted = false;
    volumeBeforeMute = 1.0;
    lastPlaybackSnapshot = { cid: null, active: false, sessionId: 0 };
    lyricRequestSeq += 1;
    playbackEndRequestSeq += 1;
  }

  return {
    get currentSong() {
      return currentSong;
    },
    get isPlaying() {
      return isPlaying;
    },
    get isPaused() {
      return isPaused;
    },
    get isLoading() {
      return isLoading;
    },
    get isPlayTogglePending() {
      return pendingPlayToggleTarget !== null;
    },
    get hasPrevious() {
      return hasPrevious;
    },
    get hasNext() {
      return hasNext;
    },
    get progress() {
      return progress;
    },
    get duration() {
      return duration;
    },
    get shuffleEnabled() {
      return shuffleEnabled;
    },
    get repeatMode() {
      return repeatMode;
    },
    get playbackEntries() {
      return playbackEntries;
    },
    get playbackOrder() {
      return playbackOrder;
    },
    get playbackIndex() {
      return playbackIndex;
    },
    get lyricsOpen() {
      return lyricsOpen;
    },
    get playlistOpen() {
      return playlistOpen;
    },
    get fullscreenOpen() {
      return fullscreenOpen;
    },
    get lyricsLoading() {
      return lyricsLoading;
    },
    get lyricsError() {
      return lyricsError;
    },
    get lyricsLines() {
      return lyricsLines;
    },
    get lyricsSongCid() {
      return lyricsSongCid;
    },
    get playingCid() {
      return playingCid;
    },
    get volume() {
      return volume;
    },
    get muted() {
      return muted;
    },
    get hasLyrics() {
      return !lyricsLoading && lyricsLines.length > 0;
    },
    get lyricsUnavailable() {
      return !lyricsLoading && lyricsLines.length === 0 && !lyricsError;
    },
    get lastPlaybackSnapshot() {
      return lastPlaybackSnapshot;
    },
    get playerHasPrevious() {
      return playbackOrder.length > 1;
    },
    get playerHasNext() {
      return playbackOrder.length > 1;
    },
    init,
    dispose,
    syncPlayerState,
    syncPlayerProgress,
    syncPlaybackEnded,
    syncPlaybackLifecycle,
    playQueueEntry,
    toggleShuffle,
    toggleRepeat,
    toggleLyrics,
    togglePlaylist,
    toggleFullscreen,
    handlePlaybackEnded,
    pause,
    resume,
    seek,
    playNext,
    playPrevious,
    setVolume,
    toggleMute,
    applyPlaybackQueue,
    buildSinglePlaybackEntry,
  };
}
