import type { AlbumDetail, PlaybackQueueEntry } from '$lib/types';

export function getSelectedAlbumCoverUrl(
  album: AlbumDetail | null
): string | null {
  return album?.coverUrl ?? null;
}

export function getPreferredAlbumArtworkUrl(
  album: AlbumDetail | null
): string | null {
  return album?.coverDeUrl ?? album?.coverUrl ?? null;
}

export function buildAlbumPlaybackEntries(
  album: AlbumDetail | null
): PlaybackQueueEntry[] {
  if (!album) return [];

  const coverUrl = album.coverUrl;
  return album.songs.map((entry) => ({
    cid: entry.cid,
    name: entry.name,
    artists: entry.artists,
    coverUrl,
  }));
}
