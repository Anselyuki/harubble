import type { Album } from '$lib/types';
import type { LibrarySearchScope } from '$lib/types';

export interface RecentQuery {
  query: string;
  scope: LibrarySearchScope;
  timestamp: number;
}

let query = $state('');
let scope = $state<LibrarySearchScope>('all');
let recentPlayed = $state<Album[]>([]);
let recentQueries = $state<RecentQuery[]>([]);
let loading = $state(false);

function reset() {
  query = '';
  scope = 'all';
  recentPlayed = [];
  recentQueries = [];
  loading = false;
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
  get loading() {
    return loading;
  },
  set loading(v: boolean) {
    loading = v;
  },
  reset,
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    reset();
  });
}
