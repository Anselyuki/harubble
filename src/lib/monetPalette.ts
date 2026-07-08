import type { ThemeTokenSet } from './types';
import { hexToRgb, rgbToHex, deriveAccentHoverHex } from './theme';

type RgbTuple = [number, number, number];
type HslTuple = [number, number, number];

function rgbToHsl(rgb: RgbTuple): HslTuple {
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
  return [h * 360, s, l];
}

function hslToRgb(hsl: HslTuple): RgbTuple {
  const [h, s, l] = hsl;
  if (s === 0) {
    const v = Math.round(l * 255);
    return [v, v, v];
  }
  const hue2rgb = (p: number, q: number, t: number) => {
    const tt = t < 0 ? t + 1 : t > 1 ? t - 1 : t;
    if (tt < 1 / 6) return p + (q - p) * 6 * tt;
    if (tt < 1 / 2) return q;
    if (tt < 2 / 3) return p + (q - p) * (2 / 3 - tt) * 6;
    return p;
  };
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const hNorm = h / 360;
  return [
    Math.round(hue2rgb(p, q, hNorm + 1 / 3) * 255),
    Math.round(hue2rgb(p, q, hNorm) * 255),
    Math.round(hue2rgb(p, q, hNorm - 1 / 3) * 255),
  ];
}

function toneFromHsl(hsl: HslTuple, tone: number): RgbTuple {
  return hslToRgb([hsl[0], hsl[1], tone]);
}

