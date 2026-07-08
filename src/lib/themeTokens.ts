import type { ThemeColorSlots, ThemePalette, ThemeTokenSet } from './types';
import {
  transitionCssVariables,
  hexToRgb,
  deriveAccentHoverHex,
  rgbToHex,
} from './theme';
import { getReadableForeground } from './monetPalette';

type RgbTuple = [number, number, number];

const LIGHT_TOKENS: Omit<
  ThemeTokenSet,
  | 'accent'
  | 'accentHover'
  | 'accentRgb'
  | 'accentHoverRgb'
  | 'accentReadableForeground'
  | 'accentHoverReadableForeground'
  | 'destructive'
  | 'destructiveRgb'
  | 'ring'
  | 'surfaceState'
> = {
  bgPrimary: '#ffffff',
  bgSecondary: '#f5f5f7',
  bgTertiary: '#e8e8ed',
  bgElevated: 'rgba(255, 255, 255, 0.8)',
  textPrimary: '#1d1d1f',
  textSecondary: '#6e6e73',
  textTertiary: '#86868b',
  border: 'rgba(0, 0, 0, 0.08)',
  surfaceBase: '#ffffff',
  surfaceSidebar: '#f0f0f2',
  surfaceOverlay: 'rgba(245, 245, 247, 0.76)',
};

const DARK_TOKENS: typeof LIGHT_TOKENS = {
  bgPrimary: '#000000',
  bgSecondary: '#1c1c1e',
  bgTertiary: '#2c2c2e',
  bgElevated: 'rgba(28, 28, 30, 0.8)',
  textPrimary: '#ffffff',
  textSecondary: '#8e8e93',
  textTertiary: '#636366',
  border: 'rgba(255, 255, 255, 0.08)',
  surfaceBase: '#000000',
  surfaceSidebar: '#1c1c1e',
  surfaceOverlay: 'rgba(28, 28, 30, 0.76)',
};

function getEffectiveScheme(): 'light' | 'dark' {
  return document.documentElement.classList.contains('dark') ? 'dark' : 'light';
}

export function resolveAppThemeTokenSet(
  themeColors: ThemeColorSlots,
  scheme: 'light' | 'dark' = getEffectiveScheme()
): ThemeTokenSet {
  const base = scheme === 'dark' ? DARK_TOKENS : LIGHT_TOKENS;
  const accentRgb = hexToRgb(themeColors.accent);
  const accentHoverHex = deriveAccentHoverHex(themeColors.accent);
  const accentHoverRgb = hexToRgb(accentHoverHex);
  const destructiveHex = themeColors.danger;
  const destructiveRgb = hexToRgb(destructiveHex);

  return {
    accent: themeColors.accent,
    accentHover: accentHoverHex,
    accentRgb: accentRgb.join(', '),
    accentHoverRgb: accentHoverRgb.join(', '),
    accentReadableForeground: getReadableForeground(accentRgb),
    accentHoverReadableForeground: getReadableForeground(accentHoverRgb),
    bgPrimary: base.bgPrimary,
    bgSecondary: base.bgSecondary,
    bgTertiary: base.bgTertiary,
    bgElevated: base.bgElevated,
    textPrimary: base.textPrimary,
    textSecondary: base.textSecondary,
    textTertiary: base.textTertiary,
    border: base.border,
    ring: `rgba(${accentRgb.join(', ')}, 0.3)`,
    destructive: destructiveHex,
    destructiveRgb: destructiveRgb.join(', '),
    surfaceState: `rgba(${accentRgb.join(', ')}, 0.08)`,
    surfaceBase: base.surfaceBase,
    surfaceSidebar: base.surfaceSidebar,
    surfaceOverlay: base.surfaceOverlay,
  };
}

