<script lang="ts">
  interface Props {
    songId: string;
    index: number;
    draggable: boolean;
    onDragStart: (event: DragEvent, index: number) => void;
    onDragOver: (event: DragEvent, index: number) => void;
    onDragEnd: () => void;
    onDrop: (event: DragEvent, index: number) => void;
    onRemove: (songId: string) => void;
  }

  let {
    songId,
    index,
    draggable,
    onDragStart,
    onDragOver,
    onDragEnd,
    onDrop,
    onRemove,
  }: Props = $props();

  let isDragOver = $state(false);
</script>

<div
  class="collection-song-row"
  class:is-drag-over={isDragOver}
  role="listitem"
  draggable={draggable ? 'true' : undefined}
  ondragstart={(e) => {
    if (draggable) onDragStart(e, index);
  }}
  ondragover={(e) => {
    e.preventDefault();
    isDragOver = true;
    onDragOver(e, index);
  }}
  ondragleave={() => {
    isDragOver = false;
  }}
  ondrop={(e) => {
    e.preventDefault();
    isDragOver = false;
    onDrop(e, index);
  }}
  ondragend={onDragEnd}
>
  {#if draggable}
    <span class="drag-handle" aria-hidden="true">⠿</span>
  {/if}
  <span class="song-index">{index + 1}</span>
  <span class="song-id">{songId}</span>
  {#if draggable}
    <button
      type="button"
      class="remove-btn"
      title="移除"
      aria-label="从合集中移除"
      onclick={(e) => {
        e.stopPropagation();
        onRemove(songId);
      }}
    >
      ✕
    </button>
  {/if}
</div>

<style>
  .collection-song-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    border-radius: 10px;
    background: rgba(15, 23, 42, 0);
    transition: background-color 0.12s ease;
    cursor: default;
  }

  .collection-song-row.is-drag-over {
    background: rgba(var(--accent-rgb), 0.08);
  }

  .collection-song-row[draggable='true'] {
    cursor: grab;
  }

  .collection-song-row[draggable='true']:active {
    cursor: grabbing;
  }

  .drag-handle {
    font-size: 14px;
    color: var(--text-tertiary);
    cursor: grab;
    user-select: none;
    flex-shrink: 0;
  }

  .drag-handle:active {
    cursor: grabbing;
  }

  .song-index {
    width: 28px;
    text-align: center;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .song-id {
    flex: 1;
    min-width: 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .remove-btn {
    appearance: none;
    border: none;
    background: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 6px;
    font-size: 12px;
    opacity: 0;
    transition:
      opacity 0.12s ease,
      color 0.12s ease,
      background-color 0.12s ease;
  }

  .collection-song-row:hover .remove-btn {
    opacity: 1;
  }

  .remove-btn:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.08);
  }
</style>
