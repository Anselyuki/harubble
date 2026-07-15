import type {
  Album,
  HistoryEntry,
  LibraryIndexState,
  LibrarySearchScope,
  SearchLibraryRequest,
  SearchLibraryResponse,
} from '$lib/types';
import { searchStore, type RecentQuery } from './store.svelte';
import {
  formatLibraryError,
  formatSearchError,
} from '$lib/features/shell/domainErrors';
import * as m from '$lib/paraglide/messages.js';

const STORAGE_KEY = 'harubble:recent-searches';
const MAX_RECENT_QUERIES = 4;
const MAX_RECENT_PLAYED = 20;
const HISTORY_FETCH_LIMIT = 500;
const SEARCH_DEBOUNCE_MS = 220;
const SEARCH_PAGE_SIZE = 20;

interface SearchControllerDeps {
  getRecentHistory: (limit: number) => Promise<HistoryEntry[]>;
  getAlbums: () => Album[];
  searchLibrary: (
    request: SearchLibraryRequest
  ) => Promise<SearchLibraryResponse>;
  notifyError: (message: string) => void;
}

export function createSearchController(deps: SearchControllerDeps) {
  let initialized = false;
  let loadRequestSeq = 0;
  let searchRequestSeq = 0;
  let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null;

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
    searchStore.recentPlayedLoading = true;
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
          error: formatLibraryError(e),
        })
      );
    } finally {
      if (seq === loadRequestSeq) {
        searchStore.recentPlayedLoading = false;
      }
    }
  }

  function cancelScheduledSearch() {
    if (!searchDebounceTimer) return;
    clearTimeout(searchDebounceTimer);
    searchDebounceTimer = null;
  }

  function clearActiveSearch() {
    cancelScheduledSearch();
    searchRequestSeq += 1;
    searchStore.searchLoading = false;
    searchStore.loadingMore = false;
    searchStore.response = null;
    searchStore.searchError = null;
  }

  async function runSearch(
    query: string,
    scope: LibrarySearchScope,
    offset = 0
  ) {
    const trimmedQuery = query.trim();
    if (!trimmedQuery) {
      clearActiveSearch();
      return;
    }

    const requestSeq = ++searchRequestSeq;
    const isLoadingMore = offset > 0;
    if (isLoadingMore) {
      searchStore.loadingMore = true;
    } else {
      searchStore.searchLoading = true;
      searchStore.searchError = null;
      const current = searchStore.response;
      if (
        current &&
        (current.query !== trimmedQuery || current.scope !== scope)
      ) {
        searchStore.response = null;
      }
    }

    try {
      const response = await deps.searchLibrary({
        query: trimmedQuery,
        scope,
        limit: SEARCH_PAGE_SIZE,
        offset,
      });
      if (requestSeq !== searchRequestSeq) return;

      searchStore.indexState = response.indexState;
      const current = searchStore.response;
      if (
        isLoadingMore &&
        current?.query === response.query &&
        current.scope === response.scope
      ) {
        searchStore.response = {
          ...response,
          items: [...current.items, ...response.items],
        };
      } else {
        searchStore.response = response;
      }
    } catch (error) {
      if (requestSeq !== searchRequestSeq) return;
      const detail = formatSearchError(error);
      searchStore.searchError = detail;
      deps.notifyError(m.search_error_failed({ error: detail }));
    } finally {
      if (requestSeq === searchRequestSeq) {
        searchStore.searchLoading = false;
        searchStore.loadingMore = false;
      }
    }
  }

  function scheduleSearch() {
    cancelScheduledSearch();
    searchRequestSeq += 1;
    searchStore.loadingMore = false;
    const trimmedQuery = searchStore.query.trim();
    if (!trimmedQuery) {
      clearActiveSearch();
      return;
    }

    searchStore.searchLoading = true;
    searchStore.searchError = null;
    const current = searchStore.response;
    if (
      current &&
      (current.query !== trimmedQuery || current.scope !== searchStore.scope)
    ) {
      searchStore.response = null;
    }
    searchDebounceTimer = setTimeout(() => {
      searchDebounceTimer = null;
      void runSearch(searchStore.query, searchStore.scope);
    }, SEARCH_DEBOUNCE_MS);
  }

  function setQuery(q: string) {
    searchStore.query = q;
    scheduleSearch();
  }

  function setScope(s: LibrarySearchScope) {
    searchStore.scope = s;
    if (searchStore.query.trim()) {
      scheduleSearch();
    }
  }

  function submitSearch(): Promise<void> | undefined {
    const q = searchStore.query.trim();
    if (!q) return;
    saveRecentQuery(q, searchStore.scope);
    cancelScheduledSearch();
    return runSearch(q, searchStore.scope);
  }

  function rerunQuery(entry: RecentQuery): Promise<void> {
    cancelScheduledSearch();
    searchStore.query = entry.query;
    searchStore.scope = entry.scope;
    saveRecentQuery(entry.query, entry.scope);
    return runSearch(entry.query, entry.scope);
  }

  function loadMore(): Promise<void> | undefined {
    const response = searchStore.response;
    if (
      !response ||
      searchStore.searchLoading ||
      searchStore.loadingMore ||
      response.items.length >= response.total
    ) {
      return;
    }
    return runSearch(response.query, response.scope, response.items.length);
  }

  function retrySearch(): Promise<void> | undefined {
    const query = searchStore.query.trim();
    if (!query) return;
    cancelScheduledSearch();
    return runSearch(query, searchStore.scope);
  }

  function handleIndexStateChanged(state: LibraryIndexState) {
    searchStore.indexState = state;
    if (searchStore.response) {
      searchStore.response = {
        ...searchStore.response,
        indexState: state,
      };
    }
    if (state === 'ready' && searchStore.query.trim()) {
      void retrySearch();
    }
  }

  function dispose() {
    loadRequestSeq += 1;
    searchRequestSeq += 1;
    cancelScheduledSearch();
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
    get recentPlayedLoading() {
      return searchStore.recentPlayedLoading;
    },
    get searchLoading() {
      return searchStore.searchLoading;
    },
    get loadingMore() {
      return searchStore.loadingMore;
    },
    get response() {
      return searchStore.response;
    },
    get searchError() {
      return searchStore.searchError;
    },
    get indexState() {
      return searchStore.indexState;
    },
    init,
    dispose,
    loadRecentPlayed,
    setQuery,
    setScope,
    submitSearch,
    rerunQuery,
    loadMore,
    retrySearch,
    handleIndexStateChanged,
  };
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    searchStore.reset();
  });
}