const APP_TOKEN_CSS_MAP: Record<keyof ThemeTokenSet, string> = {
  accent: '--accent',
  accentHover: '--accent-hover',
  accentRgb: '--accent-rgb',
  accentHoverRgb: '--accent-hover-rgb',
  accentReadableForeground: '--accent-readable-foreground',
  accentHoverReadableForeground: '--accent-hover-readable-foreground',
  bgPrimary: '--bg-primary',
  bgSecondary: '--bg-secondary',
  bgTertiary: '--bg-tertiary',
  bgElevated: '--bg-elevated',
  textPrimary: '--text-primary',
  textSecondary: '--text-secondary',
  textTertiary: '--text-tertiary',
  border: '--border',
  ring: '--ring',
  destructive: '--destructive',
  destructiveRgb: '--destructive-rgb',
  surfaceState: '--surface-state',
  surfaceBase: '--surface-base',
  surfaceSidebar: '--surface-sidebar',
  surfaceOverlay: '--surface-overlay',
};

const ACCENT_TOKEN_KEYS: (keyof ThemeTokenSet)[] = [
  'accent',
  'accentHover',
  'accentRgb',
  'accentHoverRgb',
  'accentReadableForeground',
  'accentHoverReadableForeground',
  'ring',
  'surfaceState',
  'destructive',
  'destructiveRgb',
];

const CONTEXT_TOKEN_ALLOWLIST = [
  '--accent',
  '--accent-hover',
  '--accent-rgb',
  '--accent-hover-rgb',
  '--accent-readable-foreground',
  '--accent-hover-readable-foreground',
  '--ring',
  '--surface-state',
  '--album-accent',
  '--album-accent-hover',
  '--album-accent-rgb',
  '--album-accent-hover-rgb',
  '--album-accent-readable-foreground',
  '--wave-color-0',
  '--wave-color-1',
  '--wave-color-2',
  '--wave-color-3',
] as const;

export interface ApplyThemeOptions {
  target?: HTMLElement;
  animate?: boolean;
}

export function applyAppThemeTokenSet(
  tokens: ThemeTokenSet,
  options: ApplyThemeOptions = {}
): void {
  const { target = document.documentElement, animate = true } = options;

  const cssVars: Record<string, string> = {};
  for (const [key, cssVar] of Object.entries(APP_TOKEN_CSS_MAP)) {
    // accent token 由 applyContextThemePalette 写入，两者必须成对调用
    if (ACCENT_TOKEN_KEYS.includes(key as keyof ThemeTokenSet)) continue;
    cssVars[cssVar] = tokens[key as keyof ThemeTokenSet];
  }

  if (animate && target === document.documentElement) {
    transitionCssVariables(cssVars, 'app-tokens');
  } else {
    for (const [prop, value] of Object.entries(cssVars)) {
      target.style.setProperty(prop, value);
    }
  }
}

export function applyContextThemePalette(
  palette: ThemePalette | null,
  baseTokens: ThemeTokenSet,
  scheme: 'light' | 'dark' = 'light',
  options: ApplyThemeOptions = {}
): void {
  const { target = document.documentElement, animate = true } = options;

  const accentHex = palette?.accentHex ?? baseTokens.accent;
  const accentHoverHex = palette?.accentHoverHex ?? baseTokens.accentHover;
  const accentRgb: RgbTuple = palette?.accentRgb ?? hexToRgb(baseTokens.accent);
  const accentHoverRgb: RgbTuple =
    palette?.accentHoverRgb ?? hexToRgb(accentHoverHex);
  const readableFg = getReadableForeground(accentRgb);

  const adapted = palette ? adaptPaletteToScheme(palette, scheme) : null;

  const surfaceHex = adapted?.surface ?? baseTokens.bgSecondary;
  const surfaceRgb = hexToRgb(surfaceHex);
  const textPrimaryHex = adapted?.textPrimary ?? baseTokens.textPrimary;
  const textSecondaryHex = adapted?.textSecondary ?? baseTokens.textSecondary;
  const tintHex = adapted?.tint ?? baseTokens.textSecondary;
  const tintRgb = hexToRgb(tintHex);
  const dangerHex = adapted?.danger ?? baseTokens.destructive;
  const dangerRgb = hexToRgb(dangerHex);

  const cssVars: Record<string, string> = {
    '--accent': accentHex,
    '--accent-hover': accentHoverHex,
    '--accent-rgb': accentRgb.join(', '),
    '--accent-hover-rgb': accentHoverRgb.join(', '),
    '--accent-readable-foreground': readableFg,
    '--accent-hover-readable-foreground': getReadableForeground(accentHoverRgb),
    '--ring': `rgba(${accentRgb.join(', ')}, 0.3)`,
    '--surface-state': `rgba(${accentRgb.join(', ')}, 0.08)`,
    '--theme-surface': surfaceHex,
    '--theme-surface-rgb': surfaceRgb.join(', '),
    '--theme-text-primary': textPrimaryHex,
    '--theme-text-secondary': textSecondaryHex,
    '--theme-tint': tintHex,
    '--theme-tint-rgb': tintRgb.join(', '),
    '--destructive': dangerHex,
    '--destructive-rgb': dangerRgb.join(', '),
    '--album-accent': accentHex,
    '--album-accent-hover': accentHoverHex,
    '--album-accent-rgb': accentRgb.join(', '),
    '--album-accent-hover-rgb': accentHoverRgb.join(', '),
    '--album-accent-readable-foreground': readableFg,
  };

  const colors =
    palette && palette.waveColors.length > 0 ? palette.waveColors : [accentRgb];
  for (let i = 0; i < 4; i++) {
    const rgb = colors[i % colors.length];
    cssVars[`--wave-color-${i}`] = `${rgb[0]}, ${rgb[1]}, ${rgb[2]}`;
  }

  if (animate && target === document.documentElement) {
    transitionCssVariables(cssVars, 'context-palette');
  } else {
    for (const [prop, value] of Object.entries(cssVars)) {
      target.style.setProperty(prop, value);
    }
  }
}

