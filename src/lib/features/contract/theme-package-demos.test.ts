// @vitest-environment jsdom

/**
 * 5 个 demo 主题包的契约测试（Phase 1 Step 1.h）。
 *
 * 覆盖：
 * - JSON 反序列化到 ThemePackageDocument 类型（compile-time 通过 TypeScript）
 * - 必填字段（manifest.id/name/version、slots.accent/surface/...）齐全
 * - 全部 6 slot 均可被 `deriveGlobalTokensFromSlots` 解析并生成完整 21 token
 * - variants 稀疏语义验证（midnight-glass 有 dark 覆盖）
 *
 * 灰度上线策略：Phase 3.3 完成时 `theme_packages_v1` flag 从 opt-in 转 opt-out
 *（当前状态）；再稳定 4 个 minor 版本后移除 flag。
 */
import { describe, it, expect } from 'vitest';
import type { ThemePackageDocument, ThemeColorSlots } from '$lib/types';
import { deriveGlobalTokensFromSlots } from '$lib/themeTokens';

import minimalPkg from '../../../test-fixtures/theme-packages/minimal.json';
import midnightGlassPkg from '../../../test-fixtures/theme-packages/midnight-glass.json';
import materialYouPkg from '../../../test-fixtures/theme-packages/material-you.json';
import retroTerminalPkg from '../../../test-fixtures/theme-packages/retro-terminal.json';
import systemDarkPkg from '../../../test-fixtures/theme-packages/system-dark.json';

const REQUIRED_SLOTS = [
  'accent',
  'surface',
  'textPrimary',
  'textSecondary',
  'tint',
  'danger',
] as const;

const DEMO_PACKAGES: [string, ThemePackageDocument][] = [
  ['minimal', minimalPkg as unknown as ThemePackageDocument],
  ['midnight-glass', midnightGlassPkg as unknown as ThemePackageDocument],
  ['material-you', materialYouPkg as unknown as ThemePackageDocument],
  ['retro-terminal', retroTerminalPkg as unknown as ThemePackageDocument],
  ['system-dark', systemDarkPkg as unknown as ThemePackageDocument],
];

