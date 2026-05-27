import type { Album, HistoryEntry, LibrarySearchScope } from '$lib/types';
import { searchStore, type RecentQuery } from './store.svelte';
import * as m from '$lib/paraglide/messages.js';

const STORAGE_KEY = 'harubble:recent-searches';
const MAX_RECENT_QUERIES = 4;
const MAX_RECENT_PLAYED = 20;
const HISTORY_FETCH_LIMIT = 500;

interface SearchControllerDeps {
  getRecentHistory: (limit: number) => Promise<HistoryEntry[]>;
  getAlbums: () => Album[];
  notifyError: (message: string) => void;
}

export function createSearchController(deps: SearchControllerDeps) {
  let initialized = false;
  let loadRequestSeq = 0;

  function init() {
    if (initialized) return;
    initialized = true;
    loadRecentQueries();
    void loadRecentPlayed();
  }

  function loadRecentQueries() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw) as unknown;
      if (Array.isArray(parsed)) {
        searchStore.recentQueries = parsed as RecentQuery[];
      }
    } catch {
      // localStorage 损坏时静默忽略
    }
  }

  function saveRecentQuery(q: string, s: LibrarySearchScope) {
    const existing = searchStore.recentQueries.filter(
      (entry) => !(entry.query === q && entry.scope === s)
    );
    const updated: RecentQuery[] = [
      { query: q, scope: s, timestamp: Date.now() },
      ...existing,
    ].slice(0, MAX_RECENT_QUERIES);
    searchStore.recentQueries = updated;
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
    } catch {
      // 存储配额超限时静默忽略
    }
  }

  async function loadRecentPlayed() {
    const seq = ++loadRequestSeq;
    searchStore.loading = true;
    try {
      const history = await deps.getRecentHistory(HISTORY_FETCH_LIMIT);
      if (seq !== loadRequestSeq) return;
      const albums = deps.getAlbums();
      // 仅用于本次去重计算的临时容器，不参与响应式追踪，故用原生 Map/Set。
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      const albumMap = new Map(albums.map((a) => [a.cid, a]));
      // eslint-disable-next-line svelte/prefer-svelte-reactivity
      const seen = new Set<string>();
      const result: Album[] = [];
      for (const entry of history) {
        if (seen.has(entry.albumCid)) continue;
        seen.add(entry.albumCid);
        const album = albumMap.get(entry.albumCid);
        if (album) result.push(album);
        if (result.length >= MAX_RECENT_PLAYED) break;
      }
      searchStore.recentPlayed = result;
    } catch (e: unknown) {
      if (seq !== loadRequestSeq) return;
      deps.notifyError(
        m.search_error_load_recently_played({
          error: e instanceof Error ? e.message : String(e),
        })
      );
    } finally {
      if (seq === loadRequestSeq) {
        searchStore.loading = false;
      }
    }
  }

  function setQuery(q: string) {
    searchStore.query = q;
  }

  function setScope(s: LibrarySearchScope) {
    searchStore.scope = s;
  }

  function submitSearch() {
    const q = searchStore.query.trim();
    if (!q) return;
    saveRecentQuery(q, searchStore.scope);
  }

  function rerunQuery(entry: RecentQuery) {
    searchStore.query = entry.query;
    searchStore.scope = entry.scope;
    saveRecentQuery(entry.query, entry.scope);
  }

  function dispose() {
    loadRequestSeq += 1;
    initialized = false;
    searchStore.reset();
  }

  return {
    get query() {
      return searchStore.query;
    },
    get scope() {
      return searchStore.scope;
    },
    get recentPlayed() {
      return searchStore.recentPlayed;
    },
    get recentQueries() {
      return searchStore.recentQueries;
    },
    get loading() {
      return searchStore.loading;
    },
    init,
    dispose,
    loadRecentPlayed,
    setQuery,
    setScope,
    submitSearch,
    rerunQuery,
  };
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    searchStore.reset();
  });
}
