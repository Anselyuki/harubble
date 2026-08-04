import type {
  ColorScheme,
  ThemeColorSlot,
  ThemeColorSlots,
  ThemePreferences,
} from './types';

export type { ColorScheme, ThemeColorSlot, ThemeColorSlots, ThemePreferences };

export interface ThemePreset {
  id: string;
  label: string;
  description: string;
  colors: ThemeColorSlots;
}

export const THEME_COLOR_SLOTS = [
  'accent',
  'surface',
  'textPrimary',
  'textSecondary',
  'tint',
  'danger',
] as const satisfies readonly ThemeColorSlot[];

export const DEFAULT_THEME_PRESET_ID = 'harubble-classic';

/**
 * Phase 0 Step 0.c 校准的系统内置浅色 slot 组合。
 *
 * 用途：`deriveGlobalTokensFromSlots(SYSTEM_LIGHT_SLOTS, 'light')` 与旧硬编码
 * `LIGHT_TOKENS` 的 bg/text/surface 系列 token 逐字段匹配（tint 混合系数配合 0.08/0.184）。
 * 派生输出与旧路径的最大差异 ≤ 1 RGB 通道（因四舍五入产生，视觉不可辨）。
 *
 * 与用户可导入的主题包 slot 声明结构一致，未来可作为 `system-light` 内置主题包导出。
 */
export const SYSTEM_LIGHT_SLOTS: ThemeColorSlots = {
  accent: '#7c3aed',
  surface: '#ffffff',
  textPrimary: '#1d1d1f',
  textSecondary: '#6e6e73',
  tint: '#82829B',
  danger: '#ef4444',
};

/**
 * Phase 0 Step 0.c 校准的系统内置深色 slot 组合。
 *
 * 用途：`deriveGlobalTokensFromSlots(SYSTEM_DARK_SLOTS, 'dark')` 与旧硬编码
 * `DARK_TOKENS` 的 bg/text/surface 系列 token 匹配（tint 混合系数配合 0.12/0.187）。
 * 派生输出与旧路径 bgSecondary R/G/B 差异 ≤ 1，textTertiary 通过降饱和+减亮度公式派生。
 */
export const SYSTEM_DARK_SLOTS: ThemeColorSlots = {
  accent: '#7c3aed',
  surface: '#000000',
  textPrimary: '#ffffff',
  textSecondary: '#8e8e93',
  tint: '#E9E9FA',
  danger: '#ef4444',
};

export const HARUBBLE_CLASSIC_COLORS: ThemeColorSlots = {
  accent: '#FFE47A',
  surface: '#D1D6DB',
  textPrimary: '#4A5056',
  textSecondary: '#596066',
  tint: '#899CB0',
  danger: '#C74F4F',
};

export const THEME_PRESETS: ThemePreset[] = [
  {
    id: DEFAULT_THEME_PRESET_ID,
    label: 'settings_theme_preset_harubble_classic_name',
    description: 'settings_theme_preset_harubble_classic_description',
    colors: HARUBBLE_CLASSIC_COLORS,
  },
  {
    id: 'clear-aqua',
    label: 'settings_theme_preset_clear_aqua_name',
    description: 'settings_theme_preset_clear_aqua_description',
    colors: {
      accent: '#2FC6D6',
      surface: '#DDEFF1',
      textPrimary: '#243B43',
      textSecondary: '#55727A',
      tint: '#9BD6DE',
      danger: '#D95F5F',
    },
  },
  {
    id: 'night-console',
    label: 'settings_theme_preset_night_console_name',
    description: 'settings_theme_preset_night_console_description',
    colors: {
      accent: '#7CFF8A',
      surface: '#20262A',
      textPrimary: '#E5F3E7',
      textSecondary: '#9FB3A6',
      tint: '#3F6E59',
      danger: '#FF6B6B',
    },
  },
];

export const DEFAULT_THEME_PREFERENCES: ThemePreferences = {
  presetId: DEFAULT_THEME_PRESET_ID,
  customColors: {},
  colorScheme: 'auto',
  dynamicAlbumAccent: true,
};

export function isThemeColorSlot(value: string): value is ThemeColorSlot {
  return THEME_COLOR_SLOTS.includes(value as ThemeColorSlot);
}

export function isValidThemeHex(value: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(value);
}

export function normalizeThemeHex(value: string): string | null {
  const trimmed = value.trim();
  return isValidThemeHex(trimmed) ? trimmed.toUpperCase() : null;
}

export function getThemePreset(
  presetId: string | null | undefined
): ThemePreset {
  return (
    THEME_PRESETS.find((preset) => preset.id === presetId) ?? THEME_PRESETS[0]
  );
}

export function resolveThemeColors(
  preferences: ThemePreferences | null | undefined
): ThemeColorSlots {
  const preset = getThemePreset(preferences?.presetId);
  const colors: ThemeColorSlots = { ...preset.colors };

  for (const [slot, value] of Object.entries(preferences?.customColors ?? {})) {
    if (!isThemeColorSlot(slot) || typeof value !== 'string') continue;
    const normalized = normalizeThemeHex(value);
    if (normalized) {
      colors[slot] = normalized;
    }
  }

  return colors;
}
