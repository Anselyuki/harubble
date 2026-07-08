// @vitest-environment jsdom

import { describe, expect, it } from 'vitest';
import {
  resolveMonetTokenSet,
  rgbToHsl,
  hslToRgb,
  deltaE76,
  getContrastRatio,
  getRelativeLuminance,
} from './monetPalette';
import { hexToRgb } from './theme';
import type { ThemeTokenSet } from './types';

describe('color space utilities', () => {
  it('converts RGB to HSL and back', () => {
    const cases: [[number, number, number], string][] = [
      [[255, 0, 0], 'red'],
      [[0, 255, 0], 'green'],
      [[0, 0, 255], 'blue'],
      [[128, 128, 128], 'gray'],
      [[255, 255, 255], 'white'],
      [[0, 0, 0], 'black'],
    ];
    for (const [rgb, label] of cases) {
      const hsl = rgbToHsl(rgb);
      const roundTrip = hslToRgb(hsl);
      expect(roundTrip, `round-trip ${label}`).toEqual(rgb);
    }
  });

  it('computes Delta E >= 15 between visually distinct colors', () => {
    const red: [number, number, number] = [255, 0, 0];
    const blue: [number, number, number] = [0, 0, 255];
    expect(deltaE76(red, blue)).toBeGreaterThan(15);
  });

  it('computes relative luminance correctly for black and white', () => {
    expect(getRelativeLuminance([0, 0, 0])).toBeCloseTo(0);
    expect(getRelativeLuminance([255, 255, 255])).toBeCloseTo(1);
  });

  it('computes 21:1 contrast between black and white', () => {
    expect(getContrastRatio([0, 0, 0], [255, 255, 255])).toBeCloseTo(21);
  });
});

describe('resolveMonetTokenSet', () => {
  const SEED_COLORS = ['#3B82F6', '#EF4444', '#10B981', '#FFE47A', '#8B5CF6'];

  it('produces a complete ThemeTokenSet from a seed color (light)', () => {
    const tokens = resolveMonetTokenSet('#3B82F6', 'light');
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

  it('accent readable foreground has >= 4.5:1 contrast against accent', () => {
    for (const seed of SEED_COLORS) {
      for (const scheme of ['light', 'dark'] as const) {
        const tokens = resolveMonetTokenSet(seed, scheme);
        const accentRgb = hexToRgb(tokens.accent);
        const fgRgb = hexToRgb(tokens.accentReadableForeground);
        const ratio = getContrastRatio(accentRgb, fgRgb);
        expect(
          ratio,
          `${seed} ${scheme}: accent fg contrast ${ratio.toFixed(2)}`
        ).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it('text primary has >= 4.5:1 contrast against bg primary', () => {
    for (const seed of SEED_COLORS) {
      for (const scheme of ['light', 'dark'] as const) {
        const tokens = resolveMonetTokenSet(seed, scheme);
        const bgRgb = hexToRgb(tokens.bgPrimary);
        const textRgb = hexToRgb(tokens.textPrimary);
        const ratio = getContrastRatio(bgRgb, textRgb);
        expect(
          ratio,
          `${seed} ${scheme}: text/bg contrast ${ratio.toFixed(2)}`
        ).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it('text secondary has >= 3:1 contrast against bg primary', () => {
    for (const seed of SEED_COLORS) {
      for (const scheme of ['light', 'dark'] as const) {
        const tokens = resolveMonetTokenSet(seed, scheme);
        const bgRgb = hexToRgb(tokens.bgPrimary);
        const textRgb = hexToRgb(tokens.textSecondary);
        const ratio = getContrastRatio(bgRgb, textRgb);
        expect(
          ratio,
          `${seed} ${scheme}: secondary text contrast ${ratio.toFixed(2)}`
        ).toBeGreaterThanOrEqual(3);
      }
    }
  });

  it('does not contain album/wave variables in token keys', () => {
    const tokens = resolveMonetTokenSet('#3B82F6', 'dark');
    const keys = Object.keys(tokens);
    const forbidden = keys.filter(
      (k) => k.includes('album') || k.includes('wave')
    );
    expect(forbidden).toEqual([]);
  });

  it('dark and light modes produce different bg/text tones', () => {
    const light = resolveMonetTokenSet('#3B82F6', 'light');
    const dark = resolveMonetTokenSet('#3B82F6', 'dark');
    expect(light.bgPrimary).not.toBe(dark.bgPrimary);
    expect(light.textPrimary).not.toBe(dark.textPrimary);
  });
});
