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
      // controller 不再在 playQueueEntry 结束时主动拉取 PlayerState，改由后端事件推动。
      // 这里显式模拟事件到达以覆盖等价路径。
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

    it('applies ended event for a newer session', () => {
      const ctrl = createPlayerController(makeDeps());
      ctrl.syncPlayerState(
        makePlayerState({
          isPlaying: true,
          isPaused: false,
          songCid: 'song-a',
          sessionId: 3,
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

      expect(ctrl.progress).toBe(10);
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
