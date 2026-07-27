<script lang="ts">
  import {
    gsap,
    getMotionDuration,
    killTweens,
    MOTION,
  } from '$lib/design/gsap';
  import SidebarItemButton from '$lib/components/app/sidebar/SidebarItemButton.svelte';
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
  let contentEl = $state<HTMLElement | undefined>();
  // svelte-ignore state_referenced_locally
  let contentMounted = $state(defaultExpanded && !empty);

  const shouldShow = $derived(expanded && !empty);

  $effect(() => {
    if (shouldShow) contentMounted = true;
  });

  $effect(() => {
    if (!contentEl || !shouldShow) return;
    killTweens(contentEl);
    gsap.fromTo(
      contentEl,
      { height: 0, overflow: 'hidden' },
      {
        height: 'auto',
        overflow: 'visible',
        duration: getMotionDuration(MOTION.OVERLAY_IN),
        ease: 'ios-out',
      }
    );
  });

  $effect(() => {
    if (shouldShow || !contentMounted || !contentEl) return;
    killTweens(contentEl);
    gsap.to(contentEl, {
      height: 0,
      overflow: 'hidden',
      duration: getMotionDuration(MOTION.BASE_OUT),
      ease: 'ios-in',
      onComplete: () => {
        contentMounted = false;
      },
    });
  });

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
          role="toolbar"
          tabindex={-1}
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

  {#if contentMounted}
    <div class="collapsible-group-content" bind:this={contentEl}>
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
    border-radius: var(--shape-md);
    cursor: pointer;
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
