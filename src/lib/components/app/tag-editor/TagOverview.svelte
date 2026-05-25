<script lang="ts">
  import TagDimensionRow from './TagDimensionRow.svelte';
  import TagAddDialog from './TagAddDialog.svelte';
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
</script>

<div class="tag-overview">
  {#each merged.tagDimensions as dim (dim.key)}
    <div class="dimension-entry">
      <TagDimensionRow
        dimensionKey={dim.key}
        dimensionLabel={dim.label['zh-CN'] ?? dim.key}
        values={selectedEntityTags[dim.key] ?? []}
        isEditing={editingDimension === dim.key}
        {onSetTag}
        {onRemoveTag}
      />
      <TagAddDialog
        dimensionKey={dim.key}
        dimensionLabel={dim.label['zh-CN'] ?? dim.key}
        values={selectedEntityTags[dim.key] ?? []}
        {tagLibrary}
        {onSetTag}
        {onRemoveTag}
        open={editingDimension === dim.key}
        onOpenChange={(open) => {
          editingDimension = open ? dim.key : null;
        }}
      />
    </div>
  {/each}
</div>

<style>
  .tag-overview {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .dimension-entry {
    display: flex;
    align-items: baseline;
    gap: 0;
  }
</style>
