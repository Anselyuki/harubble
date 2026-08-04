import { describe, expect, it } from 'vitest';
import type {
  ThemeColorSlots,
  ThemePackageDocument,
  ThemePackageElevation,
} from '$lib/types';
import { deriveGlobalTokensFromSlots } from '$lib/themeTokens';

import arkPackage from './ark-ui-ark.json';
import corporatePackage from './ark-ui-corporate.json';
import endfieldPackage from './ark-ui-endfield.json';
import exaPackage from './ark-ui-exa.json';
import popucomPackage from './ark-ui-popucom.json';

const PACKAGES = [
  arkPackage,
  endfieldPackage,
  exaPackage,
  popucomPackage,
  corporatePackage,
] as unknown as ThemePackageDocument[];

const EXPECTED_FAMILIES = [
  'ark',
  'endfield',
  'exa',
  'popucom',
  'corporate',
] as const;

const COLOR_SLOTS = [
  'accent',
  'surface',
  'textPrimary',
  'textSecondary',
  'tint',
  'danger',
] as const;

const CSS_BLACKLIST = [
  'url(',
  'expression(',
  '@import',
  'javascript:',
  '<script',
  'behavior:',
  '-moz-binding',
  'vbscript:',
  'data:text/html',
] as const;

function hexToRgb(hex: string): [number, number, number] {
  return [
    Number.parseInt(hex.slice(1, 3), 16),
    Number.parseInt(hex.slice(3, 5), 16),
    Number.parseInt(hex.slice(5, 7), 16),
  ];
}

function luminance(hex: string): number {
  const channels = hexToRgb(hex).map((channel) => {
    const value = channel / 255;
    return value <= 0.03928
      ? value / 12.92
      : Math.pow((value + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * channels[0]! + 0.7152 * channels[1]! + 0.0722 * channels[2]!;
}

function contrastRatio(first: string, second: string): number {
  const firstLuminance = luminance(first);
  const secondLuminance = luminance(second);
  const lighter = Math.max(firstLuminance, secondLuminance);
  const darker = Math.min(firstLuminance, secondLuminance);
  return (lighter + 0.05) / (darker + 0.05);
}

describe('Ark UI inspired built-in theme packages', () => {
  it('declares five unique, stable built-in identities', () => {
    expect(PACKAGES.map((pkg) => pkg.manifest.id)).toEqual(
      EXPECTED_FAMILIES.map((family) => `ark-ui-${family}`)
    );
    expect(new Set(PACKAGES.map((pkg) => pkg.manifest.name)).size).toBe(5);
    for (const pkg of PACKAGES) {
      expect(pkg.schemaVersion).toBe(1);
      expect(pkg.manifest.version).toBe('1.0.0');
      expect(pkg.manifest.author).toBe('Harubble Contributors');
      expect(pkg.manifest.license).toBe('MIT');
    }
  });

  it('uses the matching family and declared application depth', () => {
    for (const [index, pkg] of PACKAGES.entries()) {
      expect(pkg.visualContract).toEqual({
        family: EXPECTED_FAMILIES[index],
        depth: EXPECTED_FAMILIES[index] === 'endfield' ? 'complex' : 'moderate',
      });
    }
  });

  it('provides complete hex-only base, light, and dark color slots', () => {
    for (const pkg of PACKAGES) {
      for (const slots of [
        pkg.slots,
        pkg.variants?.light,
        pkg.variants?.dark,
      ]) {
        expect(slots).toBeDefined();
        for (const key of COLOR_SLOTS) {
          expect(slots?.[key]).toMatch(/^#[0-9A-F]{6}$/);
        }
      }
    }
  });

  it('keeps body and secondary text readable in both variants', () => {
    for (const pkg of PACKAGES) {
      for (const slots of [pkg.variants!.light!, pkg.variants!.dark!]) {
        expect(
          contrastRatio(slots.accent!, slots.surface!)
        ).toBeGreaterThanOrEqual(3);
        expect(
          contrastRatio(slots.textPrimary!, slots.surface!)
        ).toBeGreaterThanOrEqual(4.5);
        expect(
          contrastRatio(slots.textSecondary!, slots.surface!)
        ).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it('derives finite app tokens from every color variant', () => {
    for (const pkg of PACKAGES) {
      for (const [scheme, slots] of [
        ['light', pkg.variants!.light!],
        ['dark', pkg.variants!.dark!],
      ] as const) {
        const tokens = deriveGlobalTokensFromSlots(
          slots as ThemeColorSlots,
          scheme
        );
        expect(
          Object.values(tokens).every((value) => !value.includes('NaN'))
        ).toBe(true);
      }
    }
  });

  it('stays within current sanitizer scalar limits', () => {
    for (const pkg of PACKAGES) {
      expect(Math.max(...Object.values(pkg.motion ?? {}))).toBeLessThanOrEqual(
        5000
      );
      const { pill = 0, ...shape } = pkg.shape ?? {};
      expect(Math.max(...Object.values(shape))).toBeLessThanOrEqual(200);
      expect(pill).toBeLessThanOrEqual(65535);
      expect(Math.max(...Object.values(pkg.density ?? {}))).toBeLessThanOrEqual(
        128
      );
      expect(Math.max(...Object.values(pkg.blur ?? {}))).toBeLessThanOrEqual(
        128
      );
    }
  });

  it('uses sanitizer-safe elevation and font values', () => {
    for (const pkg of PACKAGES) {
      for (const value of Object.values(
        pkg.elevation ?? ({} as ThemePackageElevation)
      )) {
        expect(value.length).toBeLessThanOrEqual(512);
        expect(value).not.toMatch(/[;{}<>\\]/);
        expect(
          CSS_BLACKLIST.some((term) => value.toLowerCase().includes(term))
        ).toBe(false);
      }
      for (const value of Object.values(pkg.fontFamily ?? {})) {
        expect(value.length).toBeLessThanOrEqual(256);
        expect(value).toMatch(/^[a-zA-Z0-9 ,._-]+$/);
      }
    }
  });

  it('keeps base and scheme custom variables namespaced and sanitizer-safe', () => {
    for (const pkg of PACKAGES) {
      for (const variables of [
        pkg.cssVariables,
        pkg.cssVariableVariants?.light,
        pkg.cssVariableVariants?.dark,
      ]) {
        const entries = Object.entries(variables ?? {});
        expect(entries.length).toBeLessThanOrEqual(64);
        for (const [key, value] of entries) {
          expect(key).toMatch(/^--theme-custom-[a-z0-9-]+$/);
          expect(key).not.toContain('album');
          expect(value.length).toBeLessThanOrEqual(256);
          expect(value).not.toMatch(/[;{}<>\\]/);
          expect(
            CSS_BLACKLIST.some((term) => value.toLowerCase().includes(term))
          ).toBe(false);
        }
      }
    }
  });

  it('moves panel and rule semantics into complete light/dark variants', () => {
    for (const pkg of PACKAGES) {
      expect(pkg.cssVariables?.['--theme-custom-panel']).toBeUndefined();
      expect(pkg.cssVariables?.['--theme-custom-rule']).toBeUndefined();
      for (const scheme of ['light', 'dark'] as const) {
        expect(
          pkg.cssVariableVariants?.[scheme]?.['--theme-custom-panel']
        ).toBeDefined();
        expect(
          pkg.cssVariableVariants?.[scheme]?.['--theme-custom-rule']
        ).toBeDefined();
      }
    }
  });
});
