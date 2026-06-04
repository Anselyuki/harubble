// @vitest-environment jsdom

import { describe, expect, it } from 'vitest';
import {
  DEFAULT_THEME_PRESET_ID,
  HARUBBLE_CLASSIC_COLORS,
  THEME_PRESETS,
  isValidThemeHex,
  resolveThemeColors,
} from './themePresets';
import {
  applyAlbumAccentPalette,
  applyThemeColors,
  deriveThemeCssVariables,
} from './theme';

describe('theme presets', () => {
  it('resolves the default preset when preferences are missing', () => {
    expect(resolveThemeColors(null)).toEqual(HARUBBLE_CLASSIC_COLORS);
  });

  it('resolves the selected preset without custom colors', () => {
    const colors = resolveThemeColors({
      presetId: DEFAULT_THEME_PRESET_ID,
      customColors: {},
    });

    expect(colors.accent).toBe('#FFE47A');
    expect(colors.surface).toBe('#D1D6DB');
    expect(colors.textPrimary).toBe('#4A5056');
    expect(colors.textSecondary).toBe('#596066');
    expect(colors.tint).toBe('#899CB0');
    expect(colors.danger).toBe('#C74F4F');
  });

  it('falls back to the default preset when the preset id is unknown', () => {
    expect(
      resolveThemeColors({
        presetId: 'unknown-preset',
        customColors: {},
      })
    ).toEqual(HARUBBLE_CLASSIC_COLORS);
  });

  it('includes the required preset ids with complete color slots', () => {
    const presetsById = new Map(
      THEME_PRESETS.map((preset) => [preset.id, preset])
    );

    expect([...presetsById.keys()]).toEqual([
      'harubble-classic',
      'clear-aqua',
      'night-console',
    ]);

    for (const preset of THEME_PRESETS) {
      expect(Object.keys(preset.colors).sort()).toEqual([
        'accent',
        'danger',
        'surface',
        'textPrimary',
        'textSecondary',
        'tint',
      ]);
    }

    expect(presetsById.get('harubble-classic')?.colors.tint).toBe('#899CB0');
  });

  it('applies custom slot overrides over the selected preset', () => {
    expect(
      resolveThemeColors({
        presetId: DEFAULT_THEME_PRESET_ID,
        customColors: {
          accent: '#123abc',
          danger: '#aa3300',
        },
      })
    ).toMatchObject({
      accent: '#123ABC',
      danger: '#AA3300',
      surface: '#D1D6DB',
    });
  });

  it('drops invalid custom hex values while preserving valid slots', () => {
    expect(
      resolveThemeColors({
        presetId: DEFAULT_THEME_PRESET_ID,
        customColors: {
          accent: '123456',
          surface: '#12345G',
          tint: '#0f1a2b',
        },
      })
    ).toMatchObject({
      accent: '#FFE47A',
      surface: '#D1D6DB',
      tint: '#0F1A2B',
    });
  });

  it('validates only six-digit hex colors with a leading hash', () => {
    expect(isValidThemeHex('#ABCDEF')).toBe(true);
    expect(isValidThemeHex('#abcdef')).toBe(true);
    expect(isValidThemeHex('ABCDEF')).toBe(false);
    expect(isValidThemeHex('#ABCDE')).toBe(false);
    expect(isValidThemeHex('#ABCDEG')).toBe(false);
  });
});

