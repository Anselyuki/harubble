// @vitest-environment jsdom

/**
 * Visual Contract 状态源单元测试（Phase 3 Step 3.1）。
 *
 * 覆盖：resolve 支持集内直通、未知值 fallback 并生成 warning、
 * DOM 属性写入与 JS $state 同步。
 */
import { beforeEach, describe, expect, it } from 'vitest';
import {
  SUPPORTED_THEME_DEPTHS,
  SUPPORTED_THEME_FAMILIES,
  applyVisualContract,
  getVisualContract,
  resolveVisualContract,
} from './visualContract.svelte';

describe('resolveVisualContract 支持集与 fallback', () => {
  it('支持集内的 family/depth 直通', () => {
    const result = resolveVisualContract({
      family: 'material',
      depth: 'deep',
    });
    expect(result.family).toBe('material');
    expect(result.depth).toBe('deep');
    expect(result.warnings).toEqual([]);
  });

  it('已加入支持集的 terminal family 直通不 warn（Phase 3.3）', () => {
    const result = resolveVisualContract({
      family: 'terminal',
      depth: 'flat',
    });
    expect(result.family).toBe('terminal');
    expect(result.warnings).toEqual([]);
  });

  it('未支持的 family fallback 到 glass 并 warn', () => {
    const result = resolveVisualContract({
      family: 'brutalist',
      depth: 'balanced',
    });
    expect(result.family).toBe('glass');
    expect(result.warnings.some((w) => w.includes('family=brutalist'))).toBe(
      true
    );
  });

  it('未支持的 depth fallback 到 balanced 并 warn', () => {
    const result = resolveVisualContract({
      family: 'glass',
      depth: 'unknown-depth',
    });
    expect(result.depth).toBe('balanced');
    expect(result.warnings.some((w) => w.includes('depth=unknown-depth'))).toBe(
      true
    );
  });

  it('缺失字段 fallback 到默认（不产生 warning）', () => {
    const result = resolveVisualContract({});
    expect(result.family).toBe('glass');
    expect(result.depth).toBe('balanced');
    expect(result.warnings).toEqual([]);
  });

  it('null / undefined 参数 fallback 到默认', () => {
    expect(resolveVisualContract(null).family).toBe('glass');
    expect(resolveVisualContract(undefined).family).toBe('glass');
  });
});

describe('applyVisualContract 同步 JS state 与 DOM 属性', () => {
  beforeEach(() => {
    delete document.documentElement.dataset.themeFamily;
    delete document.documentElement.dataset.themeDepth;
  });

  it('applyVisualContract 写入 data-theme-* 属性', () => {
    applyVisualContract({ family: 'material', depth: 'deep' });
    expect(document.documentElement.dataset.themeFamily).toBe('material');
    expect(document.documentElement.dataset.themeDepth).toBe('deep');
  });

  it('applyVisualContract 同步 getVisualContract() 返回值', () => {
    applyVisualContract({ family: 'material', depth: 'flat' });
    const state = getVisualContract();
    expect(state.family).toBe('material');
    expect(state.depth).toBe('flat');
  });

  it('applyVisualContract 未支持值 fallback 并返回 warnings', () => {
    const warnings = applyVisualContract({ family: 'nonexistent' });
    expect(document.documentElement.dataset.themeFamily).toBe('glass');
    expect(warnings.length).toBeGreaterThan(0);
  });

  it('applyVisualContract(null) 恢复到默认 glass/balanced', () => {
    applyVisualContract({ family: 'material', depth: 'deep' });
    applyVisualContract(null);
    expect(document.documentElement.dataset.themeFamily).toBe('glass');
    expect(document.documentElement.dataset.themeDepth).toBe('balanced');
  });
});

describe('支持集常量结构稳定', () => {
  it('SUPPORTED_THEME_FAMILIES Phase 3.1 至少包含 glass 与 material', () => {
    expect(SUPPORTED_THEME_FAMILIES).toContain('glass');
    expect(SUPPORTED_THEME_FAMILIES).toContain('material');
  });

  it('SUPPORTED_THEME_DEPTHS 包含 flat / balanced / deep', () => {
    expect(SUPPORTED_THEME_DEPTHS).toEqual(['flat', 'balanced', 'deep']);
  });
});
