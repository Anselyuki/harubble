import type {
  PlaybackContext,
  PlaybackQueueEntry,
  PlayerState,
  PlaybackEndedEvent,
  PlaybackFormatState,
} from '$lib/types';
import { parseLyricText } from './lyrics';
import { buildPlaybackContext } from './queue';
import {
  isPlaybackSupersededError,
  shouldApplyPlaybackEnded,
  shouldIgnorePlaybackError,
  type PlaybackSnapshot,
} from './playback-contract';
import * as m from '$lib/paraglide/messages.js';
import {
  formatLibraryError,
  formatPlaybackError,
} from '$lib/features/shell/domainErrors';

interface PlayerControllerDeps {
  playSong: (
    songCid: string,
    coverUrl: string | null,
    context: PlaybackContext | null
  ) => Promise<void>;
  playNextTrack: () => Promise<void>;
  playPreviousTrack: () => Promise<void>;
  pausePlayback: () => Promise<void>;
  resumePlayback: () => Promise<void>;
  seekCurrentPlayback: (positionSecs: number) => Promise<void>;
  setPlaybackVolume: (volume: number) => Promise<number>;
  getPlayerState: () => Promise<PlayerState>;
  getSongLyrics: (songCid: string) => Promise<string | null>;
  recordSongHeat: (songCid: string, coverUrl: string | null) => void;
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
  let playbackFormat = $state<PlaybackFormatState | null>(null);
  let volumeBeforeMute = 1.0;
  let playbackEndRequestSeq = 0;
  let playRequestSeq = 0;
  let heatFiredSessionId = -1;
  // 记录本地对 volume/muted 的最近一次用户修改时间。事件回环期间不允许后端事件
  // 覆盖用户在拖动过程中的临时值，否则滑块会明显 "弹回"。
  let localVolumeMutationAt = 0;
  const LOCAL_VOLUME_GUARD_MS = 600;
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
    // 保护 volume/muted：用户刚刚在本地做了修改的短窗口内，忽略后端事件的回环，
    // 否则一条稍稍滞后的 player-state-changed 会让滑块或静音图标"弹回"旧值。
    const volumeGuardActive =
      Date.now() - localVolumeMutationAt < LOCAL_VOLUME_GUARD_MS;
    if (!volumeGuardActive) {
      if (Math.abs(volume - state.volume) > 0.001) volume = state.volume;
      if (state.volume > 0 && muted) muted = false;
    }
    if (playbackFormat !== state.playbackFormat) {
      playbackFormat = state.playbackFormat;
    }
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
      lyricsError = formatLibraryError(error);
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

    // 播放进度达到 50% 时记录热度，每个播放会话只触发一次
    if (
      state.isPlaying &&
      state.duration > 0 &&
      state.progress / state.duration >= 0.5 &&
      state.songCid &&
      state.sessionId !== heatFiredSessionId
    ) {
      heatFiredSessionId = state.sessionId;
      deps.recordSongHeat(state.songCid, state.coverUrl ?? null);
    }
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
      if (requestSeq !== playRequestSeq) return;
      // 事件回环 (player-state-changed) 是主要同步路径；仅在事件尚未把 currentSong
      // 推进到 entry.cid 的窄窗口（IPC ack 早于事件到达）里做一次兜底拉取，避免
      // 无条件二次同步导致把已通过事件推进的 UI 状态回滚到较旧的快照。
      if (currentSong?.cid !== entry.cid) {
        try {
          syncPlayerState(await deps.getPlayerState());
        } catch {
          // 兜底拉取失败无需向用户呈现，后续事件仍会继续同步。
        }
      }
    } catch (error) {
      if (shouldIgnorePlaybackError(error, requestSeq, playRequestSeq)) {
        return;
      }
      playingCid = null;
      deps.notifyError(
        m.player_error_play_failed({
          error: formatPlaybackError(error),
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

  function shouldRestartCompletedTrack() {
    return (
      currentSong !== null &&
      !isPlaying &&
      !isPaused &&
      !hasNext &&
      duration > 0 &&
      progress >= duration - 0.05
    );
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
          error: formatPlaybackError(error),
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
      if (shouldRestartCompletedTrack()) {
        await deps.seekCurrentPlayback(0);
      } else {
        await deps.resumePlayback();
      }
    } catch (error) {
      pendingPlayToggleTarget = null;
      deps.notifyError(
        m.player_error_resume_failed({
          error: formatPlaybackError(error),
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
      // 快速拖动进度条 / 连点歌词行会让后一次 seek supersede 前一次，
      // 前一次返回的 superseded 错误不能弹为用户看得见的 toast。
      if (isPlaybackSupersededError(error)) return;
      deps.notifyError(
        m.player_error_seek_failed({
          error: formatPlaybackError(error),
        })
      );
    }
  }

  async function playNext() {
    if (!playbackOrder.length) return;
    // 走后端 play_next 命令而不是重新用 play_song 触发 —— 后者会使用 NewSelection
    // 5 秒缓冲预算，而 play_next / play_previous 使用 InteractiveRestart 的 1 秒
    // 缓冲，切歌响应更贴近用户预期，也与 OS 媒体键行为保持一致。
    try {
      await deps.playNextTrack();
    } catch (error) {
      if (isPlaybackSupersededError(error)) return;
      deps.notifyError(
        m.player_error_play_failed({
          error: formatPlaybackError(error),
        })
      );
    }
  }

  async function playPrevious() {
    if (!currentSong) return;

    if (progress > 3 && !isLoading) {
      await seek(0);
      return;
    }

    try {
      await deps.playPreviousTrack();
    } catch (error) {
      if (isPlaybackSupersededError(error)) return;
      deps.notifyError(
        m.player_error_play_failed({
          error: formatPlaybackError(error),
        })
      );
    }
  }

  async function setVolume(gain: number) {
    const clamped = Math.max(0, Math.min(1, gain));
    volume = clamped;
    if (clamped > 0) muted = false;
    localVolumeMutationAt = Date.now();
    try {
      await deps.setPlaybackVolume(clamped);
    } catch (error) {
      deps.notifyError(formatPlaybackError(error));
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
    localVolumeMutationAt = Date.now();
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
    playbackFormat = null;
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
    get playbackFormat() {
      return playbackFormat;
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
