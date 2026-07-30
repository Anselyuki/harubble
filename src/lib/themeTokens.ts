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
  | 'tint'
  | 'tintRgb'
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

const FEATURE_FLAG_KEY = 'theme_derive_from_slots';

/**
 * 检查是否启用"从 slot 派生全局 token"的 feature flag。
 *
 * 优先级：环境变量 > localStorage > 默认关闭。
 * 启用后 `resolveAppThemeTokenSet` 会走 `deriveGlobalTokensFromSlots` 派生路径，
 * 关闭时继续走 LIGHT_TOKENS/DARK_TOKENS 硬编码常量查表。
 *
 * 用途：Phase 0 灰度切换保护，出现视觉退化时可即时回滚（无需发版）。
 * SSR 与非浏览器环境（如测试 Node 环境）安全返回 false。
 */
export function isSlotDerivationEnabled(): boolean {
  if (typeof window === 'undefined') return false;
  try {
    return window.localStorage.getItem(FEATURE_FLAG_KEY) === '1';
  } catch {
    return false;
  }
}

function mixColors(a: RgbTuple, b: RgbTuple, ratio: number): RgbTuple {
  return [
    Math.round(a[0] * (1 - ratio) + b[0] * ratio),
    Math.round(a[1] * (1 - ratio) + b[1] * ratio),
    Math.round(a[2] * (1 - ratio) + b[2] * ratio),
  ];
}

function adjustSidebarLightness(
  surfaceRgb: RgbTuple,
  scheme: 'light' | 'dark'
): string {
  const [h, s, l] = rgbToHsl(surfaceRgb);
  const delta = scheme === 'dark' ? 0.05 : -0.04;
  return rgbToHex(hslToRgb(h, s, Math.max(0, Math.min(1, l + delta))));
}

function deriveTextTertiary(
  textSecondaryRgb: RgbTuple,
  scheme: 'light' | 'dark'
): string {
  const [h, s, l] = rgbToHsl(textSecondaryRgb);
  const newS = Math.max(0, s - 0.05);
  const newL =
    scheme === 'light' ? Math.min(1, l + 0.1) : Math.max(0, l - 0.08);
  return rgbToHex(hslToRgb(h, newS, newL));
}

/**
 * 从 6 个颜色 slot 派生完整的 23 个全局 ThemeTokenSet。
 *
 * 用途：App Theme 路径的唯一 token 来源，替代硬编码的 LIGHT_TOKENS/DARK_TOKENS 常量查表。
 * 入参：ThemeColorSlots 6 个 slot（accent/surface/textPrimary/textSecondary/tint/danger）
 *       + scheme（light/dark，仅影响混合比例与方向）。
 * 出参：21 个字段完整填充的 ThemeTokenSet。
 * 副作用：无（纯函数）。
 *
 * 派生规则（与 color-slot-activation.md §3 对齐）：
 * - surface → bgPrimary、surfaceBase（直接）、surfaceSidebar（±亮度 4-5%）、surfaceOverlay（+76% alpha）
 * - textPrimary → textPrimary（直接）
 * - textSecondary → textSecondary（直接）、textTertiary（降饱和+调亮度）
 * - tint → bgSecondary（surface+tint 8-12% 混合）、bgTertiary（16-20% 混合）、bgElevated（bgSecondary+80% alpha）、border（tint+8% alpha）
 * - accent → accent/accentHover/accentRgb/ring/surfaceState 全套派生
 * - danger → destructive/destructiveRgb
 */
