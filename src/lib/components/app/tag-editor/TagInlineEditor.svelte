<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { animateIn, killTweens } from '$lib/design/gsap';
  import type { TagEditorLocalizedValue } from '$lib/types';
  import {
    filterCandidates,
    type TagLibrary,
  } from '$lib/features/tagEditor/tagLibrary';

  interface Props {
    dimensionKey: string;
    values: TagEditorLocalizedValue[];
    tagLibrary: TagLibrary;
    onSetTag: (
      dimensionKey: string,
      values: TagEditorLocalizedValue[]
    ) => Promise<void>;
    onRemoveTag: (dimensionKey: string) => Promise<void>;
    onClose: () => void;
  }

  let {
    dimensionKey,
    values,
    tagLibrary,
    onSetTag,
    onRemoveTag,
    onClose,
  }: Props = $props();

  let query = $state('');
  let showEnInput = $state(false);
  let pendingZhValue = $state('');
  let enValue = $state('');
  let containerEl: HTMLDivElement | undefined = $state();

  let candidates = $derived(
    filterCandidates(tagLibrary, dimensionKey, values, query)
  );
  let isNewValue = $derived(
    query.trim() !== '' &&
      !candidates.some(
        (c) => (c['zh-CN'] ?? '').toLowerCase() === query.trim().toLowerCase()
      )
  );

  function displayValue(val: TagEditorLocalizedValue): string {
    return val['zh-CN'] || val['en-US'] || Object.values(val)[0] || '';
  }

  async function handleSelectCandidate(val: TagEditorLocalizedValue) {
    await onSetTag(dimensionKey, [...values, val]);
    query = '';
  }

  async function handleCreateNew() {
    if (!query.trim()) return;
    pendingZhValue = query.trim();
    showEnInput = true;
  }

  async function handleConfirmCreate() {
    const newVal: TagEditorLocalizedValue = { 'zh-CN': pendingZhValue };
    if (enValue.trim()) {
      newVal['en-US'] = enValue.trim();
    }
    await onSetTag(dimensionKey, [...values, newVal]);
    query = '';
    pendingZhValue = '';
    enValue = '';
    showEnInput = false;
  }

  function handleSkipEn() {
    const newVal: TagEditorLocalizedValue = { 'zh-CN': pendingZhValue };
    void onSetTag(dimensionKey, [...values, newVal]);
    query = '';
    pendingZhValue = '';
    showEnInput = false;
  }

  async function handleRemoveValue(index: number) {
    const updated = values.filter((_, i) => i !== index);
    if (updated.length === 0) {
      await onRemoveTag(dimensionKey);
    } else {
      await onSetTag(dimensionKey, updated);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && query.trim()) {
      if (isNewValue) {
        void handleCreateNew();
      } else if (candidates.length > 0) {
        void handleSelectCandidate(candidates[0]);
      }
    }
    if (e.key === 'Escape') {
      onClose();
    }
  }

  function handleClickOutside(e: MouseEvent) {
    if (containerEl && !containerEl.contains(e.target as Node)) {
      onClose();
    }
  }

  $effect(() => {
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  });

  $effect(() => {
    if (containerEl) {
      killTweens(containerEl);
      animateIn(
        containerEl,
        { height: 0, opacity: 0 },
        { height: 'auto', opacity: 1 },
        250,
        'ios-spring'
      );
    }
  });
</script>

<div class="inline-editor" bind:this={containerEl}>
  <div class="selected-tags">
    {#each values as val, idx (`${idx}-${val['zh-CN'] ?? ''}`)}
      <span class="selected-chip">
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

  {#if showEnInput}
    <div class="en-input-row">
      <input
        bind:value={enValue}
        placeholder={m.tag_editor_en_optional()}
        class="editor-input"
        onkeydown={(e) => {
          if (e.key === 'Enter') void handleConfirmCreate();
          if (e.key === 'Escape') handleSkipEn();
        }}
      />
      <button type="button" class="confirm-btn" onclick={handleConfirmCreate}
        >OK</button
      >
      <button type="button" class="skip-btn" onclick={handleSkipEn}
        >{m.tag_editor_cancel()}</button
      >
    </div>
  {:else}
    <input
      bind:value={query}
      placeholder={m.tag_editor_add_tag_placeholder()}
      class="editor-input"
      onkeydown={handleKeydown}
    />

    <div class="candidates">
      {#if isNewValue}
        <button
          type="button"
          class="candidate-item create-new"
          onclick={handleCreateNew}
        >
          {m.tag_editor_create_new({ value: query.trim() })}
        </button>
      {/if}
      {#if candidates.length > 0}
        <div class="candidates-label">{m.tag_editor_existing_tags()}</div>
        <div class="candidates-list">
          {#each candidates as val (`${val['zh-CN'] ?? ''}-${val['en-US'] ?? ''}`)}
            <button
              type="button"
              class="candidate-chip"
              onclick={() => handleSelectCandidate(val)}
              >{displayValue(val)}</button
            >
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .inline-editor {
    padding: 0.75rem;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 8px;
    background: var(--bg-secondary, rgba(255, 255, 255, 0.02));
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    overflow: hidden;
  }

  .selected-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .selected-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.125rem 0.5rem;
    font-size: 0.75rem;
    background: var(--color-chip-bg, #f3f4f6);
    border-radius: 9999px;
    color: var(--text-primary);
    font-family: var(--font-body);
  }

  .chip-remove {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.875rem;
    line-height: 1;
    color: var(--text-secondary);
    padding: 0;
  }

  .chip-remove:hover {
    color: var(--color-danger, #ef4444);
  }

  .editor-input {
    width: 100%;
    font-size: 0.8125rem;
    padding: 0.375rem 0.5rem;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: 6px;
    background: transparent;
    color: var(--text-primary);
    font-family: var(--font-body);
  }

  .editor-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .candidates {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .candidates-label {
    font-size: 0.6875rem;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .candidates-list {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
  }

  .candidate-chip {
    display: inline-flex;
    align-items: center;
    padding: 0.125rem 0.5rem;
    font-size: 0.75rem;
    background: transparent;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: 9999px;
    color: var(--text-primary);
    cursor: pointer;
    font-family: var(--font-body);
  }

  .candidate-chip:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .candidate-item.create-new {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.375rem 0.5rem;
    font-size: 0.75rem;
    background: transparent;
    border: 1px dashed var(--accent);
    border-radius: 6px;
    color: var(--accent);
    cursor: pointer;
    font-family: var(--font-body);
  }

  .en-input-row {
    display: flex;
    gap: 0.25rem;
    align-items: center;
  }

  .en-input-row .editor-input {
    flex: 1;
  }

  .confirm-btn,
  .skip-btn {
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    cursor: pointer;
    border: none;
  }

  .confirm-btn {
    background: var(--accent);
    color: white;
  }

  .skip-btn {
    background: transparent;
    border: 1px solid var(--color-border, #d1d5db);
    color: var(--text-secondary);
  }
</style>
