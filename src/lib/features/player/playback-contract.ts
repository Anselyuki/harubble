import { PlaybackCommandError } from '$lib/api';
import type { PlaybackEndedEvent } from '$lib/types';

export interface PlaybackSnapshot {
  cid: string | null;
  active: boolean;
  sessionId: number;
}

export function shouldIgnorePlaybackError(
  error: unknown,
  requestSeq: number,
  activeRequestSeq: number
): boolean {
  return (
    requestSeq !== activeRequestSeq ||
    (error instanceof PlaybackCommandError && error.code === 'superseded')
  );
}

export function shouldApplyPlaybackEnded(
  event: PlaybackEndedEvent,
  currentCid: string | null,
  snapshot: PlaybackSnapshot
): boolean {
  return currentCid === event.songCid && event.sessionId >= snapshot.sessionId;
}
