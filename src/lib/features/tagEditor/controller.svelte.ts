import type {
  Album,
  AlbumDetail,
  SongEntry,
  ConflictResolution,
  TagEditorEntityType,
  TagEditorLocalizedValue,
  TagEditorMergeConflict,
  TagEditorMergeResult,
  TagEditorRegistry,
} from '$lib/types';
import { open, save } from '@tauri-apps/plugin-dialog';
import { tagEditorStore } from './store.svelte';

interface TagEditorControllerDeps {
  getTagEditorMerged: () => Promise<TagEditorRegistry>;
  getTagEditorLocalOverlay: () => Promise<TagEditorRegistry>;
  setTagEditorEntityTag: (
    entityType: TagEditorEntityType,
    cid: string,
    dimensionKey: string,
    values: TagEditorLocalizedValue[]
  ) => Promise<void>;
  removeTagEditorEntityTag: (
    entityType: TagEditorEntityType,
    cid: string,
    dimensionKey: string
  ) => Promise<void>;
  addTagEditorDimension: (
    key: string,
    labelZh: string,
    labelEn: string
  ) => Promise<void>;
  removeTagEditorDimension: (key: string) => Promise<void>;
  applyTagEditorRemoteUpdate: (
    newRemote: TagEditorRegistry
  ) => Promise<TagEditorMergeResult>;
  resolveTagEditorConflict: (
    entityType: TagEditorEntityType,
    cid: string,
    dimensionKey: string,
    keep: ConflictResolution
  ) => Promise<void>;
  exportTagEditorRegistry: (path: string) => Promise<void>;
  importTagEditorRegistry: (path: string) => Promise<TagEditorMergeResult>;
  getAlbumDetail: (albumCid: string) => Promise<AlbumDetail>;
  getAlbums: () => Album[];
  notifyError: (message: string) => void;
}

