import type {
  Album,
  LibraryIndexState,
  LibrarySearchScope,
  SearchLibraryResponse,
} from '$lib/types';

export interface RecentQuery {
  query: string;
  scope: LibrarySearchScope;
  timestamp: number;
}

let query = $state('');
let scope = $state<LibrarySearchScope>('all');
let recentPlayed = $state<Album[]>([]);
let recentQueries = $state<RecentQuery[]>([]);
let recentPlayedLoading = $state(false);
let searchLoading = $state(false);
let loadingMore = $state(false);
let response = $state<SearchLibraryResponse | null>(null);
let searchError = $state<string | null>(null);
let indexState = $state<LibraryIndexState>('notReady');

function reset() {
  query = '';
  scope = 'all';
  recentPlayed = [];
  recentQueries = [];
  recentPlayedLoading = false;
  searchLoading = false;
  loadingMore = false;
  response = null;
  searchError = null;
  indexState = 'notReady';
}

export const searchStore = {
  get query() {
    return query;
  },
  set query(v: string) {
    query = v;
  },
  get scope() {
    return scope;
  },
  set scope(v: LibrarySearchScope) {
    scope = v;
  },
  get recentPlayed() {
    return recentPlayed;
  },
  set recentPlayed(v: Album[]) {
    recentPlayed = v;
  },
  get recentQueries() {
    return recentQueries;
  },
  set recentQueries(v: RecentQuery[]) {
    recentQueries = v;
  },
  get recentPlayedLoading() {
    return recentPlayedLoading;
  },
  set recentPlayedLoading(v: boolean) {
    recentPlayedLoading = v;
  },
  get searchLoading() {
    return searchLoading;
  },
  set searchLoading(v: boolean) {
    searchLoading = v;
  },
  get loadingMore() {
    return loadingMore;
  },
  set loadingMore(v: boolean) {
    loadingMore = v;
  },
  get response() {
    return response;
  },
  set response(v: SearchLibraryResponse | null) {
    response = v;
  },
  get searchError() {
    return searchError;
  },
  set searchError(v: string | null) {
    searchError = v;
  },
  get indexState() {
    return indexState;
  },
  set indexState(v: LibraryIndexState) {
    indexState = v;
  },
  reset,
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    reset();
  });
}
