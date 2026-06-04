import type { ThemeColorSlots, ThemePalette } from './types';

type RgbTuple = [number, number, number];

export const DEFAULT_THEME_PALETTE: ThemePalette = {
  accentHex: '#fa2d48',
  accentHoverHex: '#ff3b5c',
  accentRgb: [250, 45, 72],
  accentHoverRgb: [255, 59, 92],
  waveColors: [[250, 45, 72]],
};

export function hexToRgb(hex: string): RgbTuple {
  return [
    Number.parseInt(hex.slice(1, 3), 16),
    Number.parseInt(hex.slice(3, 5), 16),
    Number.parseInt(hex.slice(5, 7), 16),
  ];
}

function toLinearRgbChannel(value: number): number {
  const channel = value / 255;
  return channel <= 0.03928
    ? channel / 12.92
    : Math.pow((channel + 0.055) / 1.055, 2.4);
}

const MIN_TEXT_CONTRAST_RATIO = 4.5;

function getRelativeLuminance([red, green, blue]: RgbTuple): number {
  return (
    0.2126 * toLinearRgbChannel(red) +
    0.7152 * toLinearRgbChannel(green) +
    0.0722 * toLinearRgbChannel(blue)
  );
}

function getContrastRatio(firstRgb: RgbTuple, secondRgb: RgbTuple): number {
  const firstLuminance = getRelativeLuminance(firstRgb);
  const secondLuminance = getRelativeLuminance(secondRgb);
  const lighter = Math.max(firstLuminance, secondLuminance);
  const darker = Math.min(firstLuminance, secondLuminance);

  return (lighter + 0.05) / (darker + 0.05);
}

function mixRgb(fromRgb: RgbTuple, toRgb: RgbTuple, amount: number): RgbTuple {
  return fromRgb.map((channel, index) =>
    Math.round(channel + (toRgb[index] - channel) * amount)
  ) as RgbTuple;
}

export function rgbToHex(rgb: RgbTuple): string {
  return `#${rgb
    .map((channel) => channel.toString(16).padStart(2, '0'))
    .join('')}`.toUpperCase();
}

function getReadableForegroundColor(rgb: RgbTuple): string {
  const darkTone = mixRgb(rgb, [0, 0, 0], 0.92);
  const lightTone = mixRgb(rgb, [255, 255, 255], 0.92);
  const targetTone =
    getContrastRatio(rgb, darkTone) >= getContrastRatio(rgb, lightTone)
      ? darkTone
      : lightTone;

  let low = 0;
  let high = 1;
  let readableTone = targetTone;

  for (let index = 0; index < 12; index += 1) {
    const amount = (low + high) / 2;
    const candidate = mixRgb(rgb, targetTone, amount);
    const contrast = getContrastRatio(rgb, candidate);

    if (contrast >= MIN_TEXT_CONTRAST_RATIO) {
      readableTone = candidate;
      high = amount;
    } else {
      low = amount;
    }
  }

  return rgbToHex(readableTone);
}

export function deriveAccentHoverHex(accentHex: string): string {
  const accentRgb = hexToRgb(accentHex);
  const target: RgbTuple =
    getRelativeLuminance(accentRgb) > 0.45 ? [0, 0, 0] : [255, 255, 255];
  return rgbToHex(mixRgb(accentRgb, target, 0.12));
}

export function deriveThemeCssVariables(
  themeColors: ThemeColorSlots
): Record<string, string> {
  const accentRgb = hexToRgb(themeColors.accent);
  const accentHoverHex = deriveAccentHoverHex(themeColors.accent);
  const accentHoverRgb = hexToRgb(accentHoverHex);
  const surfaceRgb = hexToRgb(themeColors.surface);
  const tintRgb = hexToRgb(themeColors.tint);

  return {
    '--accent': themeColors.accent,
    '--accent-hover': accentHoverHex,
    '--accent-rgb': accentRgb.join(', '),
    '--accent-hover-rgb': accentHoverRgb.join(', '),
    '--accent-readable-foreground': getReadableForegroundColor(accentRgb),
    '--accent-hover-readable-foreground':
      getReadableForegroundColor(accentHoverRgb),
    '--theme-surface': themeColors.surface,
    '--theme-surface-rgb': surfaceRgb.join(', '),
    '--theme-text-primary': themeColors.textPrimary,
    '--theme-text-secondary': themeColors.textSecondary,
    '--theme-tint': themeColors.tint,
    '--theme-tint-rgb': tintRgb.join(', '),
    '--destructive': themeColors.danger,
  };
}

function applyCssVariables(nextValues: Record<string, string>): void {
  const root = document.documentElement;
  for (const [property, value] of Object.entries(nextValues)) {
    if (root.style.getPropertyValue(property) !== value) {
      root.style.setProperty(property, value);
    }
  }
}

export function applyThemeColors(themeColors: ThemeColorSlots): void {
  applyCssVariables(deriveThemeCssVariables(themeColors));
}

export function applyAlbumAccentPalette(
  palette: ThemePalette | null,
  baseThemeColors: ThemeColorSlots
): void {
  const accentHex = palette?.accentHex ?? baseThemeColors.accent;
  const accentHoverHex =
    palette?.accentHoverHex ?? deriveAccentHoverHex(baseThemeColors.accent);
  const accentRgb = palette?.accentRgb ?? hexToRgb(baseThemeColors.accent);
  const accentHoverRgb = palette?.accentHoverRgb ?? hexToRgb(accentHoverHex);
  const nextValues: Record<string, string> = {
    '--album-accent': accentHex,
    '--album-accent-hover': accentHoverHex,
    '--album-accent-rgb': accentRgb.join(', '),
    '--album-accent-hover-rgb': accentHoverRgb.join(', '),
    '--album-accent-readable-foreground': getReadableForegroundColor(accentRgb),
  };

  const colors =
    palette && palette.waveColors.length > 0 ? palette.waveColors : [accentRgb];
  for (let i = 0; i < 4; i += 1) {
    const rgb = colors[i % colors.length];
    nextValues[`--wave-color-${i}`] = `${rgb[0]}, ${rgb[1]}, ${rgb[2]}`;
  }

  applyCssVariables(nextValues);
}

export function applyThemePalette(
  palette: ThemePalette = DEFAULT_THEME_PALETTE
): void {
  const nextValues: Record<string, string> = {
    '--accent': palette.accentHex,
    '--accent-hover': palette.accentHoverHex,
    '--accent-rgb': palette.accentRgb.join(', '),
    '--accent-hover-rgb': palette.accentHoverRgb.join(', '),
    '--accent-readable-foreground': getReadableForegroundColor(
      palette.accentRgb
    ),
    '--accent-hover-readable-foreground': getReadableForegroundColor(
      palette.accentHoverRgb
    ),
  };

  const colors =
    palette.waveColors.length > 0 ? palette.waveColors : [palette.accentRgb];
  for (let i = 0; i < 4; i += 1) {
    const rgb = colors[i % colors.length];
    nextValues[`--wave-color-${i}`] = `${rgb[0]}, ${rgb[1]}, ${rgb[2]}`;
  }

  applyCssVariables(nextValues);
}
