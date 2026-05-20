import { getContext, setContext } from 'svelte';
import type { Collection, CollectionSummary } from '$lib/types';
import { COLLECTION_CONTEXT_KEY } from './keys';

export interface CollectionContext {
  readonly collections: CollectionSummary[];
  readonly selectedCollectionId: string | null;
  readonly selectedCollection: Collection | null;
  readonly isLoading: boolean;
  readonly isDetailLoading: boolean;
  readonly formDialogOpen: boolean;
  readonly formDialogMode: 'create' | 'edit';
  selectCollection: (id: string) => void;
  openCreateDialog: () => void;
  openEditDialog: () => void;
  closeFormDialog: () => void;
  handleCreate: (name: string, description: string) => Promise<void>;
  handleUpdate: (
    id: string,
    name: string,
    description: string
  ) => Promise<void>;
  handleDelete: () => Promise<void>;
  handleExport: () => Promise<void>;
  handleRemoveSongs: (songCids: string[]) => Promise<void>;
  handleReorderSongs: (songCids: string[]) => Promise<void>;
  handleAddSongs: (collectionId: string, songCids: string[]) => Promise<void>;
  loadCollections: () => Promise<void>;
}

export function setCollectionContext(ctx: CollectionContext): void {
  setContext(COLLECTION_CONTEXT_KEY, ctx);
}

export function getCollectionContext(): CollectionContext {
  return getContext<CollectionContext>(COLLECTION_CONTEXT_KEY);
}
