import { afterEach, describe, expect, it, vi } from 'vitest';
import type {
  Album,
  AlbumDetail,
  SongEntry,
  TagEditorMergeConflict,
  TagEditorMergeResult,
  TagEditorRegistry,
} from '$lib/types';
import { createTagEditorController } from './controller.svelte';
import { tagEditorStore } from './store.svelte';
import * as m from '$lib/paraglide/messages.js';

function makeRegistry(
  overrides: Partial<TagEditorRegistry> = {}
): TagEditorRegistry {
  return {
    schemaVersion: 1,
    updatedAt: '2026-01-01T00:00:00Z',
    tagDimensions: [],
    typeDefinitions: {},
    albums: [],
    songs: {},
    ...overrides,
  };
}

function makeAlbum(cid: string, overrides: Partial<Album> = {}): Album {
  return {
    cid,
    name: `Album ${cid}`,
    coverUrl: 'https://example.com/cover.jpg',
    artists: ['Artist'],
    download: {
      isDownloaded: false,
      downloadStatus: 'missing',
      inventoryVersion: '',
    },
    tags: [],
    ...overrides,
  };
}

function makeSong(cid: string): SongEntry {
  return {
    cid,
    name: `Song ${cid}`,
    artists: ['Artist'],
    download: {
      isDownloaded: false,
      downloadStatus: 'missing',
      inventoryVersion: '',
    },
    tags: [],
  };
}

function makeAlbumDetail(cid: string, songs: SongEntry[] = []): AlbumDetail {
  return {
    cid,
    name: `Album ${cid}`,
    intro: null,
    belong: '',
    coverUrl: 'https://example.com/cover.jpg',
    coverDeUrl: null,
    artists: [],
    download: {
      isDownloaded: false,
      downloadStatus: 'missing',
      inventoryVersion: '',
    },
    tags: [],
    songs,
  };
}

function makeConflict(
  overrides: Partial<TagEditorMergeConflict> & {
    cid: string;
    dimensionKey: string;
  }
): TagEditorMergeConflict {
  return {
    entityType: 'album',
    baseValues: null,
    remoteValues: null,
    localValues: null,
    ...overrides,
  };
}

function makeMergeResult(
  conflicts: TagEditorMergeConflict[] = [],
  autoMergedCount = 0
): TagEditorMergeResult {
  return { conflicts, autoMergedCount };
}

function makeDeps(
  overrides: Partial<Parameters<typeof createTagEditorController>[0]> = {}
) {
  return {
    getTagEditorMerged: vi
      .fn<() => Promise<TagEditorRegistry>>()
      .mockResolvedValue(makeRegistry()),
    getTagEditorLocalOverlay: vi
      .fn<() => Promise<TagEditorRegistry>>()
      .mockResolvedValue(makeRegistry()),
    setTagEditorEntityTag: vi.fn().mockResolvedValue(undefined),
    removeTagEditorEntityTag: vi.fn().mockResolvedValue(undefined),
    addTagEditorDimension: vi.fn().mockResolvedValue(undefined),
    removeTagEditorDimension: vi.fn().mockResolvedValue(undefined),
    applyTagEditorRemoteUpdate: vi.fn().mockResolvedValue(makeMergeResult()),
    resolveTagEditorConflict: vi.fn().mockResolvedValue(undefined),
    exportTagEditorRegistry: vi.fn().mockResolvedValue(undefined),
    importTagEditorRegistry: vi.fn().mockResolvedValue(makeMergeResult()),
    pickSavePath: vi
      .fn<(defaultName: string) => Promise<string | null>>()
      .mockResolvedValue('/tmp/tag_registry.json'),
    pickOpenPath: vi
      .fn<() => Promise<string | null>>()
      .mockResolvedValue('/tmp/tag_registry.json'),
    getAlbumDetail: vi
      .fn<(albumCid: string) => Promise<AlbumDetail>>()
      .mockResolvedValue(makeAlbumDetail('a')),
    getAlbums: vi.fn<() => Album[]>().mockReturnValue([]),
    notifyError: vi.fn<(message: string) => void>(),
    ...overrides,
  };
}

afterEach(() => {
  tagEditorStore.reset();
});

