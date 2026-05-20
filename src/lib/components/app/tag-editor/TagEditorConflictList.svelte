<script lang="ts">
  import TagEditorConflictItem from './TagEditorConflictItem.svelte';
  import type { ConflictResolution, TagEditorMergeConflict } from '$lib/types';

  interface Props {
    conflicts: TagEditorMergeConflict[];
    onResolve: (
      conflict: TagEditorMergeConflict,
      resolution: ConflictResolution
    ) => Promise<void>;
  }

  let { conflicts, onResolve }: Props = $props();
</script>

<section class="conflict-section">
  <h3 class="conflict-heading">冲突列表 ({conflicts.length})</h3>
  <div class="conflict-list">
    {#each conflicts as conflict (`${conflict.entityType}:${conflict.cid}:${conflict.dimensionKey}`)}
      <TagEditorConflictItem {conflict} {onResolve} />
    {/each}
  </div>
</section>

<style>
  .conflict-section {
    padding: 1.5rem;
    border-top: 2px solid var(--color-warning, #f59e0b);
  }

  .conflict-heading {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--color-warning, #d97706);
    margin-bottom: 0.75rem;
    font-family: var(--font-sans);
  }

  .conflict-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
</style>
