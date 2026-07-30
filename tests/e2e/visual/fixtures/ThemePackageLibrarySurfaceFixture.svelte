<script lang="ts">
  import { onMount } from 'svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import ThemePackageLibrarySection from '$lib/components/app/shell/settings/ThemePackageLibrarySection.svelte';
  import { applyThemePackageDocument } from '$lib/features/shell/themePackageManager.svelte';
  import { resolveThemePackageColors } from '$lib/features/shell/themePackageRuntime.svelte';
  import {
    applyAppThemeTokenSet,
    deriveGlobalTokensFromSlots,
  } from '$lib/themeTokens';
  import { DEFAULT_THEME_PREFERENCES } from '$lib/themePresets';
  import endfieldPackageJson from '$lib/theme-packages/builtins/ark-ui-endfield.json';
  import type { ThemePackageDocument, ThemePackageSummary } from '$lib/types';

  const endfieldPackage =
    endfieldPackageJson as unknown as ThemePackageDocument;
  const requestedScheme = new URLSearchParams(window.location.search).get(
    'scheme'
  );
  const scheme: 'light' | 'dark' =
    requestedScheme === 'dark' ? 'dark' : 'light';
  const longValidPackageId = 'a'.repeat(64);

  let activePackageId = $state<string | null>('ark-ui-endfield');
  let previewingId = $state<string | null>(null);
  let revision = $state(7);
  let segmentMode = $state<'auto' | 'light'>('auto');
  let installedPackages = $state<ThemePackageSummary[]>([
    {
      id: 'ark-ui-ark',
      name: 'Industrial Cyan',
      version: '1.0.0',
      status: 'committed',
      builtin: true,
    },
    {
      id: 'ark-ui-exa',
      name: 'Cosmic Archive',
      version: '1.0.0',
      status: 'committed',
      builtin: true,
    },
    {
      id: 'ark-ui-endfield',
      name: 'Field Signal',
      version: '1.0.0',
      status: 'committed',
      builtin: true,
    },
    {
      id: 'ark-ui-popucom',
      name: 'Platform Joy',
      version: '1.0.0',
      status: 'committed',
      builtin: true,
    },
    {
      id: 'ark-ui-corporate',
      name: 'Studio Grid',
      version: '1.0.0',
      status: 'committed',
      builtin: true,
    },
    {
      id: longValidPackageId,
      name: 'Long identifier contract fixture',
      version: '1.0.0',
      status: 'committed',
      builtin: false,
    },
  ]);

  const manager = {
    get currentRevision() {
      return revision;
    },
    get activePackageId() {
      return activePackageId;
    },
    get previewingId() {
      return previewingId;
    },
    get installedPackages() {
      return installedPackages;
    },
    get latestError() {
      return null;
    },
    hydrate: async () => {},
    setActive: async (id: string | null) => {
      activePackageId = id;
      previewingId = null;
      revision += 1;
    },
    preview: async (id: string) => {
      previewingId = id;
    },
    dismissPreview: async () => {
      previewingId = null;
    },
    importFromFile: async () => installedPackages[0],
    importFromUrl: async () => installedPackages[0],
    uninstall: async (id: string) => {
      installedPackages = installedPackages.filter((pkg) => pkg.id !== id);
    },
  };

  onMount(() => {
    const root = document.documentElement;
    root.classList.toggle('dark', scheme === 'dark');
    root.classList.toggle('light', scheme === 'light');
    applyAppThemeTokenSet(
      deriveGlobalTokensFromSlots(
        resolveThemePackageColors(
          DEFAULT_THEME_PREFERENCES,
          endfieldPackage,
          scheme
        ),
        scheme
      )
    );
    applyThemePackageDocument(endfieldPackage, scheme);
  });
</script>

<main
  class="qa-settings-shell app-side-sheet"
  data-side="right"
  data-testid="endfield-settings-fixture"
  data-scheme={scheme}
  data-active-package-id={activePackageId ?? ''}
  data-previewing-id={previewingId ?? ''}
>
  <header class="sheet-header qa-settings-header">
    <h1>Download settings</h1>
    <p>Theme package surface QA</p>
  </header>
  <div class="sheet-body">
    <ThemePackageLibrarySection
      {manager}
      sectionTitle="Theme packages"
      sectionDescription="Import and manage application theme packages."
    />
    <section
      class="sheet-section settings-section qa-button-contract"
      data-testid="endfield-button-contract"
    >
      <header class="qa-button-contract-heading">
        <h2>Shared actions</h2>
      </header>
      <div class="qa-button-row">
        <Button data-testid="qa-button-primary">Primary</Button>
        <Button data-testid="qa-button-secondary" variant="secondary"
          >Secondary</Button
        >
        <Button data-testid="qa-button-outline" variant="outline"
          >Outline</Button
        >
        <Button data-testid="qa-button-danger" variant="destructive"
          >Danger</Button
        >
      </div>
      <div class="settings-segment" aria-label="Scheme">
        <button
          type="button"
          class:active={segmentMode === 'auto'}
          aria-pressed={segmentMode === 'auto'}
          onclick={() => (segmentMode = 'auto')}>Auto</button
        >
        <button
          type="button"
          class:active={segmentMode === 'light'}
          aria-pressed={segmentMode === 'light'}
          onclick={() => (segmentMode = 'light')}>Light</button
        >
      </div>
    </section>
  </div>
</main>

<style>
  :global(body) {
    display: grid;
    min-width: 0;
    min-height: 100vh;
    place-items: start center;
    overflow: auto;
    margin: 0;
    background: var(--bg-primary);
  }

  .qa-settings-shell {
    position: relative;
    display: flex;
    width: min(100%, 576px);
    min-height: 100vh;
    flex-direction: column;
    color: var(--text-primary);
  }

  .qa-settings-header {
    display: grid;
    gap: 6px;
  }

  .qa-settings-header h1,
  .qa-settings-header p {
    margin: 0;
  }

  .qa-settings-header h1 {
    font-family: var(--font-display);
    font-size: 24px;
  }

  .qa-settings-header p {
    color: var(--text-secondary);
    font-size: 13px;
  }

  .qa-button-contract {
    display: grid;
    gap: 12px;
  }

  .qa-button-contract-heading h2 {
    margin: 0;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 700;
  }

  .qa-button-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .settings-segment {
    display: inline-grid;
    width: fit-content;
    grid-auto-flow: column;
    grid-auto-columns: minmax(72px, 1fr);
    overflow: hidden;
    border: 1px solid var(--sheet-border);
    background: var(--sheet-row-bg);
    padding: 2px;
  }

  .settings-segment button {
    border: 0;
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    cursor: pointer;
  }

  .settings-segment button.active {
    background: var(--accent);
    color: var(--accent-readable-foreground);
  }
</style>
