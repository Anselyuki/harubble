<script lang="ts">
  import type { Component, Snippet } from 'svelte';
  import type { LucideProps } from '@lucide/svelte';
  import { getSidebarItemActivation } from './sidebar-item-button';

  interface Props {
    element?: 'button' | 'div';
    label: string;
    icon?: Component<LucideProps>;
    collapsed: boolean;
    active?: boolean;
    hiddenLabel?: boolean;
    title?: string;
    ariaCurrent?: 'page';
    role?: string;
    ariaSelected?: boolean;
    ariaExpanded?: boolean;
    disabled?: boolean;
    expandOnCollapsedClick?: boolean;
    onRequestExpand?: () => void;
    onclick?: () => void;
    labelEl?: HTMLSpanElement | null;
    children?: Snippet;
  }

  let {
    element = 'button',
    label,
    icon: Icon,
    collapsed,
    active = false,
    hiddenLabel = collapsed,
    title,
    ariaCurrent,
    role,
    ariaSelected,
    ariaExpanded,
    disabled = false,
    expandOnCollapsedClick = false,
    onRequestExpand,
    onclick,
    labelEl = $bindable(null),
    children,
  }: Props = $props();

  const roleValue = $derived(element === 'button' ? role : (role ?? 'button'));
  const tabIndex = $derived(element === 'button' || disabled ? undefined : 0);

  function handleClick() {
    const activation = getSidebarItemActivation({
      collapsed,
      expandOnCollapsedClick,
      hasRequestExpand: onRequestExpand !== undefined,
    });

    if (activation === 'expand') {
      onRequestExpand?.();
      return;
    }

    onclick?.();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (element === 'button') return;
    if (event.key !== 'Enter' && event.key !== ' ') return;

    event.preventDefault();
    handleClick();
  }
</script>

<svelte:element
  this={element}
  type={element === 'button' ? 'button' : undefined}
  class="sidebar-item-button"
  class:active
  class:collapsed
  class:disabled
  role={roleValue}
  tabindex={tabIndex}
  aria-current={ariaCurrent}
  aria-selected={ariaSelected}
  aria-expanded={ariaExpanded}
  aria-disabled={disabled || undefined}
  title={collapsed ? (title ?? label) : title}
  aria-label={collapsed ? label : undefined}
  onclick={handleClick}
  onkeydown={handleKeydown}
>
  {#if Icon}
    <span class="sidebar-item-icon">
      <Icon size={16} aria-hidden={true} />
    </span>
  {/if}
  <span
    class="sidebar-item-label"
    class:hidden={hiddenLabel}
    data-sidebar-item-label
    bind:this={labelEl}>{label}</span
  >
  {#if children && !collapsed}
    <span class="sidebar-item-trailing">
      {@render children()}
    </span>
  {/if}
</svelte:element>

<style>
  .sidebar-item-button {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    height: 36px;
    padding: 0 0.75rem;
    border: none;
    border-radius: 8px;
    background: none;
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
    font-family: var(--font-body);
    font-size: 0.8125rem;
    font-weight: 500;
    cursor: pointer;
    text-align: left;
    transition:
      background var(--motion-fast) ease,
      color var(--motion-fast) ease;
  }

  .sidebar-item-button.collapsed {
    width: 36px;
    padding: 0;
    justify-content: center;
  }

  .sidebar-item-button:hover {
    background: var(--hover-bg-elevated);
    color: var(--text-primary);
  }

  .sidebar-item-button.active {
    background: var(--surface-state);
    color: var(--text-primary);
    font-weight: 600;
  }

  .sidebar-item-button.disabled {
    cursor: default;
  }

  .sidebar-item-button.disabled:hover {
    background: none;
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
  }

  .sidebar-item-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .sidebar-item-label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sidebar-item-label.hidden {
    max-width: 0;
    opacity: 0;
  }

  .sidebar-item-trailing {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 2px;
  }

  @media (prefers-reduced-motion: reduce) {
    .sidebar-item-label {
      transition: none;
    }
  }
</style>
