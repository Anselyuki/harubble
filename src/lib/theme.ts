import type { ThemeColorSlots, ThemePalette } from './types';
import { gsap, getMotionDuration, MOTION } from '$lib/design/gsap';

type RgbTuple = [number, number, number];

export const DEFAULT_THEME_PALETTE: ThemePalette = {
  accentHex: '#fa2d48',
  accentHoverHex: '#ff3b5c',
  accentRgb: [250, 45, 72],
  accentHoverRgb: [255, 59, 92],
  waveColors: [[250, 45, 72]],
};

let activeTween: gsap.core.Tween | null = null;

const THEME_TRANSITION_MS = MOTION.SLOW;

function hexToRgbTuple(hex: string): RgbTuple {
  return [
    Number.parseInt(hex.slice(1, 3), 16),
    Number.parseInt(hex.slice(3, 5), 16),
    Number.parseInt(hex.slice(5, 7), 16),
  ];
}

function rgbTupleToHex(rgb: RgbTuple): string {
  return `#${rgb
    .map((channel) => channel.toString(16).padStart(2, '0'))
    .join('')}`.toLowerCase();
}

function parseRgbString(str: string): RgbTuple | null {
  const parts = str.split(',').map((s) => parseInt(s.trim(), 10));
  if (parts.some((n) => isNaN(n))) return null;
  return [parts[0]!, parts[1]!, parts[2]!] as RgbTuple;
}

function isRgbString(str: string): boolean {
  return /^\d+\s*,\s*\d+\s*,\s*\d+$/.test(str);
}

function isHexColor(str: string): boolean {
  return /^#[0-9A-Fa-f]{3,8}$/.test(str);
}

/**
 * 将 CSS 变量值转换为可插值的 RGB 三元组。
 *
 * 支持两种格式：
 * - Hex 颜色（`#rrggbb`）→ 解析为 RGB
 * - 逗号分隔 RGB 字符串（`r, g, b`）→ 直接拆分
 *
 * 非颜色值返回 null，由上层直接设置。
 *
 * @param name CSS 变量名，用于 proxy 对象的键
 * @param value CSS 变量值
 * @returns { key: string, rgb: RgbTuple } 或 null
 */
function extractRgbValue(
  name: string,
  value: string
): { key: string; rgb: RgbTuple; isRgbString: boolean } | null {
  if (isHexColor(value)) {
    return { key: name, rgb: hexToRgbTuple(value), isRgbString: false };
  }
  if (isRgbString(value)) {
    const rgb = parseRgbString(value);
    if (!rgb) return null;
    return { key: name, rgb, isRgbString: true };
  }
  return null;
}

/**
 * 将一组 CSS 变量转换为可插值的 RGB proxy 对象。
 *
 * 每个颜色变量映射到 proxy 上的 `_r / _g / _b` 三个数值通道，
 * 初始值为当前 CSS 变量的 RGB 值。
 *
 * @param values 当前生效的 CSS 变量值
 * @param targets 目标 CSS 变量值
 * @returns { proxy: Record<string, number>, isRgbString: Map<string, boolean>, directKeys: string[] }
 */
function buildProxy(
  values: Record<string, string>,
  targets: Record<string, string>
): {
  proxy: Record<string, number>;
  isRgbString: Map<string, boolean>;
  directKeys: string[];
} {
  const proxy: Record<string, number> = {};
  const isRgbString = new Map<string, boolean>();
  const directKeys: string[] = [];

  for (const [key, target] of Object.entries(targets)) {
    const current = values[key] ?? '';
    const extracted = extractRgbValue(key, target);
    if (!extracted) {
      directKeys.push(key);
      continue;
    }
    const baseKey = extracted.key.replace(/^-+/, '');
    // 初始值取当前 CSS 变量的 RGB（而非目标值）
    const currentExtracted = extractRgbValue(key, current);
    const initialRgb = currentExtracted?.rgb ?? [0, 0, 0];
    proxy[`${baseKey}_r`] = initialRgb[0];
    proxy[`${baseKey}_g`] = initialRgb[1];
    proxy[`${baseKey}_b`] = initialRgb[2];
    isRgbString.set(key, extracted.isRgbString);
  }

  return { proxy, isRgbString, directKeys };
}

/**
 * 对一组 CSS 变量进行渐变色过渡。
 *
 * 对于每个颜色变量，从当前值插值到目标值，使用 iOS 缓动曲线。
 * 如果存在活跃的同名 tween，会先被 kill 掉以避免冲突。
 *
 * 降级策略：
 * - 如果用户偏好减少动画，则即时设置所有值
 *
 * @param targets 目标 CSS 变量值
 */
export function transitionCssVariables(targets: Record<string, string>): void {
  const root = document.documentElement;
  const duration = getMotionDuration(THEME_TRANSITION_MS);

  // 快速路径：没有旧值（首次设置），直接写入
  const firstKey = Object.keys(targets)[0];
  if (!firstKey || !root.style.getPropertyValue(firstKey)) {
    applyCssVariables(targets);
    return;
  }

  // 收集当前值和目标值中需要插值的颜色
  const currentValues: Record<string, string> = {};
  const colorTargets: Record<string, string> = {};

  for (const [key, value] of Object.entries(targets)) {
    currentValues[key] = root.style.getPropertyValue(key) || '';
    if (isHexColor(value) || isRgbString(value)) {
      colorTargets[key] = value;
    } else {
      currentValues[key] = value;
    }
  }

  // 如果没有颜色需要过渡，直接设置
  if (Object.keys(colorTargets).length === 0) {
    applyCssVariables(targets);
    return;
  }

  // 构建 proxy 对象
  const {
    proxy,
    isRgbString: rgbStringMap,
    directKeys,
  } = buildProxy(currentValues, colorTargets);

  // 如果有非颜色值，直接设置
  for (const key of directKeys) {
    root.style.setProperty(key, targets[key]!);
  }

  // kill 前一个 tween
  if (activeTween) {
    activeTween.kill();
  }

  activeTween = gsap.to(proxy, {
    duration,
    ease: 'ios',
    onUpdate: () => {
      for (const [cssKey, rgbString] of rgbStringMap) {
        const baseKey = cssKey.replace(/^-+/, '');
        const r = Math.round(proxy[`${baseKey}_r`] ?? 0);
        const g = Math.round(proxy[`${baseKey}_g`] ?? 0);
        const b = Math.round(proxy[`${baseKey}_b`] ?? 0);
        const rgb = [r, g, b] as RgbTuple;
        const value = rgbString
          ? `${rgb[0]}, ${rgb[1]}, ${rgb[2]}`
          : rgbTupleToHex(rgb);
        root.style.setProperty(cssKey, value);
      }
    },
    onComplete: () => {
      // 确保最终值精确
      for (const key of directKeys) {
        root.style.setProperty(key, targets[key]!);
      }
      activeTween = null;
    },
  });
}

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
  transitionCssVariables(deriveThemeCssVariables(themeColors));
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

  transitionCssVariables(nextValues);
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

  transitionCssVariables(nextValues);
}
