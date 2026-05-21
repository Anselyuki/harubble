import type {
  Album,
  AlbumDetail,
  LibrarySearchScope,
  LocalInventorySnapshot,
  SearchLibraryResponse,
} from '$lib/types';
import * as m from '$lib/paraglide/messages.js';

interface LibraryControllerDeps {
  delay: (ms: number) => Promise<void>;
  detailSkeletonDelayMs: number;
  minDetailDisplayMs: number;
  getAlbums: () => Promise<Album[]>;
  getAlbumDetail: (
    albumCid: string,
    inventoryVersion: string | null
  ) => Promise<AlbumDetail>;
  searchLibrary: (input: {
    query: string;
    scope: LibrarySearchScope;
  }) => Promise<SearchLibraryResponse>;
  preloadAlbumArtwork: (album: AlbumDetail) => Promise<number | null>;
  setAlbumStageAspectRatio: (value: number | null | undefined) => void;
  notifyError: (message: string) => void;
}

interface SelectAlbumOptions {
  shouldDispose?: () => boolean;
  afterSelect?: () => void | Promise<void>;
}

interface LoadAlbumsOptions {
  shouldDispose?: () => boolean;
  suppressError?: boolean;
}

interface HandleInventoryStateChangedOptions {
  shouldDispose?: () => boolean;
  invalidateInventoryCaches: (
    inventoryVersion: string | null | undefined
  ) => Promise<void>;
  onSelectionInvalidated?: () => void;
}

