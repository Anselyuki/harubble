<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
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

  const activeScopeLabel = $derived.by(
    () => labels[scopeOptions.find((o) => o.value === scope)?.labelKey ?? 'all']
  );

  function cycleScope() {
    const idx = scopeOptions.findIndex((o) => o.value === scope);
    const next = (idx + 1) % scopeOptions.length;
    onScopeChange(scopeOptions[next]?.value ?? 'all');
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      onSubmit();
    }
  }
</script>

<div class="search-bar" class:has-query={query.length > 0}>
  <Button
    variant="outline"
    size="icon"
    class="scope-btn"
    data-scope={scope}
    aria-label={activeScopeLabel}
    title={activeScopeLabel}
    onclick={cycleScope}
  >
    {activeScopeLabel}
  </Button>

  <Input
    value={query}
    placeholder={labels.placeholder}
    class="search-input"
    aria-label={labels.placeholder}
    oninput={(e) => onQueryChange((e.currentTarget as HTMLInputElement).value)}
    onkeydown={handleKeydown}
  />

  <Search size={16} class="search-icon" aria-hidden="true" />
</div>

<style>
  .search-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.72);
    border: 1px solid rgba(255, 255, 255, 0.5);
    backdrop-filter: blur(18px) saturate(1.3);
    -webkit-backdrop-filter: blur(18px) saturate(1.3);
    box-shadow:
      0 2px 16px rgba(15, 23, 42, 0.06),
      inset 0 1px 0 rgba(255, 255, 255, 0.9);
  }

  .search-bar.has-query {
    border-color: rgba(var(--accent-rgb), 0.3);
    box-shadow:
      0 2px 16px rgba(15, 23, 42, 0.06),
      inset 0 1px 0 rgba(255, 255, 255, 0.9),
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

  .search-bar :global(.scope-btn) {
    --scope-bg: var(--accent);
    interpolate-size: allow-keywords;
    width: auto;
    min-width: 28px;
    height: 28px;
    padding: 0 8px;
    border: 1px solid color-mix(in srgb, var(--scope-bg) 72%, white 28%);
    border-radius: 8px;
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
  }

  .search-bar :global(.scope-btn[data-scope='albums']) {
    --scope-bg: oklch(from var(--accent) l c calc(h + 22));
  }

  .search-bar :global(.scope-btn[data-scope='songs']) {
    --scope-bg: oklch(from var(--accent) l c calc(h - 28));
  }

  .search-bar :global(.scope-btn:active) {
    transform: scale(0.94);
  }
</style>
