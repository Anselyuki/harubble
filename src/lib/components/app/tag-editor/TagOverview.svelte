<script lang="ts">
  import TagDimensionRow from './TagDimensionRow.svelte';
  import TagInlineEditor from './TagInlineEditor.svelte';
  import {
    buildTagLibrary,
    type TagLibrary,
  } from '$lib/features/tagEditor/tagLibrary';
  import type { TagEditorLocalizedValue, TagEditorRegistry } from '$lib/types';

  interface Props {
    merged: TagEditorRegistry;
    selectedEntityTags: Record<string, TagEditorLocalizedValue[]>;
    onSetTag: (
      dimensionKey: string,
      values: TagEditorLocalizedValue[]
    ) => Promise<void>;
    onRemoveTag: (dimensionKey: string) => Promise<void>;
  }

  let { merged, selectedEntityTags, onSetTag, onRemoveTag }: Props = $props();

  let editingDimension = $state<string | null>(null);
  let tagLibrary = $derived<TagLibrary>(buildTagLibrary(merged));

  function handleStartEdit(dimensionKey: string) {
    editingDimension = dimensionKey;
  }

  function handleCloseEdit() {
    editingDimension = null;
  }
</script>

<div class="tag-overview">
  {#each merged.tagDimensions as dim (dim.key)}
    <TagDimensionRow
      dimensionKey={dim.key}
      dimensionLabel={dim.label['zh-CN'] ?? dim.key}
      values={selectedEntityTags[dim.key] ?? []}
      isEditing={editingDimension === dim.key}
      onStartEdit={() => handleStartEdit(dim.key)}
    />
    {#if editingDimension === dim.key}
      <TagInlineEditor
        dimensionKey={dim.key}
        values={selectedEntityTags[dim.key] ?? []}
        {tagLibrary}
        {onSetTag}
        {onRemoveTag}
        onClose={handleCloseEdit}
      />
    {/if}
  {/each}
</div>

<style>
  .tag-overview {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
</style>
