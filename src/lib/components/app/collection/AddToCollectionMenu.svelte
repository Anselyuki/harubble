<script lang="ts">
  import type { CollectionSummary } from '$lib/types';
  import FolderPlusIcon from '@lucide/svelte/icons/folder-plus';
  import { Popover } from 'bits-ui';
  import * as m from '$lib/paraglide/messages.js';

  interface Props {
    collections: CollectionSummary[];
    onAdd: (collectionId: string) => void;
  }

  let { collections, onAdd }: Props = $props();

  let open = $state(false);

  const userCollections = $derived.by(() =>
    collections.filter((c) => !c.isOfficial)
  );

  function stopRowInteraction(event: MouseEvent) {
    event.stopPropagation();
  }

  function handleSelect(id: string) {
    onAdd(id);
    open = false;
  }
</script>

<span class="add-to-collection-wrapper">
  <Popover.Root bind:open>
    <Popover.Trigger
      class="add-to-collection-btn"
      title={m.collection_menu_add_aria()}
      aria-label={m.collection_menu_add_aria()}
      aria-haspopup="menu"
      onclick={stopRowInteraction}
    >
      <FolderPlusIcon size={14} />
    </Popover.Trigger>

    <Popover.Portal>
      <Popover.Content
        class="add-to-collection-menu"
        side="bottom"
        align="end"
        sideOffset={4}
        collisionPadding={16}
        strategy="fixed"
        onclick={stopRowInteraction}
      >
        <div role="menu" aria-label={m.collection_menu_add_aria()}>
          {#if userCollections.length === 0}
            <div class="menu-empty">{m.collection_menu_empty()}</div>
          {:else}
            {#each userCollections as col (col.id)}
              <button
                type="button"
                class="menu-item"
                role="menuitem"
                onclick={() => handleSelect(col.id)}
              >
                <span class="menu-item-name">{col.name}</span>
                <span class="menu-item-count">{col.songCount}</span>
              </button>
            {/each}
          {/if}
        </div>
      </Popover.Content>
    </Popover.Portal>
  </Popover.Root>
</span>

<style>
  .add-to-collection-wrapper {
    display: inline-flex;
  }

  :global(.add-to-collection-btn) {
    appearance: none;
    border: none;
    background: none;
    color: var(--text-tertiary);
    cursor: pointer;
    width: 40px;
    height: 40px;
    padding: 4px;
    border-radius: var(--shape-sm);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  :global(.add-to-collection-btn:hover) {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.08);
  }

  :global(.add-to-collection-menu) {
    z-index: var(--z-popover);
    min-width: 160px;
    max-width: 220px;
    max-height: min(200px, calc(100vh - 32px));
    overflow-y: auto;
    padding: 4px;
    border-radius: 10px;
    background: var(--surface-elevated, rgba(30, 41, 59, 0.98));
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    transform-origin: var(--bits-popover-content-transform-origin);
  }

  .menu-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    min-height: 40px;
    padding: 6px 10px;
    border: none;
    border-radius: var(--shape-sm);
    background: none;
    color: var(--text-secondary);
    font-family: var(--font-body);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    text-align: left;
  }

  .menu-item:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-primary);
  }

  .menu-item-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .menu-item-count {
    font-size: 11px;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .menu-empty {
    padding: 12px 10px;
    font-size: 12px;
    color: var(--text-tertiary);
    text-align: center;
  }
</style>
