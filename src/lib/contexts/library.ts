import { getContext, setContext } from 'svelte';
import type { Album, AlbumDetail } from '$lib/types';
import { LIBRARY_CONTEXT_KEY } from './keys';

export interface LibraryContext {
  readonly albums: Album[];
  readonly selectedAlbum: AlbumDetail | null;
  readonly selectedAlbumCid: string | null;
  readonly loadingAlbums: boolean;
  readonly loadingDetail: boolean;
  readonly errorMsg: string | null;
  readonly showDetailSkeleton: boolean;
  readonly albumRequestSeq: number;
  readonly selectedAlbumArtworkUrl: string | null;
  readonly selectionModeEnabled: boolean;
  readonly selectedSongCids: string[];
  handleSelectAlbum: (album: Album) => void | Promise<void>;
  toggleSelectionMode: () => void;
  selectAllSongs: () => void;
  deselectAllSongs: () => void;
  invertSongSelection: () => void;
  toggleSongSelection: (songCid: string) => void;
  isSongSelected: (songCid: string) => boolean;
}

export function setLibraryContext(ctx: LibraryContext): void {
  setContext(LIBRARY_CONTEXT_KEY, ctx);
}

export function getLibraryContext(): LibraryContext {
  return getContext<LibraryContext>(LIBRARY_CONTEXT_KEY);
}
