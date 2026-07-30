import { describe, expect, it } from 'vitest';
import type { ThemePackageDocument } from '$lib/types';
import {
  getThemePackageRuntime,
  resolveThemePackageColors,
  setThemePackageRuntimeDocument,
} from './themePackageRuntime.svelte';

const PACKAGE: ThemePackageDocument = {
  schemaVersion: 1,
  manifest: {
    id: 'test-family',
    name: 'Test Family',
    version: '1.0.0',
  },
  slots: {
    accent: '#112233',
    surface: '#F0F0F0',
    textPrimary: '#202020',
    textSecondary: '#505050',
    tint: '#808080',
    danger: '#CC2233',
  },
  variants: {
    dark: {
      surface: '#101010',
      textPrimary: '#F4F4F4',
    },
  },
};

describe('themePackageRuntime', () => {
  it('主题包激活时隔离 legacy customColors，并按当前 scheme 应用包变体', () => {
    const colors = resolveThemePackageColors(
      {
        presetId: 'harubble-classic',
        customColors: { accent: '#ABCDEF' },
      },
      PACKAGE,
      'dark'
    );

    expect(colors.accent).toBe('#112233');
    expect(colors.surface).toBe('#101010');
    expect(colors.textPrimary).toBe('#F4F4F4');
    expect(colors.textSecondary).toBe('#505050');
  });

  it('停用主题包后恢复 preset customColors', () => {
    const colors = resolveThemePackageColors(
      {
        presetId: 'harubble-classic',
        customColors: { accent: '#ABCDEF' },
      },
      null,
      'dark'
    );

    expect(colors.accent).toBe('#ABCDEF');
  });

  it('忽略运行时无法安全派生的非 #RRGGBB 色值', () => {
    const colors = resolveThemePackageColors(
      { presetId: 'harubble-classic', customColors: {} },
      {
        ...PACKAGE,
        slots: { ...PACKAGE.slots, accent: 'rgba(1, 2, 3, 0.5)' },
      },
      'light'
    );

    expect(colors.accent).toBe('#FFE47A');
  });

  it('暴露稳定的响应式文档状态', () => {
    setThemePackageRuntimeDocument(PACKAGE);
    expect(getThemePackageRuntime().document?.manifest.id).toBe('test-family');
    setThemePackageRuntimeDocument(null);
    expect(getThemePackageRuntime().document).toBeNull();
  });
});
