<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import type { TagEditorDimension } from '$lib/types';

  interface Props {
    open: boolean;
    dimensions: TagEditorDimension[];
    onAddDimension: (
      key: string,
      labelZh: string,
      labelEn: string
    ) => Promise<void>;
    onRemoveDimension: (key: string) => Promise<void>;
    onOpenChange: (open: boolean) => void;
  }

  let {
    open,
    dimensions,
    onAddDimension,
    onRemoveDimension,
    onOpenChange,
  }: Props = $props();

  let newDimKey = $state('');
  let newDimZh = $state('');
  let newDimEn = $state('');
  let confirmingDelete = $state<string | null>(null);

  async function handleAdd() {
    if (!newDimKey.trim() || !newDimZh.trim()) return;
    await onAddDimension(newDimKey.trim(), newDimZh.trim(), newDimEn.trim());
    newDimKey = '';
    newDimZh = '';
    newDimEn = '';
  }

  async function handleRemove(key: string) {
    if (confirmingDelete === key) {
      await onRemoveDimension(key);
      confirmingDelete = null;
    } else {
      confirmingDelete = key;
    }
  }
</script>

<Dialog.Root {open} {onOpenChange}>
  <Dialog.Content class="dimension-manage-dialog">
    <Dialog.Header>
      <Dialog.Title>{m.tag_editor_dimension_manage_title()}</Dialog.Title>
    </Dialog.Header>

    <div class="dialog-body">
      <section class="existing-dims">
        <h4 class="sub-heading">{m.tag_editor_dimension_existing()}</h4>
        <ul class="dim-list">
          {#each dimensions as dim (dim.key)}
            <li class="dim-item">
              <span class="dim-item-label">
                {dim.label['zh-CN'] ?? dim.key}
                <span class="dim-item-key">({dim.key})</span>
              </span>
              <button
                type="button"
                class="dim-delete-btn"
                class:confirming={confirmingDelete === dim.key}
                onclick={() => handleRemove(dim.key)}
              >
                {confirmingDelete === dim.key
                  ? m.tag_editor_dimension_delete_confirm({
                      key: dim.label['zh-CN'] ?? dim.key,
                    })
                  : '×'}
              </button>
            </li>
          {/each}
        </ul>
      </section>

      <section class="add-dim">
        <h4 class="sub-heading">{m.tag_editor_dimension_add_new()}</h4>
        <div class="add-form">
          <input bind:value={newDimKey} placeholder="key" class="form-input" />
          <input
            bind:value={newDimZh}
            placeholder={m.tag_editor_placeholder_dim_zh()}
            class="form-input"
          />
          <input
            bind:value={newDimEn}
            placeholder={m.tag_editor_placeholder_dim_en()}
            class="form-input"
          />
          <Button size="sm" onclick={handleAdd}>{m.tag_editor_add()}</Button>
        </div>
      </section>
    </div>

    <Dialog.Footer>
      <Button variant="outline" onclick={() => onOpenChange(false)}
        >{m.tag_editor_close()}</Button
      >
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<style>
  .dialog-body {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    padding: 0.5rem 0;
  }

  .sub-heading {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 0.5rem;
  }

  .dim-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .dim-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.375rem 0.5rem;
    border-radius: 6px;
    font-size: 0.8125rem;
  }

  .dim-item:hover {
    background: var(--hover-bg-elevated);
  }

  .dim-item-label {
    color: var(--text-primary);
  }

  .dim-item-key {
    font-size: 0.6875rem;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    margin-left: 0.25rem;
  }

  .dim-delete-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    font-size: 1rem;
    padding: 0.125rem 0.375rem;
    border-radius: 4px;
  }

  .dim-delete-btn:hover {
    color: var(--color-danger, #ef4444);
  }

  .dim-delete-btn.confirming {
    font-size: 0.6875rem;
    color: var(--color-danger, #ef4444);
    border: 1px solid var(--color-danger, #ef4444);
  }

  .add-form {
    display: flex;
    gap: 0.25rem;
    align-items: center;
  }

  .form-input {
    flex: 1;
    font-size: 0.75rem;
    padding: 0.375rem 0.5rem;
    border: 1px solid var(--color-border, #d1d5db);
    border-radius: 6px;
    background: transparent;
    color: var(--text-primary);
    font-family: var(--font-body);
  }
</style>