describe('createTagEditorController - loadData', () => {
  it('populates merged & localOverlay in parallel', async () => {
    const merged = makeRegistry({ schemaVersion: 7 });
    const overlay = makeRegistry({ schemaVersion: 3 });
    const deps = makeDeps({
      getTagEditorMerged: vi.fn().mockResolvedValue(merged),
      getTagEditorLocalOverlay: vi.fn().mockResolvedValue(overlay),
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.loadData();

    expect(ctrl.merged).toBe(merged);
    expect(ctrl.localOverlay).toBe(overlay);
    expect(ctrl.loading).toBe(false);
  });

  it('sets loading true during in-flight and false on completion', async () => {
    let resolve!: (v: TagEditorRegistry) => void;
    const deps = makeDeps({
      getTagEditorMerged: vi
        .fn<() => Promise<TagEditorRegistry>>()
        .mockImplementationOnce(() => new Promise((r) => (resolve = r))),
    });
    const ctrl = createTagEditorController(deps);
    const p = ctrl.loadData();
    expect(ctrl.loading).toBe(true);
    resolve(makeRegistry());
    await p;
    expect(ctrl.loading).toBe(false);
  });

  it('only applies the last result when two loadData races', async () => {
    let resolveA!: (v: TagEditorRegistry) => void;
    let resolveB!: (v: TagEditorRegistry) => void;
    const getTagEditorMerged = vi
      .fn<() => Promise<TagEditorRegistry>>()
      .mockImplementationOnce(() => new Promise((r) => (resolveA = r)))
      .mockImplementationOnce(() => new Promise((r) => (resolveB = r)));
    const deps = makeDeps({ getTagEditorMerged });
    const ctrl = createTagEditorController(deps);

    const p1 = ctrl.loadData();
    const p2 = ctrl.loadData();
    resolveB(makeRegistry({ schemaVersion: 2 }));
    await p2;
    resolveA(makeRegistry({ schemaVersion: 1 }));
    await p1;

    expect(ctrl.merged?.schemaVersion).toBe(2);
    expect(ctrl.loading).toBe(false);
  });

  it('notifies error when merged fetch rejects', async () => {
    const notifyError = vi.fn();
    const deps = makeDeps({
      getTagEditorMerged: vi.fn().mockRejectedValue(new Error('boom')),
      notifyError,
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.loadData();
    expect(notifyError).toHaveBeenCalledTimes(1);
    expect(notifyError.mock.calls[0]?.[0]).toBe(
      m.tag_editor_error_load({
        error: m.domain_generic_error(),
      })
    );
    expect(ctrl.loading).toBe(false);
  });
});

describe('createTagEditorController - selection primitives', () => {
  it('selectEntity sets entity type & cid', () => {
    const ctrl = createTagEditorController(makeDeps());
    ctrl.selectEntity('song', 'sng-1');
    expect(ctrl.selectedEntityType).toBe('song');
    expect(ctrl.selectedCid).toBe('sng-1');
  });

  it('selectSongForEdit switches to song scope while keeping editingAlbum', async () => {
    const deps = makeDeps({
      getAlbumDetail: vi
        .fn()
        .mockResolvedValue(makeAlbumDetail('a', [makeSong('s1')])),
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.selectAlbumForEdit(makeAlbum('a'));

    ctrl.selectSongForEdit(makeSong('s1'));
    expect(ctrl.editingSong?.cid).toBe('s1');
    expect(ctrl.selectedEntityType).toBe('song');
    expect(ctrl.selectedCid).toBe('s1');
    expect(ctrl.editingAlbum?.cid).toBe('a');
  });

  it('backToAlbum clears editingSong and points selection to editingAlbum', async () => {
    const deps = makeDeps({
      getAlbumDetail: vi.fn().mockResolvedValue(makeAlbumDetail('a')),
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.selectAlbumForEdit(makeAlbum('a'));
    ctrl.selectSongForEdit(makeSong('s1'));

    ctrl.backToAlbum();
    expect(ctrl.editingSong).toBeNull();
    expect(ctrl.selectedEntityType).toBe('album');
    expect(ctrl.selectedCid).toBe('a');
  });

  it('backToAlbum is a no-op when there is no editingAlbum', () => {
    const ctrl = createTagEditorController(makeDeps());
    ctrl.selectEntity('song', 's1');
    ctrl.backToAlbum();
    expect(ctrl.editingSong).toBeNull();
    // 无 editingAlbum，selection 不应被覆盖回 album
    expect(ctrl.selectedEntityType).toBe('song');
    expect(ctrl.selectedCid).toBe('s1');
  });

  it('setAlbumSearchQuery updates the store value', () => {
    const ctrl = createTagEditorController(makeDeps());
    ctrl.setAlbumSearchQuery('hello');
    expect(ctrl.albumSearchQuery).toBe('hello');
  });
});

describe('createTagEditorController - setTag / removeTag', () => {
  it('setTag is a no-op without a selectedCid', async () => {
    const setTagEditorEntityTag = vi.fn();
    const deps = makeDeps({ setTagEditorEntityTag });
    const ctrl = createTagEditorController(deps);
    await ctrl.setTag('genre', [{ 'zh-CN': '摇滚', 'en-US': 'Rock' }]);
    expect(setTagEditorEntityTag).not.toHaveBeenCalled();
  });

  it('setTag calls backend then reloads', async () => {
    const setTagEditorEntityTag = vi.fn().mockResolvedValue(undefined);
    const getTagEditorMerged = vi.fn().mockResolvedValue(makeRegistry());
    const deps = makeDeps({ setTagEditorEntityTag, getTagEditorMerged });
    const ctrl = createTagEditorController(deps);
    ctrl.selectEntity('album', 'a');

    await ctrl.setTag('genre', [{ 'zh-CN': '摇滚', 'en-US': 'Rock' }]);

    expect(setTagEditorEntityTag).toHaveBeenCalledWith('album', 'a', 'genre', [
      { 'zh-CN': '摇滚', 'en-US': 'Rock' },
    ]);
    expect(getTagEditorMerged).toHaveBeenCalled();
  });

  it('setTag notifies error on failure and skips reload', async () => {
    const notifyError = vi.fn();
    const getTagEditorMerged = vi.fn().mockResolvedValue(makeRegistry());
    const deps = makeDeps({
      setTagEditorEntityTag: vi.fn().mockRejectedValue(new Error('write fail')),
      getTagEditorMerged,
      notifyError,
    });
    const ctrl = createTagEditorController(deps);
    ctrl.selectEntity('album', 'a');
    await ctrl.setTag('genre', []);
    expect(notifyError).toHaveBeenCalledTimes(1);
    expect(getTagEditorMerged).not.toHaveBeenCalled();
  });

  it('removeTag is a no-op without a selectedCid', async () => {
    const removeTagEditorEntityTag = vi.fn();
    const deps = makeDeps({ removeTagEditorEntityTag });
    const ctrl = createTagEditorController(deps);
    await ctrl.removeTag('genre');
    expect(removeTagEditorEntityTag).not.toHaveBeenCalled();
  });

  it('removeTag calls backend then reloads', async () => {
    const removeTagEditorEntityTag = vi.fn().mockResolvedValue(undefined);
    const getTagEditorMerged = vi.fn().mockResolvedValue(makeRegistry());
    const deps = makeDeps({ removeTagEditorEntityTag, getTagEditorMerged });
    const ctrl = createTagEditorController(deps);
    ctrl.selectEntity('song', 's1');
    await ctrl.removeTag('genre');
    expect(removeTagEditorEntityTag).toHaveBeenCalledWith(
      'song',
      's1',
      'genre'
    );
    expect(getTagEditorMerged).toHaveBeenCalled();
  });
});

describe('createTagEditorController - dimensions', () => {
  it('addDimension calls backend then reloads', async () => {
    const addTagEditorDimension = vi.fn().mockResolvedValue(undefined);
    const getTagEditorMerged = vi.fn().mockResolvedValue(makeRegistry());
    const deps = makeDeps({ addTagEditorDimension, getTagEditorMerged });
    const ctrl = createTagEditorController(deps);
    await ctrl.addDimension('genre', '流派', 'Genre');
    expect(addTagEditorDimension).toHaveBeenCalledWith(
      'genre',
      '流派',
      'Genre'
    );
    expect(getTagEditorMerged).toHaveBeenCalled();
  });

  it('addDimension notifies error and skips reload on failure', async () => {
    const notifyError = vi.fn();
    const getTagEditorMerged = vi.fn().mockResolvedValue(makeRegistry());
    const deps = makeDeps({
      addTagEditorDimension: vi.fn().mockRejectedValue(new Error('dup key')),
      getTagEditorMerged,
      notifyError,
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.addDimension('genre', '流派', 'Genre');
    expect(notifyError).toHaveBeenCalledTimes(1);
    expect(getTagEditorMerged).not.toHaveBeenCalled();
  });

  it('removeDimension calls backend then reloads', async () => {
    const removeTagEditorDimension = vi.fn().mockResolvedValue(undefined);
    const getTagEditorMerged = vi.fn().mockResolvedValue(makeRegistry());
    const deps = makeDeps({ removeTagEditorDimension, getTagEditorMerged });
    const ctrl = createTagEditorController(deps);
    await ctrl.removeDimension('genre');
    expect(removeTagEditorDimension).toHaveBeenCalledWith('genre');
    expect(getTagEditorMerged).toHaveBeenCalled();
  });
});

describe('createTagEditorController - resolveConflict', () => {
  it('removes the resolved conflict from store and reloads', async () => {
    const c1 = makeConflict({ cid: 'a', dimensionKey: 'genre' });
    const c2 = makeConflict({ cid: 'b', dimensionKey: 'era' });
    tagEditorStore.conflicts = [c1, c2];

    const resolveTagEditorConflict = vi.fn().mockResolvedValue(undefined);
    const getTagEditorMerged = vi.fn().mockResolvedValue(makeRegistry());
    const deps = makeDeps({ resolveTagEditorConflict, getTagEditorMerged });
    const ctrl = createTagEditorController(deps);

    await ctrl.resolveConflict(c1, 'keepLocal');

    expect(resolveTagEditorConflict).toHaveBeenCalledWith(
      'album',
      'a',
      'genre',
      'keepLocal'
    );
    expect(ctrl.conflicts).toEqual([c2]);
    expect(getTagEditorMerged).toHaveBeenCalled();
  });

  it('keeps the conflict in the store and notifies error on failure', async () => {
    const c1 = makeConflict({ cid: 'a', dimensionKey: 'genre' });
    tagEditorStore.conflicts = [c1];
    const notifyError = vi.fn();
    const deps = makeDeps({
      resolveTagEditorConflict: vi.fn().mockRejectedValue(new Error('boom')),
      notifyError,
    });
    const ctrl = createTagEditorController(deps);

    await ctrl.resolveConflict(c1, 'keepRemote');

    expect(ctrl.conflicts).toEqual([c1]);
    expect(notifyError).toHaveBeenCalledTimes(1);
  });

  it('does not accidentally drop conflicts with same cid but different dimension', async () => {
    const same = makeConflict({ cid: 'a', dimensionKey: 'genre' });
    const other = makeConflict({ cid: 'a', dimensionKey: 'era' });
    tagEditorStore.conflicts = [same, other];

    const deps = makeDeps();
    const ctrl = createTagEditorController(deps);
    await ctrl.resolveConflict(same, 'keepLocal');
    expect(ctrl.conflicts).toEqual([other]);
  });
});

describe('createTagEditorController - selectAlbumForEdit (sync flow)', () => {
  it('loads songs from album detail and updates selection', async () => {
    const songs = [makeSong('s1'), makeSong('s2')];
    const deps = makeDeps({
      getAlbumDetail: vi.fn().mockResolvedValue(makeAlbumDetail('a', songs)),
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.selectAlbumForEdit(makeAlbum('a'));

    expect(ctrl.editingAlbum?.cid).toBe('a');
    expect(ctrl.editingAlbumSongs.map((s) => s.cid)).toEqual(['s1', 's2']);
    expect(ctrl.selectedEntityType).toBe('album');
    expect(ctrl.selectedCid).toBe('a');
    expect(ctrl.editingSong).toBeNull();
    expect(ctrl.loadingSongs).toBe(false);
  });

  it('clears songs and notifies on detail fetch failure', async () => {
    const notifyError = vi.fn();
    const deps = makeDeps({
      getAlbumDetail: vi.fn().mockRejectedValue(new Error('load fail')),
      notifyError,
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.selectAlbumForEdit(makeAlbum('a'));
    expect(ctrl.editingAlbumSongs).toEqual([]);
    expect(ctrl.loadingSongs).toBe(false);
    expect(notifyError).toHaveBeenCalledTimes(1);
  });
});

describe('createTagEditorController - selectAlbumForEditAsync (race-aware)', () => {
  it('returns true and updates state when the load is the latest', async () => {
    const songs = [makeSong('s1')];
    const deps = makeDeps({
      getAlbumDetail: vi.fn().mockResolvedValue(makeAlbumDetail('a', songs)),
    });
    const ctrl = createTagEditorController(deps);
    const ok = await ctrl.selectAlbumForEditAsync(makeAlbum('a'));
    expect(ok).toBe(true);
    expect(ctrl.editingAlbum?.cid).toBe('a');
    expect(ctrl.editingAlbumSongs).toEqual(songs);
  });

  it('returns false and does not mutate state when superseded', async () => {
    let resolveA!: (v: AlbumDetail) => void;
    let resolveB!: (v: AlbumDetail) => void;
    const getAlbumDetail = vi
      .fn<(cid: string) => Promise<AlbumDetail>>()
      .mockImplementationOnce(() => new Promise((r) => (resolveA = r)))
      .mockImplementationOnce(() => new Promise((r) => (resolveB = r)));
    const deps = makeDeps({ getAlbumDetail });
    const ctrl = createTagEditorController(deps);

    const p1 = ctrl.selectAlbumForEditAsync(makeAlbum('a'));
    const p2 = ctrl.selectAlbumForEditAsync(makeAlbum('b'));
    resolveB(makeAlbumDetail('b', [makeSong('sb')]));
    const okB = await p2;
    resolveA(makeAlbumDetail('a', [makeSong('sa')]));
    const okA = await p1;

    expect(okB).toBe(true);
    expect(okA).toBe(false);
    expect(ctrl.editingAlbum?.cid).toBe('b');
  });

  it('returns false when shouldDispose reports true post-fetch', async () => {
    const deps = makeDeps({
      getAlbumDetail: vi.fn().mockResolvedValue(makeAlbumDetail('a')),
    });
    const ctrl = createTagEditorController(deps);
    let disposed = false;
    const p = ctrl.selectAlbumForEditAsync(makeAlbum('a'), () => disposed);
    disposed = true;
    const ok = await p;
    expect(ok).toBe(false);
    expect(ctrl.editingAlbum).toBeNull();
  });

  it('returns false and notifies error on rejection', async () => {
    const notifyError = vi.fn();
    const deps = makeDeps({
      getAlbumDetail: vi.fn().mockRejectedValue(new Error('load fail')),
      notifyError,
    });
    const ctrl = createTagEditorController(deps);
    const ok = await ctrl.selectAlbumForEditAsync(makeAlbum('a'));
    expect(ok).toBe(false);
    expect(notifyError).toHaveBeenCalledTimes(1);
  });
});

describe('createTagEditorController - restoreEditingState', () => {
  it('resets and returns true when albumCid is null', async () => {
    tagEditorStore.editingAlbum = makeAlbum('old');
    tagEditorStore.editingAlbumSongs = [makeSong('x')];

    const ctrl = createTagEditorController(makeDeps());
    const ok = await ctrl.restoreEditingState(null, null);
    expect(ok).toBe(true);
    expect(ctrl.editingAlbum).toBeNull();
    expect(ctrl.editingAlbumSongs).toEqual([]);
  });

  it('returns false when the requested album is not in the library list', async () => {
    const deps = makeDeps({
      getAlbums: vi.fn<() => Album[]>().mockReturnValue([makeAlbum('other')]),
    });
    const ctrl = createTagEditorController(deps);
    const ok = await ctrl.restoreEditingState('missing', null);
    expect(ok).toBe(false);
    expect(ctrl.editingAlbum).toBeNull();
  });

  it('restores album selection when songCid is null', async () => {
    const songs = [makeSong('s1')];
    const deps = makeDeps({
      getAlbums: vi.fn<() => Album[]>().mockReturnValue([makeAlbum('a')]),
      getAlbumDetail: vi.fn().mockResolvedValue(makeAlbumDetail('a', songs)),
    });
    const ctrl = createTagEditorController(deps);
    const ok = await ctrl.restoreEditingState('a', null);
    expect(ok).toBe(true);
    expect(ctrl.editingAlbum?.cid).toBe('a');
    expect(ctrl.editingAlbumSongs).toEqual(songs);
    expect(ctrl.selectedEntityType).toBe('album');
    expect(ctrl.selectedCid).toBe('a');
    expect(ctrl.editingSong).toBeNull();
  });

  it('restores song selection when songCid matches a song in the album', async () => {
    const songs = [makeSong('s1'), makeSong('s2')];
    const deps = makeDeps({
      getAlbums: vi.fn<() => Album[]>().mockReturnValue([makeAlbum('a')]),
      getAlbumDetail: vi.fn().mockResolvedValue(makeAlbumDetail('a', songs)),
    });
    const ctrl = createTagEditorController(deps);
    const ok = await ctrl.restoreEditingState('a', 's2');
    expect(ok).toBe(true);
    expect(ctrl.editingSong?.cid).toBe('s2');
    expect(ctrl.selectedEntityType).toBe('song');
    expect(ctrl.selectedCid).toBe('s2');
  });

  it('falls back to album selection when songCid is not in the album', async () => {
    const songs = [makeSong('s1')];
    const deps = makeDeps({
      getAlbums: vi.fn<() => Album[]>().mockReturnValue([makeAlbum('a')]),
      getAlbumDetail: vi.fn().mockResolvedValue(makeAlbumDetail('a', songs)),
    });
    const ctrl = createTagEditorController(deps);
    const ok = await ctrl.restoreEditingState('a', 'missing');
    expect(ok).toBe(true);
    expect(ctrl.editingSong).toBeNull();
    expect(ctrl.selectedEntityType).toBe('album');
    expect(ctrl.selectedCid).toBe('a');
  });

  it('resets store and notifies error on detail fetch failure', async () => {
    const notifyError = vi.fn();
    const deps = makeDeps({
      getAlbums: vi.fn<() => Album[]>().mockReturnValue([makeAlbum('a')]),
      getAlbumDetail: vi.fn().mockRejectedValue(new Error('boom')),
      notifyError,
    });
    const ctrl = createTagEditorController(deps);
    const ok = await ctrl.restoreEditingState('a', null);
    expect(ok).toBe(false);
    expect(notifyError).toHaveBeenCalledTimes(1);
    expect(ctrl.editingAlbum).toBeNull();
  });

  it('returns false without mutation when shouldDispose fires', async () => {
    const deps = makeDeps({
      getAlbums: vi.fn<() => Album[]>().mockReturnValue([makeAlbum('a')]),
      getAlbumDetail: vi.fn().mockResolvedValue(makeAlbumDetail('a')),
    });
    const ctrl = createTagEditorController(deps);
    let disposed = false;
    const p = ctrl.restoreEditingState('a', null, () => disposed);
    disposed = true;
    const ok = await p;
    expect(ok).toBe(false);
    expect(ctrl.editingAlbum).toBeNull();
  });
});

describe('createTagEditorController - export / import', () => {
  it('exportRegistry aborts when user cancels dialog', async () => {
    const exportTagEditorRegistry = vi.fn();
    const deps = makeDeps({
      pickSavePath: vi.fn().mockResolvedValue(null),
      exportTagEditorRegistry,
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.exportRegistry();
    expect(exportTagEditorRegistry).not.toHaveBeenCalled();
  });

  it('exportRegistry calls backend with the chosen path', async () => {
    const exportTagEditorRegistry = vi.fn().mockResolvedValue(undefined);
    const deps = makeDeps({
      pickSavePath: vi.fn().mockResolvedValue('/path/out.json'),
      exportTagEditorRegistry,
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.exportRegistry();
    expect(exportTagEditorRegistry).toHaveBeenCalledWith('/path/out.json');
  });

  it('exportRegistry notifies error on backend failure', async () => {
    const notifyError = vi.fn();
    const deps = makeDeps({
      exportTagEditorRegistry: vi.fn().mockRejectedValue(new Error('io fail')),
      notifyError,
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.exportRegistry();
    expect(notifyError).toHaveBeenCalledTimes(1);
  });

  it('importRegistry aborts when user cancels dialog', async () => {
    const importTagEditorRegistry = vi.fn();
    const deps = makeDeps({
      pickOpenPath: vi.fn().mockResolvedValue(null),
      importTagEditorRegistry,
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.importRegistry();
    expect(importTagEditorRegistry).not.toHaveBeenCalled();
  });

  it('importRegistry stores conflicts and reloads after import', async () => {
    const conflict = makeConflict({ cid: 'a', dimensionKey: 'genre' });
    const importTagEditorRegistry = vi
      .fn()
      .mockResolvedValue(makeMergeResult([conflict], 1));
    const getTagEditorMerged = vi.fn().mockResolvedValue(makeRegistry());
    const deps = makeDeps({ importTagEditorRegistry, getTagEditorMerged });
    const ctrl = createTagEditorController(deps);

    await ctrl.importRegistry();
    expect(ctrl.conflicts).toEqual([conflict]);
    expect(getTagEditorMerged).toHaveBeenCalled();
  });

  it('importRegistry with no conflicts leaves existing store conflicts alone', async () => {
    // 现有的 conflicts 不被本次导入清除；仅当 result.conflicts.length > 0 才覆盖。
    const preexisting = makeConflict({ cid: 'x', dimensionKey: 'y' });
    tagEditorStore.conflicts = [preexisting];

    const deps = makeDeps({
      importTagEditorRegistry: vi.fn().mockResolvedValue(makeMergeResult([])),
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.importRegistry();
    expect(ctrl.conflicts).toEqual([preexisting]);
  });

  it('importRegistry notifies error on backend failure', async () => {
    const notifyError = vi.fn();
    const deps = makeDeps({
      importTagEditorRegistry: vi.fn().mockRejectedValue(new Error('bad json')),
      notifyError,
    });
    const ctrl = createTagEditorController(deps);
    await ctrl.importRegistry();
    expect(notifyError).toHaveBeenCalledTimes(1);
  });
});

describe('createTagEditorController - filteredAlbums derived', () => {
  it('returns all albums when the query is empty or whitespace', () => {
    const albums = [makeAlbum('a'), makeAlbum('b')];
    const deps = makeDeps({
      getAlbums: vi.fn<() => Album[]>().mockReturnValue(albums),
    });
    const ctrl = createTagEditorController(deps);
    expect(ctrl.filteredAlbums.map((a) => a.cid)).toEqual(['a', 'b']);
    ctrl.setAlbumSearchQuery('   ');
    expect(ctrl.filteredAlbums.map((a) => a.cid)).toEqual(['a', 'b']);
  });

  it('filters by album name case-insensitively', () => {
    const albums = [
      makeAlbum('a', { name: 'Hello World' }),
      makeAlbum('b', { name: 'Foo Bar' }),
    ];
    const deps = makeDeps({
      getAlbums: vi.fn<() => Album[]>().mockReturnValue(albums),
    });
    const ctrl = createTagEditorController(deps);
    ctrl.setAlbumSearchQuery('WORLD');
    expect(ctrl.filteredAlbums.map((a) => a.cid)).toEqual(['a']);
  });

  it('filters by artist name case-insensitively', () => {
    const albums = [
      makeAlbum('a', { artists: ['Yorushika'] }),
      makeAlbum('b', { artists: ['Aimer'] }),
    ];
    const deps = makeDeps({
      getAlbums: vi.fn<() => Album[]>().mockReturnValue(albums),
    });
    const ctrl = createTagEditorController(deps);
    ctrl.setAlbumSearchQuery('AIM');
    expect(ctrl.filteredAlbums.map((a) => a.cid)).toEqual(['b']);
  });
});

describe('createTagEditorController - dispose', () => {
  it('resets store and supersedes an in-flight loadData', async () => {
    let resolve!: (v: TagEditorRegistry) => void;
    const deps = makeDeps({
      getTagEditorMerged: vi
        .fn<() => Promise<TagEditorRegistry>>()
        .mockImplementationOnce(() => new Promise((r) => (resolve = r))),
    });
    const ctrl = createTagEditorController(deps);
    ctrl.selectEntity('song', 's1');
    const p = ctrl.loadData();
    ctrl.dispose();
    resolve(makeRegistry({ schemaVersion: 9 }));
    await p;

    expect(ctrl.merged).toBeNull();
    expect(ctrl.selectedCid).toBeNull();
    expect(ctrl.selectedEntityType).toBe('album');
  });
});
