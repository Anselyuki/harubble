import type {
  Album,
  SongEntry,
  TagEditorEntityType,
  TagEditorLocalizedValue,
  TagEditorMergeConflict,
  TagEditorRegistry,
  TagEditorTagSet,
} from '$lib/types';

let merged = $state<TagEditorRegistry | null>(null);
let localOverlay = $state<TagEditorRegistry | null>(null);
let selectedEntityType = $state<TagEditorEntityType>('album');
let selectedCid = $state<string | null>(null);
let conflicts = $state<TagEditorMergeConflict[]>([]);
let loading = $state(false);
let editingAlbum = $state<Album | null>(null);
let editingAlbumSongs = $state<SongEntry[]>([]);
let editingSong = $state<SongEntry | null>(null);
let loadingSongs = $state(false);
let albumSearchQuery = $state('');

function reset() {
  merged = null;
  localOverlay = null;
  selectedEntityType = 'album';
  selectedCid = null;
  conflicts = [];
  loading = false;
  editingAlbum = null;
  editingAlbumSongs = [];
  editingSong = null;
  loadingSongs = false;
  albumSearchQuery = '';
}

export const tagEditorStore = {
  get merged() {
    return merged;
  },
  set merged(value: TagEditorRegistry | null) {
    merged = value;
  },
  get localOverlay() {
    return localOverlay;
  },
  set localOverlay(value: TagEditorRegistry | null) {
    localOverlay = value;
  },
  get selectedEntityType() {
    return selectedEntityType;
  },
  set selectedEntityType(value: TagEditorEntityType) {
    selectedEntityType = value;
  },
  get selectedCid() {
    return selectedCid;
  },
  set selectedCid(value: string | null) {
    selectedCid = value;
  },
  get conflicts() {
    return conflicts;
  },
  set conflicts(value: TagEditorMergeConflict[]) {
    conflicts = value;
  },
  get loading() {
    return loading;
  },
  set loading(value: boolean) {
    loading = value;
  },
  get selectedEntityTags(): Record<string, TagEditorLocalizedValue[]> {
    if (!merged || !selectedCid) return {};
    if (selectedEntityType === 'song') {
      const songEntry: TagEditorTagSet | undefined = (
        merged.songs as Partial<Record<string, TagEditorTagSet>>
      )[selectedCid];
      return songEntry ? songEntry.tags : {};
    }
    const entry = merged.albums.find((a) => a.cid === selectedCid);
    if (!entry) return {};
    const tags: Record<string, TagEditorLocalizedValue[]> = {};
    if (entry.type) {
      const typeDef: TagEditorLocalizedValue | undefined = (
        merged.typeDefinitions as Partial<
          Record<string, TagEditorLocalizedValue>
        >
      )[entry.type];
      if (typeDef) {
        tags['type'] = [typeDef];
      } else {
        tags['type'] = [{ 'zh-CN': entry.type, 'en-US': entry.type }];
      }
    }
    if (entry.faction) tags['faction'] = [entry.faction];
    if (entry.character) tags['character'] = [entry.character];
    // 合并 extra 字段中的自定义维度
    for (const [key, value] of Object.entries(entry)) {
      if (
        key !== 'cid' &&
        key !== 'type' &&
        key !== 'name' &&
        key !== 'releaseDate' &&
        key !== 'faction' &&
        key !== 'character' &&
        Array.isArray(value) &&
        value.length > 0
      ) {
        tags[key] = value as TagEditorLocalizedValue[];
      }
    }
    return tags;
  },
  get editingAlbum() {
    return editingAlbum;
  },
  set editingAlbum(value: Album | null) {
    editingAlbum = value;
  },
  get editingAlbumSongs() {
    return editingAlbumSongs;
  },
  set editingAlbumSongs(value: SongEntry[]) {
    editingAlbumSongs = value;
  },
  get editingSong() {
    return editingSong;
  },
  set editingSong(value: SongEntry | null) {
    editingSong = value;
  },
  get loadingSongs() {
    return loadingSongs;
  },
  set loadingSongs(value: boolean) {
    loadingSongs = value;
  },
  get albumSearchQuery() {
    return albumSearchQuery;
  },
  set albumSearchQuery(value: string) {
    albumSearchQuery = value;
  },
  reset,
};

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    reset();
  });
}
