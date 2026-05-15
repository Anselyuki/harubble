<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { toolbarIconButton } from '$lib/design/variants';
  import { RefreshCw, ArrowDown, Settings, Search } from '@lucide/svelte';
  import type { LibrarySearchScope } from '$lib/types';
  import type { AppView } from '$lib/features/shell/store.svelte';

  interface Props {
    activeDownloadCount: number;
    isRefreshing?: boolean;
    settingsOpen?: boolean;
    downloadPanelOpen?: boolean;
    searchQuery: string;
    searchScope: LibrarySearchScope;
    currentView: AppView;
    onRefresh: () => void;
    onOpenDownloads: () => void;
    onOpenSettings: () => void;
    onSearchQueryChange: (query: string) => void;
    onSearchScopeChange: (scope: LibrarySearchScope) => void;
    onNavigate: (view: AppView) => void;
  }

  let {
    activeDownloadCount,
    isRefreshing = false,
    settingsOpen = false,
    downloadPanelOpen = false,
    searchQuery,
    searchScope,
    currentView,
    onRefresh,
    onOpenDownloads,
    onOpenSettings,
    onSearchQueryChange,
    onSearchScopeChange,
    onNavigate,
  }: Props = $props();

  let expanded = $state(false);
  let searchInputEl: HTMLInputElement | undefined = $state();
  let containerEl: HTMLDivElement | undefined = $state();

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      refresh: m.shell_toolbar_refresh(),
      downloads: m.shell_toolbar_downloads(),
      settings: m.shell_toolbar_settings(),
      searchPlaceholder: m.sidebar_search_placeholder(),
    };
  });

  const scopeOptions = $derived.by(() => {
    void localeState.current;
    return [
      { value: 'all' as LibrarySearchScope, label: 'ALL' },
      {
        value: 'albums' as LibrarySearchScope,
        label: m.library_search_scope_albums(),
      },
      {
        value: 'songs' as LibrarySearchScope,
        label: m.library_search_scope_songs(),
      },
    ];
  });

  const activeScopeLabel = $derived.by(
    () => scopeOptions.find((o) => o.value === searchScope)?.label ?? 'ALL'
  );

  function cycleSearchScope() {
    const idx = scopeOptions.findIndex((o) => o.value === searchScope);
    const next = (idx + 1) % scopeOptions.length;
    onSearchScopeChange(scopeOptions[next]?.value ?? 'all');
  }

  function handleTriggerClick() {
    expanded = true;
    if (currentView !== 'overview') onNavigate('overview');
    requestAnimationFrame(() => {
      setTimeout(() => searchInputEl?.focus(), 300);
    });
  }

  function collapse() {
    expanded = false;
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && expanded) {
      collapse();
    }
  }

  function handleClickOutside(event: MouseEvent) {
    if (
      expanded &&
      containerEl &&
      !containerEl.contains(event.target as Node)
    ) {
      collapse();
    }
  }

  $effect(() => {
    if (expanded) {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleKeydown);
    }
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeydown);
    };
  });

  const hasActiveSearch = $derived(searchQuery.length > 0);
</script>

