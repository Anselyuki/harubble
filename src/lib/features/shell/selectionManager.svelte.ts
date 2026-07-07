import type { AlbumDetail, SongEntry } from '$lib/types';

interface SelectionManagerDeps {
  getSelectedAlbum: () => AlbumDetail | null;
}

export function createSelectionManager(deps: SelectionManagerDeps) {
  let selectedSongCids = $state<string[]>([]);
  let selectionModeEnabled = $state(false);

  function isSongSelected(songCid: string): boolean {
    return selectedSongCids.includes(songCid);
  }

  function toggleSongSelection(songCid: string) {
    if (selectedSongCids.includes(songCid)) {
      selectedSongCids = selectedSongCids.filter((cid) => cid !== songCid);
      return;
    }
    selectedSongCids = [...selectedSongCids, songCid];
  }

  function clearSongSelection() {
    selectedSongCids = [];
  }

  function selectAllSongs() {
    const album = deps.getSelectedAlbum();
    if (!album) return;
    selectedSongCids = album.songs.map((s: SongEntry) => s.cid);
  }

  function deselectAllSongs() {
    selectedSongCids = [];
  }

  function invertSongSelection() {
    const album = deps.getSelectedAlbum();
    if (!album) return;
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- ephemeral, non-reactive lookup
    const allCids = new Set(album.songs.map((s: SongEntry) => s.cid));
    // eslint-disable-next-line svelte/prefer-svelte-reactivity -- ephemeral, non-reactive lookup
    const currentSelected = new Set(selectedSongCids);
    selectedSongCids = [...allCids].filter((cid) => !currentSelected.has(cid));
  }

  function toggleSelectionMode() {
    selectionModeEnabled = !selectionModeEnabled;
    if (!selectionModeEnabled) {
      clearSongSelection();
    }
  }

  function setSelectionModeEnabled(value: boolean) {
    selectionModeEnabled = value;
  }

  return {
    get selectedSongCids() {
      return selectedSongCids;
    },
    get selectionModeEnabled() {
      return selectionModeEnabled;
    },
    isSongSelected,
    toggleSongSelection,
    clearSongSelection,
    selectAllSongs,
    deselectAllSongs,
    invertSongSelection,
    toggleSelectionMode,
    setSelectionModeEnabled,
  };
}

export type SelectionManager = ReturnType<typeof createSelectionManager>;
