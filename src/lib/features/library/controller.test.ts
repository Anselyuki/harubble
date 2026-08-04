import { describe, expect, it, vi } from 'vitest';
import type {
  Album,
  AlbumCatalogSnapshot,
  AlbumDetail,
  LocalInventorySnapshot,
} from '$lib/types';
import { createLibraryController } from './controller.svelte';
import { createAlbumCatalogController } from './albumCatalog.svelte';

function makeInventorySnapshot(
  inventoryVersion: string,
  status: LocalInventorySnapshot['status'] = 'idle'
): LocalInventorySnapshot {
  return {
    rootOutputDir: '/tmp',
    status,
    inventoryVersion,
    startedAt: null,
    finishedAt: null,
    scannedFileCount: 0,
    matchedTrackCount: 0,
    verifiedTrackCount: 0,
    lastError: null,
  };
}

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

function makeAlbumDetail(cid: string): AlbumDetail {
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
    songs: [],
  };
}

function makeDeps(
  overrides: Partial<Parameters<typeof createLibraryController>[0]> = {}
) {
  return {
    delay: vi.fn().mockResolvedValue(undefined),
    detailSkeletonDelayMs: 0,
    minDetailDisplayMs: 0,
    albumCatalog: makeCatalog(),
    getAlbumDetail: vi.fn().mockResolvedValue(makeAlbumDetail('a')),
    refreshAlbumDetail: vi.fn().mockResolvedValue(makeAlbumDetail('a')),
    preloadAlbumArtwork: vi.fn().mockResolvedValue(null),
    warmAlbumArtwork: vi.fn(),
    setAlbumStageAspectRatio: vi.fn(),
    notifyError: vi.fn(),
    ...overrides,
  };
}

function makeCatalog(initialAlbums: Album[] = []) {
  let albums = initialAlbums;
  let initialLoading = false;
  let refreshing = false;
  let revision = 0;
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
    bootstrap: vi.fn(async () => albums),
    refresh: vi.fn(async () => albums),
    setAlbums(nextAlbums: Album[]) {
      albums = nextAlbums;
    },
    setRevision(nextRevision: number) {
      revision = nextRevision;
    },
    setLoading(nextInitialLoading: boolean, nextRefreshing: boolean) {
      initialLoading = nextInitialLoading;
      refreshing = nextRefreshing;
    },
  };
}

function makeCatalogSnapshot(
  revision: number,
  checkedAt: number,
  albums: Album[]
): AlbumCatalogSnapshot {
  return { revision, checkedAt, albums };
}

function createCatalogBackedLibrary(
  catalogDeps: Omit<
    Parameters<typeof createAlbumCatalogController>[0],
    'onRemoteCatalogChanged'
  >,
  libraryOverrides: Partial<Parameters<typeof createLibraryController>[0]> = {},
  onSelectionInvalidated?: () => void
) {
  const controllerRef: {
    current: ReturnType<typeof createLibraryController> | null;
  } = { current: null };
  const albumCatalog = createAlbumCatalogController({
    ...catalogDeps,
    onRemoteCatalogChanged: (albums) =>
      controllerRef.current?.handleRemoteCatalogChanged(albums, {
        onSelectionInvalidated,
      }),
  });
  const controller = createLibraryController(
    makeDeps({ ...libraryOverrides, albumCatalog })
  );
  controllerRef.current = controller;
  return { albumCatalog, controller };
}

