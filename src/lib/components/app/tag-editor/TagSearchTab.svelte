<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import type { TagEditorLocalizedValue } from '$lib/types';
  import type { TagLocale } from './tagAddUtils';
  import { displayValue, tagIdentity } from './tagAddUtils';

  interface Props {
    searchQuery: string;
    searchResults: TagEditorLocalizedValue[];
    values: TagEditorLocalizedValue[];
    tagLocales: TagLocale[];
    editingTag: TagEditorLocalizedValue | null;
    onSelect: (val: TagEditorLocalizedValue) => void;
    onEditSave: (
      originalTag: TagEditorLocalizedValue,
      editValues: Record<string, string>
    ) => Promise<void> | void;
    onEditAndAdd: (editValues: Record<string, string>) => Promise<void> | void;
  }

  let {
    searchQuery = $bindable(),
    searchResults,
    values,
    tagLocales,
    editingTag = $bindable(null),
    onSelect,
    onEditSave,
    onEditAndAdd,
  }: Props = $props();

  let editValues = $state<Record<string, string>>({});

  function isAlreadyAdded(val: TagEditorLocalizedValue): boolean {
    const id = tagIdentity(val);
    return values.some((v) => tagIdentity(v) === id);
  }

  function startEdit(val: TagEditorLocalizedValue) {
    editingTag = val;
    editValues = {};
    for (const loc of tagLocales) {
      editValues[loc.key] = val[loc.key] ?? '';
    }
  }

  function cancelEdit() {
    editingTag = null;
    editValues = {};
  }

  async function handleSave() {
    if (!editingTag) return;
    const hasAnyValue = tagLocales.some((loc) => editValues[loc.key].trim());
    if (!hasAnyValue) return;
    await onEditSave(editingTag, { ...editValues });
    cancelEdit();
  }

  async function handleAddSave() {
    const hasAnyValue = tagLocales.some((loc) => editValues[loc.key].trim());
    if (!hasAnyValue) return;
    await onEditAndAdd({ ...editValues });
    cancelEdit();
  }
</script>

<input
  bind:value={searchQuery}
  placeholder={m.tag_editor_search_placeholder()}
  class="form-input search-input"
/>

<div class="candidates">
  {#if searchResults.length > 0}
    <div class="candidates-list">
      {#each searchResults as val (`${val['zh-CN'] ?? ''}-${val['en-US'] ?? ''}`)}
        {#if editingTag && tagIdentity(editingTag) === tagIdentity(val)}
          <div class="edit-row">
            {#each tagLocales as loc (loc.key)}
              <div class="locale-field">
                <span class="locale-label">{loc.label}</span>
                <input
                  value={editValues[loc.key] ?? ''}
                  oninput={(e) => {
                    editValues[loc.key] = (e.target as HTMLInputElement).value;
                  }}
                  class="form-input edit-input"
                  onkeydown={(e) => {
                    if (e.key === 'Enter') {
                      e.stopPropagation();
                      if (isAlreadyAdded(val)) void handleSave();
                      else void handleAddSave();
                    }
                  }}
                />
              </div>
            {/each}
            <div class="edit-actions">
              {#if isAlreadyAdded(val)}
                <Button size="xs" variant="secondary" onclick={handleSave}
                  >{m.tag_editor_edit_save()}</Button
                >
              {:else}
                <Button size="xs" variant="secondary" onclick={handleAddSave}
                  >{m.tag_editor_edit_add()}</Button
                >
              {/if}
              <Button size="xs" variant="ghost" onclick={cancelEdit}
                >{m.tag_editor_cancel()}</Button
              >
            </div>
          </div>
        {:else}
          <div
            class="candidate-chip"
            class:already-added={isAlreadyAdded(val)}
            role="button"
            tabindex="0"
            onclick={() => onSelect(val)}
            onkeydown={(e) => {
              if (e.key === 'Enter') {
                e.stopPropagation();
                onSelect(val);
              }
            }}
          >
            <span class="chip-text">
              {displayValue(val)}
              {#if val['en-US'] && val['en-US'] !== val['zh-CN']}
                <span class="chip-en">{val['en-US']}</span>
              {/if}
            </span>
            <button
              type="button"
              class="edit-btn-inset"
              onclick={(e) => {
                e.stopPropagation();
                startEdit(val);
              }}
              onkeydown={(e) => {
                if (e.key === 'Enter') {
                  e.stopPropagation();
                  startEdit(val);
                }
              }}
              aria-label={m.tag_editor_edit_i18n()}
              title={m.tag_editor_edit_i18n()}>✎</button
            >
          </div>
        {/if}
      {/each}
    </div>
  {:else if searchQuery.trim()}
    <div class="no-results">{m.tag_editor_no_results()}</div>
  {/if}
</div>

<style>
  .form-input {
    width: 100%;
    padding: 0.375rem 0.5rem;
    font-size: 0.75rem;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: 6px;
    background: var(--bg-input, transparent);
    color: var(--text-primary);
    outline: none;
    font-family: var(--font-body);
  }

  .form-input:focus {
    border-color: var(--color-primary, #6366f1);
  }

  .candidates {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    max-height: 180px;
    overflow-y: auto;
  }

  .candidates-list {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .candidate-chip {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.375rem 0.25rem 0.5rem;
    font-size: 0.75rem;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: 9999px;
    background: transparent;
    color: var(--text-primary);
    cursor: pointer;
    font-family: var(--font-body);
    text-align: left;
  }

  .candidate-chip:hover:not(.already-added) {
    border-color: var(--color-primary, #6366f1);
    color: var(--color-primary, #6366f1);
  }

  .candidate-chip.already-added {
    opacity: 0.5;
    cursor: default;
  }

  .chip-text {
    flex: 1;
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
  }

  .chip-en {
    font-size: 0.6875rem;
    color: var(--text-secondary);
  }

  .edit-btn-inset {
    flex-shrink: 0;
    width: 1rem;
    height: 1rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.625rem;
    border: none;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 50%;
    opacity: 0;
  }

  .candidate-chip:hover .edit-btn-inset {
    opacity: 1;
  }

  .edit-btn-inset:hover {
    color: var(--color-primary, #6366f1);
    background: var(--color-chip-bg, #f3f4f6);
  }

  .edit-row {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    padding: 0.375rem;
    border: 1px solid var(--color-primary, #6366f1);
    border-radius: 6px;
    background: var(--color-chip-bg, #f3f4f6);
  }

  .edit-input {
    padding: 0.25rem 0.375rem;
  }

  .edit-actions {
    display: flex;
    gap: 0.25rem;
    justify-content: flex-end;
  }

  .locale-field {
    display: flex;
    align-items: center;
    gap: 0.375rem;
  }

  .locale-label {
    flex-shrink: 0;
    font-size: 0.6875rem;
    font-weight: 600;
    color: var(--text-primary);
    min-width: 2rem;
    text-align: right;
  }

  .no-results {
    font-size: 0.75rem;
    color: var(--text-primary);
    padding: 0.25rem 0;
  }
</style>
