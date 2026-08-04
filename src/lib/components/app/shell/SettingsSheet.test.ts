import { describe, expect, it } from 'vitest';

describe('PreferencesSettingsSection output directory display', () => {
  it('shows the full path in an overlay instead of expanding the input over controls', async () => {
    // @ts-expect-error Vitest runs in Node and reads the source component file.
    const { readFileSync } = await import('node:fs');
    const source = readFileSync(
      'src/lib/components/app/shell/settings/PreferencesSettingsSection.svelte',
      'utf8'
    );

    expect(source).toContain('settings-path-tooltip');
    expect(source).toContain('{#snippet child({ props })}');
    expect(source).toContain('...triggerProps');
    expect(source).not.toContain('width: max-content');
    expect(source).not.toContain('max-width: calc(100% + 8rem)');
  });
});

describe('SettingsSheet section composition', () => {
  it('imports and renders the five settings section components', async () => {
    // @ts-expect-error Vitest runs in Node and reads the source component file.
    const { readFileSync } = await import('node:fs');
    const source = readFileSync(
      'src/lib/components/app/shell/SettingsSheet.svelte',
      'utf8'
    );

    expect(source).toContain('<Tooltip.Provider>');

    const sections = [
      'PreferencesSettingsSection',
      'ThemeSettingsSection',
      'NotificationsSettingsSection',
      'CacheSettingsSection',
      'LogsSettingsSection',
    ];

    for (const section of sections) {
      expect(source).toContain(
        `import ${section} from '$lib/components/app/shell/settings/${section}.svelte'`
      );
      expect(source).toContain(`<${section}`);
    }

    expect(source).toContain('packageColorsLocked');
    expect(source).toContain('themePackageManager.activePackageId');
    expect(source).toContain('themePackageManager.previewingId');
    expect(source).toContain('aria-current={activeSection === section.id');
    expect(source).toContain("? 'location'");
    expect(source).toContain('syncActiveSectionFromScroll');
    expect(source).toContain("inline: 'nearest'");
    expect(source).toMatch(
      /\.settings-section-nav\s*\{[^}]*overflow-x:\s*auto;[^}]*scrollbar-width:\s*none;/s
    );
    expect(source).toMatch(
      /\.settings-section-nav::-webkit-scrollbar\s*\{[^}]*display:\s*none;/s
    );
    expect(source).not.toMatch(
      /\.settings-section-nav\s*\{[^}]*scrollbar-width:\s*thin;/s
    );
  });
});
