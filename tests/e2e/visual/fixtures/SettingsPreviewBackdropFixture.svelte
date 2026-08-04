<script lang="ts">
  import { onMount } from 'svelte';
  import SettingsSheet from '$lib/components/app/shell/SettingsSheet.svelte';
  import type { ThemePackageManager } from '$lib/features/shell/themePackageManager.svelte';
  import {
    applyAppThemeTokenSet,
    applyContextThemePalette,
    deriveGlobalTokensFromSlots,
  } from '$lib/themeTokens';
  import arkPackageJson from '$lib/theme-packages/builtins/ark-ui-ark.json';
  import corporatePackageJson from '$lib/theme-packages/builtins/ark-ui-corporate.json';
  import endfieldPackageJson from '$lib/theme-packages/builtins/ark-ui-endfield.json';
  import exaPackageJson from '$lib/theme-packages/builtins/ark-ui-exa.json';
  import popucomPackageJson from '$lib/theme-packages/builtins/ark-ui-popucom.json';
  import type {
    ThemeColorSlots,
    ThemePackageDocument,
    ThemePackageSummary,
  } from '$lib/types';

  const params = new URLSearchParams(window.location.search);
  const builtInPackages = {
    ark: arkPackageJson,
    corporate: corporatePackageJson,
    endfield: endfieldPackageJson,
    exa: exaPackageJson,
    popucom: popucomPackageJson,
  } as unknown as Record<string, ThemePackageDocument>;
  const previewPackage =
    builtInPackages[params.get('family') ?? 'ark'] ?? builtInPackages.ark!;
  const samePackageState = params.get('same-package') === '1';
  const packages: ThemePackageSummary[] = [
    {
      id: 'ark-ui-endfield',
      name: 'Field Signal',
      version: '1.0.0',
      status: 'committed',
      builtin: true,
    },
    {
      id: 'ark-ui-ark',
      name: 'Industrial Cyan',
      version: '1.0.0',
      status: 'committed',
      builtin: true,
    },
  ];

  let settingsOpen = $state(true);
  let revision = $state(18);
  let activePackageId = $state<string | null>('ark-ui-endfield');
  let previewingId = $state<string | null>(
    samePackageState ? 'ark-ui-endfield' : 'ark-ui-ark'
  );

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
      return packages;
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
    // Explicit dismissal exits preview, while unmount dismissal keeps the
    // deterministic preview alive for the close/reopen reset scenario.
    dismissPreview: async () => {
      if (settingsOpen) previewingId = null;
    },
    importFromFile: async () => packages[0],
    importFromUrl: async () => packages[0],
    uninstall: async () => {},
  };

  onMount(() => {
    const scheme = 'light' as const;
    const root = document.documentElement;
    const slots = {
      ...previewPackage.slots,
      ...previewPackage.variants?.[scheme],
    } as ThemeColorSlots;
    const tokens = deriveGlobalTokensFromSlots(slots, scheme);
    const family = previewPackage.visualContract?.family ?? 'ark';
    const depth = previewPackage.visualContract?.depth ?? 'moderate';

    root.classList.remove('light', 'dark');
    root.classList.add(scheme);
    root.style.colorScheme = scheme;
    root.dataset.arkTheme = family;
    root.dataset.arkDepth = depth;
    root.dataset.themeFamily = family;
    root.dataset.themeDepth = depth;

    for (const [key, value] of Object.entries(previewPackage.shape ?? {})) {
      root.style.setProperty(`--shape-${key}`, `${value}px`);
    }
    for (const [key, value] of Object.entries(previewPackage.density ?? {})) {
      root.style.setProperty(`--density-${key}`, `${value}px`);
    }
    for (const [key, value] of Object.entries(previewPackage.elevation ?? {})) {
      root.style.setProperty(`--elevation-${key}`, value);
    }
    for (const [key, value] of Object.entries(previewPackage.blur ?? {})) {
      root.style.setProperty(`--blur-${key}`, `${value}px`);
    }

    const cssVariables = {
      ...(previewPackage.cssVariables ?? {}),
      ...(previewPackage.cssVariableVariants?.[scheme] ?? {}),
    };
    for (const [key, value] of Object.entries(cssVariables)) {
      root.style.setProperty(key, value);
    }

    const fontVariableMap = {
      body: '--font-body',
      display: '--font-display',
      mono: '--font-mono',
    } as const;
    for (const [key, value] of Object.entries(
      previewPackage.fontFamily ?? {}
    )) {
      root.style.setProperty(
        fontVariableMap[key as keyof typeof fontVariableMap],
        value
      );
    }

    applyAppThemeTokenSet(tokens, { animate: false });
    applyContextThemePalette(null, tokens, scheme, { animate: false });

    (
      window as typeof window & {
        __SETTINGS_PREVIEW_BACKDROP_FIXTURE_READY__?: boolean;
      }
    ).__SETTINGS_PREVIEW_BACKDROP_FIXTURE_READY__ = true;
  });
