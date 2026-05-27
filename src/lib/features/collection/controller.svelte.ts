import type { Collection, CollectionSummary } from '$lib/types';

interface CollectionControllerDeps {
  listCollections: () => Promise<CollectionSummary[]>;
  getCollection: (id: string) => Promise<Collection>;
  createCollection: (
    name: string,
    description: string,
    coverPath?: string | null
  ) => Promise<Collection>;
  updateCollection: (
    id: string,
    name?: string | null,
    description?: string | null,
    coverPath?: string | null | undefined
  ) => Promise<Collection>;
  deleteCollection: (id: string) => Promise<void>;
  addSongsToCollection: (id: string, songIds: string[]) => Promise<void>;
  removeSongsFromCollection: (id: string, songIds: string[]) => Promise<void>;
  reorderCollectionSongs: (id: string, songIds: string[]) => Promise<void>;
  exportCollection: (id: string) => Promise<string>;
  importCollection: (json: string) => Promise<Collection>;
  navigateToCollection: () => void;
  notifyInfo: (message: string) => void;
  notifyError: (message: string) => void;
}

export function createCollectionController(deps: CollectionControllerDeps) {
  let collections = $state<CollectionSummary[]>([]);
  let selectedCollectionId = $state<string | null>(null);
  let selectedCollection = $state<Collection | null>(null);
  let isLoading = $state(false);
  let isDetailLoading = $state(false);
  let formDialogOpen = $state(false);
  let formDialogMode = $state<'create' | 'edit'>('create');

  async function loadCollections(): Promise<void> {
    isLoading = true;
    try {
      collections = await deps.listCollections();
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(`加载合集列表失败: ${message}`);
    } finally {
      isLoading = false;
    }
  }

  async function selectCollection(id: string): Promise<void> {
    if (id === selectedCollectionId && selectedCollection) {
      deps.navigateToCollection();
      return;
    }

    selectedCollectionId = id;
    isDetailLoading = true;
    deps.navigateToCollection();

    try {
      selectedCollection = await deps.getCollection(id);
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(`加载合集详情失败: ${message}`);
      selectedCollection = null;
    } finally {
      isDetailLoading = false;
    }
  }

  function deselectCollection(): void {
    selectedCollectionId = null;
    selectedCollection = null;
  }

  async function handleCreate(
    name: string,
    description: string,
    coverPath?: string | null
  ): Promise<void> {
    try {
      const created = await deps.createCollection(name, description, coverPath);
      await loadCollections();
      await selectCollection(created.id);
      deps.notifyInfo('合集创建成功');
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(`创建合集失败: ${message}`);
    }
  }

  async function handleUpdate(
    id: string,
    name?: string | null,
    description?: string | null,
    coverPath?: string | null | undefined
  ): Promise<void> {
    try {
      const updated = await deps.updateCollection(
        id,
        name,
        description,
        coverPath
      );
      await loadCollections();
      if (selectedCollectionId === id) {
        selectedCollection = updated;
      }
      deps.notifyInfo('合集已更新');
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(`更新合集失败: ${message}`);
    }
  }

  async function handleDelete(id: string): Promise<void> {
    try {
      await deps.deleteCollection(id);
      if (selectedCollectionId === id) {
        deselectCollection();
      }
      await loadCollections();
      deps.notifyInfo('合集已删除');
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(`删除合集失败: ${message}`);
    }
  }

  async function handleAddSongs(
    collectionId: string,
    songIds: string[]
  ): Promise<void> {
    try {
      await deps.addSongsToCollection(collectionId, songIds);
      if (selectedCollectionId === collectionId) {
        selectedCollection = await deps.getCollection(collectionId);
      }
      await loadCollections();
      deps.notifyInfo('歌曲已添加到合集');
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(`添加歌曲失败: ${message}`);
    }
  }

  async function handleRemoveSongs(
    collectionId: string,
    songIds: string[]
  ): Promise<void> {
    try {
      await deps.removeSongsFromCollection(collectionId, songIds);
      if (selectedCollectionId === collectionId) {
        selectedCollection = await deps.getCollection(collectionId);
      }
      await loadCollections();
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(`移除歌曲失败: ${message}`);
    }
  }

  async function handleReorderSongs(
    collectionId: string,
    songIds: string[]
  ): Promise<void> {
    try {
      await deps.reorderCollectionSongs(collectionId, songIds);
      if (selectedCollectionId === collectionId) {
        selectedCollection = await deps.getCollection(collectionId);
      }
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(`排序失败: ${message}`);
    }
  }

  async function handleExport(id: string): Promise<void> {
    try {
      const json = await deps.exportCollection(id);
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = 'collection.json';
      anchor.click();
      URL.revokeObjectURL(url);
      deps.notifyInfo('合集已导出');
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(`导出失败: ${message}`);
    }
  }

  async function handleImport(): Promise<void> {
    try {
      const json = await new Promise<string | null>((resolve, reject) => {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = '.json,application/json';
        input.onchange = () => {
          const file = input.files?.[0];
          if (!file) {
            resolve(null);
            return;
          }
          const reader = new FileReader();
          reader.onload = () => resolve(reader.result as string);
          reader.onerror = () => reject(new Error('文件读取失败'));
          reader.readAsText(file);
        };
        input.oncancel = () => resolve(null);
        input.click();
      });
      if (!json) return;
      const imported = await deps.importCollection(json);
      await loadCollections();
      await selectCollection(imported.id);
      deps.notifyInfo('合集已导入');
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(`导入失败: ${message}`);
    }
  }

  function openCreateDialog(): void {
    formDialogMode = 'create';
    formDialogOpen = true;
  }

  function openEditDialog(): void {
    formDialogMode = 'edit';
    formDialogOpen = true;
  }

  function closeFormDialog(): void {
    formDialogOpen = false;
  }

  function dispose(): void {
    collections = [];
    selectedCollectionId = null;
    selectedCollection = null;
    isLoading = false;
    isDetailLoading = false;
    formDialogOpen = false;
  }

  return {
    get collections() {
      return collections;
    },
    get selectedCollectionId() {
      return selectedCollectionId;
    },
    get selectedCollection() {
      return selectedCollection;
    },
    get isLoading() {
      return isLoading;
    },
    get isDetailLoading() {
      return isDetailLoading;
    },
    get formDialogOpen() {
      return formDialogOpen;
    },
    set formDialogOpen(value: boolean) {
      formDialogOpen = value;
    },
    get formDialogMode() {
      return formDialogMode;
    },
    loadCollections,
    selectCollection,
    deselectCollection,
    /** 临时存根，将在 Task 5 中替换为真实实现 */
    clearSelection() {},
    handleCreate,
    handleUpdate,
    handleDelete,
    handleAddSongs,
    handleRemoveSongs,
    handleReorderSongs,
    handleExport,
    handleImport,
    openCreateDialog,
    openEditDialog,
    closeFormDialog,
    dispose,
  };
}
