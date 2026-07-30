<script lang="ts">
  import * as Tooltip from '$lib/components/ui/tooltip/index.js';
  import { gsap, getMotionDuration, MOTION } from '$lib/design/gsap';
  import type { TagEditorLocalizedValue } from '$lib/types';
  import * as m from '$lib/paraglide/messages.js';

  interface Props {
    dimensionKey: string;
    dimensionLabel: string;
    values: TagEditorLocalizedValue[];
    isEditing: boolean;
    onSetTag: (
      dimensionKey: string,
      values: TagEditorLocalizedValue[]
    ) => Promise<void>;
    onRemoveTag: (dimensionKey: string) => Promise<void>;
  }

  let {
    dimensionKey,
    dimensionLabel,
    values,
    isEditing,
    onSetTag,
    onRemoveTag,
  }: Props = $props();

  let dragIdx = $state<number | null>(null);
  let dropIdx = $state<number | null>(null);

  function displayValue(val: TagEditorLocalizedValue): string {
    return val['zh-CN'] || val['en-US'] || Object.values(val)[0] || '';
  }

  function tooltipValue(val: TagEditorLocalizedValue): string | null {
    const en = val['en-US'];
    const zh = val['zh-CN'];
    if (en && zh && en !== zh) return en;
    return null;
  }

  async function handleRemoveValue(index: number) {
    const updated = values.filter((_, i) => i !== index);
    if (updated.length === 0) {
      await onRemoveTag(dimensionKey);
    } else {
      await onSetTag(dimensionKey, updated);
    }
  }

  function handleChipEnter(e: Event) {
    if (!window.matchMedia('(hover: hover) and (pointer: fine)').matches)
      return;
    const wrapper = e.currentTarget as HTMLElement;
    const btn = wrapper.querySelector<HTMLElement>('.chip-delete');
    if (!btn) return;
    gsap.killTweensOf(btn);
    gsap.to(btn, {
      width: 40,
      minWidth: 40,
      opacity: 1,
      marginLeft: 2,
      duration: getMotionDuration(MOTION.FAST),
      ease: 'ios-out',
    });
  }

  function handleChipLeave(e: Event) {
    if (!window.matchMedia('(hover: hover) and (pointer: fine)').matches)
      return;
    const wrapper = e.currentTarget as HTMLElement;
    const btn = wrapper.querySelector<HTMLElement>('.chip-delete');
    if (!btn) return;
    gsap.killTweensOf(btn);
    gsap.to(btn, {
      width: 0,
      minWidth: 0,
      opacity: 0,
      marginLeft: 0,
      duration: getMotionDuration(MOTION.MICRO),
      ease: 'ios-in',
    });
  }

  function handleDragStart(e: DragEvent, idx: number) {
    dragIdx = idx;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', String(idx));
    }
  }

  function handleDragOver(e: DragEvent, idx: number) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    dropIdx = idx;
  }

  function handleDragLeave() {
    dropIdx = null;
  }

  async function handleDrop(e: DragEvent, targetIdx: number) {
    e.preventDefault();
    dropIdx = null;
    if (dragIdx === null || dragIdx === targetIdx) {
      dragIdx = null;
      return;
    }
    const reordered = [...values];
    const [moved] = reordered.splice(dragIdx, 1);
    reordered.splice(targetIdx, 0, moved);
    dragIdx = null;
    await onSetTag(dimensionKey, reordered);
  }

  function handleDragEnd() {
    dragIdx = null;
    dropIdx = null;
  }
</script>

<div
  class="dimension-row"
  class:editing={isEditing}
  role="group"
  aria-label={dimensionLabel}
>
  <span class="dim-label">{dimensionLabel}</span>

  <div class="dim-chips" role="list">
    {#if values.length > 0}
      {#each values as val, idx (`${dimensionKey}-${idx}-${val['zh-CN'] ?? ''}`)}
        {@const tip = tooltipValue(val)}
        <span
          class="chip-wrapper"
          class:dragging={dragIdx === idx}
          class:drop-target={dropIdx === idx && dragIdx !== idx}
          draggable="true"
          role="listitem"
          onmouseenter={handleChipEnter}
          onmouseleave={handleChipLeave}
          onfocusin={handleChipEnter}
          onfocusout={(e) => {
            const next = (e as FocusEvent).relatedTarget as Node | null;
            if (!next || !e.currentTarget.contains(next)) handleChipLeave(e);
          }}
          ondragstart={(e) => handleDragStart(e, idx)}
          ondragover={(e) => handleDragOver(e, idx)}
          ondragleave={handleDragLeave}
          ondrop={(e) => handleDrop(e, idx)}
          ondragend={handleDragEnd}
        >
          {#if tip}
            <Tooltip.Root>
              <Tooltip.Trigger>
                <span class="value-chip">{displayValue(val)}</span>
              </Tooltip.Trigger>
              <Tooltip.Content>{tip}</Tooltip.Content>
            </Tooltip.Root>
          {:else}
            <span class="value-chip">{displayValue(val)}</span>
          {/if}
          <button
            type="button"
            class="chip-delete"
            onclick={() => handleRemoveValue(idx)}
            aria-label={m.tag_editor_remove_tag_aria()}>×</button
          >
        </span>
      {/each}
    {/if}
  </div>
</div>

<style>
  .dimension-row {
    display: flex;
    align-items: baseline;
    gap: 0.75rem;
    padding: 0.375rem 0;
    flex: 1;
    min-width: 0;
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

  .chip-wrapper {
    display: inline-flex;
    align-items: center;
    border-radius: var(--shape-pill);
    cursor: grab;
  }

  .chip-wrapper:active {
    cursor: grabbing;
  }

  .chip-wrapper.dragging {
    opacity: 0.4;
  }

  .chip-wrapper.drop-target {
    box-shadow: -2px 0 0 0 var(--accent);
  }

  .chip-delete {
    width: 40px;
    min-width: 40px;
    height: 40px;
    opacity: 1;
    overflow: hidden;
    margin-left: 2px;
    border: none;
    background: none;
    color: var(--text-tertiary);
    font-size: 0.75rem;
    line-height: 1;
    cursor: pointer;
    padding: 0;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .chip-delete:hover {
    color: var(--color-danger, #ef4444);
  }

  .chip-delete:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  @media (hover: hover) and (pointer: fine) {
    .chip-delete {
      width: 0;
      min-width: 0;
      opacity: 0;
      margin-left: 0;
    }

    .chip-wrapper:hover .chip-delete,
    .chip-wrapper:focus-within .chip-delete {
      width: 40px;
      min-width: 40px;
      opacity: 1;
      margin-left: 2px;
    }
  }

  .value-chip {
    display: inline-flex;
    align-items: center;
    padding: 0.125rem 0.5rem;
    font-size: 0.75rem;
    background: var(--color-chip-bg, #f3f4f6);
    border: none;
    border-radius: var(--shape-pill);
    color: var(--text-primary);
    font-family: var(--font-body);
    pointer-events: none;
  }
</style>
