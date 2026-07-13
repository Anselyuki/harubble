import type {
  Album,
  SeriesGroup,
  HistoryEntry,
  HomepageStatus,
  TagDimension,
  TagGroup,
} from '$lib/types';
import { homeStore } from './store.svelte';
import { formatLibraryError } from '$lib/features/shell/domainErrors';
import * as m from '$lib/paraglide/messages.js';

interface HomeControllerDeps {
  getLatestAlbums: (limit: number) => Promise<Album[]>;
  getAlbumsBySeriesGroup: () => Promise<SeriesGroup[]>;
  getRecentHistory: (limit: number) => Promise<HistoryEntry[]>;
  getHomepageStatus: () => Promise<HomepageStatus>;
  clearListeningHistory: () => Promise<number>;
  getTagDimensions: () => Promise<TagDimension[]>;
  getAlbumsByTagDimension: (dimensionKey: string) => Promise<TagGroup[]>;
  notifyError: (message: string) => void;
}

const CACHE_TTL_MS = 5 * 60 * 1000;
const LATEST_ALBUMS_LIMIT = 12;
const RECENT_HISTORY_LIMIT = 20;

export function createHomeController(deps: HomeControllerDeps) {
  let initialized = false;
  let loadRequestSeq = 0;

  function init() {
    if (initialized) return;
    initialized = true;
  }

  async function loadHomepageData(options?: { force?: boolean }) {
    const now = Date.now();
    const lastLoaded = homeStore.lastLoadedAt;

    if (
      !options?.force &&
      lastLoaded !== null &&
      now - lastLoaded < CACHE_TTL_MS
    ) {
      return;
    }

    const requestSeq = ++loadRequestSeq;
    homeStore.loading = true;

    const results = await Promise.allSettled([
      deps.getLatestAlbums(LATEST_ALBUMS_LIMIT),
      deps.getAlbumsBySeriesGroup(),
      deps.getRecentHistory(RECENT_HISTORY_LIMIT),
      deps.getHomepageStatus(),
      deps.getTagDimensions(),
    ]);

    if (requestSeq !== loadRequestSeq) return;

    if (results[0].status === 'fulfilled') {
      homeStore.latestAlbums = results[0].value;
    } else {
      const reason = results[0].reason;
      deps.notifyError(
        m.home_error_load_albums({
          error: formatLibraryError(reason),
        })
      );
    }

    if (results[1].status === 'fulfilled') {
      homeStore.seriesGroups = results[1].value;
      if (results[1].value.length > 0) {
        homeStore.belongReady = true;
      }
    }

    if (results[2].status === 'fulfilled') {
      homeStore.recentHistory = results[2].value;
    }

    if (results[3].status === 'fulfilled') {
      homeStore.status = results[3].value;
    }

    if (results[4].status === 'fulfilled') {
      homeStore.tagDimensions = results[4].value;
      if (
        results[4].value.length > 0 &&
        homeStore.selectedDimensionKey === null
      ) {
        homeStore.selectedDimensionKey = results[4].value[0].key;
      }
    }

    if (results[4].status === 'fulfilled' && results[4].value.length > 0) {
      void loadTagGroups(results[4].value[0].key);
    }

    homeStore.loading = false;
    homeStore.lastLoadedAt = Date.now();
  }

  async function loadTagGroups(dimensionKey: string) {
    const requestSeq = ++loadRequestSeq;
    try {
      const groups = await deps.getAlbumsByTagDimension(dimensionKey);
      if (requestSeq !== loadRequestSeq) return;
      homeStore.tagGroups = groups;
    } catch (e: unknown) {
      deps.notifyError(
        m.home_error_load_tag_groups({
          error: formatLibraryError(e),
        })
      );
    }
  }

  async function selectDimension(dimensionKey: string) {
    if (homeStore.selectedDimensionKey === dimensionKey) return;
    homeStore.selectedDimensionKey = dimensionKey;
    homeStore.tagGroups = [];
    await loadTagGroups(dimensionKey);
  }

  async function refreshRecentHistory() {
    try {
      const entries = await deps.getRecentHistory(RECENT_HISTORY_LIMIT);
      homeStore.recentHistory = entries;
    } catch {
      // 刷新失败静默忽略，不影响已有数据
    }
  }

  async function refreshHomepage() {
    await loadHomepageData({ force: true });
  }

  async function refreshSeriesGroups() {
    const requestSeq = loadRequestSeq;
    try {
      const groups = await deps.getAlbumsBySeriesGroup();
      if (requestSeq !== loadRequestSeq) return;
      homeStore.seriesGroups = groups;
      homeStore.belongReady = true;
    } catch {
      // belong 刷新失败不阻塞
    }
  }

  async function handleClearHistory() {
    try {
      await deps.clearListeningHistory();
      homeStore.recentHistory = [];
    } catch (e: unknown) {
      deps.notifyError(
        m.home_error_clear_history({
          error: formatLibraryError(e),
        })
      );
    }
  }

  function handleBelongReady() {
    homeStore.belongReady = true;
    void refreshSeriesGroups();
  }

  function dispose() {
    loadRequestSeq += 1;
    initialized = false;
    homeStore.reset();
  }

  return {
    get latestAlbums() {
      return homeStore.latestAlbums;
    },
    get seriesGroups() {
      return homeStore.seriesGroups;
    },
    get recentHistory() {
      return homeStore.recentHistory;
    },
    get status() {
      return homeStore.status;
    },
    get loading() {
      return homeStore.loading;
    },
    get belongReady() {
      return homeStore.belongReady;
    },
    get tagDimensions() {
      return homeStore.tagDimensions;
    },
    get tagGroups() {
      return homeStore.tagGroups;
    },
    get selectedDimensionKey() {
      return homeStore.selectedDimensionKey;
    },
    init,
    loadHomepageData,
    refreshHomepage,
    refreshRecentHistory,
    refreshSeriesGroups,
    selectDimension,
    handleClearHistory,
    handleBelongReady,
    dispose,
  };
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    homeStore.reset();
  });
}
