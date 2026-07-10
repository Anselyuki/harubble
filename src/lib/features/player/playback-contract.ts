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
  return requestSeq !== activeRequestSeq || isPlaybackSupersededError(error);
}

export function isPlaybackSupersededError(error: unknown): boolean {
  return error instanceof PlaybackCommandError && error.code === 'superseded';
}

// 严格大于：同一 sessionId 的重复 ended 事件应当被 guard 拒掉，避免在后端出现重复
// emit 时把队列一次性推进两首。
export function shouldApplyPlaybackEnded(
  event: PlaybackEndedEvent,
  currentCid: string | null,
  snapshot: PlaybackSnapshot
): boolean {
  return currentCid === event.songCid && event.sessionId > snapshot.sessionId;
}