export function deriveGlobalTokensFromSlots(
  slots: ThemeColorSlots,
  scheme: 'light' | 'dark'
): ThemeTokenSet {
  const surfaceRgb = hexToRgb(slots.surface);
  const tintRgb = hexToRgb(slots.tint);
  const textSecondaryRgb = hexToRgb(slots.textSecondary);
  const accentRgb = hexToRgb(slots.accent);
  const accentHoverHex = deriveAccentHoverHex(slots.accent);
  const accentHoverRgb = hexToRgb(accentHoverHex);
  const destructiveRgb = hexToRgb(slots.danger);

  // Step 0.c 校准值：与 LIGHT_TOKENS/DARK_TOKENS 精确匹配的混合系数
  // - 浅色 k1=0.08 / k2=0.184：配合 tint=#82829B 得到 bgSecondary=#F5F5F7、bgTertiary=#E8E8ED
  // - 深色 k1=0.12 / k2=0.187：配合 tint=#E9E9FA 得到 bgSecondary=#1C1C1E、bgTertiary≈#2C2C2E
  const secondaryRatio = scheme === 'dark' ? 0.12 : 0.08;
  const tertiaryRatio = scheme === 'dark' ? 0.187 : 0.184;
  const bgSecondaryRgb = mixColors(surfaceRgb, tintRgb, secondaryRatio);
  const bgTertiaryRgb = mixColors(surfaceRgb, tintRgb, tertiaryRatio);

  const border =
    scheme === 'dark'
      ? `rgba(255, 255, 255, 0.08)`
      : `rgba(${tintRgb.join(', ')}, 0.08)`;

  return {
    accent: slots.accent,
    accentHover: accentHoverHex,
    accentRgb: accentRgb.join(', '),
    accentHoverRgb: accentHoverRgb.join(', '),
    accentReadableForeground: getReadableForeground(accentRgb),
    accentHoverReadableForeground: getReadableForeground(accentHoverRgb),
    bgPrimary: slots.surface,
    bgSecondary: rgbToHex(bgSecondaryRgb),
    bgTertiary: rgbToHex(bgTertiaryRgb),
    bgElevated: `rgba(${bgSecondaryRgb.join(', ')}, 0.8)`,
    textPrimary: slots.textPrimary,
    textSecondary: slots.textSecondary,
    textTertiary: deriveTextTertiary(textSecondaryRgb, scheme),
    tint: slots.tint,
    tintRgb: tintRgb.join(', '),
    border,
    ring: `rgba(${accentRgb.join(', ')}, 0.3)`,
    destructive: slots.danger,
    destructiveRgb: destructiveRgb.join(', '),
    surfaceState: `rgba(${accentRgb.join(', ')}, 0.08)`,
    surfaceBase: slots.surface,
    surfaceSidebar: adjustSidebarLightness(surfaceRgb, scheme),
    surfaceOverlay: `rgba(${surfaceRgb.join(', ')}, 0.76)`,
  };
}

export function resolveAppThemeTokenSet(
  themeColors: ThemeColorSlots,
  scheme: 'light' | 'dark' = getEffectiveScheme()
): ThemeTokenSet {
  // Phase 0 Step 0.d：feature flag 保护的派生路径切换
  if (isSlotDerivationEnabled()) {
    return deriveGlobalTokensFromSlots(themeColors, scheme);
  }

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
    tint: themeColors.tint,
    tintRgb: hexToRgb(themeColors.tint).join(', '),
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
  tint: '--theme-tint',
  tintRgb: '--theme-tint-rgb',
  border: '--border',
  ring: '--ring',
  destructive: '--destructive',
  destructiveRgb: '--destructive-rgb',
  surfaceState: '--surface-state',
  surfaceBase: '--surface-base',
  surfaceSidebar: '--surface-sidebar',
  surfaceOverlay: '--surface-overlay',
};

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
    cssVars[cssVar] = tokens[key as keyof ThemeTokenSet];
  }
  cssVars['--theme-surface'] = tokens.bgPrimary;
  cssVars['--theme-surface-rgb'] = hexToRgb(tokens.bgPrimary).join(', ');
  cssVars['--theme-accent'] = tokens.accent;
  cssVars['--theme-text-primary'] = tokens.textPrimary;
  cssVars['--theme-text-secondary'] = tokens.textSecondary;

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
  _scheme: 'light' | 'dark' = 'light',
  options: ApplyThemeOptions = {}
): void {
  const { target = document.documentElement, animate = true } = options;

  const accentHex = palette?.accentHex ?? baseTokens.accent;
  const accentHoverHex = palette?.accentHoverHex ?? baseTokens.accentHover;
  const accentRgb: RgbTuple = palette?.accentRgb ?? hexToRgb(baseTokens.accent);
  const accentHoverRgb: RgbTuple =
    palette?.accentHoverRgb ?? hexToRgb(accentHoverHex);
  const readableFg = getReadableForeground(accentRgb);

  const cssVars: Record<string, string> = {
    '--accent': accentHex,
    '--accent-hover': accentHoverHex,
    '--accent-rgb': accentRgb.join(', '),
    '--accent-hover-rgb': accentHoverRgb.join(', '),
    '--accent-readable-foreground': readableFg,
    '--accent-hover-readable-foreground': getReadableForeground(accentHoverRgb),
    '--ring': `rgba(${accentRgb.join(', ')}, 0.3)`,
    '--surface-state': `rgba(${accentRgb.join(', ')}, 0.08)`,
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

export { LIGHT_TOKENS, DARK_TOKENS, CONTEXT_TOKEN_ALLOWLIST };
