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

  it('provides the default preset name for both Chinese and English locales', async () => {
    // @ts-expect-error Vitest runs in Node and reads the source message files.
    const { readFileSync } = await import('node:fs');
    const zhCnMessages = JSON.parse(
      readFileSync('messages/zh-CN.json', 'utf8')
    );
    const enUsMessages = JSON.parse(
      readFileSync('messages/en-US.json', 'utf8')
    );

    const key = 'settings_theme_preset_harubble_classic_name';
    const zhName = zhCnMessages[key];
    const enName = enUsMessages[key];

    // 仅校验结构契约：两个 locale 都提供了非空译文，且未漏翻退化为同一字符串；
    // 不锁死具体译文，避免文案调整时误报。
    expect(typeof zhName).toBe('string');
    expect(zhName.length).toBeGreaterThan(0);
    expect(typeof enName).toBe('string');
    expect(enName.length).toBeGreaterThan(0);
    expect(zhName).not.toBe(enName);
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
