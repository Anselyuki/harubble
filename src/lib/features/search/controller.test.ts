// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type {
  Album,
  HistoryEntry,
  SearchLibraryRequest,
  SearchLibraryResponse,
} from '$lib/types';
import { createSearchController } from './controller.svelte';
import { searchStore } from './store.svelte';
import * as m from '$lib/paraglide/messages.js';

const STORAGE_KEY = 'harubble:recent-searches';

function makeAlbum(cid: string): Album {
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
  };
}

function makeHistoryEntry(
  overrides: Partial<HistoryEntry> & { albumCid: string }
): HistoryEntry {
  return {
    songCid: `song-${overrides.albumCid}`,
    songName: 'Song',
    albumName: `Album ${overrides.albumCid}`,
    coverUrl: null,
    artists: [],
    heat: 0,
    playedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

function makeSearchItem(index: number) {
  return {
    kind: 'song' as const,
    albumCid: `album-${index}`,
    songCid: `song-${index}`,
    albumTitle: `Album ${index}`,
    songTitle: `Song ${index}`,
    artistLine: 'Artist',
    matchedFields: ['title' as const],
  };
}

function makeDeps(
  overrides: Partial<Parameters<typeof createSearchController>[0]> = {}
) {
  return {
    getRecentHistory: vi
      .fn<(limit: number) => Promise<HistoryEntry[]>>()
      .mockResolvedValue([]),
    getAlbums: vi.fn<() => Album[]>().mockReturnValue([]),
    searchLibrary: vi
      .fn<(request: SearchLibraryRequest) => Promise<SearchLibraryResponse>>()
      .mockImplementation(async (request) => ({
        items: [],
        total: 0,
        query: request.query,
        scope: request.scope,
        indexState: 'ready',
      })),
    notifyError: vi.fn<(message: string) => void>(),
    ...overrides,
  };
}

beforeEach(() => {
  vi.useFakeTimers();
  localStorage.clear();
});

afterEach(() => {
  vi.clearAllTimers();
  vi.useRealTimers();
  // 模块级 $state 会在测试间残留，显式 reset。
  searchStore.reset();
});

describe('createSearchController - init & recent queries hydration', () => {
  it('hydrates recentQueries from localStorage on init', () => {
    const stored = [
      { query: 'foo', scope: 'all', timestamp: 1 },
      { query: 'bar', scope: 'songs', timestamp: 2 },
    ];
    localStorage.setItem(STORAGE_KEY, JSON.stringify(stored));

    const ctrl = createSearchController(makeDeps());
    ctrl.init();

    expect(ctrl.recentQueries).toEqual(stored);
  });

  it('ignores malformed JSON in localStorage without throwing', () => {
    localStorage.setItem(STORAGE_KEY, '{not json');
    const ctrl = createSearchController(makeDeps());
    expect(() => ctrl.init()).not.toThrow();
    expect(ctrl.recentQueries).toEqual([]);
  });

  it('ignores non-array stored payloads', () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ query: 'foo' }));
    const ctrl = createSearchController(makeDeps());
    ctrl.init();
    expect(ctrl.recentQueries).toEqual([]);
  });

  it('is idempotent: init() only triggers loadRecentPlayed once', () => {
    const getRecentHistory = vi
      .fn<(limit: number) => Promise<HistoryEntry[]>>()
      .mockResolvedValue([]);
    const ctrl = createSearchController(makeDeps({ getRecentHistory }));
    ctrl.init();
    ctrl.init();
    expect(getRecentHistory).toHaveBeenCalledTimes(1);
  });
});

