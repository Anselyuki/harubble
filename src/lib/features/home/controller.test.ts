import { afterEach, describe, expect, it, vi } from 'vitest';
import type { Album } from '$lib/types';
import { createHomeController } from './controller.svelte';

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

function makeCatalog(initialAlbums: Album[]) {
  let albums = initialAlbums;
  return {
    get albums() {
      return albums;
    },
    initialLoading: false,
    bootstrap: vi.fn(async () => albums),
    refresh: vi.fn(async () => albums),
    setAlbums(nextAlbums: Album[]) {
      albums = nextAlbums;
    },
  };
}

function makeDeps(
  overrides: Partial<Parameters<typeof createHomeController>[0]> = {}
) {
  return {
    albumCatalog: makeCatalog([]),
    getAlbumsBySeriesGroup: vi.fn().mockResolvedValue([]),
    getRecentHistory: vi.fn().mockResolvedValue([]),
    getHomepageStatus: vi.fn().mockResolvedValue({}),
    clearListeningHistory: vi.fn().mockResolvedValue(0),
    getTagDimensions: vi.fn().mockResolvedValue([]),
    getAlbumsByTagDimension: vi.fn().mockResolvedValue([]),
    notifyError: vi.fn(),
    ...overrides,
  };
}

let controllerToDispose: ReturnType<typeof createHomeController> | null = null;

afterEach(() => {
  controllerToDispose?.dispose();
  controllerToDispose = null;
});

describe('createHomeController', () => {
  it('derives the latest twelve albums from the shared catalog', () => {
    const albums = Array.from({ length: 14 }, (_, index) =>
      makeAlbum(String(index))
    );
    const albumCatalog = makeCatalog(albums);
    const controller = createHomeController(makeDeps({ albumCatalog }));
    controllerToDispose = controller;

    expect(controller.latestAlbums.map((album) => album.cid)).toEqual(
      albums.slice(0, 12).map((album) => album.cid)
    );

    albumCatalog.setAlbums([makeAlbum('new'), ...albums]);
    expect(controller.latestAlbums[0]?.cid).toBe('new');
    expect(controller.latestAlbums).toHaveLength(12);
  });

  it('uses a forced catalog refresh for a full homepage refresh', async () => {
    const albumCatalog = makeCatalog([]);
    const controller = createHomeController(makeDeps({ albumCatalog }));
    controllerToDispose = controller;

    await controller.refreshHomepage();

    expect(albumCatalog.refresh).toHaveBeenCalledWith({
      forceRemote: true,
      silent: false,
      reason: 'manual',
    });
    expect(albumCatalog.bootstrap).not.toHaveBeenCalled();
  });
});
