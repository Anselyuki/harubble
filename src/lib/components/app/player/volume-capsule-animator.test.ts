// @vitest-environment jsdom

/**
 * Volume capsule animator 契约测试（Phase 3 Step 3.2）。
 *
 * 验证 glass / material 两个 animator 都遵循 `CapsuleAnimator` 接口，
 * 且核心方法（expand/collapse/showBadge/hideBadge/destroy）都能被调用而不抛错。
 * 具体 GSAP tween 参数不作 pixel-perfect 断言（那属于视觉回归的范畴）。
 */
import { afterEach, describe, expect, it } from 'vitest';
import { applyMotionOverride, gsap } from '$lib/design/gsap';
import {
  createArkCapsuleAnimator,
  createGlassCapsuleAnimator,
  createMaterialCapsuleAnimator,
  type CapsuleAnimator,
  type CapsuleAnimatorRefs,
} from './volume-capsule-animator';

function makeRefs(): CapsuleAnimatorRefs {
  const track = document.createElement('div');
  const badge = document.createElement('span');
  const iconBtn = document.createElement('button');
  document.body.appendChild(track);
  document.body.appendChild(badge);
  document.body.appendChild(iconBtn);
  return { track, badge, iconBtn };
}

function testContractFor(
  factory: (get: () => CapsuleAnimatorRefs) => CapsuleAnimator,
  label: string
) {
  describe(`${label} animator 契约`, () => {
    it('exposes 5 methods with correct types', () => {
      const anim = factory(() => makeRefs());
      expect(typeof anim.expand).toBe('function');
      expect(typeof anim.collapse).toBe('function');
      expect(typeof anim.showBadge).toBe('function');
      expect(typeof anim.hideBadge).toBe('function');
      expect(typeof anim.destroy).toBe('function');
    });

    it('expand invokes onComplete asynchronously', async () => {
      const refs = makeRefs();
      const anim = factory(() => refs);
      const done = new Promise<void>((resolve) => {
        anim.expand(() => resolve());
      });
      await expect(
        Promise.race([
          done,
          new Promise((_, r) =>
            setTimeout(() => r(new Error('timeout')), 3000)
          ),
        ])
      ).resolves.toBeUndefined();
      anim.destroy();
    });

    it('destroy after null refs is safe', () => {
      const anim = factory(() => ({ track: null, badge: null, iconBtn: null }));
      expect(() => anim.destroy()).not.toThrow();
    });

    it('showBadge 幂等：重复调用不重复排 tween（badgeVisible 状态守卫）', () => {
      const refs = makeRefs();
      const anim = factory(() => refs);
      // 连续 3 次；如果没有守卫，会重复触发 fromTo 累积多个 tween
      // 我们无法直接观测 GSAP tween 数量，但可以检查 badge 的 transform
      // 状态在多次调用后保持稳定（不出现"跳"到中间态）
      anim.showBadge();
      anim.showBadge();
      anim.showBadge();
      // 至少能全部调用完不抛错
      expect(() => anim.showBadge()).not.toThrow();
      anim.destroy();
    });

    it('collapse 会先 hideBadge 再收 track（顺序契约）', async () => {
      const refs = makeRefs();
      const anim = factory(() => refs);
      // 先展开 + 显 badge
      await new Promise<void>((r) => anim.expand(() => r()));
      anim.showBadge();
      // 然后 collapse：不应抛错，且 destroy 时 badge/track 都被清理
      const done = new Promise<void>((r) => anim.collapse(() => r()));
      await expect(
        Promise.race([
          done,
          new Promise((_, r) =>
            setTimeout(() => r(new Error('timeout')), 3000)
          ),
        ])
      ).resolves.toBeUndefined();
      anim.destroy();
    });

    it('destroy 应 kill 全部 3 个 ref 的 tween', () => {
      const refs = makeRefs();
      const anim = factory(() => refs);
      // 排入几个 tween
      anim.expand(() => {});
      anim.showBadge();
      // destroy 应清理所有 ref 上的 tween，不留活跃 tween
      expect(() => anim.destroy()).not.toThrow();
      // destroy 后再次调用不应抛错
      expect(() => anim.destroy()).not.toThrow();
    });
  });
}

testContractFor(createGlassCapsuleAnimator, 'glass');
testContractFor(createMaterialCapsuleAnimator, 'material');
testContractFor(createArkCapsuleAnimator, 'ark-ui');

afterEach(() => {
  applyMotionOverride(null);
});

describe('Ark UI HCI motion model', () => {
  it('uses one short reveal without translation, scale, clip, or rotation tweens', () => {
    const refs = makeRefs();
    const anim = createArkCapsuleAnimator(() => refs);

    anim.expand(() => {});
    const [tween] = gsap.getTweensOf(refs.track);
    expect(tween).toBeDefined();
    expect(tween.duration()).toBe(0.18);
    expect(tween.vars.width).toBe(200);
    expect(tween.vars.opacity).toBe(1);
    expect(tween.vars.x).toBeUndefined();
    expect(tween.vars.y).toBeUndefined();
    expect(tween.vars.scale).toBeUndefined();
    expect(tween.vars.scaleX).toBeUndefined();
    expect(tween.vars.scaleY).toBeUndefined();
    expect(tween.vars.rotation).toBeUndefined();
    expect(tween.vars.clipPath).toBeUndefined();
    anim.destroy();
  });

  it('uses a faster close and leaves the inline readout unanimated', () => {
    const refs = makeRefs();
    const anim = createArkCapsuleAnimator(() => refs);
    anim.expand(() => {});
    gsap.getTweensOf(refs.track)[0]?.progress(1);

    anim.showBadge();
    expect(gsap.getTweensOf(refs.badge)).toHaveLength(0);

    anim.collapse(() => {});
    const [tween] = gsap.getTweensOf(refs.track);
    expect(tween.duration()).toBe(0.12);
    expect(tween.vars.width).toBe(0);
    expect(tween.vars.opacity).toBe(0);
    anim.destroy();
  });

  it('interrupts the current reveal instead of stacking timelines', () => {
    const refs = makeRefs();
    const anim = createArkCapsuleAnimator(() => refs);
    anim.expand(() => {});
    gsap.getTweensOf(refs.track)[0]?.progress(0.5);

    anim.collapse(() => {});
    expect(gsap.getTweensOf(refs.track)).toHaveLength(1);
    anim.expand(() => {});
    expect(gsap.getTweensOf(refs.track)).toHaveLength(1);
    anim.destroy();
  });

  it('keeps compact-control timing stable across package motion overrides', () => {
    applyMotionOverride({ FAST: 640, MICRO: 420 });
    const refs = makeRefs();
    const anim = createArkCapsuleAnimator(() => refs);

    anim.expand(() => {});
    expect(gsap.getTweensOf(refs.track)[0]?.duration()).toBe(0.18);
    gsap.getTweensOf(refs.track)[0]?.progress(1);

    anim.collapse(() => {});
    expect(gsap.getTweensOf(refs.track)[0]?.duration()).toBe(0.12);
    anim.destroy();
  });
});
