<script lang="ts">
  import { Button } from '$lib/components/ui/button/index.js';
  import * as m from '$lib/paraglide/messages.js';
  import type { ConflictResolution, TagEditorMergeConflict } from '$lib/types';

  interface Props {
    conflict: TagEditorMergeConflict;
    onResolve: (
      conflict: TagEditorMergeConflict,
      resolution: ConflictResolution
    ) => Promise<void>;
  }

  let { conflict, onResolve }: Props = $props();

  function formatValues(
    values: Record<string, string | undefined>[] | null
  ): string {
    if (!values || values.length === 0) return m.tag_editor_conflict_empty();
    return values
      .map((v) => v['zh-CN'] || v['en-US'] || Object.values(v)[0] || '')
      .join(', ');
  }
</script>

<div class="conflict-item">
  <div class="conflict-meta">
    <span class="conflict-entity"
      >{conflict.entityType === 'album'
        ? m.tag_editor_conflict_entity_album()
        : m.tag_editor_conflict_entity_song()}: {conflict.cid}</span
    >
    <span class="conflict-dim"
      >{m.tag_editor_conflict_dimension()}: {conflict.dimensionKey}</span
    >
  </div>

  <div class="conflict-values">
    <div class="value-row">
      <span class="value-label">{m.tag_editor_conflict_base()}</span>
      <span class="value-text">{formatValues(conflict.baseValues)}</span>
    </div>
    <div class="value-row">
      <span class="value-label">{m.tag_editor_conflict_remote()}</span>
      <span class="value-text remote"
        >{formatValues(conflict.remoteValues)}</span
      >
    </div>
    <div class="value-row">
      <span class="value-label">{m.tag_editor_conflict_local()}</span>
      <span class="value-text local">{formatValues(conflict.localValues)}</span>
    </div>
  </div>

  <div class="conflict-actions">
    <Button size="sm" onclick={() => onResolve(conflict, 'keepLocal')}
      >{m.tag_editor_conflict_keep_local()}</Button
    >
    <Button
      size="sm"
      variant="outline"
      onclick={() => onResolve(conflict, 'keepRemote')}
      >{m.tag_editor_conflict_keep_remote()}</Button
    >
  </div>
</div>

<style>
  .conflict-item {
    padding: 0.75rem;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .conflict-meta {
    display: flex;
    gap: 1rem;
    font-size: 0.75rem;
    font-family: var(--font-mono, monospace);
    color: var(--color-text-secondary, #6b7280);
  }

  .conflict-values {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .value-row {
    display: flex;
    gap: 0.5rem;
    font-size: 0.75rem;
    font-family: var(--font-body);
  }

  .value-label {
    width: 3rem;
    flex-shrink: 0;
    color: var(--color-text-secondary, #9ca3af);
  }

  .value-text {
    color: var(--color-text-primary, #374151);
  }

  .value-text.remote {
    color: var(--color-info, #2563eb);
  }

  .value-text.local {
    color: var(--color-success, #16a34a);
  }

  .conflict-actions {
    display: flex;
    gap: 0.5rem;
    padding-top: 0.25rem;
  }
</style>
