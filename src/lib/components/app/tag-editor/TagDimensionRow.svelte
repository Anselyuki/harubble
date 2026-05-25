<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import type { TagEditorLocalizedValue } from '$lib/types';

  interface Props {
    dimensionKey: string;
    dimensionLabel: string;
    values: TagEditorLocalizedValue[];
    isEditing: boolean;
    onStartEdit: () => void;
  }

  let {
    dimensionKey: _dimensionKey,
    dimensionLabel,
    values,
    isEditing,
    onStartEdit,
  }: Props = $props();

  function displayValue(val: TagEditorLocalizedValue): string {
    return val['zh-CN'] || val['en-US'] || Object.values(val)[0] || '';
  }

  function tooltipValue(val: TagEditorLocalizedValue): string | null {
    const en = val['en-US'];
    const zh = val['zh-CN'];
    if (en && zh && en !== zh) return en;
    return null;
  }
</script>

<div
  class="dimension-row"
  class:editing={isEditing}
  role="group"
  aria-label={dimensionLabel}
>
  <span class="dim-label">{dimensionLabel}</span>

  <div class="dim-chips">
    {#if values.length === 0}
      <button type="button" class="add-placeholder" onclick={onStartEdit}>
        + {m.tag_editor_add_tag_button()}
      </button>
    {:else}
      {#each values as val, idx (`${idx}-${val['zh-CN'] ?? ''}`)}
        {@const tip = tooltipValue(val)}
        {#if tip}
          <Tooltip.Root>
            <Tooltip.Trigger>
              <button type="button" class="value-chip" onclick={onStartEdit}
                >{displayValue(val)}</button
              >
            </Tooltip.Trigger>
            <Tooltip.Content>{tip}</Tooltip.Content>
          </Tooltip.Root>
        {:else}
          <button type="button" class="value-chip" onclick={onStartEdit}
            >{displayValue(val)}</button
          >
        {/if}
      {/each}
      <button
        type="button"
        class="add-btn"
        onclick={onStartEdit}
        aria-label={m.tag_editor_add_tag_button()}>+</button
      >
    {/if}
  </div>
</div>

<style>
  .dimension-row {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    padding: 0.375rem 0;
  }

  .dim-label {
    flex-shrink: 0;
    width: 60px;
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-family: var(--font-body);
  }

  .dim-chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.375rem;
    flex: 1;
    min-width: 0;
  }

  .value-chip {
    display: inline-flex;
    align-items: center;
    padding: 0.125rem 0.5rem;
    font-size: 0.75rem;
    background: var(--color-chip-bg, #f3f4f6);
    border: none;
    border-radius: 9999px;
    color: var(--text-primary);
    font-family: var(--font-body);
    cursor: pointer;
    transition: none;
  }

  .value-chip:hover {
    background: var(--color-chip-bg-hover, #e5e7eb);
  }

  .add-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    font-size: 0.75rem;
    border: 1px dashed var(--color-border, #d1d5db);
    border-radius: 9999px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .add-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .add-placeholder {
    display: inline-flex;
    align-items: center;
    padding: 0.125rem 0.5rem;
    font-size: 0.75rem;
    border: 1px dashed var(--color-border, #d1d5db);
    border-radius: 9999px;
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
    font-family: var(--font-body);
  }

  .add-placeholder:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
