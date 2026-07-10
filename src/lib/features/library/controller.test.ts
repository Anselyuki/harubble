import { describe, expect, it, vi } from 'vitest';
import type { Album, AlbumDetail, LocalInventorySnapshot } from '$lib/types';
import { createLibraryController } from './controller.svelte';

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
    getAlbums: vi.fn().mockResolvedValue([]),
    getAlbumDetail: vi.fn().mockResolvedValue(makeAlbumDetail('a')),
    searchLibrary: vi.fn().mockResolvedValue({
      items: [],
      total: 0,
      query: '',
      scope: 'all',
      indexState: 'ready',
    }),
    preloadAlbumArtwork: vi.fn().mockResolvedValue(null),
    warmAlbumArtwork: vi.fn(),
    setAlbumStageAspectRatio: vi.fn(),
    notifyError: vi.fn(),
    ...overrides,
  };
}

describe('createLibraryController', () => {
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
      const getAlbumDetail = vi.fn().mockResolvedValue(refreshedDetail);
      const deps = makeDeps({ getAlbumDetail });
      const ctrl = createLibraryController(deps);

      await ctrl.selectAlbum(makeAlbum('a'));
      const callCountAfterSelect = getAlbumDetail.mock.calls.length;

      await ctrl.replaceAlbumsAndRefreshCurrentSelection([makeAlbum('a')]);

      expect(getAlbumDetail.mock.calls.length).toBeGreaterThan(
        callCountAfterSelect
      );
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
  });
});
