import { describe, expect, it, vi } from 'vitest';
import type { PlayerState, PlaybackEndedEvent } from '$lib/types';
import { createPlayerController } from './controller.svelte';

function makePlayerState(overrides: Partial<PlayerState> = {}): PlayerState {
  return {
    sessionId: 1,
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
    ...overrides,
  };
}

function makeDeps(
  overrides: Partial<Parameters<typeof createPlayerController>[0]> = {}
): Parameters<typeof createPlayerController>[0] {
  return {
    playSong: vi.fn().mockResolvedValue(undefined),
    playNextTrack: vi.fn().mockResolvedValue(undefined),
    playPreviousTrack: vi.fn().mockResolvedValue(undefined),
    pausePlayback: vi.fn().mockResolvedValue(undefined),
    resumePlayback: vi.fn().mockResolvedValue(undefined),
    seekCurrentPlayback: vi.fn().mockResolvedValue(undefined),
    setPlaybackVolume: vi.fn().mockResolvedValue(1),
    getPlayerState: vi.fn().mockResolvedValue(makePlayerState()),
    getSongLyrics: vi.fn().mockResolvedValue(null),
    recordSongHeat: vi.fn(),
    notifyError: vi.fn(),
    ...overrides,
  };
}

describe('createPlayerController', () => {
  describe('syncPlayerState', () => {
    it('accepts the initial idle session snapshot', () => {
      const ctrl = createPlayerController(makeDeps());

      ctrl.syncPlayerState(makePlayerState({ sessionId: 0, volume: 0.4 }));

      expect(ctrl.volume).toBe(0.4);
      expect(ctrl.lastPlaybackSnapshot.sessionId).toBe(0);
    });

    it('projects isPlaying and progress from PlayerState', () => {
      const ctrl = createPlayerController(makeDeps());
      ctrl.syncPlayerState(
        makePlayerState({ isPlaying: true, progress: 42, duration: 100 })
      );

      expect(ctrl.isPlaying).toBe(true);
      expect(ctrl.progress).toBe(42);
      expect(ctrl.duration).toBe(100);
    });

    it('clears playingCid once loading is false via subsequent state event', async () => {
      const deps = makeDeps({
        playSong: vi.fn().mockResolvedValue(undefined),
      });
      const ctrl = createPlayerController(deps);

      const entries = [
        { cid: 'song-a', name: 'Song A', artists: [], coverUrl: null },
      ];
      ctrl.applyPlaybackQueue(entries, 'song-a');
      await ctrl.playQueueEntry(entries[0]);
      ctrl.syncPlayerState(
        makePlayerState({
          isPlaying: true,
          songCid: 'song-a',
          isLoading: false,
        })
      );

      expect(ctrl.playingCid).toBeNull();
    });

    it('reads back the settled state when only the loading event arrived', async () => {
      const settledState = makePlayerState({
        sessionId: 2,
        songCid: 'song-a',
        songName: 'Song A',
        isPlaying: true,
        isLoading: false,
      });
      const getPlayerState = vi.fn().mockResolvedValue(settledState);
      const playSong = vi.fn();
      const deps = makeDeps({
        getPlayerState,
        playSong,
      });
      const ctrl = createPlayerController(deps);
      playSong.mockImplementation(async () => {
        ctrl.syncPlayerState(
          makePlayerState({
            sessionId: 2,
            songCid: 'song-a',
            songName: 'Song A',
            isLoading: true,
          })
        );
      });
      const entries = [
        { cid: 'song-a', name: 'Song A', artists: [], coverUrl: null },
      ];

      ctrl.applyPlaybackQueue(entries, 'song-a');
      await ctrl.playQueueEntry(entries[0]);

      expect(getPlayerState).toHaveBeenCalledOnce();
      expect(ctrl.isLoading).toBe(false);
      expect(ctrl.isPlaying).toBe(true);
      expect(ctrl.playingCid).toBeNull();
    });

    it('keeps an event-settled state that arrives during the readback', async () => {
      let resolveReadback: ((state: PlayerState) => void) | undefined;
      const getPlayerState = vi.fn().mockImplementation(
        () =>
          new Promise<PlayerState>((resolve) => {
            resolveReadback = resolve;
          })
      );
      const playSong = vi.fn();
      const deps = makeDeps({
        getPlayerState,
        playSong,
      });
      const ctrl = createPlayerController(deps);
      playSong.mockImplementation(async () => {
        ctrl.syncPlayerState(
          makePlayerState({
            sessionId: 2,
            songCid: 'song-a',
            songName: 'Song A',
            isLoading: true,
          })
        );
      });
      const entries = [
        { cid: 'song-a', name: 'Song A', artists: [], coverUrl: null },
      ];

      ctrl.applyPlaybackQueue(entries, 'song-a');
      const playPromise = ctrl.playQueueEntry(entries[0]);
      await vi.waitFor(() => expect(getPlayerState).toHaveBeenCalledOnce());
      ctrl.syncPlayerState(
        makePlayerState({
          sessionId: 2,
          songCid: 'song-a',
          songName: 'Song A',
          isPlaying: true,
          isLoading: false,
          progress: 12,
        })
      );
      resolveReadback?.(
        makePlayerState({
          sessionId: 2,
          songCid: 'song-a',
          songName: 'Song A',
          isLoading: true,
        })
      );
      await playPromise;

      expect(ctrl.isLoading).toBe(false);
      expect(ctrl.isPlaying).toBe(true);
      expect(ctrl.progress).toBe(12);
    });
  });

  describe('playQueueEntry pending guard', () => {
    it('clears a stale pause-pending target when switching to a new track', async () => {
      const deps = makeDeps();
      const ctrl = createPlayerController(deps);

      const initialEntries = [
        { cid: 'song-a', name: 'Song A', artists: [], coverUrl: null },
      ];
      ctrl.applyPlaybackQueue(initialEntries, 'song-a');
      await ctrl.playQueueEntry(initialEntries[0]);
      ctrl.syncPlayerState(
        makePlayerState({
          sessionId: 2,
          songCid: 'song-a',
          songName: 'Song A',
          isPlaying: true,
          isLoading: false,
        })
      );

      // 用户按下暂停：pending 落到 'paused'，但暂停事件尚未回来。
      const pausePromise = ctrl.pause();
      expect(ctrl.isPlayTogglePending).toBe(true);
      await pausePromise;

      // 暂停命令仍在飞行中，用户又双击了另一首歌：新的 playQueueEntry 必须清掉
      // 无法再被匹配的旧 pending，否则 loading 状态永远不会退出。
      const nextEntries = [
        { cid: 'song-b', name: 'Song B', artists: [], coverUrl: null },
      ];
      ctrl.applyPlaybackQueue(nextEntries, 'song-b');
      await ctrl.playQueueEntry(nextEntries[0]);
      ctrl.syncPlayerState(
        makePlayerState({
          sessionId: 3,
          songCid: 'song-b',
          songName: 'Song B',
          isPlaying: true,
          isLoading: false,
        })
      );

      expect(ctrl.isPlayTogglePending).toBe(false);
      expect(ctrl.isLoading).toBe(false);
      expect(ctrl.isPlaying).toBe(true);
    });
  });

  describe('syncPlaybackEnded', () => {
    it('ignores ended events from a stale session', () => {
      const ctrl = createPlayerController(makeDeps());
      ctrl.syncPlayerState(
        makePlayerState({ isPlaying: true, songCid: 'song-a', sessionId: 5 })
      );

      const staleEvent: PlaybackEndedEvent = {
        sessionId: 3,
        songCid: 'song-a',
        progress: 200,
        duration: 200,
      };
      ctrl.syncPlaybackEnded(staleEvent);

      expect(ctrl.progress).toBe(0);
    });

    it('applies ended event for the current session', () => {
      const ctrl = createPlayerController(makeDeps());
      ctrl.syncPlayerState(
        makePlayerState({
          isPlaying: true,
          isPaused: false,
          songCid: 'song-a',
          sessionId: 4,
          progress: 0,
          duration: 200,
        })
      );

      const event: PlaybackEndedEvent = {
        sessionId: 4,
        songCid: 'song-a',
        progress: 200,
        duration: 200,
      };
      ctrl.syncPlaybackEnded(event);

      expect(ctrl.progress).toBe(200);
      expect(ctrl.isPlaying).toBe(false);
    });

    it('rejects a duplicate ended event with the same session id', () => {
      const ctrl = createPlayerController(makeDeps());
      ctrl.syncPlayerState(
        makePlayerState({
          isPlaying: true,
          isPaused: false,
          songCid: 'song-a',
          sessionId: 4,
          progress: 10,
          duration: 200,
        })
      );

      const event: PlaybackEndedEvent = {
        sessionId: 4,
        songCid: 'song-a',
        progress: 200,
        duration: 200,
      };
      ctrl.syncPlaybackEnded(event);
      ctrl.syncPlaybackEnded({ ...event, progress: 150 });

      expect(ctrl.progress).toBe(200);
    });

    it('handles the completed state snapshot and following ended event once', async () => {
      const playSong = vi.fn().mockResolvedValue(undefined);
      const ctrl = createPlayerController(makeDeps({ playSong }));
      const entries = [
        { cid: 'song-a', name: 'Song A', artists: [], coverUrl: null },
        { cid: 'song-b', name: 'Song B', artists: [], coverUrl: null },
      ];
      ctrl.applyPlaybackQueue(entries, 'song-b');
      ctrl.toggleRepeat('all');
      ctrl.syncPlayerState(
        makePlayerState({
          sessionId: 4,
          songCid: 'song-b',
          songName: 'Song B',
          isPlaying: true,
          progress: 190,
          duration: 200,
        })
      );

      ctrl.syncPlayerState(
        makePlayerState({
          sessionId: 4,
          songCid: 'song-b',
          songName: 'Song B',
          progress: 200,
          duration: 200,
        })
      );
      ctrl.syncPlaybackEnded({
        sessionId: 4,
        songCid: 'song-b',
        progress: 200,
        duration: 200,
      });

      await vi.waitFor(() => expect(playSong).toHaveBeenCalledOnce());
      expect(playSong).toHaveBeenCalledWith(
        'song-a',
        null,
        expect.objectContaining({ currentIndex: 0 })
      );
    });

    it('does not let late progress roll back a completed session', () => {
      const ctrl = createPlayerController(makeDeps());
      ctrl.syncPlayerState(
        makePlayerState({
          sessionId: 6,
          songCid: 'song-a',
          songName: 'Song A',
          isPlaying: true,
          progress: 190,
          duration: 200,
        })
      );
      ctrl.syncPlayerState(
        makePlayerState({
          sessionId: 6,
          songCid: 'song-a',
          songName: 'Song A',
          progress: 200,
          duration: 200,
        })
      );

      ctrl.syncPlayerProgress(
        makePlayerState({
          sessionId: 6,
          songCid: 'song-a',
          songName: 'Song A',
          isPlaying: true,
          progress: 199.5,
          duration: 200,
        })
      );

      expect(ctrl.progress).toBe(200);
      expect(ctrl.isPlaying).toBe(false);
    });
  });

  describe('repeat mode playback end behavior', () => {
    const entries = [
      { cid: 'song-a', name: 'Song A', artists: [], coverUrl: null },
      { cid: 'song-b', name: 'Song B', artists: [], coverUrl: null },
    ];

    it('defaults to repeat off and stops at the end of the queue', async () => {
      const playSong = vi.fn().mockResolvedValue(undefined);
      const ctrl = createPlayerController(makeDeps({ playSong }));
      ctrl.applyPlaybackQueue(entries, 'song-b');

      await ctrl.handlePlaybackEnded('song-b');

      expect(ctrl.repeatMode).toBe('off');
      expect(playSong).not.toHaveBeenCalled();
    });

    it('stops after the selected entry when repeat is off', async () => {
      const playSong = vi.fn().mockResolvedValue(undefined);
      const ctrl = createPlayerController(makeDeps({ playSong }));
      ctrl.applyPlaybackQueue(entries, 'song-a');

      await ctrl.handlePlaybackEnded('song-a');

      expect(playSong).not.toHaveBeenCalled();
    });

    it('wraps to the first entry in repeat all mode', async () => {
      const playSong = vi.fn().mockResolvedValue(undefined);
      const ctrl = createPlayerController(makeDeps({ playSong }));
      ctrl.applyPlaybackQueue(entries, 'song-b');
      ctrl.toggleRepeat('all');

      await ctrl.handlePlaybackEnded('song-b');

      expect(playSong).toHaveBeenCalledOnce();
      expect(playSong).toHaveBeenCalledWith(
        'song-a',
        null,
        expect.objectContaining({ currentIndex: 0 })
      );
    });

    it('restarts the current entry in repeat one mode', async () => {
      const playSong = vi.fn().mockResolvedValue(undefined);
      const ctrl = createPlayerController(makeDeps({ playSong }));
      ctrl.applyPlaybackQueue(entries, 'song-b');
      ctrl.toggleRepeat('one');

      await ctrl.handlePlaybackEnded('song-b');

      expect(playSong).toHaveBeenCalledOnce();
      expect(playSong).toHaveBeenCalledWith(
        'song-b',
        null,
        expect.objectContaining({ currentIndex: 1 })
      );
    });
  });

  describe('resume', () => {
    it('restarts a completed track from the beginning when no next track exists', async () => {
      const seekCurrentPlayback = vi.fn().mockResolvedValue(undefined);
      const resumePlayback = vi.fn().mockResolvedValue(undefined);
      const deps = makeDeps({
        seekCurrentPlayback,
        resumePlayback,
        getPlayerState: vi.fn().mockResolvedValue(
          makePlayerState({
            sessionId: 2,
            songCid: 'song-a',
            songName: 'Song A',
            isPlaying: true,
            progress: 0,
            duration: 180,
          })
        ),
      });
      const ctrl = createPlayerController(deps);
      ctrl.syncPlayerState(
        makePlayerState({
          songCid: 'song-a',
          songName: 'Song A',
          progress: 180,
          duration: 180,
          hasNext: false,
        })
      );

      await ctrl.resume();

      expect(seekCurrentPlayback).toHaveBeenCalledOnce();
      expect(seekCurrentPlayback).toHaveBeenCalledWith(0);
      expect(resumePlayback).not.toHaveBeenCalled();
      expect(ctrl.isPlaying).toBe(true);
      expect(ctrl.progress).toBe(0);
    });

    it('transitions a completed track through loading to playing', async () => {
      let resolveSeek: (() => void) | undefined;
      const seekCurrentPlayback = vi.fn(
        () =>
          new Promise<void>((resolve) => {
            resolveSeek = resolve;
          })
      );
      const getPlayerState = vi.fn().mockResolvedValue(
        makePlayerState({
          sessionId: 4,
          songCid: 'song-a',
          songName: 'Song A',
          isPlaying: true,
          progress: 0,
          duration: 180,
        })
      );
      const ctrl = createPlayerController(
        makeDeps({ seekCurrentPlayback, getPlayerState })
      );
      ctrl.syncPlayerState(
        makePlayerState({
          sessionId: 1,
          songCid: 'song-a',
          songName: 'Song A',
          progress: 180,
          duration: 180,
        })
      );

      const resumePromise = ctrl.resume();
      expect(ctrl.isPlayTogglePending).toBe(true);
      expect(seekCurrentPlayback).toHaveBeenCalledWith(0);

      ctrl.syncPlayerState(
        makePlayerState({
          sessionId: 4,
          songCid: 'song-a',
          songName: 'Song A',
          isLoading: true,
          progress: 0,
          duration: 180,
        })
      );
      expect(ctrl.isLoading).toBe(true);
      expect(ctrl.isPlayTogglePending).toBe(true);

      resolveSeek?.();
      await resumePromise;

      expect(ctrl.isLoading).toBe(false);
      expect(ctrl.isPlaying).toBe(true);
      expect(ctrl.isPlayTogglePending).toBe(false);
      expect(ctrl.progress).toBe(0);
    });

    it('keeps using resume for a paused track', async () => {
      const seekCurrentPlayback = vi.fn().mockResolvedValue(undefined);
      const resumePlayback = vi.fn().mockResolvedValue(undefined);
      const deps = makeDeps({
        seekCurrentPlayback,
        resumePlayback,
        getPlayerState: vi.fn().mockResolvedValue(
          makePlayerState({
            songCid: 'song-a',
            songName: 'Song A',
            isPlaying: true,
            progress: 90,
            duration: 180,
          })
        ),
      });
      const ctrl = createPlayerController(deps);
      ctrl.syncPlayerState(
        makePlayerState({
          songCid: 'song-a',
          songName: 'Song A',
          isPaused: true,
          progress: 90,
          duration: 180,
          hasNext: false,
        })
      );

      await ctrl.resume();

      expect(resumePlayback).toHaveBeenCalledOnce();
      expect(seekCurrentPlayback).not.toHaveBeenCalled();
    });

    it('restarts the completed final track when repeat is off', async () => {
      const seekCurrentPlayback = vi.fn().mockResolvedValue(undefined);
      const resumePlayback = vi.fn().mockResolvedValue(undefined);
      const ctrl = createPlayerController(
        makeDeps({
          seekCurrentPlayback,
          resumePlayback,
          getPlayerState: vi.fn().mockResolvedValue(
            makePlayerState({
              sessionId: 2,
              songCid: 'song-b',
              songName: 'Song B',
              isPlaying: true,
              duration: 180,
            })
          ),
        })
      );
      const entries = [
        { cid: 'song-a', name: 'Song A', artists: [], coverUrl: null },
        { cid: 'song-b', name: 'Song B', artists: [], coverUrl: null },
      ];
      ctrl.applyPlaybackQueue(entries, 'song-b');
      ctrl.syncPlayerState(
        makePlayerState({
          songCid: 'song-b',
          songName: 'Song B',
          progress: 180,
          duration: 180,
          hasNext: true,
        })
      );

      await ctrl.resume();

      expect(seekCurrentPlayback).toHaveBeenCalledOnce();
      expect(seekCurrentPlayback).toHaveBeenCalledWith(0);
      expect(resumePlayback).not.toHaveBeenCalled();
    });
  });

  describe('syncPlayerProgress heat', () => {
    it('fires recordSongHeat once at 50% progress', () => {
      const recordSongHeat = vi.fn();
      const ctrl = createPlayerController(makeDeps({ recordSongHeat }));
      ctrl.syncPlayerState(
        makePlayerState({ isPlaying: true, songCid: 'song-a', sessionId: 1 })
      );

      ctrl.syncPlayerProgress(
        makePlayerState({
          isPlaying: true,
          songCid: 'song-a',
          sessionId: 1,
          progress: 60,
          duration: 100,
        })
      );
      ctrl.syncPlayerProgress(
        makePlayerState({
          isPlaying: true,
          songCid: 'song-a',
          sessionId: 1,
          progress: 80,
          duration: 100,
        })
      );

      expect(recordSongHeat).toHaveBeenCalledTimes(1);
    });

    it('fires recordSongHeat again for a new session', () => {
      const recordSongHeat = vi.fn();
      const ctrl = createPlayerController(makeDeps({ recordSongHeat }));

      ctrl.syncPlayerState(
        makePlayerState({
          isPlaying: true,
          songCid: 'song-a',
          sessionId: 1,
        })
      );
      ctrl.syncPlayerProgress(
        makePlayerState({
          isPlaying: true,
          songCid: 'song-a',
          sessionId: 1,
          progress: 60,
          duration: 100,
        })
      );
      ctrl.syncPlayerState(
        makePlayerState({
          isPlaying: true,
          songCid: 'song-b',
          sessionId: 2,
        })
      );
      ctrl.syncPlayerProgress(
        makePlayerState({
          isPlaying: true,
          songCid: 'song-b',
          sessionId: 2,
          progress: 60,
          duration: 100,
        })
      );

      expect(recordSongHeat).toHaveBeenCalledTimes(2);
    });
  });
});
