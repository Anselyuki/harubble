import { describe, expect, it } from 'vitest';
import { PlaybackCommandError } from '$lib/api';
import {
  hasPlaybackCompleted,
  isPlaybackSupersededError,
  shouldApplyPlaybackEnded,
  shouldApplyPlaybackProgress,
  shouldIgnorePlaybackError,
} from './playback-contract';

describe('playback contract helpers', () => {
  it('recognizes a naturally completed playback state', () => {
    expect(
      hasPlaybackCompleted({
        songCid: 'song-a',
        isPlaying: false,
        isPaused: false,
        isLoading: false,
        progress: 179.96,
        duration: 180,
      })
    ).toBe(true);
  });

  it('does not treat a paused track near its end as completed', () => {
    expect(
      hasPlaybackCompleted({
        songCid: 'song-a',
        isPlaying: false,
        isPaused: true,
        isLoading: false,
        progress: 180,
        duration: 180,
      })
    ).toBe(false);
  });

  it('accepts progress only for the same actively playing session', () => {
    const incoming = {
      sessionId: 8,
      songCid: 'song-a',
      isPlaying: true,
    };

    expect(shouldApplyPlaybackProgress(incoming, incoming)).toBe(true);
    expect(
      shouldApplyPlaybackProgress(incoming, {
        ...incoming,
        isPlaying: false,
      })
    ).toBe(false);
    expect(
      shouldApplyPlaybackProgress(incoming, {
        ...incoming,
        sessionId: 9,
      })
    ).toBe(false);
    expect(
      shouldApplyPlaybackProgress({ ...incoming, songCid: 'song-b' }, incoming)
    ).toBe(false);
  });

  it('suppresses superseded playback command errors', () => {
    const error = new PlaybackCommandError({
      code: 'superseded',
      message: 'Playback request was superseded',
      retryable: false,
      sessionId: 4,
    });

    expect(shouldIgnorePlaybackError(error, 2, 2)).toBe(true);
  });

  it('suppresses stale playback request errors', () => {
    expect(shouldIgnorePlaybackError(new Error('late failure'), 1, 2)).toBe(
      true
    );
  });

  it('keeps current non-superseded playback errors visible', () => {
    expect(shouldIgnorePlaybackError(new Error('decode failed'), 2, 2)).toBe(
      false
    );
  });

  it('detects superseded playback command errors directly', () => {
    expect(
      isPlaybackSupersededError(
        new PlaybackCommandError({
          code: 'superseded',
          message: 'Playback request was superseded',
          retryable: false,
          sessionId: 4,
        })
      )
    ).toBe(true);
    expect(isPlaybackSupersededError(new Error('boom'))).toBe(false);
  });

  it('accepts an unhandled ended event for the current session', () => {
    expect(
      shouldApplyPlaybackEnded(
        { sessionId: 8, songCid: 'song-a', progress: 10, duration: 10 },
        'song-a',
        8,
        7
      )
    ).toBe(true);
  });

  it('rejects stale playback-ended events', () => {
    expect(
      shouldApplyPlaybackEnded(
        { sessionId: 6, songCid: 'song-a', progress: 10, duration: 10 },
        'song-a',
        7,
        5
      )
    ).toBe(false);
  });

  it('rejects duplicate playback-ended events for the same session', () => {
    expect(
      shouldApplyPlaybackEnded(
        { sessionId: 8, songCid: 'song-a', progress: 10, duration: 10 },
        'song-a',
        8,
        8
      )
    ).toBe(false);
  });

  it('rejects playback-ended events for another song', () => {
    expect(
      shouldApplyPlaybackEnded(
        { sessionId: 8, songCid: 'song-b', progress: 10, duration: 10 },
        'song-a',
        8,
        7
      )
    ).toBe(false);
  });
});
