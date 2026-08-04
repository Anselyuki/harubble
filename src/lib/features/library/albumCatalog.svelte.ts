import type {
  Album,
  AlbumCatalogRefreshedEvent,
  AlbumCatalogSnapshot,
} from '$lib/types';

export type AlbumCatalogRefreshReason =
  | 'bootstrap'
  | 'manual'
  | 'view-enter'
  | 'timer'
  | 'focus'
  | 'online'
  | 'inventory'
  | 'event';

export interface AlbumCatalogRefreshOptions {
  forceRemote: boolean;
  silent: boolean;
  reason: AlbumCatalogRefreshReason;
}

interface AlbumCatalogControllerDeps {
  getAlbumCatalog: () => Promise<AlbumCatalogSnapshot>;
  refreshAlbumCatalog: () => Promise<AlbumCatalogSnapshot>;
  onRemoteCatalogChanged?: (
    albums: Album[],
    revision: number
  ) => void | Promise<void>;
  now?: () => number;
}

const DEFAULT_MAX_AGE_MS = 5 * 60 * 1000;

function stringArraysEqual(left: string[], right: string[]) {
  return (
    left.length === right.length &&
    left.every((value, index) => value === right[index])
  );
}

function albumsEqual(left: Album[], right: Album[]) {
  if (left === right) return true;
  if (left.length !== right.length) return false;

  return left.every((album, index) => {
    const other = right[index];
    return (
      album.cid === other.cid &&
      album.name === other.name &&
      album.coverUrl === other.coverUrl &&
      stringArraysEqual(album.artists, other.artists) &&
      album.download.isDownloaded === other.download.isDownloaded &&
      album.download.downloadStatus === other.download.downloadStatus &&
      album.download.inventoryVersion === other.download.inventoryVersion &&
      album.tags.length === other.tags.length &&
      album.tags.every((tag, tagIndex) => {
        const otherTag = other.tags[tagIndex];
        return (
          tag.dimension === otherTag.dimension &&
          stringArraysEqual(tag.values, otherTag.values) &&
          stringArraysEqual(
            tag.colors?.map((color) => color ?? '') ?? [],
            otherTag.colors?.map((color) => color ?? '') ?? []
          )
        );
      })
    );
  });
}

export function createAlbumCatalogController(deps: AlbumCatalogControllerDeps) {
  let albums = $state<Album[]>([]);
  let initialLoading = $state(false);
  let refreshing = $state(false);
  let revision = $state(0);
  let lastRemoteSyncAt = $state<number | null>(null);
  let lastError = $state<unknown>(null);
  let initialized = false;
  let disposed = false;
  let requestSequence = 0;
  let activeRequest: Promise<Album[]> | null = null;
  let activeRequestForcesRemote = false;
  let queuedForceRequest: Promise<Album[]> | null = null;
  let pendingEventRevision = 0;
  let eventSyncRequest: Promise<void> | null = null;

  const now = deps.now ?? Date.now;

  async function applySnapshot(snapshot: AlbumCatalogSnapshot) {
    if (lastRemoteSyncAt === null || snapshot.checkedAt > lastRemoteSyncAt) {
      lastRemoteSyncAt = snapshot.checkedAt;
    }

    if (snapshot.revision < revision) {
      return;
    }

    const remoteCatalogChanged = snapshot.revision > revision;
    if (!albumsEqual(albums, snapshot.albums)) {
      albums = snapshot.albums;
    }
    revision = snapshot.revision;
    initialized = true;
    if (remoteCatalogChanged) {
      await deps.onRemoteCatalogChanged?.(albums, revision);
    }
  }

  function beginRefresh(forceRemote: boolean) {
    const requestId = ++requestSequence;
    const isInitialLoad = !initialized;
    initialLoading = isInitialLoad;
    refreshing = !isInitialLoad;
    activeRequestForcesRemote = forceRemote;

    const request = (async () => {
      try {
        const snapshot = await (forceRemote
          ? deps.refreshAlbumCatalog()
          : deps.getAlbumCatalog());
        if (!disposed && requestId === requestSequence) {
          await applySnapshot(snapshot);
          lastError = null;
        }
        return albums;
      } catch (error) {
        if (!disposed && requestId === requestSequence) {
          lastError = error;
        }
        throw error;
      } finally {
        if (!disposed && requestId === requestSequence) {
          initialLoading = false;
          refreshing = false;
          activeRequest = null;
          activeRequestForcesRemote = false;
        }
      }
    })();

    activeRequest = request;
    return request;
  }

  async function refresh(options: AlbumCatalogRefreshOptions) {
    if (disposed) return albums;

    let request = queuedForceRequest ?? activeRequest;
    if (
      !queuedForceRequest &&
      request &&
      options.forceRemote &&
      !activeRequestForcesRemote &&
      initialized
    ) {
      queuedForceRequest ??= request
        .catch(() => albums)
        .then(() => (disposed ? albums : beginRefresh(true)))
        .finally(() => {
          queuedForceRequest = null;
        });
      request = queuedForceRequest;
    } else if (!request) {
      request = beginRefresh(options.forceRemote);
    }
    try {
      return await request;
    } catch (error) {
      if (!options.silent) {
        throw error;
      }
      return albums;
    }
  }

  async function bootstrap() {
    if (initialized) {
      return albums;
    }

    return refresh({
      forceRemote: false,
      silent: false,
      reason: 'bootstrap',
    });
  }

  async function ensureFresh(maxAgeMs = DEFAULT_MAX_AGE_MS) {
    const checkedAt = lastRemoteSyncAt;
    if (checkedAt !== null && now() - checkedAt < maxAgeMs) {
      return;
    }

    await refresh({
      forceRemote: true,
      silent: true,
      reason: 'view-enter',
    });
  }

  async function handleRefreshedEvent(event: AlbumCatalogRefreshedEvent) {
    if (disposed) return;

    if (lastRemoteSyncAt === null || event.checkedAt > lastRemoteSyncAt) {
      lastRemoteSyncAt = event.checkedAt;
    }

    if (!event.changed || event.revision <= revision) {
      return;
    }

    pendingEventRevision = Math.max(pendingEventRevision, event.revision);
    if (eventSyncRequest) {
      return eventSyncRequest;
    }

    eventSyncRequest = (async () => {
      const pendingRequest = queuedForceRequest ?? activeRequest;
      if (pendingRequest) {
        try {
          await pendingRequest;
        } catch {
          // The cached snapshot read below is still useful after a failed caller request.
        }
      }

      while (revision < pendingEventRevision) {
        const targetRevision = pendingEventRevision;
        await refresh({
          forceRemote: false,
          silent: true,
          reason: 'event',
        });
        if (revision < targetRevision) {
          break;
        }
      }
    })();

    try {
      await eventSyncRequest;
    } finally {
      eventSyncRequest = null;
    }
  }

  function dispose() {
    disposed = true;
    requestSequence += 1;
    activeRequest = null;
    activeRequestForcesRemote = false;
    queuedForceRequest = null;
    eventSyncRequest = null;
    initialLoading = false;
    refreshing = false;
  }

  return {
    get albums() {
      return albums;
    },
    get initialLoading() {
      return initialLoading;
    },
    get refreshing() {
      return refreshing;
    },
    get revision() {
      return revision;
    },
    get lastRemoteSyncAt() {
      return lastRemoteSyncAt;
    },
    get lastError() {
      return lastError;
    },
    bootstrap,
    ensureFresh,
    refresh,
    handleRefreshedEvent,
    dispose,
  };
}

export type AlbumCatalogController = ReturnType<
  typeof createAlbumCatalogController
>;