function toLinearRgbChannel(value: number): number {
  const c = value / 255;
  return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

function getRelativeLuminance(rgb: RgbTuple): number {
  return (
    0.2126 * toLinearRgbChannel(rgb[0]) +
    0.7152 * toLinearRgbChannel(rgb[1]) +
    0.0722 * toLinearRgbChannel(rgb[2])
  );
}

function getContrastRatio(a: RgbTuple, b: RgbTuple): number {
  const la = getRelativeLuminance(a);
  const lb = getRelativeLuminance(b);
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

function getReadableForeground(bgRgb: RgbTuple): string {
  const mixRgb = (from: RgbTuple, to: RgbTuple, t: number): RgbTuple =>
    from.map((c, i) => Math.round(c + (to[i] - c) * t)) as RgbTuple;
  const darkTone = mixRgb(bgRgb, [0, 0, 0], 0.92);
  const lightTone = mixRgb(bgRgb, [255, 255, 255], 0.92);
  const target =
    getContrastRatio(bgRgb, darkTone) >= getContrastRatio(bgRgb, lightTone)
      ? darkTone
      : lightTone;
  let lo = 0;
  let hi = 1;
  let result = target;
  for (let i = 0; i < 12; i++) {
    const mid = (lo + hi) / 2;
    const candidate = mixRgb(bgRgb, target, mid);
    if (getContrastRatio(bgRgb, candidate) >= 4.5) {
      result = candidate;
      hi = mid;
    } else {
      lo = mid;
    }
  }
  return rgbToHex(result);
}

function deltaE76(a: RgbTuple, b: RgbTuple): number {
  const toLab = (rgb: RgbTuple): [number, number, number] => {
    let r = rgb[0] / 255;
    let g = rgb[1] / 255;
    let bl = rgb[2] / 255;
    r = r > 0.04045 ? Math.pow((r + 0.055) / 1.055, 2.4) : r / 12.92;
    g = g > 0.04045 ? Math.pow((g + 0.055) / 1.055, 2.4) : g / 12.92;
    bl = bl > 0.04045 ? Math.pow((bl + 0.055) / 1.055, 2.4) : bl / 12.92;
    let x = (r * 0.4124564 + g * 0.3575761 + bl * 0.1804375) / 0.95047;
    let y = r * 0.2126729 + g * 0.7151522 + bl * 0.072175;
    let z = (r * 0.0193339 + g * 0.119192 + bl * 0.9503041) / 1.08883;
    const f = (t: number) =>
      t > 0.008856 ? Math.cbrt(t) : 7.787 * t + 16 / 116;
    x = f(x);
    y = f(y);
    z = f(z);
    return [116 * y - 16, 500 * (x - y), 200 * (y - z)];
  };
  const labA = toLab(a);
  const labB = toLab(b);
  return Math.sqrt(
    (labA[0] - labB[0]) ** 2 +
      (labA[1] - labB[1]) ** 2 +
      (labA[2] - labB[2]) ** 2
  );
}

interface TonalPalette {
  accent: RgbTuple;
  secondary: RgbTuple;
  bgPrimary: RgbTuple;
  bgSecondary: RgbTuple;
  bgTertiary: RgbTuple;
  surfaceSidebar: RgbTuple;
  textPrimary: RgbTuple;
  textSecondary: RgbTuple;
  textTertiary: RgbTuple;
  border: RgbTuple;
}

function generateTonalPalette(
  seedHsl: HslTuple,
  scheme: 'light' | 'dark'
): TonalPalette {
  const hue = seedHsl[0];
  const sat = seedHsl[1];
  const neutralSat = Math.min(sat * 0.12, 0.06);

  if (scheme === 'light') {
    const accent = toneFromHsl([hue, Math.min(sat, 0.9), 0.45], 0.45);
    let secondary = toneFromHsl([(hue + 30) % 360, sat * 0.6, 0.4], 0.4);
    if (deltaE76(accent, secondary) < 15) {
      secondary = toneFromHsl([(hue + 60) % 360, sat * 0.7, 0.38], 0.38);
    }
    return {
      accent,
      secondary,
      bgPrimary: toneFromHsl([hue, neutralSat, 0.99], 0.99),
      bgSecondary: toneFromHsl([hue, neutralSat, 0.965], 0.965),
      bgTertiary: toneFromHsl([hue, neutralSat, 0.93], 0.93),
      surfaceSidebar: toneFromHsl([hue, neutralSat, 0.955], 0.955),
      textPrimary: toneFromHsl([hue, neutralSat * 2, 0.1], 0.1),
      textSecondary: toneFromHsl([hue, neutralSat * 1.5, 0.4], 0.4),
      textTertiary: toneFromHsl([hue, neutralSat, 0.53], 0.53),
      border: toneFromHsl([hue, neutralSat, 0.88], 0.88),
    };
  }

  const accent = toneFromHsl([hue, Math.min(sat, 0.85), 0.65], 0.65);
  let secondary = toneFromHsl([(hue + 30) % 360, sat * 0.55, 0.6], 0.6);
  if (deltaE76(accent, secondary) < 15) {
    secondary = toneFromHsl([(hue + 60) % 360, sat * 0.65, 0.58], 0.58);
  }
  return {
    accent,
    secondary,
    bgPrimary: toneFromHsl([hue, neutralSat, 0.06], 0.06),
    bgSecondary: toneFromHsl([hue, neutralSat, 0.1], 0.1),
    bgTertiary: toneFromHsl([hue, neutralSat, 0.14], 0.14),
    surfaceSidebar: toneFromHsl([hue, neutralSat, 0.09], 0.09),
    textPrimary: toneFromHsl([hue, neutralSat * 0.5, 0.95], 0.95),
    textSecondary: toneFromHsl([hue, neutralSat, 0.62], 0.62),
    textTertiary: toneFromHsl([hue, neutralSat, 0.42], 0.42),
    border: toneFromHsl([hue, neutralSat, 0.2], 0.2),
  };
}

export function resolveMonetTokenSet(
  seedHex: string,
  scheme: 'light' | 'dark'
): ThemeTokenSet {
  const seedRgb = hexToRgb(seedHex);
  const seedHsl = rgbToHsl(seedRgb);
  const palette = generateTonalPalette(seedHsl, scheme);
  const accentHex = rgbToHex(palette.accent);
  const accentHoverHex = deriveAccentHoverHex(accentHex);
  const accentRgb = hexToRgb(accentHex);
  const accentHoverRgb = hexToRgb(accentHoverHex);
  const destructiveRgb: RgbTuple = [199, 79, 79];

  return {
    accent: accentHex,
    accentHover: accentHoverHex,
    accentRgb: accentRgb.join(', '),
    accentHoverRgb: accentHoverRgb.join(', '),
    accentReadableForeground: getReadableForeground(accentRgb),
    accentHoverReadableForeground: getReadableForeground(accentHoverRgb),
    bgPrimary: rgbToHex(palette.bgPrimary),
    bgSecondary: rgbToHex(palette.bgSecondary),
    bgTertiary: rgbToHex(palette.bgTertiary),
    bgElevated:
      scheme === 'light'
        ? `rgba(${palette.bgPrimary.join(', ')}, 0.8)`
        : `rgba(${palette.bgSecondary.join(', ')}, 0.8)`,
    textPrimary: rgbToHex(palette.textPrimary),
    textSecondary: rgbToHex(palette.textSecondary),
    textTertiary: rgbToHex(palette.textTertiary),
    border:
      scheme === 'light'
        ? `rgba(${palette.border.join(', ')}, 0.35)`
        : `rgba(${palette.border.join(', ')}, 0.4)`,
    ring: `rgba(${accentRgb.join(', ')}, 0.3)`,
    destructive: rgbToHex(destructiveRgb),
    destructiveRgb: destructiveRgb.join(', '),
    surfaceState: `rgba(${accentRgb.join(', ')}, 0.08)`,
    surfaceBase: rgbToHex(palette.bgPrimary),
    surfaceSidebar: rgbToHex(palette.surfaceSidebar),
    surfaceOverlay:
      scheme === 'light'
        ? `rgba(${palette.bgSecondary.join(', ')}, 0.76)`
        : `rgba(${palette.bgSecondary.join(', ')}, 0.76)`,
  };
}

export {
  rgbToHsl,
  hslToRgb,
  deltaE76,
  getContrastRatio,
  getRelativeLuminance,
  getReadableForeground,
};