export function createLibraryController(deps: LibraryControllerDeps) {
  let initialized = false;
  let albums = $state<Album[]>([]);
  let selectedAlbum = $state<AlbumDetail | null>(null);
  let selectedAlbumCid = $state<string | null>(null);
  let loadingAlbums = $state(false);
  let loadingDetail = $state(false);
  let errorMsg = $state('');
  let librarySearchQuery = $state('');
  let librarySearchScope = $state<LibrarySearchScope>('all');
  let librarySearchLoading = $state(false);
  let librarySearchResponse = $state<SearchLibraryResponse | null>(null);
  let pendingScrollToSongCid = $state<string | null>(null);
  let showDetailSkeleton = $state(false);
  let localInventory = $state<LocalInventorySnapshot | null>(null);
  let localInventoryVersionInitialized = $state(false);
  let albumRequestSeq = $state(0);
  let librarySearchRequestSeq = 0;
  let inventoryRefreshRequestSeq = 0;
  let detailSkeletonTimer: ReturnType<typeof setTimeout> | null = null;
  let librarySearchDebounceTimer: ReturnType<typeof setTimeout> | null = null;

  function init() {
    if (initialized) return;
    initialized = true;
  }

  function armDetailSkeleton() {
    if (detailSkeletonTimer) {
      clearTimeout(detailSkeletonTimer);
    }

    showDetailSkeleton = false;
    detailSkeletonTimer = setTimeout(() => {
      if (loadingDetail) {
        showDetailSkeleton = true;
      }
    }, deps.detailSkeletonDelayMs);
  }

  function clearDetailSkeleton() {
    if (detailSkeletonTimer) {
      clearTimeout(detailSkeletonTimer);
      detailSkeletonTimer = null;
    }
    showDetailSkeleton = false;
  }

  async function runLibrarySearch(query: string, scope: LibrarySearchScope) {
    const trimmedQuery = query.trim();
    const requestSeq = ++librarySearchRequestSeq;

    if (!trimmedQuery) {
      librarySearchLoading = false;
      librarySearchResponse = null;
      return;
    }

    librarySearchLoading = true;
    try {
      const response = await deps.searchLibrary({
        query: trimmedQuery,
        scope,
      });
      if (requestSeq !== librarySearchRequestSeq) return;
      librarySearchResponse = response;
    } catch (error) {
      if (requestSeq !== librarySearchRequestSeq) return;
      const message = error instanceof Error ? error.message : String(error);
      librarySearchResponse = {
        items: [],
        total: 0,
        query: trimmedQuery,
        scope,
        indexState: 'notReady',
      };
      deps.notifyError(m.library_error_search_failed({ error: message }));
    } finally {
      if (requestSeq === librarySearchRequestSeq) {
        librarySearchLoading = false;
      }
    }
  }

  function scheduleLibrarySearch() {
    if (librarySearchDebounceTimer) {
      clearTimeout(librarySearchDebounceTimer);
    }

    const trimmedQuery = librarySearchQuery.trim();
    if (!trimmedQuery) {
      librarySearchRequestSeq += 1;
      librarySearchLoading = false;
      librarySearchResponse = null;
      return;
    }

    librarySearchDebounceTimer = setTimeout(() => {
      void runLibrarySearch(librarySearchQuery, librarySearchScope);
    }, 220);
  }

  async function loadAlbums(options?: LoadAlbumsOptions): Promise<Album[]> {
    const shouldDispose = options?.shouldDispose;
    const suppressError = options?.suppressError ?? true;
    loadingAlbums = true;

    try {
      const albumList = await deps.getAlbums();
      if (shouldDispose?.()) {
        return albums;
      }
      albums = albumList;
      errorMsg = '';
      return albumList;
    } catch (error) {
      if (!shouldDispose?.()) {
        errorMsg = error instanceof Error ? error.message : String(error);
      }
      if (!suppressError) {
        throw error;
      }
      return albums;
    } finally {
      if (!shouldDispose?.()) {
        loadingAlbums = false;
      }
    }
  }

  async function selectAlbum(album: Album, options?: SelectAlbumOptions) {
    const shouldDispose = options?.shouldDispose;
    if (shouldDispose?.()) {
      return;
    }

    if (album.cid === selectedAlbumCid && !loadingDetail) {
      return;
    }

    const requestSeq = ++albumRequestSeq;
    selectedAlbumCid = album.cid;
    loadingDetail = true;
    if (!selectedAlbum) {
      armDetailSkeleton();
    } else {
      clearDetailSkeleton();
    }

    const startTime = Date.now();
    try {
      const detail = await deps.getAlbumDetail(
        album.cid,
        localInventory?.inventoryVersion ?? null
      );
      if (shouldDispose?.() || requestSeq !== albumRequestSeq) return;
      const artworkAspectRatio = await deps.preloadAlbumArtwork(detail);
      if (shouldDispose?.() || requestSeq !== albumRequestSeq) return;
      selectedAlbum = detail;
      deps.setAlbumStageAspectRatio(artworkAspectRatio);
      errorMsg = '';
      await options?.afterSelect?.();
      if (shouldDispose?.() || requestSeq !== albumRequestSeq) return;
    } catch (error) {
      if (shouldDispose?.() || requestSeq !== albumRequestSeq) return;
      errorMsg = error instanceof Error ? error.message : String(error);
    } finally {
      if (!shouldDispose?.() && requestSeq === albumRequestSeq) {
        const elapsed = Date.now() - startTime;
        if (elapsed < deps.minDetailDisplayMs) {
          await deps.delay(deps.minDetailDisplayMs - elapsed);
        }
        if (!shouldDispose?.() && requestSeq === albumRequestSeq) {
          clearDetailSkeleton();
          loadingDetail = false;
        }
      }
    }
  }

  async function replaceAlbumsAndRefreshCurrentSelection(
    nextAlbums: Album[],
    options?: SelectAlbumOptions
  ) {
    const shouldDispose = options?.shouldDispose;
    if (shouldDispose?.()) {
      return;
    }

    albums = nextAlbums;
    if (!selectedAlbumCid) {
      return;
    }

    const currentAlbumCid = selectedAlbumCid;
    const refreshedAlbum = nextAlbums.find(
      (album) => album.cid === currentAlbumCid
    );
    if (!refreshedAlbum) {
      selectedAlbum = null;
      selectedAlbumCid = null;
      clearPendingScrollToSong();
      clearDetailSkeleton();
      loadingDetail = false;
      return;
    }

    const requestSeq = ++albumRequestSeq;
    loadingDetail = true;
    if (!selectedAlbum) {
      armDetailSkeleton();
    } else {
      clearDetailSkeleton();
    }

    try {
      const detail = await deps.getAlbumDetail(
        currentAlbumCid,
        localInventory?.inventoryVersion ?? null
      );
      if (shouldDispose?.() || requestSeq !== albumRequestSeq) return;
      const artworkAspectRatio = await deps.preloadAlbumArtwork(detail);
      if (shouldDispose?.() || requestSeq !== albumRequestSeq) return;
      deps.setAlbumStageAspectRatio(artworkAspectRatio);
      selectedAlbum = detail;
      await options?.afterSelect?.();
    } catch (error) {
      if (shouldDispose?.() || requestSeq !== albumRequestSeq) return;
      deps.notifyError(
        m.library_error_refresh_album_failed({
          error: error instanceof Error ? error.message : String(error),
        })
      );
    } finally {
      if (!shouldDispose?.() && requestSeq === albumRequestSeq) {
        clearDetailSkeleton();
        loadingDetail = false;
      }
    }
  }

  async function reloadAlbumsAndRefreshCurrentSelection(
    options?: SelectAlbumOptions
  ) {
    const nextAlbums = await loadAlbums({
      shouldDispose: options?.shouldDispose,
      suppressError: false,
    });
    if (options?.shouldDispose?.()) {
      return;
    }
    await replaceAlbumsAndRefreshCurrentSelection(nextAlbums, options);
  }

  function initializeInventory(snapshot: LocalInventorySnapshot | null) {
    if (localInventoryVersionInitialized) {
      return;
    }

    localInventory = snapshot;
    localInventoryVersionInitialized = true;
  }

  async function handleInventoryStateChanged(
    snapshot: LocalInventorySnapshot,
    options: HandleInventoryStateChangedOptions
  ) {
    const shouldDispose = options.shouldDispose;
    const previousVersion = localInventory?.inventoryVersion ?? null;
    const previousStatus = localInventory?.status ?? null;
    localInventory = snapshot;
    localInventoryVersionInitialized = true;
    const inventoryVersionChanged =
      previousVersion !== snapshot.inventoryVersion;
    const scanJustCompleted =
      snapshot.status === 'completed' && previousStatus !== 'completed';

    if (inventoryVersionChanged) {
      await options.invalidateInventoryCaches(previousVersion);
      if (shouldDispose?.()) {
        return;
      }
    }

    if (!scanJustCompleted) {
      return;
    }

    const refreshedAlbums = await loadAlbums({ shouldDispose });
    if (shouldDispose?.()) {
      return;
    }

    const currentSelectedAlbumCid = selectedAlbumCid;
    if (!currentSelectedAlbumCid) {
      return;
    }

    const refreshedAlbum = refreshedAlbums.find(
      (album) => album.cid === currentSelectedAlbumCid
    );
    if (!refreshedAlbum) {
      selectedAlbum = null;
      selectedAlbumCid = null;
      clearPendingScrollToSong();
      clearDetailSkeleton();
      loadingDetail = false;
      options.onSelectionInvalidated?.();
      return;
    }

    const refreshRequestSeq = ++inventoryRefreshRequestSeq;

    try {
      const detail = await deps.getAlbumDetail(
        currentSelectedAlbumCid,
        snapshot.inventoryVersion
      );
      if (
        shouldDispose?.() ||
        refreshRequestSeq !== inventoryRefreshRequestSeq ||
        selectedAlbumCid !== currentSelectedAlbumCid
      ) {
        return;
      }
      selectedAlbum = detail;
    } catch {
      // Keep current UI state if refresh fails.
    }
  }

  function setSearchQuery(query: string) {
    librarySearchQuery = query;
    scheduleLibrarySearch();
  }

  function setSearchScope(scope: LibrarySearchScope) {
    librarySearchScope = scope;
    scheduleLibrarySearch();
  }

  function setPendingScrollToSong(songCid: string | null) {
    pendingScrollToSongCid = songCid;
  }

  function deselectAlbum() {
    selectedAlbum = null;
    selectedAlbumCid = null;
    clearPendingScrollToSong();
    clearDetailSkeleton();
    loadingDetail = false;
  }

  function clearPendingScrollToSong(songCid?: string) {
    if (!songCid || pendingScrollToSongCid === songCid) {
      pendingScrollToSongCid = null;
    }
  }

  function dispose() {
    initialized = false;
    clearDetailSkeleton();
    if (librarySearchDebounceTimer) {
      clearTimeout(librarySearchDebounceTimer);
      librarySearchDebounceTimer = null;
    }
    librarySearchRequestSeq += 1;
    inventoryRefreshRequestSeq += 1;
    albumRequestSeq += 1;
  }

  return {
    get albums() {
      return albums;
    },
    get selectedAlbum() {
      return selectedAlbum;
    },
    get selectedAlbumCid() {
      return selectedAlbumCid;
    },
    get loadingAlbums() {
      return loadingAlbums;
    },
    get loadingDetail() {
      return loadingDetail;
    },
    get errorMsg() {
      return errorMsg;
    },
    get librarySearchQuery() {
      return librarySearchQuery;
    },
    get librarySearchScope() {
      return librarySearchScope;
    },
    get librarySearchLoading() {
      return librarySearchLoading;
    },
    get librarySearchResponse() {
      return librarySearchResponse;
    },
    get pendingScrollToSongCid() {
      return pendingScrollToSongCid;
    },
    get showDetailSkeleton() {
      return showDetailSkeleton;
    },
    get albumRequestSeq() {
      return albumRequestSeq;
    },
    init,
    dispose,
    loadAlbums,
    selectAlbum,
    deselectAlbum,
    replaceAlbumsAndRefreshCurrentSelection,
    reloadAlbumsAndRefreshCurrentSelection,
    initializeInventory,
    handleInventoryStateChanged,
    setSearchQuery,
    setSearchScope,
    setPendingScrollToSong,
    clearPendingScrollToSong,
  };
}
