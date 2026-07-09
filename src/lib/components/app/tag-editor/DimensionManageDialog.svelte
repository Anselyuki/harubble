<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
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
      <section class="sheet-section existing-dims">
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

      <section class="sheet-section add-dim">
        <h4 class="sub-heading">{m.tag_editor_dimension_add_new()}</h4>
        <div class="add-form">
          <Input
            bind:value={newDimKey}
            placeholder={m.tag_editor_placeholder_dim_key()}
            class="h-9 border-[var(--dialog-border)] bg-[var(--dialog-control-bg)]"
          />
          <Input
            bind:value={newDimZh}
            placeholder={m.tag_editor_placeholder_dim_zh()}
            class="h-9 border-[var(--dialog-border)] bg-[var(--dialog-control-bg)]"
          />
          <Input
            bind:value={newDimEn}
            placeholder={m.tag_editor_placeholder_dim_en()}
            class="h-9 border-[var(--dialog-border)] bg-[var(--dialog-control-bg)]"
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
  /* .dialog-body 由全局 .app-dialog .dialog-body 处理，移除局部定义以避免覆盖 */

  .sub-heading {
    margin: 0 0 8px;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 700;
  }

  .dim-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
  }

  .dim-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px;
    border-radius: 6px;
    font-size: 13px;
  }

  .dim-item + .dim-item {
    border-top: 1px solid var(--dialog-border);
  }

  .dim-item:hover {
    background: var(--dialog-row-hover-bg);
  }

  .dim-item-label {
    color: var(--text-primary);
  }

  .dim-item-key {
    font-size: 11px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    margin-left: 4px;
  }

  .dim-delete-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    font-size: 16px;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: inherit;
  }

  .dim-delete-btn:hover {
    color: var(--destructive);
  }

  .dim-delete-btn.confirming {
    font-size: 11px;
    color: var(--destructive);
    border: 1px solid var(--destructive);
  }

  .add-form {
    display: flex;
    gap: 6px;
    align-items: center;
  }
</style>