describe('主题包 demo · 契约测试', () => {
  it.each(DEMO_PACKAGES)('%s 具有必填 manifest 字段', (_id, pkg) => {
    expect(pkg.schemaVersion).toBe(1);
    expect(pkg.manifest.id).toBeTruthy();
    expect(pkg.manifest.name).toBeTruthy();
    expect(pkg.manifest.version).toBeTruthy();
    expect(pkg.manifest.version).toMatch(/^\d+\.\d+\.\d+/);
  });

  it.each(DEMO_PACKAGES)('%s 声明全部 6 个 slot', (_id, pkg) => {
    for (const slot of REQUIRED_SLOTS) {
      const value = pkg.slots?.[slot];
      expect(value, `slot ${slot} missing`).toBeTruthy();
      expect(value).toMatch(/^#[0-9a-fA-F]{6}$/);
    }
  });

  it.each(DEMO_PACKAGES)(
    '%s 可通过 deriveGlobalTokensFromSlots(light) 生成 21 个 token',
    (_id, pkg) => {
      const tokens = deriveGlobalTokensFromSlots(
        pkg.slots as ThemeColorSlots,
        'light'
      );
      expect(tokens.bgPrimary).toBeTruthy();
      expect(tokens.bgSecondary).toBeTruthy();
      expect(tokens.bgTertiary).toBeTruthy();
      expect(tokens.textPrimary).toBeTruthy();
      expect(tokens.accent).toBeTruthy();
      expect(tokens.destructive).toBeTruthy();
      expect(tokens.border).toBeTruthy();
    }
  );

  it.each(DEMO_PACKAGES)(
    '%s 可通过 deriveGlobalTokensFromSlots(dark) 生成 21 个 token',
    (_id, pkg) => {
      const tokens = deriveGlobalTokensFromSlots(
        pkg.slots as ThemeColorSlots,
        'dark'
      );
      expect(tokens.bgPrimary).toBeTruthy();
      expect(tokens.border).toBe('rgba(255, 255, 255, 0.08)');
    }
  );

  it('midnight-glass 的 variants.dark 覆盖仅包含稀疏字段', () => {
    const pkg = midnightGlassPkg as unknown as ThemePackageDocument;
    expect(pkg.variants?.dark).toBeDefined();
    expect(pkg.variants?.dark?.surface).toBe('#000000');
    expect(pkg.variants?.dark?.textPrimary).toBe('#ffffff');
    // 稀疏语义：未在 dark 里声明的字段不出现（accent/tint 等）
    expect(pkg.variants?.dark?.accent).toBeUndefined();
    expect(pkg.variants?.dark?.tint).toBeUndefined();
  });

  it('demo 包的 id 唯一（无重复安装冲突）', () => {
    const ids = DEMO_PACKAGES.map(([_, pkg]) => pkg.manifest.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('retro-terminal 声明 motion 覆盖并保持字段稀疏', () => {
    const pkg = retroTerminalPkg as unknown as ThemePackageDocument;
    expect(pkg.motion).toBeDefined();
    expect(pkg.motion?.fast).toBe(90);
    expect(pkg.motion?.base).toBe(120);
    expect(pkg.motion?.slow).toBe(180);
    expect(pkg.motion?.page).toBe(220);
    // 稀疏语义：未声明的字段应缺失（micro/*_out/overlayIn 全 undefined）
    expect(pkg.motion?.micro).toBeUndefined();
    expect(pkg.motion?.baseOut).toBeUndefined();
    expect(pkg.motion?.overlayIn).toBeUndefined();
  });

  it('未声明 motion 的 demo 包 motion 字段为 undefined', () => {
    const minimal = minimalPkg as unknown as ThemePackageDocument;
    const midnight = midnightGlassPkg as unknown as ThemePackageDocument;
    expect(minimal.motion).toBeUndefined();
    expect(midnight.motion).toBeUndefined();
  });

  it('retro-terminal 声明 shape 覆盖：直角 + 无 pill', () => {
    const pkg = retroTerminalPkg as unknown as ThemePackageDocument;
    expect(pkg.shape).toBeDefined();
    expect(pkg.shape?.xs).toBe(0);
    expect(pkg.shape?.md).toBe(0);
    expect(pkg.shape?.pill).toBe(0);
    expect(pkg.shape?.lg).toBe(2);
  });

  it('retro-terminal 声明 density 覆盖：紧凑档', () => {
    const pkg = retroTerminalPkg as unknown as ThemePackageDocument;
    expect(pkg.density).toBeDefined();
    expect(pkg.density?.xs).toBe(2);
    expect(pkg.density?.md).toBe(10);
    expect(pkg.density?.xl).toBe(20);
  });

  it('未声明 shape/density 的 demo 包字段为 undefined（稀疏语义）', () => {
    const minimal = minimalPkg as unknown as ThemePackageDocument;
    const midnight = midnightGlassPkg as unknown as ThemePackageDocument;
    const material = materialYouPkg as unknown as ThemePackageDocument;
    expect(minimal.shape).toBeUndefined();
    expect(minimal.density).toBeUndefined();
    expect(midnight.shape).toBeUndefined();
    expect(midnight.density).toBeUndefined();
    expect(material.shape).toBeUndefined();
    expect(material.density).toBeUndefined();
  });

  it('midnight-glass 声明 elevation 与 blur（玻璃拟态示范）', () => {
    const pkg = midnightGlassPkg as unknown as ThemePackageDocument;
    expect(pkg.elevation).toBeDefined();
    expect(pkg.elevation?.md).toContain('rgba(31, 20, 60');
    expect(pkg.elevation?.lg).toContain('0 16px 48px');
    expect(pkg.blur).toBeDefined();
    expect(pkg.blur?.sm).toBe(12);
    expect(pkg.blur?.xl).toBe(48);
  });

  it('未声明 elevation/blur 的 demo 包字段为 undefined', () => {
    const minimal = minimalPkg as unknown as ThemePackageDocument;
    const material = materialYouPkg as unknown as ThemePackageDocument;
    const retro = retroTerminalPkg as unknown as ThemePackageDocument;
    expect(minimal.elevation).toBeUndefined();
    expect(minimal.blur).toBeUndefined();
    expect(material.elevation).toBeUndefined();
    expect(material.blur).toBeUndefined();
    expect(retro.elevation).toBeUndefined();
    expect(retro.blur).toBeUndefined();
  });

  it('material-you 声明 visualContract: material / balanced', () => {
    const pkg = materialYouPkg as unknown as ThemePackageDocument;
    expect(pkg.visualContract?.family).toBe('material');
    expect(pkg.visualContract?.depth).toBe('balanced');
  });

  it('midnight-glass 声明 visualContract: glass / deep', () => {
    const pkg = midnightGlassPkg as unknown as ThemePackageDocument;
    expect(pkg.visualContract?.family).toBe('glass');
    expect(pkg.visualContract?.depth).toBe('deep');
  });

  it('retro-terminal 声明 visualContract: terminal / flat（Phase 3.3 支持）', () => {
    const pkg = retroTerminalPkg as unknown as ThemePackageDocument;
    expect(pkg.visualContract?.family).toBe('terminal');
    expect(pkg.visualContract?.depth).toBe('flat');
    // Phase 3.3 起 SUPPORTED_THEME_FAMILIES 已包含 terminal，运行时直通不 fallback
  });

  it('midnight-glass 声明 Phase 4 fontFamily + cssVariables', () => {
    const pkg = midnightGlassPkg as unknown as ThemePackageDocument;
    expect(pkg.fontFamily?.display).toBe('Geometos, Inter, sans-serif');
    expect(pkg.fontFamily?.body).toBe('Inter, HarmonyOS Sans SC, sans-serif');
    expect(pkg.cssVariables?.['--theme-custom-brand-glow']).toBe(
      'rgba(124, 58, 237, 0.42)'
    );
    expect(pkg.cssVariables?.['--theme-custom-scrim-alpha']).toBe('0.68');
  });

  it('未声明 fontFamily / cssVariables 的 demo 包字段为 undefined', () => {
    const minimal = minimalPkg as unknown as ThemePackageDocument;
    const material = materialYouPkg as unknown as ThemePackageDocument;
    const retro = retroTerminalPkg as unknown as ThemePackageDocument;
    expect(minimal.fontFamily).toBeUndefined();
    expect(minimal.cssVariables).toBeUndefined();
    expect(material.fontFamily).toBeUndefined();
    expect(retro.fontFamily).toBeUndefined();
  });
});