describe('createSearchController - loadRecentPlayed', () => {
  it('dedups history by albumCid and preserves history order', async () => {
    const history = [
      makeHistoryEntry({ albumCid: 'a' }),
      makeHistoryEntry({ albumCid: 'b' }),
      makeHistoryEntry({ albumCid: 'a' }), // 重复 albumCid，应被跳过
      makeHistoryEntry({ albumCid: 'c' }),
    ];
    const albums = [makeAlbum('a'), makeAlbum('b'), makeAlbum('c')];
    const deps = makeDeps({
      getRecentHistory: vi.fn().mockResolvedValue(history),
      getAlbums: vi.fn().mockReturnValue(albums),
    });
    const ctrl = createSearchController(deps);
    await ctrl.loadRecentPlayed();

    expect(ctrl.recentPlayed.map((a) => a.cid)).toEqual(['a', 'b', 'c']);
  });

  it('drops history entries whose album is not in the library', async () => {
    const history = [
      makeHistoryEntry({ albumCid: 'known' }),
      makeHistoryEntry({ albumCid: 'unknown' }),
    ];
    const deps = makeDeps({
      getRecentHistory: vi.fn().mockResolvedValue(history),
      getAlbums: vi.fn().mockReturnValue([makeAlbum('known')]),
    });
    const ctrl = createSearchController(deps);
    await ctrl.loadRecentPlayed();

    expect(ctrl.recentPlayed.map((a) => a.cid)).toEqual(['known']);
  });

  it('caps recentPlayed at 20 entries', async () => {
    const history = Array.from({ length: 30 }, (_, i) =>
      makeHistoryEntry({ albumCid: `a${i}` })
    );
    const albums = history.map((h) => makeAlbum(h.albumCid));
    const deps = makeDeps({
      getRecentHistory: vi.fn().mockResolvedValue(history),
      getAlbums: vi.fn().mockReturnValue(albums),
    });
    const ctrl = createSearchController(deps);
    await ctrl.loadRecentPlayed();

    expect(ctrl.recentPlayed).toHaveLength(20);
    expect(ctrl.recentPlayed[0]?.cid).toBe('a0');
    expect(ctrl.recentPlayed[19]?.cid).toBe('a19');
  });

  it('only applies the last result when two loadRecentPlayed races', async () => {
    let resolveA!: (v: HistoryEntry[]) => void;
    let resolveB!: (v: HistoryEntry[]) => void;
    const getRecentHistory = vi
      .fn<(limit: number) => Promise<HistoryEntry[]>>()
      .mockImplementationOnce(() => new Promise((r) => (resolveA = r)))
      .mockImplementationOnce(() => new Promise((r) => (resolveB = r)));

    const deps = makeDeps({
      getRecentHistory,
      getAlbums: vi
        .fn<() => Album[]>()
        .mockReturnValue([makeAlbum('a'), makeAlbum('b')]),
    });
    const ctrl = createSearchController(deps);

    const p1 = ctrl.loadRecentPlayed();
    const p2 = ctrl.loadRecentPlayed();

    resolveB([makeHistoryEntry({ albumCid: 'b' })]);
    await p2;
    resolveA([makeHistoryEntry({ albumCid: 'a' })]);
    await p1;

    expect(ctrl.recentPlayed.map((a) => a.cid)).toEqual(['b']);
    expect(ctrl.recentPlayedLoading).toBe(false);
  });

  it('sets loading true while in-flight and false on completion', async () => {
    let resolve!: (v: HistoryEntry[]) => void;
    const deps = makeDeps({
      getRecentHistory: vi
        .fn<(limit: number) => Promise<HistoryEntry[]>>()
        .mockImplementationOnce(() => new Promise((r) => (resolve = r))),
    });
    const ctrl = createSearchController(deps);

    const p = ctrl.loadRecentPlayed();
    expect(ctrl.recentPlayedLoading).toBe(true);
    resolve([]);
    await p;
    expect(ctrl.recentPlayedLoading).toBe(false);
  });

  it('notifies error and clears loading on rejection', async () => {
    const notifyError = vi.fn();
    const deps = makeDeps({
      getRecentHistory: vi
        .fn<(limit: number) => Promise<HistoryEntry[]>>()
        .mockRejectedValue(new Error('boom')),
      notifyError,
    });
    const ctrl = createSearchController(deps);

    await ctrl.loadRecentPlayed();

    expect(notifyError).toHaveBeenCalledTimes(1);
    expect(notifyError.mock.calls[0]?.[0]).toBe(
      m.search_error_load_recently_played({
        error: m.domain_generic_error(),
      })
    );
    expect(ctrl.recentPlayedLoading).toBe(false);
  });

  it('does not notify error for a superseded rejected load', async () => {
    const notifyError = vi.fn();
    let rejectA!: (e: Error) => void;
    const getRecentHistory = vi
      .fn<(limit: number) => Promise<HistoryEntry[]>>()
      .mockImplementationOnce(() => new Promise((_r, rej) => (rejectA = rej)))
      .mockResolvedValueOnce([]);

    const deps = makeDeps({ getRecentHistory, notifyError });
    const ctrl = createSearchController(deps);

    const p1 = ctrl.loadRecentPlayed();
    const p2 = ctrl.loadRecentPlayed();
    rejectA(new Error('stale'));
    await p1;
    await p2;

    expect(notifyError).not.toHaveBeenCalled();
  });

  it('requests history with fetch limit 500', async () => {
    const getRecentHistory = vi
      .fn<(limit: number) => Promise<HistoryEntry[]>>()
      .mockResolvedValue([]);
    const ctrl = createSearchController(makeDeps({ getRecentHistory }));
    await ctrl.loadRecentPlayed();
    expect(getRecentHistory).toHaveBeenCalledWith(500);
  });
});

