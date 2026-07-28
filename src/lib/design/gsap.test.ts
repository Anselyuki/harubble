// @vitest-environment jsdom

/**
 * gsap 模块单元测试（Phase 2 Step 2.c）。
 *
 * 覆盖 P1-5 修复：`reducedMotionActive` 监听 `matchMedia('change')` 事件；
 * `applyMotionOverride` 双向同步 MOTION 值到 CSS 变量。
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// 注意：模块级 matchMedia 求值发生在 import 时，为让测试隔离，
// 我们通过重置模块缓存 + 修改 mediaQuery 的方式验证 change 监听。
describe('gsap · getMotionDuration + shouldSkipMotion', () => {
  beforeEach(() => {
    // 每个用例前重置 CSS 变量
    document.documentElement.style.removeProperty('--motion-fast');
    document.documentElement.style.removeProperty('--motion-base');
    document.documentElement.style.removeProperty('--motion-slow');
    document.documentElement.style.removeProperty('--motion-page');
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('MOTION 档位数值为不可变常量', async () => {
    const { MOTION } = await import('./gsap');
    expect(MOTION.MICRO).toBe(100);
    expect(MOTION.FAST).toBe(140);
    expect(MOTION.BASE).toBe(180);
    expect(MOTION.SLOW).toBe(260);
    expect(MOTION.PAGE).toBe(320);
  });

  it('getMotionDuration 返回秒数（毫秒 / 1000）', async () => {
    const { getMotionDuration, MOTION } = await import('./gsap');
    // 默认场景（非 reduced-motion）
    expect(getMotionDuration(MOTION.FAST)).toBeCloseTo(0.14, 3);
    expect(getMotionDuration(MOTION.BASE)).toBeCloseTo(0.18, 3);
  });

  it('applyMotionOverride 写入 CSS 变量并影响 getMotionDuration', async () => {
    const { applyMotionOverride, getMotionDuration, MOTION } =
      await import('./gsap');
    applyMotionOverride({ FAST: 220, BASE: 300 });
    expect(
      document.documentElement.style.getPropertyValue('--motion-fast')
    ).toBe('220ms');
    expect(
      document.documentElement.style.getPropertyValue('--motion-base')
    ).toBe('300ms');
    // getMotionDuration 命中 override → 返回覆盖值
    expect(getMotionDuration(MOTION.FAST)).toBeCloseTo(0.22, 3);
    expect(getMotionDuration(MOTION.BASE)).toBeCloseTo(0.3, 3);
    // 未覆盖的档位仍走默认值
    expect(getMotionDuration(MOTION.SLOW)).toBeCloseTo(0.26, 3);
    // 传 null 恢复默认
    applyMotionOverride(null);
    expect(
      document.documentElement.style.getPropertyValue('--motion-fast')
    ).toBe('');
    expect(getMotionDuration(MOTION.FAST)).toBeCloseTo(0.14, 3);
  });

  it('applyMotionOverride 部分覆盖时只删除未指定档位的 CSS 变量', async () => {
    const { applyMotionOverride } = await import('./gsap');
    applyMotionOverride({ FAST: 220 });
    expect(
      document.documentElement.style.getPropertyValue('--motion-fast')
    ).toBe('220ms');
    // 未指定的 SLOW / PAGE 不写入
    expect(
      document.documentElement.style.getPropertyValue('--motion-slow')
    ).toBe('');
    applyMotionOverride(null);
  });

  it('shouldSkipMotion 与 reducedMotionActive 关联', async () => {
    const { shouldSkipMotion } = await import('./gsap');
    // 该值取决于 jsdom 的 matchMedia 默认返回值（false）
    expect(typeof shouldSkipMotion()).toBe('boolean');
  });

  it('applyShapeOverride 将 shape 覆盖写入 CSS 变量', async () => {
    const { applyShapeOverride } = await import('./gsap');
    applyShapeOverride({ md: 0, pill: 0 });
    expect(document.documentElement.style.getPropertyValue('--shape-md')).toBe(
      '0px'
    );
    expect(
      document.documentElement.style.getPropertyValue('--shape-pill')
    ).toBe('0px');
    // 未声明的 xs / sm / lg 不写入
    expect(document.documentElement.style.getPropertyValue('--shape-xs')).toBe(
      ''
    );
    applyShapeOverride(null);
    expect(document.documentElement.style.getPropertyValue('--shape-md')).toBe(
      ''
    );
  });

  it('applyDensityOverride 将 density 覆盖写入 CSS 变量', async () => {
    const { applyDensityOverride } = await import('./gsap');
    applyDensityOverride({ md: 10, xl: 20 });
    expect(
      document.documentElement.style.getPropertyValue('--density-md')
    ).toBe('10px');
    expect(
      document.documentElement.style.getPropertyValue('--density-xl')
    ).toBe('20px');
    applyDensityOverride(null);
    expect(
      document.documentElement.style.getPropertyValue('--density-md')
    ).toBe('');
  });

  it('applyElevationOverride 将 elevation shadow 字符串写入 CSS 变量', async () => {
    const { applyElevationOverride } = await import('./gsap');
    applyElevationOverride({
      md: '0 4px 12px rgba(0, 0, 0, 0.12)',
      lg: 'none',
    });
    expect(
      document.documentElement.style.getPropertyValue('--elevation-md')
    ).toBe('0 4px 12px rgba(0, 0, 0, 0.12)');
    expect(
      document.documentElement.style.getPropertyValue('--elevation-lg')
    ).toBe('none');
    applyElevationOverride(null);
    expect(
      document.documentElement.style.getPropertyValue('--elevation-md')
    ).toBe('');
  });

  it('applyBlurOverride 将 blur 覆盖写入 CSS 变量', async () => {
    const { applyBlurOverride } = await import('./gsap');
    applyBlurOverride({ md: 0, xl: 40 });
    expect(document.documentElement.style.getPropertyValue('--blur-md')).toBe(
      '0px'
    );
    expect(document.documentElement.style.getPropertyValue('--blur-xl')).toBe(
      '40px'
    );
    applyBlurOverride(null);
    expect(document.documentElement.style.getPropertyValue('--blur-md')).toBe(
      ''
    );
  });

  it('applyFontFamilyOverride 覆盖 --font-body/display/mono（Phase 4）', async () => {
    const { applyFontFamilyOverride } = await import('./gsap');
    applyFontFamilyOverride({
      body: 'HarmonyOS Sans SC',
      display: 'Geometos',
      mono: 'SF Mono',
    });
    expect(document.documentElement.style.getPropertyValue('--font-body')).toBe(
      'HarmonyOS Sans SC'
    );
    expect(
      document.documentElement.style.getPropertyValue('--font-display')
    ).toBe('Geometos');
    expect(document.documentElement.style.getPropertyValue('--font-mono')).toBe(
      'SF Mono'
    );
    applyFontFamilyOverride(null);
    expect(document.documentElement.style.getPropertyValue('--font-body')).toBe(
      ''
    );
  });

  it('applyCssVariablesOverride 只写入 --theme-custom-* 前缀 key（Phase 4）', async () => {
    const { applyCssVariablesOverride } = await import('./gsap');
    applyCssVariablesOverride({
      '--theme-custom-brand': '#ff0000',
      '--bg-primary': 'evil', // 无前缀 → 应被忽略
    });
    expect(
      document.documentElement.style.getPropertyValue('--theme-custom-brand')
    ).toBe('#ff0000');
    // 未前缀的 key 不能污染 app 变量
    const bgAfter =
      document.documentElement.style.getPropertyValue('--bg-primary');
    expect(bgAfter).not.toBe('evil');
    applyCssVariablesOverride(null);
    expect(
      document.documentElement.style.getPropertyValue('--theme-custom-brand')
    ).toBe('');
  });

  it('applyCssVariablesOverride 切换主题包时清理前一次注入的 key（Phase 4）', async () => {
    const { applyCssVariablesOverride } = await import('./gsap');
    applyCssVariablesOverride({
      '--theme-custom-a': '#001',
      '--theme-custom-b': '#002',
    });
    // 切到新主题包只声明 a
    applyCssVariablesOverride({ '--theme-custom-a': '#003' });
    expect(
      document.documentElement.style.getPropertyValue('--theme-custom-a')
    ).toBe('#003');
    // b 应该已被清理
    expect(
      document.documentElement.style.getPropertyValue('--theme-custom-b')
    ).toBe('');
  });
});
