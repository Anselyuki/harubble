<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Search } from '@lucide/svelte';
  import type { LibrarySearchScope } from '$lib/types';

  interface Props {
    query: string;
    scope: LibrarySearchScope;
    onQueryChange: (query: string) => void;
    onScopeChange: (scope: LibrarySearchScope) => void;
    onSubmit: () => void;
  }

  let { query, scope, onQueryChange, onScopeChange, onSubmit }: Props =
    $props();

  const scopeOptions: {
    value: LibrarySearchScope;
    labelKey: 'all' | 'albums' | 'songs';
  }[] = [
    { value: 'all', labelKey: 'all' },
    { value: 'albums', labelKey: 'albums' },
    { value: 'songs', labelKey: 'songs' },
  ];

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      placeholder: m.search_bar_placeholder(),
      all: m.search_scope_all(),
      albums: m.search_scope_albums(),
      songs: m.search_scope_songs(),
    };
  });

  const activeScopeLabel = $derived(
    labels[
      scopeOptions.find((option) => option.value === scope)?.labelKey ?? 'all'
    ]
  );

  const scopeAria = $derived.by(() => {
    void localeState.current;
    return m.library_search_scope_title({ scope: activeScopeLabel });
  });

  function handleSubmit(event: SubmitEvent) {
    event.preventDefault();
    onSubmit();
  }
</script>

<form
  class="search-bar"
  class:has-query={query.length > 0}
  role="search"
  onsubmit={handleSubmit}
>
  <Input
    value={query}
    placeholder={labels.placeholder}
    class="search-input"
    data-testid="search-input"
    aria-label={labels.placeholder}
    oninput={(e) => onQueryChange((e.currentTarget as HTMLInputElement).value)}
  />

  <Search size={16} class="search-icon" aria-hidden="true" />

  <div class="scope-segment" role="group" aria-label={scopeAria}>
    {#each scopeOptions as option (option.value)}
      <button
        type="button"
        class:active={scope === option.value}
        aria-pressed={scope === option.value}
        onclick={() => onScopeChange(option.value)}
      >
        {labels[option.labelKey]}
      </button>
    {/each}
  </div>
</form>

<style>
  .search-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    border-radius: var(--shape-xl);
    background: var(--toolbar-surface);
    border: 1px solid var(--toolbar-highlight);
    backdrop-filter: blur(18px) saturate(1.3);
    -webkit-backdrop-filter: blur(18px) saturate(1.3);
    box-shadow:
      0 2px 16px rgba(15, 23, 42, 0.06),
      inset 0 1px 0 var(--toolbar-highlight);
  }

  .search-bar.has-query {
    border-color: rgba(var(--accent-rgb), 0.3);
    box-shadow:
      0 2px 16px rgba(15, 23, 42, 0.06),
      inset 0 1px 0 var(--toolbar-highlight),
      0 0 0 3px rgba(var(--accent-rgb), 0.08);
  }

  .search-bar :global(.search-input) {
    flex: 1;
    height: 32px;
    border: none;
    background: transparent;
    font-size: 0.875rem;
    padding: 0;
    box-shadow: none;
  }

  .search-bar :global(.search-input:focus-visible) {
    box-shadow: none;
    outline: none;
  }

  .search-bar :global(.search-icon) {
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .scope-segment {
    display: inline-grid;
    grid-auto-flow: column;
    grid-auto-columns: minmax(44px, auto);
    padding: 2px;
    border: 1px solid var(--toolbar-highlight);
    border-radius: var(--shape-md);
    background: color-mix(
      in srgb,
      var(--toolbar-surface) 74%,
      var(--text-primary) 6%
    );
    flex-shrink: 0;
  }

  .scope-segment button {
    min-width: 44px;
    height: 40px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--shape-sm);
    background: transparent;
    color: var(--text-secondary);
    font-family: var(--font-wide);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0;
    line-height: 1;
    white-space: nowrap;
    cursor: pointer;
    transition: var(--motion-hover);
  }

  .scope-segment button:hover {
    color: var(--text-primary);
    background: var(--hover-bg-elevated);
  }

  .scope-segment button.active {
    background: var(--accent);
    color: var(--accent-readable-foreground);
  }

  .scope-segment button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  @media (max-width: 620px) {
    .search-bar {
      flex-wrap: wrap;
    }

    .scope-segment {
      width: 100%;
      grid-auto-columns: minmax(0, 1fr);
    }
  }
</style>
