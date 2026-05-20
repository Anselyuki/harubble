<script lang="ts">
  import { untrack } from 'svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import TagEditorPanel from './TagEditorPanel.svelte';
  import TagEditorSongPanel from './TagEditorSongPanel.svelte';
  import TagEditorConflictList from './TagEditorConflictList.svelte';
  import type { PartialOptions } from 'overlayscrollbars';
  import type {
    Album,
    SongEntry,
    ConflictResolution,
    TagEditorEntityType,
    TagEditorLocalizedValue,
    TagEditorMergeConflict,
    TagEditorRegistry,
  } from '$lib/types';

  interface Props {
    runtime: {
      tagEditorController: {
        merged: TagEditorRegistry | null;
        localOverlay: TagEditorRegistry | null;
        selectedEntityType: TagEditorEntityType;
        selectedCid: string | null;
        selectedEntityTags: Record<string, TagEditorLocalizedValue[]>;
        editingAlbum: Album | null;
        editingAlbumSongs: SongEntry[];
        editingSong: SongEntry | null;
        loadingSongs: boolean;
        conflicts: TagEditorMergeConflict[];
        loading: boolean;
        loadData: () => Promise<void>;
        selectEntity: (type: TagEditorEntityType, cid: string) => void;
        selectAlbumForEdit: (album: Album) => Promise<void>;
        selectSongForEdit: (song: SongEntry) => void;
        backToAlbum: () => void;
        setTag: (
          dimensionKey: string,
          values: TagEditorLocalizedValue[]
        ) => Promise<void>;
        removeTag: (dimensionKey: string) => Promise<void>;
        addDimension: (
          key: string,
          labelZh: string,
          labelEn: string
        ) => Promise<void>;
        removeDimension: (key: string) => Promise<void>;
        resolveConflict: (
          conflict: TagEditorMergeConflict,
          resolution: ConflictResolution
        ) => Promise<void>;
      };
      overlayScrollbarOptions: PartialOptions;
    };
  }

  let { runtime }: Props = $props();

  const controller = $derived(runtime.tagEditorController);

  $effect(() => {
    untrack(() => {
      void controller.loadData();
    });
  });
</script>

<div class="tag-editor-view">
  <OverlayScrollbarsComponent
    class="tag-editor-scroll"
    options={runtime.overlayScrollbarOptions}
    defer
  >
    {#if controller.loading && !controller.merged}
      <div class="tag-editor-loading">
        <p>{m.tag_editor_loading()}</p>
      </div>
    {:else if !controller.editingAlbum}
      <div class="tag-editor-empty">
        <p>{m.tag_editor_select_album_hint()}</p>
      </div>
    {:else if controller.editingSong}
      <TagEditorSongPanel
        song={controller.editingSong}
        merged={controller.merged}
        selectedEntityTags={controller.selectedEntityTags}
        onSetTag={controller.setTag}
        onRemoveTag={controller.removeTag}
        onBack={controller.backToAlbum}
      />
    {:else}
      <TagEditorPanel
        album={controller.editingAlbum}
        songs={controller.editingAlbumSongs}
        loadingSongs={controller.loadingSongs}
        merged={controller.merged}
        selectedEntityTags={controller.selectedEntityTags}
        onSetTag={controller.setTag}
        onRemoveTag={controller.removeTag}
        onSelectSong={controller.selectSongForEdit}
        onAddDimension={controller.addDimension}
        onRemoveDimension={controller.removeDimension}
      />
    {/if}

    {#if controller.conflicts.length > 0}
      <TagEditorConflictList
        conflicts={controller.conflicts}
        onResolve={controller.resolveConflict}
      />
    {/if}
  </OverlayScrollbarsComponent>
</div>

<style>
  .tag-editor-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .tag-editor-view :global(.tag-editor-scroll) {
    flex: 1;
  }

  .tag-editor-loading,
  .tag-editor-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 4rem;
    color: var(--color-text-secondary, #6b7280);
    font-family: var(--font-body);
  }
</style>
