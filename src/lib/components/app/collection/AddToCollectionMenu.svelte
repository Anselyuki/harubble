<script lang="ts">
  import type { CollectionSummary } from '$lib/types';
  import FolderPlusIcon from '@lucide/svelte/icons/folder-plus';

  interface Props {
    collections: CollectionSummary[];
    onAdd: (collectionId: string) => void;
  }

  let { collections, onAdd }: Props = $props();

  let open = $state(false);
  let menuRef = $state<HTMLDivElement | null>(null);

  const userCollections = $derived.by(() =>
    collections.filter((c) => !c.isOfficial)
  );

  function toggle(event: MouseEvent) {
    event.stopPropagation();
    open = !open;
  }

  function handleSelect(id: string) {
    onAdd(id);
    open = false;
  }

  function handleClickOutside(event: MouseEvent) {
    if (menuRef && !menuRef.contains(event.target as Node)) {
      open = false;
    }
  }

  $effect(() => {
    if (open) {
      document.addEventListener('click', handleClickOutside, true);
      return () => {
        document.removeEventListener('click', handleClickOutside, true);
      };
    }
  });
</script>

<div class="add-to-collection-wrapper" bind:this={menuRef}>
  <button
    type="button"
    class="add-to-collection-btn"
    title="添加到合集"
    aria-label="添加到合集"
    aria-expanded={open}
    onclick={toggle}
  >
    <FolderPlusIcon size={14} />
  </button>

  {#if open}
    <div class="add-to-collection-menu" role="menu">
      {#if userCollections.length === 0}
        <div class="menu-empty">暂无自定义合集</div>
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
  {/if}
</div>

<style>
  .add-to-collection-wrapper {
    position: relative;
    display: inline-flex;
  }

  .add-to-collection-btn {
    appearance: none;
    border: none;
    background: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 4px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      color 0.12s ease,
      background-color 0.12s ease;
  }

  .add-to-collection-btn:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.08);
  }

  .add-to-collection-menu {
    position: absolute;
    top: 100%;
    right: 0;
    z-index: 50;
    min-width: 160px;
    max-width: 220px;
    max-height: 200px;
    overflow-y: auto;
    margin-top: 4px;
    padding: 4px;
    border-radius: 10px;
    background: var(--surface-elevated, rgba(30, 41, 59, 0.98));
    border: 1px solid rgba(255, 255, 255, 0.1);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }

  .menu-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--text-secondary);
    font-family: var(--font-body);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    text-align: left;
    transition:
      background-color 0.12s ease,
      color 0.12s ease;
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
