// @vitest-environment jsdom

import { describe, expect, it } from 'vitest';
import {
  HARUBBLE_CLASSIC_COLORS,
  SYSTEM_LIGHT_SLOTS,
  SYSTEM_DARK_SLOTS,
} from './themePresets';
import {
  resolveAppThemeTokenSet,
  deriveGlobalTokensFromSlots,
  isSlotDerivationEnabled,
  LIGHT_TOKENS,
  DARK_TOKENS,
} from './themeTokens';
import type { ThemeColorSlots, ThemeTokenSet } from './types';

// 用于 Step 0.b 测试的合法 slot 集合
const SAMPLE_LIGHT_SLOTS: ThemeColorSlots = {
  accent: '#5B8DEF',
  surface: '#ffffff',
  textPrimary: '#1d1d1f',
  textSecondary: '#6e6e73',
  tint: '#000000',
  danger: '#D32F2F',
};

const SAMPLE_DARK_SLOTS: ThemeColorSlots = {
  accent: '#5B8DEF',
  surface: '#000000',
  textPrimary: '#ffffff',
  textSecondary: '#8e8e93',
  tint: '#ffffff',
  danger: '#D32F2F',
};

describe('deriveGlobalTokensFromSlots', () => {
  it('light scheme: bgPrimary equals surface slot', () => {
    const tokens = deriveGlobalTokensFromSlots(SAMPLE_LIGHT_SLOTS, 'light');
    expect(tokens.bgPrimary).toBe('#ffffff');
    expect(tokens.surfaceBase).toBe('#ffffff');
  });

  it('dark scheme: bgPrimary equals surface slot', () => {
    const tokens = deriveGlobalTokensFromSlots(SAMPLE_DARK_SLOTS, 'dark');
    expect(tokens.bgPrimary).toBe('#000000');
  });

  it('textPrimary and textSecondary pass through unchanged', () => {
    const tokens = deriveGlobalTokensFromSlots(SAMPLE_LIGHT_SLOTS, 'light');
    expect(tokens.textPrimary).toBe('#1d1d1f');
    expect(tokens.textSecondary).toBe('#6e6e73');
  });

  it('light scheme: bgSecondary is darker than bgPrimary', () => {
    const tokens = deriveGlobalTokensFromSlots(SAMPLE_LIGHT_SLOTS, 'light');
    const parseBrightness = (hex: string) => {
      const r = parseInt(hex.slice(1, 3), 16);
      const g = parseInt(hex.slice(3, 5), 16);
      const b = parseInt(hex.slice(5, 7), 16);
      return r + g + b;
    };
    expect(parseBrightness(tokens.bgSecondary)).toBeLessThan(
      parseBrightness(tokens.bgPrimary)
    );
  });

  it('dark scheme: bgSecondary is brighter than bgPrimary', () => {
    const tokens = deriveGlobalTokensFromSlots(SAMPLE_DARK_SLOTS, 'dark');
    const parseBrightness = (hex: string) => {
      const r = parseInt(hex.slice(1, 3), 16);
      const g = parseInt(hex.slice(3, 5), 16);
      const b = parseInt(hex.slice(5, 7), 16);
      return r + g + b;
    };
    expect(parseBrightness(tokens.bgSecondary)).toBeGreaterThan(
      parseBrightness(tokens.bgPrimary)
    );
  });

  it('bgTertiary is more differentiated from surface than bgSecondary', () => {
    const tokens = deriveGlobalTokensFromSlots(SAMPLE_LIGHT_SLOTS, 'light');
    const parseR = (hex: string) => parseInt(hex.slice(1, 3), 16);
    const diffSecondary = Math.abs(
      parseR('#ffffff') - parseR(tokens.bgSecondary)
    );
    const diffTertiary = Math.abs(
      parseR('#ffffff') - parseR(tokens.bgTertiary)
    );
    expect(diffTertiary).toBeGreaterThan(diffSecondary);
  });

  it('dark scheme: border is rgba(255,255,255,0.08)', () => {
    const tokens = deriveGlobalTokensFromSlots(SAMPLE_DARK_SLOTS, 'dark');
    expect(tokens.border).toBe('rgba(255, 255, 255, 0.08)');
  });

  it('surfaceOverlay contains 0.76 alpha', () => {
    const tokens = deriveGlobalTokensFromSlots(SAMPLE_LIGHT_SLOTS, 'light');
    expect(tokens.surfaceOverlay).toMatch(/0\.76/);
  });

  it('bgElevated contains 0.8 alpha', () => {
    const tokens = deriveGlobalTokensFromSlots(SAMPLE_LIGHT_SLOTS, 'light');
    expect(tokens.bgElevated).toMatch(/0\.8/);
  });

  it('all 21 ThemeTokenSet fields are present and non-empty', () => {
    const tokens = deriveGlobalTokensFromSlots(SAMPLE_LIGHT_SLOTS, 'light');
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

  it('reduced-motion has no effect on derived tokens (pure function)', () => {
    const a = deriveGlobalTokensFromSlots(SAMPLE_LIGHT_SLOTS, 'light');
    const b = deriveGlobalTokensFromSlots(SAMPLE_LIGHT_SLOTS, 'light');
    expect(a).toEqual(b);
  });
});

describe('Phase 0 Step 0.c: 系统内置 preset 反向校准契约', () => {
  it('SYSTEM_LIGHT_SLOTS 派生 bgPrimary = LIGHT_TOKENS.bgPrimary', () => {
    const tokens = deriveGlobalTokensFromSlots(SYSTEM_LIGHT_SLOTS, 'light');
    expect(tokens.bgPrimary).toBe(LIGHT_TOKENS.bgPrimary);
  });

  it('SYSTEM_LIGHT_SLOTS 派生 bgSecondary = LIGHT_TOKENS.bgSecondary', () => {
    const tokens = deriveGlobalTokensFromSlots(SYSTEM_LIGHT_SLOTS, 'light');
    expect(tokens.bgSecondary.toLowerCase()).toBe(
      LIGHT_TOKENS.bgSecondary.toLowerCase()
    );
  });

  it('SYSTEM_LIGHT_SLOTS 派生 bgTertiary = LIGHT_TOKENS.bgTertiary', () => {
    const tokens = deriveGlobalTokensFromSlots(SYSTEM_LIGHT_SLOTS, 'light');
    expect(tokens.bgTertiary.toLowerCase()).toBe(
      LIGHT_TOKENS.bgTertiary.toLowerCase()
    );
  });

  it('SYSTEM_LIGHT_SLOTS 派生 textPrimary/textSecondary/surfaceBase 精确匹配', () => {
    const tokens = deriveGlobalTokensFromSlots(SYSTEM_LIGHT_SLOTS, 'light');
    expect(tokens.textPrimary).toBe(LIGHT_TOKENS.textPrimary);
    expect(tokens.textSecondary).toBe(LIGHT_TOKENS.textSecondary);
    expect(tokens.surfaceBase).toBe(LIGHT_TOKENS.surfaceBase);
  });

  it('SYSTEM_DARK_SLOTS 派生 bgPrimary = DARK_TOKENS.bgPrimary', () => {
    const tokens = deriveGlobalTokensFromSlots(SYSTEM_DARK_SLOTS, 'dark');
    expect(tokens.bgPrimary).toBe(DARK_TOKENS.bgPrimary);
  });

  it('SYSTEM_DARK_SLOTS 派生 bgSecondary 与 DARK_TOKENS.bgSecondary 每通道差 ≤ 1', () => {
    const tokens = deriveGlobalTokensFromSlots(SYSTEM_DARK_SLOTS, 'dark');
    const parseHex = (hex: string) => [
      parseInt(hex.slice(1, 3), 16),
      parseInt(hex.slice(3, 5), 16),
      parseInt(hex.slice(5, 7), 16),
    ];
    const derived = parseHex(tokens.bgSecondary);
    const expected = parseHex(DARK_TOKENS.bgSecondary);
    for (let i = 0; i < 3; i++) {
      expect(Math.abs(derived[i] - expected[i])).toBeLessThanOrEqual(1);
    }
  });

  it('SYSTEM_DARK_SLOTS 派生 border = DARK_TOKENS.border 精确匹配', () => {
    const tokens = deriveGlobalTokensFromSlots(SYSTEM_DARK_SLOTS, 'dark');
    expect(tokens.border).toBe(DARK_TOKENS.border);
  });
});

describe('isSlotDerivationEnabled feature flag', () => {
  it('returns false when localStorage flag is not set', () => {
    window.localStorage.removeItem('theme_derive_from_slots');
    expect(isSlotDerivationEnabled()).toBe(false);
  });

  it('returns true when localStorage flag is set to "1"', () => {
    window.localStorage.setItem('theme_derive_from_slots', '1');
    expect(isSlotDerivationEnabled()).toBe(true);
    window.localStorage.removeItem('theme_derive_from_slots');
  });

  it('resolveAppThemeTokenSet uses derivation path when flag enabled', () => {
    window.localStorage.setItem('theme_derive_from_slots', '1');
    const tokens = resolveAppThemeTokenSet(SAMPLE_LIGHT_SLOTS, 'light');
    // 派生路径下 bgPrimary = surface slot 值
    expect(tokens.bgPrimary).toBe(SAMPLE_LIGHT_SLOTS.surface);
    window.localStorage.removeItem('theme_derive_from_slots');
  });

  it('resolveAppThemeTokenSet uses legacy path when flag disabled', () => {
    window.localStorage.removeItem('theme_derive_from_slots');
    const tokens = resolveAppThemeTokenSet(SAMPLE_LIGHT_SLOTS, 'light');
    // 旧路径下 bgPrimary = LIGHT_TOKENS.bgPrimary 硬编码值
    expect(tokens.bgPrimary).toBe(LIGHT_TOKENS.bgPrimary);
  });
});

describe('resolveAppThemeTokenSet', () => {
  it('produces a complete ThemeTokenSet from preset colors (light)', () => {
    const tokens = resolveAppThemeTokenSet(HARUBBLE_CLASSIC_COLORS, 'light');

    expect(tokens.accent).toBe('#FFE47A');
    expect(tokens.accentHover).toMatch(/^#[0-9A-F]{6}$/);
    expect(tokens.accentRgb).toBe('255, 228, 122');
    expect(tokens.accentReadableForeground).toMatch(/^#[0-9A-F]{6}$/);
    expect(tokens.bgPrimary).toBe(LIGHT_TOKENS.bgPrimary);
    expect(tokens.textPrimary).toBe(LIGHT_TOKENS.textPrimary);
    expect(tokens.border).toBe(LIGHT_TOKENS.border);
    expect(tokens.destructive).toBe('#C74F4F');
    expect(tokens.surfaceState).toMatch(/^rgba\(255, 228, 122, 0\.08\)$/);
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
