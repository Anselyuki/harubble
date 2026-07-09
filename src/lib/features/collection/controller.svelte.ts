import type { Collection, CollectionSummary } from '$lib/types';
import * as m from '$lib/paraglide/messages.js';

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
      deps.notifyError(m.collection_error_load_list({ error: message }));
    } finally {
      isLoading = false;
    }
  }

  async function selectCollection(id: string): Promise<void> {
    if (id === selectedCollectionId && selectedCollection) {
      return;
    }
    await loadAndSelect(id);
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
      deps.notifyInfo(m.collection_notify_created());
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(m.collection_error_create({ error: message }));
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
      deps.notifyInfo(m.collection_notify_updated());
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(m.collection_error_update({ error: message }));
    }
  }

  async function handleDelete(id: string): Promise<void> {
    try {
      await deps.deleteCollection(id);
      if (selectedCollectionId === id) {
        deselectCollection();
      }
      await loadCollections();
      deps.notifyInfo(m.collection_notify_deleted());
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(m.collection_error_delete({ error: message }));
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
      deps.notifyInfo(m.collection_notify_song_added());
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(m.collection_error_song_add({ error: message }));
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
      deps.notifyError(m.collection_error_song_remove({ error: message }));
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
      deps.notifyError(m.collection_error_sort({ error: message }));
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
      deps.notifyInfo(m.collection_notify_exported());
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(m.collection_error_export({ error: message }));
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
          reader.onerror = () =>
            reject(new Error(m.collection_error_file_read()));
          reader.readAsText(file);
        };
        input.oncancel = () => resolve(null);
        input.click();
      });
      if (!json) return;
      const imported = await deps.importCollection(json);
      await loadCollections();
      await selectCollection(imported.id);
      deps.notifyInfo(m.collection_notify_imported());
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(m.collection_error_import({ error: message }));
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

  function clearSelection(): void {
    selectedCollectionId = null;
    selectedCollection = null;
  }

  async function loadAndSelect(id: string): Promise<void> {
    selectedCollectionId = id;
    isDetailLoading = true;
    try {
      selectedCollection = await deps.getCollection(id);
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      deps.notifyError(m.collection_error_load_detail({ error: message }));
      selectedCollection = null;
    } finally {
      isDetailLoading = false;
    }
  }

  async function restoreSelection(id: string): Promise<void> {
    await loadAndSelect(id);
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
    clearSelection,
    loadAndSelect,
    restoreSelection,
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
