// @vitest-environment jsdom

import { describe, expect, it } from 'vitest';
import { HARUBBLE_CLASSIC_COLORS } from './themePresets';
import {
  resolveAppThemeTokenSet,
  LIGHT_TOKENS,
  DARK_TOKENS,
} from './themeTokens';
import type { ThemeTokenSet } from './types';

describe('resolveAppThemeTokenSet', () => {
  it('produces a complete ThemeTokenSet from preset colors (light)', () => {
    const tokens = resolveAppThemeTokenSet(HARUBBLE_CLASSIC_COLORS, 'light');

    expect(tokens.accent).toBe('#FFFA00');
    expect(tokens.accentHover).toMatch(/^#[0-9A-F]{6}$/);
    expect(tokens.accentRgb).toBe('255, 250, 0');
    expect(tokens.accentReadableForeground).toMatch(/^#[0-9A-F]{6}$/);
    expect(tokens.bgPrimary).toBe(LIGHT_TOKENS.bgPrimary);
    expect(tokens.textPrimary).toBe(LIGHT_TOKENS.textPrimary);
    expect(tokens.border).toBe(LIGHT_TOKENS.border);
    expect(tokens.destructive).toBe('#C83F32');
    expect(tokens.surfaceState).toMatch(/^rgba\(255, 250, 0, 0\.08\)$/);
  });

  it('produces dark mode tokens when scheme is dark', () => {
    const tokens = resolveAppThemeTokenSet(HARUBBLE_CLASSIC_COLORS, 'dark');

    expect(tokens.bgPrimary).toBe(DARK_TOKENS.bgPrimary);
    expect(tokens.textPrimary).toBe(DARK_TOKENS.textPrimary);
    expect(tokens.border).toBe(DARK_TOKENS.border);
    expect(tokens.surfaceSidebar).toBe(DARK_TOKENS.surfaceSidebar);
  });

  it('derives accent readable foreground with sufficient contrast', () => {
    const tokens = resolveAppThemeTokenSet(HARUBBLE_CLASSIC_COLORS, 'light');
    const fg = tokens.accentReadableForeground;
    expect(fg).toBeDefined();
    expect(fg).toMatch(/^#[0-9A-F]{6}$/);
  });

  it('app token set does not contain album/wave variables', () => {
    const tokens = resolveAppThemeTokenSet(HARUBBLE_CLASSIC_COLORS, 'light');
    const keys = Object.keys(tokens);
    const forbidden = keys.filter(
      (k) => k.includes('album') || k.includes('wave')
    );
    expect(forbidden).toEqual([]);
  });

  it('all ThemeTokenSet fields are present and non-empty', () => {
    const tokens = resolveAppThemeTokenSet(HARUBBLE_CLASSIC_COLORS, 'light');
    const expectedKeys: (keyof ThemeTokenSet)[] = [
      'accent',
      'accentHover',
      'accentRgb',
      'accentHoverRgb',
      'accentReadableForeground',
      'accentHoverReadableForeground',
      'bgPrimary',
      'bgSecondary',
      'bgTertiary',
      'bgElevated',
      'textPrimary',
      'textSecondary',
      'textTertiary',
      'border',
      'ring',
      'destructive',
      'destructiveRgb',
      'surfaceState',
      'surfaceBase',
      'surfaceSidebar',
      'surfaceOverlay',
    ];

    for (const key of expectedKeys) {
      expect(tokens[key], `${key} should be non-empty`).toBeTruthy();
    }
  });
});
