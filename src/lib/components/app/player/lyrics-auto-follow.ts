export const LYRICS_AUTO_FOLLOW_DELAY_MS = 3_000;

interface LyricsAutoFollowOptions {
  isPlaying: () => boolean;
  followActiveLyric: () => void;
  delayMs?: number;
}

interface LyricsAutoFollowController {
  readonly followSuspended: boolean;
  handleUserScrollIntent(): void;
  handleScroll(): void;
  handlePlaybackChange(): void;
  destroy(): void;
}

export function createLyricsAutoFollowController({
  isPlaying,
  followActiveLyric,
  delayMs = LYRICS_AUTO_FOLLOW_DELAY_MS,
}: LyricsAutoFollowOptions): LyricsAutoFollowController {
  let followSuspended = false;
  let resumeTimer: ReturnType<typeof setTimeout> | null = null;

  function clearResumeTimer() {
    if (resumeTimer === null) return;
    clearTimeout(resumeTimer);
    resumeTimer = null;
  }

  function scheduleResume() {
    clearResumeTimer();
    if (!followSuspended || !isPlaying()) return;
    resumeTimer = setTimeout(() => {
      resumeTimer = null;
      if (!isPlaying()) return;
      followSuspended = false;
      followActiveLyric();
    }, delayMs);
  }

  return {
    get followSuspended() {
      return followSuspended;
    },
    handleUserScrollIntent() {
      followSuspended = true;
      scheduleResume();
    },
    handleScroll() {
      if (followSuspended) scheduleResume();
    },
    handlePlaybackChange() {
      if (!followSuspended) return;
      if (isPlaying()) scheduleResume();
      else clearResumeTimer();
    },
    destroy() {
      clearResumeTimer();
    },
  };
}
