import { invoke } from '@tauri-apps/api/core';
import type { Collection, CollectionSummary } from './types';

export async function listCollections(): Promise<CollectionSummary[]> {
  return invoke<CollectionSummary[]>('list_collections');
}

export async function getCollection(id: string): Promise<Collection> {
  return invoke<Collection>('get_collection', { id });
}

export async function createCollection(
  name: string,
  description: string,
  coverPath?: string | null
): Promise<Collection> {
  return invoke<Collection>('create_collection', {
    name,
    description,
    coverPath: coverPath ?? null,
  });
}

export async function updateCollection(
  id: string,
  name?: string | null,
  description?: string | null,
  coverPath?: string | null | undefined
): Promise<Collection> {
  return invoke<Collection>('update_collection', {
    id,
    name: name ?? null,
    description: description ?? null,
    coverPath: coverPath === undefined ? null : coverPath,
  });
}

export async function deleteCollection(id: string): Promise<void> {
  return invoke<void>('delete_collection', { id });
}

export async function addSongsToCollection(
  id: string,
  songIds: string[]
): Promise<void> {
  return invoke<void>('add_songs_to_collection', { id, songIds });
}

export async function removeSongsFromCollection(
  id: string,
  songIds: string[]
): Promise<void> {
  return invoke<void>('remove_songs_from_collection', { id, songIds });
}

export async function reorderCollectionSongs(
  id: string,
  songIds: string[]
): Promise<void> {
  return invoke<void>('reorder_collection_songs', { id, songIds });
}

export async function exportCollection(id: string): Promise<string> {
  return invoke<string>('export_collection', { id });
}

export async function importCollection(json: string): Promise<Collection> {
  return invoke<Collection>('import_collection', { json });
}