function adaptPaletteToScheme(
  palette: ThemePalette,
  scheme: 'light' | 'dark'
): {
  surface: string;
  textPrimary: string;
  textSecondary: string;
  tint: string;
  danger: string;
} {
  if (scheme === 'light') {
    return {
      surface: palette.surfaceHex,
      textPrimary: palette.textPrimaryHex,
      textSecondary: palette.textSecondaryHex,
      tint: palette.tintHex,
      danger: palette.dangerHex,
    };
  }

  const surfaceRgb = hexToRgb(palette.surfaceHex);
  const textPrimaryRgb = hexToRgb(palette.textPrimaryHex);
  const textSecondaryRgb = hexToRgb(palette.textSecondaryHex);
  const tintRgb = hexToRgb(palette.tintHex);

  return {
    surface: rgbToHex(invertLightness(surfaceRgb)),
    textPrimary: rgbToHex(invertLightness(textPrimaryRgb)),
    textSecondary: rgbToHex(invertLightness(textSecondaryRgb)),
    tint: rgbToHex(adjustLightnessForDark(tintRgb)),
    danger: palette.dangerHex,
  };
}

function rgbToHsl(rgb: RgbTuple): [number, number, number] {
  const r = rgb[0] / 255;
  const g = rgb[1] / 255;
  const b = rgb[2] / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h: number;
  if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
  else if (max === g) h = ((b - r) / d + 2) / 6;
  else h = ((r - g) / d + 4) / 6;
  return [h, s, l];
}

function hslToRgb(h: number, s: number, l: number): RgbTuple {
  if (s === 0) {
    const v = Math.round(l * 255);
    return [v, v, v];
  }
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const hue2rgb = (pp: number, qq: number, t: number) => {
    const tt = t < 0 ? t + 1 : t > 1 ? t - 1 : t;
    if (tt < 1 / 6) return pp + (qq - pp) * 6 * tt;
    if (tt < 1 / 2) return qq;
    if (tt < 2 / 3) return pp + (qq - pp) * (2 / 3 - tt) * 6;
    return pp;
  };
  return [
    Math.round(hue2rgb(p, q, h + 1 / 3) * 255),
    Math.round(hue2rgb(p, q, h) * 255),
    Math.round(hue2rgb(p, q, h - 1 / 3) * 255),
  ];
}

function invertLightness(rgb: RgbTuple): RgbTuple {
  const [h, s, l] = rgbToHsl(rgb);
  return hslToRgb(h, s, 1 - l);
}

function adjustLightnessForDark(rgb: RgbTuple): RgbTuple {
  const [h, s, l] = rgbToHsl(rgb);
  return hslToRgb(h, Math.min(s, 0.5), Math.max(l, 0.55));
}

export { LIGHT_TOKENS, DARK_TOKENS, CONTEXT_TOKEN_ALLOWLIST };