export function createTagEditorController(deps: TagEditorControllerDeps) {
  let loadSeq = 0;

  async function loadData() {
    const seq = ++loadSeq;
    tagEditorStore.loading = true;

    try {
      const [mergedResult, overlayResult] = await Promise.all([
        deps.getTagEditorMerged(),
        deps.getTagEditorLocalOverlay(),
      ]);
      if (seq !== loadSeq) return;
      tagEditorStore.merged = mergedResult;
      tagEditorStore.localOverlay = overlayResult;
    } catch (e: unknown) {
      deps.notifyError(
        `加载 Tag 编辑器数据失败: ${e instanceof Error ? e.message : String(e)}`
      );
    } finally {
      if (seq === loadSeq) {
        tagEditorStore.loading = false;
      }
    }
  }

  function selectEntity(entityType: TagEditorEntityType, cid: string) {
    tagEditorStore.selectedEntityType = entityType;
    tagEditorStore.selectedCid = cid;
  }

  async function setTag(
    dimensionKey: string,
    values: TagEditorLocalizedValue[]
  ) {
    const { selectedEntityType, selectedCid } = tagEditorStore;
    if (!selectedCid) return;

    try {
      await deps.setTagEditorEntityTag(
        selectedEntityType,
        selectedCid,
        dimensionKey,
        values
      );
      await loadData();
    } catch (e: unknown) {
      deps.notifyError(
        `设置 Tag 失败: ${e instanceof Error ? e.message : String(e)}`
      );
    }
  }

  async function removeTag(dimensionKey: string) {
    const { selectedEntityType, selectedCid } = tagEditorStore;
    if (!selectedCid) return;

    try {
      await deps.removeTagEditorEntityTag(
        selectedEntityType,
        selectedCid,
        dimensionKey
      );
      await loadData();
    } catch (e: unknown) {
      deps.notifyError(
        `删除 Tag 失败: ${e instanceof Error ? e.message : String(e)}`
      );
    }
  }

  async function addDimension(key: string, labelZh: string, labelEn: string) {
    try {
      await deps.addTagEditorDimension(key, labelZh, labelEn);
      await loadData();
    } catch (e: unknown) {
      deps.notifyError(
        `新增维度失败: ${e instanceof Error ? e.message : String(e)}`
      );
    }
  }

  async function removeDimension(key: string) {
    try {
      await deps.removeTagEditorDimension(key);
      await loadData();
    } catch (e: unknown) {
      deps.notifyError(
        `删除维度失败: ${e instanceof Error ? e.message : String(e)}`
      );
    }
  }

  async function resolveConflict(
    conflict: TagEditorMergeConflict,
    resolution: ConflictResolution
  ) {
    try {
      await deps.resolveTagEditorConflict(
        conflict.entityType,
        conflict.cid,
        conflict.dimensionKey,
        resolution
      );
      tagEditorStore.conflicts = tagEditorStore.conflicts.filter(
        (c) =>
          c.cid !== conflict.cid ||
          c.dimensionKey !== conflict.dimensionKey ||
          c.entityType !== conflict.entityType
      );
      await loadData();
    } catch (e: unknown) {
      deps.notifyError(
        `解决冲突失败: ${e instanceof Error ? e.message : String(e)}`
      );
    }
  }

  async function selectAlbumForEdit(album: Album) {
    tagEditorStore.editingAlbum = album;
    tagEditorStore.editingSong = null;
    tagEditorStore.selectedEntityType = 'album';
    tagEditorStore.selectedCid = album.cid;
    tagEditorStore.loadingSongs = true;

    try {
      const detail = await deps.getAlbumDetail(album.cid);
      tagEditorStore.editingAlbumSongs = detail.songs;
    } catch (e: unknown) {
      deps.notifyError(
        `加载专辑歌曲失败: ${e instanceof Error ? e.message : String(e)}`
      );
      tagEditorStore.editingAlbumSongs = [];
    } finally {
      tagEditorStore.loadingSongs = false;
    }
  }

  function selectSongForEdit(song: SongEntry) {
    tagEditorStore.editingSong = song;
    tagEditorStore.selectedEntityType = 'song';
    tagEditorStore.selectedCid = song.cid;
  }

  function backToAlbum() {
    tagEditorStore.editingSong = null;
    if (tagEditorStore.editingAlbum) {
      tagEditorStore.selectedEntityType = 'album';
      tagEditorStore.selectedCid = tagEditorStore.editingAlbum.cid;
    }
  }

  function setAlbumSearchQuery(query: string) {
    tagEditorStore.albumSearchQuery = query;
  }

  async function exportRegistry() {
    const path = await save({
      defaultPath: 'tag_registry.json',
      filters: [{ name: 'JSON', extensions: ['json'] }],
    });
    if (!path) return;

    try {
      await deps.exportTagEditorRegistry(path);
    } catch (e: unknown) {
      deps.notifyError(
        `导出失败: ${e instanceof Error ? e.message : String(e)}`
      );
    }
  }

  async function importRegistry() {
    const path = await open({
      filters: [{ name: 'JSON', extensions: ['json'] }],
    });
    if (!path) return;

    try {
      const result = await deps.importTagEditorRegistry(path);
      if (result.conflicts.length > 0) {
        tagEditorStore.conflicts = result.conflicts;
      }
      await loadData();
    } catch (e: unknown) {
      deps.notifyError(
        `导入失败: ${e instanceof Error ? e.message : String(e)}`
      );
    }
  }

  const filteredAlbums = $derived.by(() => {
    const query = tagEditorStore.albumSearchQuery.trim().toLowerCase();
    const allAlbums = deps.getAlbums();
    if (!query) return allAlbums;
    return allAlbums.filter(
      (album) =>
        album.name.toLowerCase().includes(query) ||
        album.artists.some((a) => a.toLowerCase().includes(query))
    );
  });

  function dispose() {
    loadSeq += 1;
    tagEditorStore.reset();
  }

  return {
    get merged() {
      return tagEditorStore.merged;
    },
    get localOverlay() {
      return tagEditorStore.localOverlay;
    },
    get selectedEntityType() {
      return tagEditorStore.selectedEntityType;
    },
    get selectedCid() {
      return tagEditorStore.selectedCid;
    },
    get selectedEntityTags() {
      return tagEditorStore.selectedEntityTags;
    },
    get conflicts() {
      return tagEditorStore.conflicts;
    },
    get loading() {
      return tagEditorStore.loading;
    },
    get editingAlbum() {
      return tagEditorStore.editingAlbum;
    },
    get editingAlbumSongs() {
      return tagEditorStore.editingAlbumSongs;
    },
    get editingSong() {
      return tagEditorStore.editingSong;
    },
    get loadingSongs() {
      return tagEditorStore.loadingSongs;
    },
    get albumSearchQuery() {
      return tagEditorStore.albumSearchQuery;
    },
    get filteredAlbums() {
      return filteredAlbums;
    },
    setAlbumSearchQuery,
    loadData,
    selectEntity,
    selectAlbumForEdit,
    selectSongForEdit,
    backToAlbum,
    setTag,
    removeTag,
    addDimension,
    removeDimension,
    resolveConflict,
    exportRegistry,
    importRegistry,
    dispose,
  };
}

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    tagEditorStore.reset();
  });
}
