import { getContext, setContext } from 'svelte';
import type { PlaybackQueueEntry } from '$lib/types';
import { PLAYER_CONTEXT_KEY } from './keys';

export interface PlayerContextSong {
  cid: string;
  name: string;
  artists: string[];
  coverUrl: string | null;
}

export interface PlayerContext {
  readonly currentSong: PlayerContextSong | null;
  readonly isPlaying: boolean;
  readonly isPaused: boolean;
  readonly isLoading: boolean;
  readonly isPlayTogglePending: boolean;
  readonly hasPrevious: boolean;
  readonly hasNext: boolean;
  readonly progress: number;
  readonly duration: number;
  readonly shuffleEnabled: boolean;
  readonly repeatMode: 'all' | 'one';
  readonly playbackOrder: PlaybackQueueEntry[];
  readonly lyricsOpen: boolean;
  readonly playlistOpen: boolean;
  readonly lyricsLoading: boolean;
  readonly lyricsError: string | null;
  readonly lyricsLines: { id: string; time: number | null; text: string }[];
  readonly lyricsUnavailable: boolean;
  readonly activeLyricIndex: number;
  readonly fullscreenOpen: boolean;
  readonly volume: number;
  readonly muted: boolean;
  pause: () => void;
  resume: () => void;
  seek: (positionSecs: number) => void;
  playPrevious: () => void;
  playNext: () => void;
  toggleShuffle: (next: boolean) => void;
  toggleRepeat: (next: 'all' | 'one') => void;
  toggleLyrics: () => void;
  togglePlaylist: () => void;
  toggleFullscreen: () => void;
  setVolume: (volume: number) => void;
  toggleMute: () => void;
  playQueueEntry: (
    entry: PlaybackQueueEntry,
    order?: PlaybackQueueEntry[],
    index?: number
  ) => Promise<void>;
}

export function setPlayerContext(ctx: PlayerContext): void {
  setContext(PLAYER_CONTEXT_KEY, ctx);
}

export function getPlayerContext(): PlayerContext {
  return getContext<PlayerContext>(PLAYER_CONTEXT_KEY);
}