describe('createLibraryController', () => {
  it('reads albums directly from the shared catalog', () => {
    const albumCatalog = makeCatalog([makeAlbum('a')]);
    const ctrl = createLibraryController(makeDeps({ albumCatalog }));

    expect(ctrl.albums.map((album) => album.cid)).toEqual(['a']);
    albumCatalog.setAlbums([makeAlbum('b')]);
    expect(ctrl.albums.map((album) => album.cid)).toEqual(['b']);
  });

  it('does not enter library loading while existing albums refresh', () => {
    const albumCatalog = makeCatalog([makeAlbum('a')]);
    const ctrl = createLibraryController(makeDeps({ albumCatalog }));

    albumCatalog.setLoading(false, true);

    expect(ctrl.loadingAlbums).toBe(false);
  });

  it('clears selection when an event-driven catalog update removes it', async () => {
    const albumA = makeAlbum('a');
    const onSelectionInvalidated = vi.fn();
    const getAlbumCatalog = vi
      .fn()
      .mockResolvedValueOnce(makeCatalogSnapshot(1, 100, [albumA]))
      .mockResolvedValueOnce(makeCatalogSnapshot(2, 200, [makeAlbum('b')]));
    const { albumCatalog, controller: ctrl } = createCatalogBackedLibrary(
      {
        getAlbumCatalog,
        refreshAlbumCatalog: vi.fn(),
      },
      {},
      onSelectionInvalidated
    );
    await albumCatalog.bootstrap();
    await ctrl.selectAlbum(albumA);

    await albumCatalog.handleRefreshedEvent({
      revision: 2,
      checkedAt: 200,
      changed: true,
      albumCount: 1,
    });

    expect(ctrl.selectedAlbum).toBeNull();
    expect(ctrl.selectedAlbumCid).toBeNull();
    expect(onSelectionInvalidated).toHaveBeenCalledOnce();
  });

  it('refreshes a retained selection once after a scheduled update', async () => {
    const albumA = makeAlbum('a');
    const getAlbumDetail = vi.fn().mockResolvedValue(makeAlbumDetail('a'));
    const refreshAlbumDetail = vi.fn().mockResolvedValue(makeAlbumDetail('a'));
    const { albumCatalog, controller: ctrl } = createCatalogBackedLibrary(
      {
        getAlbumCatalog: vi
          .fn()
          .mockResolvedValue(makeCatalogSnapshot(1, 100, [albumA])),
        refreshAlbumCatalog: vi
          .fn()
          .mockResolvedValue(makeCatalogSnapshot(2, 200, [albumA])),
        now: () => 200,
      },
      { getAlbumDetail, refreshAlbumDetail }
    );
    await albumCatalog.bootstrap();
    await ctrl.selectAlbum(albumA);

    await albumCatalog.ensureFresh(0);

    expect(getAlbumDetail).toHaveBeenCalledOnce();
    expect(refreshAlbumDetail).toHaveBeenCalledOnce();
    expect(ctrl.selectedAlbumCid).toBe('a');
  });

  it('does not double-refresh detail after a changed manual catalog refresh', async () => {
    const albumA = makeAlbum('a');
    const getAlbumDetail = vi.fn().mockResolvedValue(makeAlbumDetail('a'));
    const refreshAlbumDetail = vi.fn().mockResolvedValue(makeAlbumDetail('a'));
    const { albumCatalog, controller: ctrl } = createCatalogBackedLibrary(
      {
        getAlbumCatalog: vi
          .fn()
          .mockResolvedValue(makeCatalogSnapshot(1, 100, [albumA])),
        refreshAlbumCatalog: vi
          .fn()
          .mockResolvedValue(makeCatalogSnapshot(2, 200, [albumA])),
      },
      { getAlbumDetail, refreshAlbumDetail }
    );
    await albumCatalog.bootstrap();
    await ctrl.selectAlbum(albumA);
    const afterSelect = vi.fn();

    await ctrl.reloadAlbumsAndRefreshCurrentSelection({ afterSelect });

    expect(getAlbumDetail).toHaveBeenCalledOnce();
    expect(refreshAlbumDetail).toHaveBeenCalledOnce();
    expect(afterSelect).toHaveBeenCalledOnce();
  });

  it('still refreshes detail when a manual check keeps the revision', async () => {
    const albumA = makeAlbum('a');
    const getAlbumDetail = vi.fn().mockResolvedValue(makeAlbumDetail('a'));
    const refreshAlbumDetail = vi.fn().mockResolvedValue(makeAlbumDetail('a'));
    const { albumCatalog, controller: ctrl } = createCatalogBackedLibrary(
      {
        getAlbumCatalog: vi
          .fn()
          .mockResolvedValue(makeCatalogSnapshot(1, 100, [albumA])),
        refreshAlbumCatalog: vi
          .fn()
          .mockResolvedValue(makeCatalogSnapshot(1, 200, [albumA])),
      },
      { getAlbumDetail, refreshAlbumDetail }
    );
    await albumCatalog.bootstrap();
    await ctrl.selectAlbum(albumA);

    await ctrl.reloadAlbumsAndRefreshCurrentSelection();

    expect(getAlbumDetail).toHaveBeenCalledOnce();
    expect(refreshAlbumDetail).toHaveBeenCalledOnce();
  });

  describe('selectAlbum request sequence', () => {
    it('only applies the last result when two selects race', async () => {
      let resolveA!: (v: AlbumDetail) => void;
      let resolveB!: (v: AlbumDetail) => void;

      const detailA = makeAlbumDetail('album-a');
      const detailB = makeAlbumDetail('album-b');

      const getAlbumDetail = vi
        .fn()
        .mockImplementationOnce(
          () => new Promise<AlbumDetail>((r) => (resolveA = r))
        )
        .mockImplementationOnce(
          () => new Promise<AlbumDetail>((r) => (resolveB = r))
        );

      const deps = makeDeps({ getAlbumDetail });
      const ctrl = createLibraryController(deps);

      const albumA = makeAlbum('album-a');
      const albumB = makeAlbum('album-b');

      const p1 = ctrl.selectAlbum(albumA);
      const p2 = ctrl.selectAlbum(albumB);

      resolveB(detailB);
      await p2;
      resolveA(detailA);
      await p1;

      expect(ctrl.selectedAlbum?.cid).toBe('album-b');
    });

    it('skips result for disposed selection', async () => {
      const getAlbumDetail = vi.fn().mockResolvedValue(makeAlbumDetail('a'));
      const deps = makeDeps({ getAlbumDetail });
      const ctrl = createLibraryController(deps);

      let disposed = false;
      const p = ctrl.selectAlbum(makeAlbum('a'), {
        shouldDispose: () => disposed,
      });
      disposed = true;
      await p;

      expect(ctrl.selectedAlbum).toBeNull();
    });
  });

  describe('replaceAlbumsAndRefreshCurrentSelection', () => {
    it('clears selection when the current album is removed from the new list', async () => {
      const deps = makeDeps();
      const ctrl = createLibraryController(deps);

      await ctrl.selectAlbum(makeAlbum('a'));
      expect(ctrl.selectedAlbumCid).toBe('a');

      await ctrl.replaceAlbumsAndRefreshCurrentSelection([makeAlbum('b')]);

      expect(ctrl.selectedAlbum).toBeNull();
      expect(ctrl.selectedAlbumCid).toBeNull();
    });

    it('refreshes detail for the current album if it remains in the new list', async () => {
      const refreshedDetail = makeAlbumDetail('a');
      const refreshAlbumDetail = vi.fn().mockResolvedValue(refreshedDetail);
      const deps = makeDeps({ refreshAlbumDetail });
      const ctrl = createLibraryController(deps);

      await ctrl.selectAlbum(makeAlbum('a'));

      await ctrl.replaceAlbumsAndRefreshCurrentSelection([makeAlbum('a')]);

      expect(refreshAlbumDetail).toHaveBeenCalledOnce();
      expect(ctrl.selectedAlbum).toBe(refreshedDetail);
    });
  });

  describe('handleInventoryStateChanged', () => {
    it('calls invalidateInventoryCaches when inventory version changes', async () => {
      const deps = makeDeps();
      const ctrl = createLibraryController(deps);
      const invalidateInventoryCaches = vi.fn().mockResolvedValue(undefined);

      const snapshot = makeInventorySnapshot('v1');
      ctrl.initializeInventory(snapshot);

      await ctrl.handleInventoryStateChanged(makeInventorySnapshot('v2'), {
        invalidateInventoryCaches,
      });

      expect(invalidateInventoryCaches).toHaveBeenCalledWith('v1');
    });

    it('does not call invalidateInventoryCaches when version is unchanged', async () => {
      const deps = makeDeps();
      const ctrl = createLibraryController(deps);
      const invalidateInventoryCaches = vi.fn().mockResolvedValue(undefined);

      const snapshot = makeInventorySnapshot('v1');
      ctrl.initializeInventory(snapshot);

      await ctrl.handleInventoryStateChanged(makeInventorySnapshot('v1'), {
        invalidateInventoryCaches,
      });

      expect(invalidateInventoryCaches).not.toHaveBeenCalled();
    });

    it('re-reads the cached catalog after an inventory scan completes', async () => {
      const albumCatalog = makeCatalog([makeAlbum('a')]);
      const ctrl = createLibraryController(makeDeps({ albumCatalog }));
      const invalidateInventoryCaches = vi.fn().mockResolvedValue(undefined);
      ctrl.initializeInventory(makeInventorySnapshot('v1', 'scanning'));

      await ctrl.handleInventoryStateChanged(
        makeInventorySnapshot('v1', 'completed'),
        { invalidateInventoryCaches }
      );

      expect(albumCatalog.refresh).toHaveBeenCalledWith({
        forceRemote: false,
        silent: true,
        reason: 'inventory',
      });
    });
  });
});
