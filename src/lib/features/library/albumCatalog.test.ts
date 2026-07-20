import { describe, expect, it, vi } from 'vitest';
import type { Album, AlbumCatalogSnapshot } from '$lib/types';
import { createAlbumCatalogController } from './albumCatalog.svelte';

function makeAlbum(cid: string): Album {
  return {
    cid,
    name: `Album ${cid}`,
    coverUrl: `https://example.com/${cid}.jpg`,
    artists: ['Artist'],
    download: {
      isDownloaded: false,
      downloadStatus: 'missing',
      inventoryVersion: '',
    },
    tags: [],
  };
}

function makeSnapshot(
  revision: number,
  checkedAt: number,
  albumCids: string[]
): AlbumCatalogSnapshot {
  return {
    revision,
    checkedAt,
    albums: albumCids.map(makeAlbum),
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

describe('createAlbumCatalogController', () => {
  it('coalesces concurrent freshness and manual refresh requests', async () => {
    const pending = deferred<AlbumCatalogSnapshot>();
    const refreshAlbumCatalog = vi.fn(() => pending.promise);
    const controller = createAlbumCatalogController({
      getAlbumCatalog: vi.fn(),
      refreshAlbumCatalog,
    });

    const freshnessRequest = controller.ensureFresh();
    const manualRequest = controller.refresh({
      forceRemote: true,
      silent: false,
      reason: 'manual',
    });

    expect(refreshAlbumCatalog).toHaveBeenCalledTimes(1);
    pending.resolve(makeSnapshot(1, 100, ['a']));
    await Promise.all([freshnessRequest, manualRequest]);

    expect(controller.albums.map((album) => album.cid)).toEqual(['a']);
  });

  it('joins a forced refresh to the in-flight initial remote load', async () => {
    const cachedResponse = deferred<AlbumCatalogSnapshot>();
    const getAlbumCatalog = vi.fn(() => cachedResponse.promise);
    const refreshAlbumCatalog = vi.fn();
    const controller = createAlbumCatalogController({
      getAlbumCatalog,
      refreshAlbumCatalog,
    });

    const bootstrapRequest = controller.bootstrap();
    const firstForceRequest = controller.refresh({
      forceRemote: true,
      silent: false,
      reason: 'manual',
    });
    const secondForceRequest = controller.refresh({
      forceRemote: true,
      silent: false,
      reason: 'view-enter',
    });
    expect(getAlbumCatalog).toHaveBeenCalledTimes(1);
    expect(refreshAlbumCatalog).not.toHaveBeenCalled();
    cachedResponse.resolve(makeSnapshot(1, 100, ['cached']));
    await Promise.all([
      bootstrapRequest,
      firstForceRequest,
      secondForceRequest,
    ]);

    expect(refreshAlbumCatalog).not.toHaveBeenCalled();
    expect(controller.albums[0]?.cid).toBe('cached');
  });

  it('queues one forced refresh behind a later cached read', async () => {
    const cachedResponse = deferred<AlbumCatalogSnapshot>();
    const remoteResponse = deferred<AlbumCatalogSnapshot>();
    const getAlbumCatalog = vi
      .fn()
      .mockResolvedValueOnce(makeSnapshot(1, 100, ['initial']))
      .mockImplementationOnce(() => cachedResponse.promise);
    const refreshAlbumCatalog = vi.fn(() => remoteResponse.promise);
    const controller = createAlbumCatalogController({
      getAlbumCatalog,
      refreshAlbumCatalog,
    });
    await controller.bootstrap();

    const cachedRequest = controller.refresh({
      forceRemote: false,
      silent: false,
      reason: 'inventory',
    });
    const firstForceRequest = controller.refresh({
      forceRemote: true,
      silent: false,
      reason: 'manual',
    });
    const secondForceRequest = controller.refresh({
      forceRemote: true,
      silent: false,
      reason: 'view-enter',
    });

    expect(getAlbumCatalog).toHaveBeenCalledTimes(2);
    expect(refreshAlbumCatalog).not.toHaveBeenCalled();
    cachedResponse.resolve(makeSnapshot(1, 200, ['cached']));
    await cachedRequest;
    await vi.waitFor(() => {
      expect(refreshAlbumCatalog).toHaveBeenCalledTimes(1);
    });
    remoteResponse.resolve(makeSnapshot(2, 300, ['remote']));
    await Promise.all([firstForceRequest, secondForceRequest]);

    expect(controller.albums[0]?.cid).toBe('remote');
  });

  it('uses checkedAt as the freshness TTL', async () => {
    let now = 1_000;
    const getAlbumCatalog = vi
      .fn()
      .mockResolvedValue(makeSnapshot(1, now, ['a']));
    const refreshAlbumCatalog = vi
      .fn()
      .mockResolvedValue(makeSnapshot(1, now, ['a']));
    const controller = createAlbumCatalogController({
      getAlbumCatalog,
      refreshAlbumCatalog,
      now: () => now,
    });

    await controller.bootstrap();
    now += 299_999;
    await controller.ensureFresh();
    expect(refreshAlbumCatalog).not.toHaveBeenCalled();

    now += 2;
    await controller.ensureFresh();
    expect(refreshAlbumCatalog).toHaveBeenCalledTimes(1);
  });

  it('only advances checkedAt for unchanged and duplicate events', async () => {
    const getAlbumCatalog = vi
      .fn()
      .mockResolvedValue(makeSnapshot(1, 100, ['a']));
    const controller = createAlbumCatalogController({
      getAlbumCatalog,
      refreshAlbumCatalog: vi.fn(),
    });
    await controller.bootstrap();
    getAlbumCatalog.mockClear();

    await controller.handleRefreshedEvent({
      revision: 1,
      checkedAt: 200,
      changed: false,
      albumCount: 1,
    });
    await controller.handleRefreshedEvent({
      revision: 1,
      checkedAt: 300,
      changed: true,
      albumCount: 1,
    });

    expect(controller.lastRemoteSyncAt).toBe(300);
    expect(getAlbumCatalog).not.toHaveBeenCalled();
  });

  it('reads only the cached snapshot when a newer event arrives', async () => {
    const getAlbumCatalog = vi
      .fn()
      .mockResolvedValueOnce(makeSnapshot(1, 100, ['a']))
      .mockResolvedValueOnce(makeSnapshot(2, 200, ['a', 'b']));
    const refreshAlbumCatalog = vi.fn();
    const controller = createAlbumCatalogController({
      getAlbumCatalog,
      refreshAlbumCatalog,
    });
    await controller.bootstrap();

    await controller.handleRefreshedEvent({
      revision: 2,
      checkedAt: 200,
      changed: true,
      albumCount: 2,
    });

    expect(controller.revision).toBe(2);
    expect(controller.albums.map((album) => album.cid)).toEqual(['a', 'b']);
    expect(getAlbumCatalog).toHaveBeenCalledTimes(2);
    expect(refreshAlbumCatalog).not.toHaveBeenCalled();
  });

  it('keeps event-first snapshot reads in the initial loading state', async () => {
    const cachedResponse = deferred<AlbumCatalogSnapshot>();
    const controller = createAlbumCatalogController({
      getAlbumCatalog: vi.fn(() => cachedResponse.promise),
      refreshAlbumCatalog: vi.fn(),
    });

    const eventRequest = controller.handleRefreshedEvent({
      revision: 1,
      checkedAt: 100,
      changed: true,
      albumCount: 1,
    });

    expect(controller.initialLoading).toBe(true);
    expect(controller.refreshing).toBe(false);
    cachedResponse.resolve(makeSnapshot(1, 100, ['a']));
    await eventRequest;
    expect(controller.initialLoading).toBe(false);
  });

  it('re-reads after an older command response when the event arrives first', async () => {
    const oldResponse = deferred<AlbumCatalogSnapshot>();
    const getAlbumCatalog = vi
      .fn()
      .mockImplementationOnce(() => oldResponse.promise)
      .mockResolvedValueOnce(makeSnapshot(2, 200, ['new']));
    const controller = createAlbumCatalogController({
      getAlbumCatalog,
      refreshAlbumCatalog: vi.fn(),
    });

    const bootstrapRequest = controller.bootstrap();
    const eventRequest = controller.handleRefreshedEvent({
      revision: 2,
      checkedAt: 200,
      changed: true,
      albumCount: 1,
    });
    oldResponse.resolve(makeSnapshot(1, 100, ['old']));
    await Promise.all([bootstrapRequest, eventRequest]);

    expect(getAlbumCatalog).toHaveBeenCalledTimes(2);
    expect(controller.revision).toBe(2);
    expect(controller.albums[0]?.cid).toBe('new');
  });

  it('does not replace newer state with an older snapshot', async () => {
    const getAlbumCatalog = vi
      .fn()
      .mockResolvedValueOnce(makeSnapshot(2, 200, ['new']))
      .mockResolvedValueOnce(makeSnapshot(1, 300, ['old']));
    const controller = createAlbumCatalogController({
      getAlbumCatalog,
      refreshAlbumCatalog: vi.fn(),
    });
    await controller.bootstrap();

    await controller.refresh({
      forceRemote: false,
      silent: false,
      reason: 'inventory',
    });

    expect(controller.revision).toBe(2);
    expect(controller.albums[0]?.cid).toBe('new');
    expect(controller.lastRemoteSyncAt).toBe(300);
  });

  it('preserves the albums reference when a snapshot is unchanged', async () => {
    const firstSnapshot = makeSnapshot(1, 100, ['a']);
    const controller = createAlbumCatalogController({
      getAlbumCatalog: vi
        .fn()
        .mockResolvedValueOnce(firstSnapshot)
        .mockResolvedValueOnce(makeSnapshot(1, 200, ['a'])),
      refreshAlbumCatalog: vi.fn(),
    });
    await controller.bootstrap();
    const albums = controller.albums;

    await controller.refresh({
      forceRemote: false,
      silent: false,
      reason: 'inventory',
    });

    expect(controller.albums).toBe(albums);
    expect(controller.lastRemoteSyncAt).toBe(200);
  });

  it('keeps existing albums when a silent background refresh fails', async () => {
    const error = new Error('offline');
    const controller = createAlbumCatalogController({
      getAlbumCatalog: vi.fn().mockResolvedValue(makeSnapshot(1, 100, ['a'])),
      refreshAlbumCatalog: vi.fn().mockRejectedValue(error),
    });
    await controller.bootstrap();

    await expect(controller.ensureFresh(0)).resolves.toBeUndefined();

    expect(controller.albums.map((album) => album.cid)).toEqual(['a']);
    expect(controller.lastError).toBe(error);
    expect(controller.refreshing).toBe(false);
  });
});