describe('createSearchController - submitSearch & saveRecentQuery', () => {
  it('does not save when the query is empty or whitespace only', () => {
    const ctrl = createSearchController(makeDeps());
    ctrl.setQuery('   ');
    void ctrl.submitSearch();
    expect(ctrl.recentQueries).toEqual([]);
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it('inserts the trimmed query at the head of recentQueries', () => {
    const ctrl = createSearchController(makeDeps());
    ctrl.setScope('songs');
    ctrl.setQuery('  hello  ');
    void ctrl.submitSearch();

    expect(ctrl.recentQueries[0]?.query).toBe('hello');
    expect(ctrl.recentQueries[0]?.scope).toBe('songs');
    const persisted = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]');
    expect(persisted[0].query).toBe('hello');
  });

  it('dedups an existing query+scope pair, moving it to the head', () => {
    const ctrl = createSearchController(makeDeps());
    ctrl.setQuery('foo');
    void ctrl.submitSearch();
    ctrl.setQuery('bar');
    void ctrl.submitSearch();
    ctrl.setQuery('foo'); // 与首条相同 query+scope，应从尾部去重并回到 head
    void ctrl.submitSearch();

    expect(ctrl.recentQueries.map((e) => e.query)).toEqual(['foo', 'bar']);
  });

  it('treats same query with different scope as distinct entries', () => {
    const ctrl = createSearchController(makeDeps());
    ctrl.setQuery('foo');
    ctrl.setScope('all');
    void ctrl.submitSearch();
    ctrl.setScope('albums');
    void ctrl.submitSearch();

    expect(ctrl.recentQueries).toHaveLength(2);
    expect(ctrl.recentQueries.map((e) => e.scope)).toEqual(['albums', 'all']);
  });

  it('caps recentQueries at 4 entries', () => {
    const ctrl = createSearchController(makeDeps());
    for (const q of ['q1', 'q2', 'q3', 'q4', 'q5']) {
      ctrl.setQuery(q);
      void ctrl.submitSearch();
    }
    expect(ctrl.recentQueries).toHaveLength(4);
    expect(ctrl.recentQueries.map((e) => e.query)).toEqual([
      'q5',
      'q4',
      'q3',
      'q2',
    ]);
  });

  it('does not throw when localStorage.setItem throws (quota exceeded)', () => {
    const spy = vi
      .spyOn(Storage.prototype, 'setItem')
      .mockImplementation(() => {
        throw new Error('quota exceeded');
      });
    const ctrl = createSearchController(makeDeps());
    ctrl.setQuery('x');
    expect(() => ctrl.submitSearch()).not.toThrow();
    // 内存态仍应更新，尽管持久化失败
    expect(ctrl.recentQueries[0]?.query).toBe('x');
    spy.mockRestore();
  });
});

describe('createSearchController - rerunQuery', () => {
  it('applies query and scope from the entry and re-persists it as recent', () => {
    const ctrl = createSearchController(makeDeps());
    ctrl.setQuery('bar');
    ctrl.setScope('all');
    void ctrl.submitSearch();

    void ctrl.rerunQuery({ query: 'foo', scope: 'songs', timestamp: 1 });

    expect(ctrl.query).toBe('foo');
    expect(ctrl.scope).toBe('songs');
    expect(ctrl.recentQueries[0]?.query).toBe('foo');
    expect(ctrl.recentQueries[0]?.scope).toBe('songs');
  });
});

describe('createSearchController - backend search', () => {
  it('debounces query changes and sends the first page request', async () => {
    const searchLibrary = vi
      .fn<(request: SearchLibraryRequest) => Promise<SearchLibraryResponse>>()
      .mockImplementation(async (request) => ({
        items: [makeSearchItem(1)],
        total: 1,
        query: request.query,
        scope: request.scope,
        indexState: 'ready',
      }));
    const ctrl = createSearchController(makeDeps({ searchLibrary }));

    ctrl.setQuery('  alpha  ');

    expect(ctrl.searchLoading).toBe(true);
    expect(searchLibrary).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(220);

    expect(searchLibrary).toHaveBeenCalledWith({
      query: 'alpha',
      scope: 'all',
      limit: 20,
      offset: 0,
    });
    expect(ctrl.response?.items).toHaveLength(1);
    expect(ctrl.searchLoading).toBe(false);
  });

  it('only applies the latest search result', async () => {
    let resolveAlpha!: (response: SearchLibraryResponse) => void;
    let resolveBeta!: (response: SearchLibraryResponse) => void;
    const searchLibrary = vi
      .fn<(request: SearchLibraryRequest) => Promise<SearchLibraryResponse>>()
      .mockImplementationOnce(
        () => new Promise((resolve) => (resolveAlpha = resolve))
      )
      .mockImplementationOnce(
        () => new Promise((resolve) => (resolveBeta = resolve))
      );
    const ctrl = createSearchController(makeDeps({ searchLibrary }));

    ctrl.setQuery('alpha');
    const alphaRequest = ctrl.submitSearch()!;
    ctrl.setQuery('beta');
    const betaRequest = ctrl.submitSearch()!;

    resolveBeta({
      items: [makeSearchItem(2)],
      total: 1,
      query: 'beta',
      scope: 'all',
      indexState: 'ready',
    });
    await betaRequest;
    resolveAlpha({
      items: [makeSearchItem(1)],
      total: 1,
      query: 'alpha',
      scope: 'all',
      indexState: 'ready',
    });
    await alphaRequest;

    expect(ctrl.response?.query).toBe('beta');
    expect(ctrl.response?.items[0]?.songCid).toBe('song-2');
  });

  it('invalidates an in-flight result as soon as a new query is scheduled', async () => {
    let resolveAlpha!: (response: SearchLibraryResponse) => void;
    const searchLibrary = vi
      .fn<(request: SearchLibraryRequest) => Promise<SearchLibraryResponse>>()
      .mockImplementationOnce(
        () => new Promise((resolve) => (resolveAlpha = resolve))
      )
      .mockImplementationOnce(async (request) => ({
        items: [makeSearchItem(2)],
        total: 1,
        query: request.query,
        scope: request.scope,
        indexState: 'ready',
      }));
    const ctrl = createSearchController(makeDeps({ searchLibrary }));

    ctrl.setQuery('alpha');
    const alphaRequest = ctrl.submitSearch()!;
    ctrl.setQuery('beta');
    resolveAlpha({
      items: [makeSearchItem(1)],
      total: 1,
      query: 'alpha',
      scope: 'all',
      indexState: 'ready',
    });
    await alphaRequest;

    expect(ctrl.response).toBeNull();
    expect(ctrl.searchLoading).toBe(true);

    await vi.advanceTimersByTimeAsync(220);

    expect(ctrl.response?.query).toBe('beta');
    expect(ctrl.response?.items[0]?.songCid).toBe('song-2');
  });

  it('loads and appends the next page', async () => {
    const firstPage = Array.from({ length: 20 }, (_, index) =>
      makeSearchItem(index)
    );
    const secondPage = Array.from({ length: 5 }, (_, index) =>
      makeSearchItem(index + 20)
    );
    const searchLibrary = vi
      .fn<(request: SearchLibraryRequest) => Promise<SearchLibraryResponse>>()
      .mockResolvedValueOnce({
        items: firstPage,
        total: 25,
        query: 'alpha',
        scope: 'all',
        indexState: 'ready',
      })
      .mockResolvedValueOnce({
        items: secondPage,
        total: 25,
        query: 'alpha',
        scope: 'all',
        indexState: 'ready',
      });
    const ctrl = createSearchController(makeDeps({ searchLibrary }));
    ctrl.setQuery('alpha');
    await ctrl.submitSearch();

    await ctrl.loadMore();

    expect(searchLibrary).toHaveBeenLastCalledWith({
      query: 'alpha',
      scope: 'all',
      limit: 20,
      offset: 20,
    });
    expect(ctrl.response?.items).toHaveLength(25);
  });

  it('refreshes the active query when the index becomes ready', async () => {
    const searchLibrary = vi
      .fn<(request: SearchLibraryRequest) => Promise<SearchLibraryResponse>>()
      .mockResolvedValueOnce({
        items: [],
        total: 0,
        query: 'alpha',
        scope: 'all',
        indexState: 'building',
      })
      .mockResolvedValueOnce({
        items: [makeSearchItem(1)],
        total: 1,
        query: 'alpha',
        scope: 'all',
        indexState: 'ready',
      });
    const ctrl = createSearchController(makeDeps({ searchLibrary }));
    ctrl.setQuery('alpha');
    await ctrl.submitSearch();

    ctrl.handleIndexStateChanged('ready');
    await Promise.resolve();
    await Promise.resolve();

    expect(searchLibrary).toHaveBeenCalledTimes(2);
    expect(ctrl.response?.items).toHaveLength(1);
    expect(ctrl.indexState).toBe('ready');
  });

  it('preserves visible results while the index is rebuilding', async () => {
    const ctrl = createSearchController(makeDeps());
    ctrl.setQuery('alpha');
    await ctrl.submitSearch();
    searchStore.response = {
      items: [makeSearchItem(1)],
      total: 1,
      query: 'alpha',
      scope: 'all',
      indexState: 'ready',
    };

    ctrl.handleIndexStateChanged('building');

    expect(ctrl.response?.items).toHaveLength(1);
    expect(ctrl.response?.indexState).toBe('building');
  });

  it('stores a localized search error and notifies the user', async () => {
    const notifyError = vi.fn();
    const ctrl = createSearchController(
      makeDeps({
        searchLibrary: vi.fn().mockRejectedValue({ code: 'internal' }),
        notifyError,
      })
    );
    ctrl.setQuery('alpha');

    await ctrl.submitSearch();

    expect(ctrl.searchError).toBe(m.domain_generic_error());
    expect(notifyError).toHaveBeenCalledWith(
      m.search_error_failed({ error: m.domain_generic_error() })
    );
    expect(ctrl.searchLoading).toBe(false);
  });
});

describe('createSearchController - setQuery / setScope / dispose', () => {
  it('setQuery / setScope update store state directly', () => {
    const ctrl = createSearchController(makeDeps());
    ctrl.setQuery('hello');
    ctrl.setScope('albums');
    expect(ctrl.query).toBe('hello');
    expect(ctrl.scope).toBe('albums');
  });

  it('dispose resets store state and re-enables init()', () => {
    const getRecentHistory = vi
      .fn<(limit: number) => Promise<HistoryEntry[]>>()
      .mockResolvedValue([]);
    const ctrl = createSearchController(makeDeps({ getRecentHistory }));
    ctrl.init();
    ctrl.setQuery('foo');
    ctrl.dispose();

    expect(ctrl.query).toBe('');
    expect(ctrl.scope).toBe('all');
    expect(ctrl.recentQueries).toEqual([]);
    expect(ctrl.recentPlayed).toEqual([]);
    expect(ctrl.recentPlayedLoading).toBe(false);

    ctrl.init();
    expect(getRecentHistory).toHaveBeenCalledTimes(2);
  });

  it('dispose supersedes an in-flight loadRecentPlayed', async () => {
    let resolve!: (v: HistoryEntry[]) => void;
    const deps = makeDeps({
      getRecentHistory: vi
        .fn<(limit: number) => Promise<HistoryEntry[]>>()
        .mockImplementationOnce(() => new Promise((r) => (resolve = r))),
      getAlbums: vi.fn<() => Album[]>().mockReturnValue([makeAlbum('a')]),
    });
    const ctrl = createSearchController(deps);
    const p = ctrl.loadRecentPlayed();
    ctrl.dispose();
    resolve([makeHistoryEntry({ albumCid: 'a' })]);
    await p;

    expect(ctrl.recentPlayed).toEqual([]);
  });
});
