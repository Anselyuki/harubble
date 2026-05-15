<script lang="ts">
  import { slide } from 'svelte/transition';
  import SidebarItemButton from '$lib/components/app/SidebarItemButton.svelte';
  import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
  import type { LucideProps } from '@lucide/svelte';
  import type { Snippet, Component } from 'svelte';

  interface Props {
    title: string;
    icon?: Component<LucideProps>;
    defaultExpanded?: boolean;
    empty?: boolean;
    children: Snippet;
    actions?: Snippet;
  }

  let {
    title,
    icon,
    defaultExpanded = false,
    empty = false,
    children,
    actions,
  }: Props = $props();

  // svelte-ignore state_referenced_locally
  let expanded = $state(defaultExpanded);

  function toggle() {
    if (empty) return;
    expanded = !expanded;
  }

  function handleActionsClick(e: MouseEvent) {
    e.stopPropagation();
  }

  function handleActionsKeydown(e: KeyboardEvent) {
    e.stopPropagation();
  }
</script>

<div class="collapsible-group">
  {#if icon}
    <SidebarItemButton
      element={actions ? 'div' : 'button'}
      label={title}
      {icon}
      collapsed={false}
      disabled={empty}
      ariaExpanded={expanded}
      onclick={toggle}
    >
      {#if actions}
        <span
          class="collapsible-group-actions"
          onclick={handleActionsClick}
          onkeydown={handleActionsKeydown}
        >
          {@render actions()}
        </span>
      {/if}
      <span
        class="collapsible-group-chevron"
        class:is-expanded={expanded}
        class:is-disabled={empty}
      >
        <ChevronRightIcon size={12} />
      </span>
    </SidebarItemButton>
  {:else}
    <button
      type="button"
      class="collapsible-group-fallback-header"
      class:is-empty={empty}
      aria-expanded={expanded}
      onclick={toggle}
    >
      <span class="collapsible-group-title">{title}</span>
    </button>
  {/if}

  {#if expanded && !empty}
    <div class="collapsible-group-content" transition:slide={{ duration: 200 }}>
      {@render children()}
    </div>
  {/if}
</div>

<style>
  .collapsible-group {
    display: flex;
    flex-direction: column;
  }

  .collapsible-group-fallback-header {
    appearance: none;
    border: none;
    background: none;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    height: 36px;
    padding: 0 0.75rem;
    border-radius: 8px;
    cursor: pointer;
    transition: background var(--motion-fast, 0.15s) ease;
  }

  .collapsible-group-fallback-header:hover {
    background: var(--hover-bg-elevated, rgba(255, 255, 255, 0.06));
  }

  .collapsible-group-fallback-header.is-empty {
    cursor: default;
  }

  .collapsible-group-title {
    font-family: var(--font-body);
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
  }

  .collapsible-group-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.15s ease;
  }

  :global(.sidebar-item-button:hover) .collapsible-group-actions {
    opacity: 1;
  }

  .collapsible-group-chevron {
    margin-left: auto;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--surface-state, rgba(255, 255, 255, 0.06));
    color: var(--text-tertiary);
    flex-shrink: 0;
    transition: transform 0.2s ease;
  }

  .collapsible-group-chevron.is-disabled {
    opacity: 0.3;
  }

  .collapsible-group-actions + .collapsible-group-chevron {
    margin-left: 4px;
  }

  .collapsible-group-chevron.is-expanded {
    transform: rotate(90deg);
  }

  .collapsible-group-content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-top: 2px;
    padding-left: 10px;
  }
</style>
