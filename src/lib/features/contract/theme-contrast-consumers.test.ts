import { describe, expect, it } from 'vitest';

async function readSource(path: string): Promise<string> {
  // @ts-expect-error Vitest runs in Node and reads source files.
  const { readFileSync } = await import('node:fs');
  return readFileSync(path, 'utf8');
}

describe('theme contrast consumers', () => {
  it('uses derived readable foregrounds on accent-filled controls', async () => {
    const appCss = await readSource('src/app.css');
    const globalAccentConsumers = await Promise.all(
      [
        'src/lib/components/SongRow.svelte',
        'src/lib/components/app/home/HomeTagGroups.svelte',
        'src/lib/components/app/shell/SettingsSheet.svelte',
        'src/lib/components/app/shell/TopToolbar.svelte',
      ].map(readSource)
    );

    expect(
      appCss.match(
        /--player-play-text:\s*var\(--album-accent-readable-foreground\);/g
      )
    ).toHaveLength(2);
    for (const source of globalAccentConsumers) {
      expect(source).toContain('var(--accent-readable-foreground)');
    }
  });

  it('keeps the Endfield mini player scheme-aware', async () => {
    const appCss = await readSource('src/app.css');
    const miniPlayer = await readSource(
      'src/lib/components/app/player/MiniPlayerWindow.svelte'
    );
    const documentBlock = appCss.match(
      /:root\[data-ark-theme='endfield'\]\[data-ark-depth='complex'\]\.mini-player-document\s*\{([^}]*)\}/
    )?.[1];
    const surfaceBlock = appCss.match(
      /:root\[data-ark-theme='endfield'\]\[data-ark-depth='complex'\] \.mini-player\s*\{([^}]*)\}/
    )?.[1];

    expect(documentBlock).toContain(
      '--mini-player-window-bg: var(--bg-primary)'
    );
    expect(documentBlock).not.toContain('var(--ark-field-paper)');
    expect(surfaceBlock).toContain('--mini-surface: var(--bg-primary)');
    expect(surfaceBlock).toContain('--mini-border: var(--text-primary)');
    expect(appCss).toContain('color: var(--album-accent-readable-foreground)');

    expect(miniPlayer).toContain(
      "const followsSystem = colorScheme === undefined || colorScheme === 'auto'"
    );
    expect(miniPlayer).toMatch(
      /if \(lastThemePreferences && !followsSystem\)\s*\{\s*return;/
    );
  });

  it('keeps moderate-family toolbar states readable on the fixed shell', async () => {
    const appCss = await readSource('src/app.css');
    const pressedBlock = appCss.match(
      /:is\(\.top-actions > div, \.top-toolbar\)\s+button\[aria-pressed='true'\]\s*\{([^}]*)\}/
    )?.[1];
    const toolbarFocusBlock = appCss.match(
      /:is\(\.top-actions button, \.top-toolbar button\):focus-visible\s*\{([^}]*)\}/
    )?.[1];

    expect(appCss).toContain(
      '--ark-family-toolbar-focus-ring: var(--ark-family-toolbar-color)'
    );
    expect(appCss.match(/--ark-family-toolbar-shell:\s*rgb\(/g)).toHaveLength(
      6
    );
    expect(appCss).not.toMatch(
      /--ark-family-toolbar-background:[^;]*var\(--ark-family-panel-source\)/
    );
    expect(appCss).not.toMatch(
      /--ark-family-sidebar-background:\s*var\(--ark-family-panel-source\)/
    );
    expect(appCss).toContain(
      '--ark-family-sidebar-rule: var(--ark-family-shell-rule)'
    );
    expect(pressedBlock).toContain('color: var(--ark-family-toolbar-color)');
    expect(pressedBlock).not.toMatch(/color:\s*var\(--accent\)/);
    expect(pressedBlock).toContain(
      'background: color-mix(in srgb, var(--accent) 18%, transparent)'
    );
    expect(pressedBlock).toContain('box-shadow: inset 0 -2px 0 var(--accent)');
    expect(toolbarFocusBlock).toContain(
      'outline: 2px solid var(--ark-family-toolbar-focus-ring)'
    );
  });
});
