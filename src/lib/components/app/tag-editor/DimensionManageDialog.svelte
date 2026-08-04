<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import * as AlertDialog from '$lib/components/ui/alert-dialog/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import type { TagEditorDimension } from '$lib/types';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';

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
  let pendingDeleteDimension = $state<TagEditorDimension | null>(null);
  let isDeleting = $state(false);

  async function handleAdd() {
    if (!newDimKey.trim() || !newDimZh.trim()) return;
    await onAddDimension(newDimKey.trim(), newDimZh.trim(), newDimEn.trim());
    newDimKey = '';
    newDimZh = '';
    newDimEn = '';
  }

  function handleDeleteDialogOpenChange(nextOpen: boolean) {
    if (!nextOpen && !isDeleting) pendingDeleteDimension = null;
  }

  function handleManageDialogOpenChange(nextOpen: boolean) {
    if (!nextOpen) pendingDeleteDimension = null;
    onOpenChange(nextOpen);
  }

  async function handleRemove() {
    if (!pendingDeleteDimension || isDeleting) return;
    isDeleting = true;
    try {
      await onRemoveDimension(pendingDeleteDimension.key);
      pendingDeleteDimension = null;
    } finally {
      isDeleting = false;
    }
  }
</script>

<Dialog.Root {open} onOpenChange={handleManageDialogOpenChange}>
  <Dialog.Content class="dimension-manage-dialog">
    <Dialog.Header>
      <Dialog.Title>{m.tag_editor_dimension_manage_title()}</Dialog.Title>
    </Dialog.Header>

    <div class="dialog-body">
      <section class="sheet-section existing-dims">
        <h3 class="sub-heading">{m.tag_editor_dimension_existing()}</h3>
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
                aria-label={m.tag_editor_remove_dimension_aria({
                  key: dim.label['zh-CN'] ?? dim.key,
                })}
                onclick={() => (pendingDeleteDimension = dim)}
              >
                <Trash2Icon aria-hidden="true" />
              </button>
            </li>
          {/each}
        </ul>
      </section>

      <section class="sheet-section add-dim">
        <h3 class="sub-heading">{m.tag_editor_dimension_add_new()}</h3>
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

<AlertDialog.Root
  open={pendingDeleteDimension !== null}
  onOpenChange={handleDeleteDialogOpenChange}
>
  <AlertDialog.Content>
    <AlertDialog.Header>
      <AlertDialog.Title>
        {m.tag_editor_remove_dimension_aria({
          key:
            pendingDeleteDimension?.label['zh-CN'] ??
            pendingDeleteDimension?.key ??
            '',
        })}
      </AlertDialog.Title>
      <AlertDialog.Description>
        {m.tag_editor_dimension_delete_confirm({
          key:
            pendingDeleteDimension?.label['zh-CN'] ??
            pendingDeleteDimension?.key ??
            '',
        })}
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel disabled={isDeleting}
        >{m.tag_editor_cancel()}</AlertDialog.Cancel
      >
      <AlertDialog.Action
        variant="destructive"
        disabled={isDeleting}
        onclick={() => void handleRemove()}
      >
        {m.tag_editor_dimension_delete_action()}
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>

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
    border-radius: var(--shape-sm);
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
    width: 40px;
    height: 40px;
    padding: 0;
    border-radius: var(--shape-xs);
    font-family: inherit;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .dim-delete-btn :global(svg) {
    width: 16px;
    height: 16px;
  }

  .dim-delete-btn:hover {
    color: var(--destructive);
  }

  .add-form {
    display: flex;
    gap: 6px;
    align-items: center;
  }
</style>
