import { PlaybackCommandError } from '$lib/api';
import type { PlaybackEndedEvent, PlayerState } from '$lib/types';

const PLAYBACK_COMPLETION_TOLERANCE_SECS = 0.05;

type PlaybackCompletionState = Pick<
  PlayerState,
  'songCid' | 'isPlaying' | 'isPaused' | 'isLoading' | 'progress' | 'duration'
>;

type PlaybackProgressState = Pick<
  PlayerState,
  'sessionId' | 'songCid' | 'isPlaying'
>;

export interface PlaybackSnapshot {
  cid: string | null;
  active: boolean;
  sessionId: number;
}

export function hasPlaybackCompleted(state: PlaybackCompletionState): boolean {
  return (
    state.songCid !== null &&
    !state.isPlaying &&
    !state.isPaused &&
    !state.isLoading &&
    state.duration > 0 &&
    state.progress >= state.duration - PLAYBACK_COMPLETION_TOLERANCE_SECS
  );
}

export function shouldApplyPlaybackProgress(
  incoming: PlaybackProgressState,
  current: PlaybackProgressState
): boolean {
  return (
    current.isPlaying &&
    incoming.isPlaying &&
    incoming.sessionId === current.sessionId &&
    incoming.songCid === current.songCid
  );
}

export function shouldIgnorePlaybackError(
  error: unknown,
  requestSeq: number,
  activeRequestSeq: number
): boolean {
  return requestSeq !== activeRequestSeq || isPlaybackSupersededError(error);
}

export function isPlaybackSupersededError(error: unknown): boolean {
  return error instanceof PlaybackCommandError && error.code === 'superseded';
}

export function shouldApplyPlaybackEnded(
  event: PlaybackEndedEvent,
  currentCid: string | null,
  currentSessionId: number,
  lastHandledSessionId: number
): boolean {
  return (
    currentCid === event.songCid &&
    event.sessionId === currentSessionId &&
    event.sessionId > lastHandledSessionId
  );
}
