<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import type { TagEditorLocalizedValue } from '$lib/types';

  interface Props {
    dimensionKey: string;
    dimensionLabel: string;
    values: TagEditorLocalizedValue[];
    onSetTag: (
      dimensionKey: string,
      values: TagEditorLocalizedValue[]
    ) => Promise<void>;
    onRemoveTag: (dimensionKey: string) => Promise<void>;
  }

  let { dimensionKey, dimensionLabel, values, onSetTag, onRemoveTag }: Props =
    $props();

  let newValueZh = $state('');
  let newValueEn = $state('');

  function displayValue(val: TagEditorLocalizedValue): string {
    return val['zh-CN'] || val['en-US'] || Object.values(val)[0] || '';
  }
  async function handleAdd() {
    if (!newValueZh.trim()) return;
    const newVal: TagEditorLocalizedValue = { 'zh-CN': newValueZh.trim() };
    if (newValueEn.trim()) {
      newVal['en-US'] = newValueEn.trim();
    }
    await onSetTag(dimensionKey, [...values, newVal]);
    newValueZh = '';
    newValueEn = '';
  }

  async function handleRemoveValue(index: number) {
    const updated = values.filter((_, i) => i !== index);
    if (updated.length === 0) {
      await onRemoveTag(dimensionKey);
    } else {
      await onSetTag(dimensionKey, updated);
    }
  }
  function valueKey(val: TagEditorLocalizedValue, idx: number): string {
    return `${idx}-${val['zh-CN'] ?? ''}-${val['en-US'] ?? ''}`;
  }
</script>

<div class="dimension-row">
  <div class="dimension-header">
    <span class="dim-label">{dimensionLabel}</span>
    <span class="dim-key">({dimensionKey})</span>
  </div>

  <div class="dimension-values">
    {#each values as val, idx (valueKey(val, idx))}
      <span class="value-chip">
        {displayValue(val)}
        <button
          type="button"
          class="chip-remove"
          onclick={() => handleRemoveValue(idx)}
          aria-label={m.tag_editor_remove_value_aria()}>×</button
        >
      </span>
    {/each}
  </div>

  <div class="dimension-add">
    <input
      bind:value={newValueZh}
      placeholder={m.tag_editor_placeholder_zh()}
      class="value-input"
    />
    <input
      bind:value={newValueEn}
      placeholder={m.tag_editor_placeholder_en()}
      class="value-input"
    />
    <Button size="sm" onclick={handleAdd}>+</Button>
  </div>
</div>

<style>
  .dimension-row {
    padding: 0.75rem;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .dimension-header {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
  }

  .dim-label {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--color-text-primary, #1f2937);
    font-family: var(--font-body);
  }

  .dim-key {
    font-size: 0.6875rem;
    color: var(--color-text-secondary, #9ca3af);
    font-family: var(--font-mono, monospace);
  }

  .dimension-values {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .value-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.125rem 0.5rem;
    font-size: 0.75rem;
    background: var(--color-chip-bg, #f3f4f6);
    border-radius: 9999px;
    color: var(--color-text-primary, #374151);
    font-family: var(--font-body);
  }

  .chip-remove {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.875rem;
    line-height: 1;
    color: var(--color-text-secondary, #9ca3af);
    padding: 0;
  }

  .chip-remove:hover {
    color: var(--color-danger, #ef4444);
  }

  .dimension-add {
    display: flex;
    gap: 0.25rem;
    align-items: center;
  }

  .value-input {
    flex: 1;
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: 4px;
    font-family: var(--font-body);
  }
</style>
