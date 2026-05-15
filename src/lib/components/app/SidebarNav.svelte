<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import SidebarItemButton from '$lib/components/app/SidebarItemButton.svelte';
  import { Home, Library } from '@lucide/svelte';
  import type { AppView } from '$lib/features/shell/store.svelte';

  interface Props {
    currentView: AppView;
    onNavigate: (view: AppView) => void;
    collapsed?: boolean;
    onRequestExpand?: () => void;
  }

  let {
    currentView,
    onNavigate,
    collapsed = false,
    onRequestExpand,
  }: Props = $props();

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      home: m.shell_nav_home(),
      library: m.shell_nav_library(),
    };
  });

  const navItems: {
    view: AppView;
    icon: typeof Home;
    labelKey: 'home' | 'library';
    expandOnCollapsedClick: boolean;
  }[] = [
    {
      view: 'home',
      icon: Home,
      labelKey: 'home',
      expandOnCollapsedClick: false,
    },
    {
      view: 'overview',
      icon: Library,
      labelKey: 'library',
      expandOnCollapsedClick: false,
    },
  ];
</script>

<nav class="sidebar-nav" class:collapsed aria-label="Main navigation">
  {#each navItems as item (item.view)}
    <SidebarItemButton
      label={labels[item.labelKey]}
      icon={item.icon}
      {collapsed}
      active={currentView === item.view}
      hiddenLabel={collapsed}
      ariaCurrent={currentView === item.view ? 'page' : undefined}
      expandOnCollapsedClick={item.expandOnCollapsedClick}
      {onRequestExpand}
      onclick={() => onNavigate(item.view)}
    />
  {/each}
</nav>

<style>
  .sidebar-nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0;
  }

  .sidebar-nav.collapsed {
    align-items: flex-start;
    padding-left: 10px;
  }
</style>
