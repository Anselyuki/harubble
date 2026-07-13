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
  getAlbums,
  getAlbumDetail,
  getSongDetail,
  getSongLyrics,
  extractImageTheme,
  searchLibrary,
  playSong,
  pausePlayback,
  resumePlayback,
  getPlayerState,
  setPlaybackVolume,
  createDownloadJob,
  listDownloadJobs,
  cancelDownloadJob,
  clearDownloadHistory,
  getLatestAlbums,
  getRecentHistory,
  getHomepageStatus,
  getPreferences,
  setPreferences,
  getLocalInventorySnapshot,
  getAudioMetadata,
  getTagDimensions,
  getAlbumsByTagDimension,
  getTagEditorMerged,
  setTagEditorEntityTag,
  resolveTagEditorConflict,
  listLogRecords,
  getLogFileStatus,
  showMainWindow,
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

// ─── Collection ───────────────────────────────────────────────────────────────

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

// ─── Library ──────────────────────────────────────────────────────────────────

describe('IPC contract — Library domain', () => {
  it('getAlbums is a callable function', () => {
    expect(typeof getAlbums).toBe('function');
  });
  it('getAlbumDetail is a callable function', () => {
    expect(typeof getAlbumDetail).toBe('function');
  });
  it('getSongDetail is a callable function', () => {
    expect(typeof getSongDetail).toBe('function');
  });
  it('getSongLyrics is a callable function', () => {
    expect(typeof getSongLyrics).toBe('function');
  });
  it('extractImageTheme is a callable function', () => {
    expect(typeof extractImageTheme).toBe('function');
  });
});

// ─── Search ───────────────────────────────────────────────────────────────────

describe('IPC contract — Search domain', () => {
  it('searchLibrary is a callable function', () => {
    expect(typeof searchLibrary).toBe('function');
  });
});

// ─── Playback ─────────────────────────────────────────────────────────────────

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
  it('setPlaybackVolume is a callable function', () => {
    expect(typeof setPlaybackVolume).toBe('function');
  });
});

// ─── Downloads ────────────────────────────────────────────────────────────────

describe('IPC contract — Downloads domain', () => {
  it('createDownloadJob is a callable function', () => {
    expect(typeof createDownloadJob).toBe('function');
  });
  it('listDownloadJobs is a callable function', () => {
    expect(typeof listDownloadJobs).toBe('function');
  });
  it('cancelDownloadJob is a callable function', () => {
    expect(typeof cancelDownloadJob).toBe('function');
  });
  it('clearDownloadHistory is a callable function', () => {
    expect(typeof clearDownloadHistory).toBe('function');
  });
});

// ─── Homepage ─────────────────────────────────────────────────────────────────

describe('IPC contract — Homepage domain', () => {
  it('getLatestAlbums is a callable function', () => {
    expect(typeof getLatestAlbums).toBe('function');
  });
  it('getRecentHistory is a callable function', () => {
    expect(typeof getRecentHistory).toBe('function');
  });
  it('getHomepageStatus is a callable function', () => {
    expect(typeof getHomepageStatus).toBe('function');
  });
});

// ─── Preferences ──────────────────────────────────────────────────────────────

describe('IPC contract — Preferences domain', () => {
  it('getPreferences is a callable function', () => {
    expect(typeof getPreferences).toBe('function');
  });
  it('setPreferences is a callable function', () => {
    expect(typeof setPreferences).toBe('function');
  });
});

// ─── Local Inventory ──────────────────────────────────────────────────────────

describe('IPC contract — Local Inventory domain', () => {
  it('getLocalInventorySnapshot is a callable function', () => {
    expect(typeof getLocalInventorySnapshot).toBe('function');
  });
  it('getAudioMetadata is a callable function', () => {
    expect(typeof getAudioMetadata).toBe('function');
  });
});

// ─── Tag Registry ─────────────────────────────────────────────────────────────

describe('IPC contract — Tag Registry domain', () => {
  it('getTagDimensions is a callable function', () => {
    expect(typeof getTagDimensions).toBe('function');
  });
  it('getAlbumsByTagDimension is a callable function', () => {
    expect(typeof getAlbumsByTagDimension).toBe('function');
  });
});

// ─── Tag Editor ───────────────────────────────────────────────────────────────

describe('IPC contract — Tag Editor domain', () => {
  it('getTagEditorMerged is a callable function', () => {
    expect(typeof getTagEditorMerged).toBe('function');
  });
  it('setTagEditorEntityTag is a callable function', () => {
    expect(typeof setTagEditorEntityTag).toBe('function');
  });
  it('resolveTagEditorConflict is a callable function', () => {
    expect(typeof resolveTagEditorConflict).toBe('function');
  });
});

// ─── Logging ──────────────────────────────────────────────────────────────────

describe('IPC contract — Logging domain', () => {
  it('listLogRecords is a callable function', () => {
    expect(typeof listLogRecords).toBe('function');
  });
  it('getLogFileStatus is a callable function', () => {
    expect(typeof getLogFileStatus).toBe('function');
  });
});

// ─── Window ───────────────────────────────────────────────────────────────────

describe('IPC contract — Window domain', () => {
  it('showMainWindow is a callable function', () => {
    expect(typeof showMainWindow).toBe('function');
  });
});
