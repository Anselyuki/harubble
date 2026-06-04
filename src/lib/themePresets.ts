import type {
  ThemeColorSlot,
  ThemeColorSlots,
  ThemePreferences,
} from './types';

export type { ThemeColorSlot, ThemeColorSlots, ThemePreferences };

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
