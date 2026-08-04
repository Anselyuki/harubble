import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createLyricsAutoFollowController } from './lyrics-auto-follow';

describe('createLyricsAutoFollowController', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('follows the active lyric three seconds after scrolling stops', () => {
    const followActiveLyric = vi.fn();
    const controller = createLyricsAutoFollowController({
      isPlaying: () => true,
      followActiveLyric,
    });

    controller.handleUserScrollIntent();
    vi.advanceTimersByTime(2_000);
    controller.handleScroll();
    vi.advanceTimersByTime(2_999);

    expect(followActiveLyric).not.toHaveBeenCalled();
    expect(controller.followSuspended).toBe(true);

    vi.advanceTimersByTime(1);

    expect(followActiveLyric).toHaveBeenCalledOnce();
    expect(controller.followSuspended).toBe(false);
  });

  it('waits for playback to resume before starting the delay', () => {
    let isPlaying = false;
    const followActiveLyric = vi.fn();
    const controller = createLyricsAutoFollowController({
      isPlaying: () => isPlaying,
      followActiveLyric,
    });

    controller.handleUserScrollIntent();
    vi.advanceTimersByTime(3_000);
    expect(followActiveLyric).not.toHaveBeenCalled();

    isPlaying = true;
    controller.handlePlaybackChange();
    vi.advanceTimersByTime(2_999);
    expect(followActiveLyric).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(followActiveLyric).toHaveBeenCalledOnce();
  });

  it('cancels a pending resume when playback pauses', () => {
    let isPlaying = true;
    const followActiveLyric = vi.fn();
    const controller = createLyricsAutoFollowController({
      isPlaying: () => isPlaying,
      followActiveLyric,
    });

    controller.handleUserScrollIntent();
    vi.advanceTimersByTime(2_000);
    isPlaying = false;
    controller.handlePlaybackChange();
    vi.advanceTimersByTime(2_000);

    expect(followActiveLyric).not.toHaveBeenCalled();
    expect(controller.followSuspended).toBe(true);
  });
});