describe('theme CSS variables', () => {
  it('derives rgb, hover, readable foreground, and semantic slot variables', () => {
    const variables = deriveThemeCssVariables(HARUBBLE_CLASSIC_COLORS);

    expect(variables['--accent']).toBe('#FFE47A');
    expect(variables['--accent-rgb']).toBe('255, 228, 122');
    expect(variables['--accent-hover']).toMatch(/^#[0-9A-F]{6}$/);
    expect(variables['--accent-readable-foreground']).toMatch(/^#[0-9A-F]{6}$/);
    expect(variables['--theme-surface']).toBe('#D1D6DB');
    expect(variables['--theme-surface-rgb']).toBe('209, 214, 219');
    expect(variables['--theme-text-primary']).toBe('#4A5056');
    expect(variables['--theme-text-secondary']).toBe('#596066');
    expect(variables['--theme-tint']).toBe('#899CB0');
    expect(variables['--theme-tint-rgb']).toBe('137, 156, 176');
    expect(variables['--destructive']).toBe('#C74F4F');
  });

  it('applies app theme colors without setting album overlay colors', () => {
    applyThemeColors(HARUBBLE_CLASSIC_COLORS);

    const root = document.documentElement;
    expect(root.style.getPropertyValue('--accent')).toBe('#FFE47A');
    expect(root.style.getPropertyValue('--theme-surface')).toBe('#D1D6DB');
    expect(root.style.getPropertyValue('--theme-tint')).toBe('#899CB0');
    expect(root.style.getPropertyValue('--album-accent')).toBe('');
  });

  it('applies album palette only to album overlay and waveform variables', () => {
    applyThemeColors(HARUBBLE_CLASSIC_COLORS);
    applyAlbumAccentPalette(
      {
        accentHex: '#112233',
        accentHoverHex: '#223344',
        accentRgb: [17, 34, 51],
        accentHoverRgb: [34, 51, 68],
        waveColors: [
          [17, 34, 51],
          [68, 85, 102],
        ],
      },
      HARUBBLE_CLASSIC_COLORS
    );

    const root = document.documentElement;
    expect(root.style.getPropertyValue('--accent')).toBe('#FFE47A');
    expect(root.style.getPropertyValue('--theme-surface')).toBe('#D1D6DB');
    expect(root.style.getPropertyValue('--album-accent')).toBe('#112233');
    expect(root.style.getPropertyValue('--album-accent-rgb')).toBe(
      '17, 34, 51'
    );
    expect(root.style.getPropertyValue('--wave-color-0')).toBe('17, 34, 51');
    expect(root.style.getPropertyValue('--wave-color-1')).toBe('68, 85, 102');
  });

  it('resets album overlay and wave colors to theme accent when palette is null', () => {
    applyThemeColors(HARUBBLE_CLASSIC_COLORS);
    applyAlbumAccentPalette(null, HARUBBLE_CLASSIC_COLORS);

    const root = document.documentElement;
    expect(root.style.getPropertyValue('--album-accent')).toBe('#FFE47A');
    expect(root.style.getPropertyValue('--album-accent-rgb')).toBe(
      '255, 228, 122'
    );
    expect(root.style.getPropertyValue('--wave-color-0')).toBe('255, 228, 122');
  });
});

describe('theme CSS routing', () => {
  function cssBlocksFor(appCss: string, selector: string): string[] {
    const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const matches = [
      ...appCss.matchAll(new RegExp(`${escapedSelector}\\s*\\{[^}]*\\}`, 'g')),
    ].map((match) => match[0]);
    expect(matches, `${selector} block exists`).not.toHaveLength(0);
    return matches;
  }

  it('routes player and album emphasis through album overlay variables', async () => {
    // @ts-expect-error Vitest runs in Node and reads the source stylesheet.
    const { readFileSync } = await import('node:fs');
    const appCss = readFileSync('src/app.css', 'utf8');
    const albumAccentBlocks = [
      '.player-flyout',
      '.player-flyout-header::after',
      '.player-flyout-count',
      '.player-lyric-line.active',
      '.lyrics-bubble',
      '.lyrics-bubble-header::after',
      '.lyrics-bubble-count',
      '.lyrics-bubble-line.active',
      '.fullscreen-player',
      '.fullscreen-player::before',
      '.fullscreen-cover',
      '.fs-download.download-active',
      ".fs-btn[aria-pressed='true']",
      '.fs-play.playing',
      '.player-playlist-item:hover',
      '.player-playlist-item.active',
      '.album-stage-media-loading',
      '.album-stage-solidify',
      '.album-stage-divider',
      '.loading-cover',
      '.album-belong-tag',
      '.album-download-status-badge',
      '.album-detail-card .btn-primary',
    ];

    for (const selector of albumAccentBlocks) {
      const blocks = cssBlocksFor(appCss, selector);
      const combinedBlocks = blocks.join('\n');
      expect(combinedBlocks, selector).toContain('--album-accent');
      for (const block of blocks) {
        expect(block, selector).not.toMatch(/var\(--accent(?:-rgb)?\)/);
      }
    }
  });

  it('routes primary foreground through the derived accent readable foreground', async () => {
    // @ts-expect-error Vitest runs in Node and reads the source stylesheet.
    const { readFileSync } = await import('node:fs');
    const appCss = readFileSync('src/app.css', 'utf8');

    expect(appCss).toContain(
      '--primary-foreground: var(--accent-readable-foreground);'
    );
  });
});