<div class="top-actions">
  <div class="search-trigger" class:expanded bind:this={containerEl}>
    {#if expanded}
      <div class="search-expanded">
        <Button
          variant="outline"
          size="icon"
          class="search-scope-button"
          data-scope={searchScope}
          aria-label={m.library_search_scope_aria({ scope: activeScopeLabel })}
          title={m.library_search_scope_title({ scope: activeScopeLabel })}
          onclick={cycleSearchScope}
        >
          {activeScopeLabel}
        </Button>
        <Input
          bind:ref={searchInputEl}
          value={searchQuery}
          placeholder={labels.searchPlaceholder}
          class="search-input"
          aria-label={labels.searchPlaceholder}
          oninput={(event) => {
            const target = event.currentTarget as HTMLInputElement;
            onSearchQueryChange(target.value);
          }}
        />
        <Search size={14} class="search-icon-right" aria-hidden="true" />
      </div>
    {:else}
      <button
        type="button"
        class="search-circle"
        aria-label={labels.searchPlaceholder}
        onclick={handleTriggerClick}
      >
        <Search size={16} aria-hidden="true" />
      </button>
      {#if hasActiveSearch}
        <span class="search-indicator" aria-hidden="true"></span>
      {/if}
    {/if}
  </div>

  <div
    class="flex items-center gap-2 rounded-full border border-white/50 bg-white/[0.62] p-2 shadow-[0_16px_36px_rgba(15,23,42,0.12)] backdrop-blur-xl"
  >
    <Button
      size="icon"
      variant="ghost"
      class={`text-base ${toolbarIconButton({ active: false })}`}
      onclick={onRefresh}
      disabled={isRefreshing}
      aria-label={labels.refresh}
      title={labels.refresh}
    >
      <RefreshCw size={16} />
    </Button>

    <Button
      size="icon"
      variant="ghost"
      class={`relative text-base ${toolbarIconButton({ active: downloadPanelOpen })}`}
      onclick={onOpenDownloads}
      aria-label={labels.downloads}
      aria-pressed={downloadPanelOpen}
      title={labels.downloads}
    >
      <ArrowDown size={16} />
      {#if activeDownloadCount > 0}
        <span class="toolbar-badge">{activeDownloadCount}</span>
      {/if}
    </Button>

    <Button
      size="icon"
      variant="ghost"
      class={`text-base ${toolbarIconButton({ active: settingsOpen })}`}
      onclick={onOpenSettings}
      aria-label={labels.settings}
      aria-pressed={settingsOpen}
      title={labels.settings}
    >
      <Settings size={16} />
    </Button>
  </div>
</div>

<style>
  .search-trigger {
    width: 36px;
    height: 36px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.62);
    border: 1px solid rgba(255, 255, 255, 0.5);
    backdrop-filter: blur(18px) saturate(1.25);
    -webkit-backdrop-filter: blur(18px) saturate(1.25);
    box-shadow:
      0 16px 36px rgba(15, 23, 42, 0.12),
      inset 0 1px 0 rgba(255, 255, 255, 0.94);
    transition: width 0.2s ease;
    will-change: width;
    overflow: hidden;
    position: relative;
  }

  .search-trigger:hover:not(.expanded) {
    width: 88px;
  }

  .search-trigger.expanded {
    flex: 1;
    transition-duration: 0.3s;
    border-color: rgba(var(--accent-rgb), 0.36);
    box-shadow:
      0 16px 36px rgba(15, 23, 42, 0.12),
      inset 0 1px 0 rgba(255, 255, 255, 0.94),
      0 0 0 3px rgba(var(--accent-rgb), 0.1);
  }

  .search-circle {
    display: flex;
    align-items: center;
    width: 100%;
    height: 100%;
    border: none;
    background: none;
    color: rgba(15, 23, 42, 0.55);
    cursor: pointer;
    padding: 0 0 0 9px;
    margin: 0;
    gap: 6px;
  }

  .search-circle :global(svg) {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }

  .search-circle::after {
    content: '搜索';
    font-family: var(--font-body);
    font-size: 0.75rem;
    font-weight: 500;
    white-space: nowrap;
    opacity: 0;
    transition: opacity 0.15s ease 0.1s;
  }

  .search-trigger:hover:not(.expanded) .search-circle {
    color: rgba(15, 23, 42, 0.8);
  }

  .search-trigger:hover:not(.expanded) .search-circle::after {
    opacity: 1;
  }

  .search-indicator {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent, #5090ff);
    border: 1.5px solid rgba(255, 255, 255, 0.88);
    pointer-events: none;
  }

  .search-expanded {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 100%;
    padding: 0 12px;
    opacity: 0;
    animation: search-fade-in 0.2s ease 0.15s forwards;
  }

  @keyframes search-fade-in {
    to {
      opacity: 1;
    }
  }

  .search-expanded :global(.search-input) {
    flex: 1;
    height: 28px;
    border: none;
    background: transparent;
    font-size: 0.8125rem;
    padding: 0;
    box-shadow: none;
  }

  .search-expanded :global(.search-input:focus-visible) {
    box-shadow: none;
    border: none;
    outline: none;
  }

  .search-expanded :global(.search-icon-right) {
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .search-expanded :global(.search-scope-button) {
    --scope-bg: var(--accent);
    --scope-bg-hover: var(--accent-hover);
    interpolate-size: allow-keywords;
    width: auto;
    min-width: 26px;
    height: 26px;
    padding: 0 6px;
    border: 1px solid color-mix(in srgb, var(--scope-bg) 72%, white 28%);
    border-radius: 6px;
    background: var(--scope-bg);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.22),
      0 4px 10px color-mix(in srgb, var(--scope-bg) 24%, transparent);
    color: var(--accent-readable-foreground);
    font-family: var(--font-wide);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.02em;
    line-height: 1;
    white-space: nowrap;
    flex-shrink: 0;
    transition:
      width 0.25s ease,
      transform 0.15s ease,
      background-color 0.2s ease,
      border-color 0.2s ease,
      box-shadow 0.2s ease;
  }

  .search-expanded :global(.search-scope-button[data-scope='albums']) {
    --scope-bg: oklch(from var(--accent) l c calc(h + 22));
    --scope-bg-hover: oklch(from var(--accent-hover) l c calc(h + 22));
  }

  .search-expanded :global(.search-scope-button[data-scope='songs']) {
    --scope-bg: oklch(from var(--accent) l c calc(h - 28));
    --scope-bg-hover: oklch(from var(--accent-hover) l c calc(h - 28));
  }

  .search-expanded :global(.search-scope-button:hover) {
    border-color: color-mix(in srgb, var(--scope-bg-hover) 78%, white 22%);
    background: var(--scope-bg-hover);
    color: var(--accent-hover-readable-foreground);
  }

  .search-expanded :global(.search-scope-button[data-scope='all']) {
    overflow: hidden;
    isolation: isolate;
  }

  .search-expanded :global(.search-scope-button[data-scope='all']::before) {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    width: 800px;
    height: 800px;
    background: linear-gradient(
      to right,
      #ff2080,
      #c830ff,
      #5050ff,
      #00a0ff,
      #00d4a0,
      #60e840,
      #e8d020,
      #ff8020,
      #ff2080
    );
    background-size: 25% 100%;
    z-index: -2;
    pointer-events: none;
    opacity: 0;
    transform: translate(-50%, -50%) rotate(135deg);
    animation: scope-rainbow-slide 2.4s linear infinite;
    transition: opacity 0.3s ease;
  }

  .search-expanded :global(.search-scope-button[data-scope='all']::after) {
    content: '';
    position: absolute;
    inset: 0;
    z-index: -1;
    pointer-events: none;
    opacity: 0;
    background: radial-gradient(
        circle,
        rgba(255, 255, 255, 0.32) 0.7px,
        transparent 0.7px
      )
      0 0 / 3.5px 3.5px;
    transition: opacity 0.3s ease;
  }

  .search-expanded :global(.search-scope-button[data-scope='all']:hover) {
    border-color: rgba(255, 255, 255, 0.48);
    background: transparent;
    color: #fff;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
    -webkit-text-stroke: 2px rgba(0, 0, 0, 0.5);
    paint-order: stroke fill;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.32);
  }

  .search-expanded
    :global(.search-scope-button[data-scope='all']:hover::before) {
    opacity: 1;
  }

  .search-expanded
    :global(.search-scope-button[data-scope='all']:hover::after) {
    opacity: 1;
  }

  .search-expanded :global(.search-scope-button:active) {
    transform: scaleX(0.92);
    box-shadow:
      inset 0 1px 2px rgba(15, 23, 42, 0.08),
      0 2px 6px rgba(15, 23, 42, 0.06);
  }

  @keyframes scope-rainbow-slide {
    from {
      transform: translate(-50%, -50%) rotate(135deg) translateX(0);
    }
    to {
      transform: translate(-50%, -50%) rotate(135deg) translateX(-25%);
    }
  }
</style>
