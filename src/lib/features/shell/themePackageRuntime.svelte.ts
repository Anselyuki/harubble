import type {
  ThemeColorSlots,
  ThemePackageDocument,
  ThemePreferences,
} from '$lib/types';
import {
  THEME_COLOR_SLOTS,
  getThemePreset,
  normalizeThemeHex,
} from '$lib/themePresets';

/**
 * 当前实际渲染的主题包文档。它既可以来自 committed package，也可以来自预览态。
 * App Theme 的颜色派生由 themeManager 消费，其他 package token 仍由
 * themePackageManager 统一写入 DOM。
 */
const runtimeState = $state<{
  document: ThemePackageDocument | null;
}>({
  document: null,
});

export function getThemePackageRuntime(): {
  readonly document: ThemePackageDocument | null;
} {
  return runtimeState;
}

export function setThemePackageRuntimeDocument(
  document: ThemePackageDocument | null
): void {
  runtimeState.document = document;
}

function applySlotOverrides(
  target: ThemeColorSlots,
  overrides: Partial<ThemeColorSlots> | null | undefined
): void {
  if (!overrides) return;
  for (const slot of THEME_COLOR_SLOTS) {
    const raw = overrides[slot];
    if (typeof raw !== 'string') continue;
    const normalized = normalizeThemeHex(raw);
    if (normalized) target[slot] = normalized;
  }
}

/**
 * 解析主题包的六个 App Theme 色槽。
 *
 * 无主题包时沿用 preset < user overrides；主题包激活或预览时使用
 * preset 基底 < package slots < package scheme variant。
 * legacy customColors 是 preset 的单套覆盖，不能跨 light/dark 覆盖主题包变体；
 * 它们会保留在偏好中，并在停用主题包后恢复生效。
 * 专辑 Context Theme 不在这里混入，由 themeManager 在完整 token 派生后通过
 * applyContextThemePalette 的 allowlist 单独覆盖。
 */
export function resolveThemePackageColors(
  preferences: Pick<ThemePreferences, 'presetId' | 'customColors'>,
  document: ThemePackageDocument | null,
  scheme: 'light' | 'dark'
): ThemeColorSlots {
  const colors: ThemeColorSlots = {
    ...getThemePreset(preferences.presetId).colors,
  };
  if (document) {
    applySlotOverrides(colors, document.slots);
    applySlotOverrides(colors, document.variants?.[scheme]);
  } else {
    applySlotOverrides(colors, preferences.customColors);
  }
  return colors;
}