</script>

<main
  class="qa-preview-stage"
  data-testid="settings-preview-backdrop-fixture"
  data-settings-open={settingsOpen}
  data-active-package-id={activePackageId ?? ''}
  data-previewing-id={previewingId ?? ''}
>
  <header class="qa-stage-header">
    <span>ACTIVE / FIELD SIGNAL</span>
    <strong>PREVIEW / INDUSTRIAL CYAN</strong>
  </header>
  <section class="qa-stage-content" aria-label="Theme preview stage">
    <p>HARUBBLE THEME CONTROL</p>
    <h1>Preview field</h1>
    <div class="qa-stage-grid" aria-hidden="true">
      <span></span><span></span><span></span>
    </div>
    <button
      type="button"
      data-testid="qa-open-settings"
      onclick={() => (settingsOpen = true)}
    >
      打开设置
    </button>
  </section>
</main>

<SettingsSheet
  bind:open={settingsOpen}
  initialSection="theme-packages"
  themePackageManager={manager as unknown as ThemePackageManager}
  notifyInfo={() => {}}
  notifyError={() => {}}
  onOutputDirChange={() => true}
/>

<style>
  :global(html),
  :global(body) {
    min-width: 0;
    min-height: 100%;
  }

  :global(body) {
    min-height: 100vh;
    margin: 0;
    overflow-x: hidden;
    background: var(--bg-primary);
    color: var(--text-primary);
  }

  .qa-preview-stage {
    min-height: 100vh;
    background:
      linear-gradient(var(--border) 1px, transparent 1px),
      linear-gradient(90deg, var(--border) 1px, transparent 1px),
      var(--bg-primary);
    background-size: 64px 64px;
  }

  .qa-stage-header {
    display: flex;
    min-height: 64px;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 0 28px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-sidebar);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .qa-stage-header strong {
    color: var(--accent);
  }

  .qa-stage-content {
    display: grid;
    max-width: 660px;
    gap: 16px;
    padding: 72px 48px;
  }

  .qa-stage-content p,
  .qa-stage-content h1 {
    margin: 0;
  }

  .qa-stage-content p {
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 700;
  }

  .qa-stage-content h1 {
    font-family: var(--font-display);
    font-size: 44px;
    line-height: 1;
  }

  .qa-stage-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .qa-stage-grid span {
    min-height: 96px;
    border: 1px solid var(--border);
    background: var(--surface-overlay);
    box-shadow: inset 0 4px 0 var(--accent);
  }

  [data-testid='qa-open-settings'] {
    width: fit-content;
    min-height: 40px;
    padding: 0 16px;
    border: 1px solid var(--text-primary);
    border-radius: var(--shape-sm);
    background: var(--accent);
    color: var(--accent-readable-foreground);
    font: inherit;
    font-weight: 700;
  }

  @media (max-width: 520px) {
    .qa-stage-header {
      display: grid;
      align-content: center;
      justify-content: stretch;
      gap: 4px;
      padding: 0 16px;
    }

    .qa-stage-content {
      padding: 48px 20px;
    }

    .qa-stage-content h1 {
      font-size: 34px;
    }

    .qa-stage-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
