import { assertType, describe, it, expect } from 'vitest';

// Type-level assertions: TypeScript compiler verifies these at build time.
// If a wrapper's return type drifts from the declared shape in types.ts,
// the assignment below will produce a tsc error — no runtime needed.
import type { Collection, CollectionSummary } from '$lib/types';
import type { PlayerState, PlaybackStartResult } from '$lib/types';

import {
  listCollections,
  getCollection,
  createCollection,
  deleteCollection,
  exportCollection,
  importCollection,
} from '$lib/collectionApi';

import {
  playSong,
  pausePlayback,
  resumePlayback,
  getPlayerState,
} from '$lib/api';

// Compile-time shape contracts (these never run; tsc rejects mismatches).
declare const _col: Collection;
declare const _colSummary: CollectionSummary;
declare const _playerState: PlayerState;
declare const _startResult: PlaybackStartResult;
assertType<() => Promise<CollectionSummary[]>>(listCollections);
assertType<(id: string) => Promise<Collection>>(getCollection);
assertType<() => Promise<PlayerState>>(getPlayerState);
assertType<(songCid: string) => Promise<PlaybackStartResult>>(playSong);

describe('IPC contract — Collection domain', () => {
  it('listCollections is a callable function', () => {
    expect(typeof listCollections).toBe('function');
  });
  it('getCollection is a callable function', () => {
    expect(typeof getCollection).toBe('function');
  });
  it('createCollection is a callable function', () => {
    expect(typeof createCollection).toBe('function');
  });
  it('deleteCollection is a callable function', () => {
    expect(typeof deleteCollection).toBe('function');
  });
  it('exportCollection is a callable function', () => {
    expect(typeof exportCollection).toBe('function');
  });
  it('importCollection is a callable function', () => {
    expect(typeof importCollection).toBe('function');
  });
});

describe('IPC contract — Playback domain', () => {
  it('playSong is a callable function', () => {
    expect(typeof playSong).toBe('function');
  });
  it('pausePlayback is a callable function', () => {
    expect(typeof pausePlayback).toBe('function');
  });
  it('resumePlayback is a callable function', () => {
    expect(typeof resumePlayback).toBe('function');
  });
  it('getPlayerState is a callable function', () => {
    expect(typeof getPlayerState).toBe('function');
  });
});
