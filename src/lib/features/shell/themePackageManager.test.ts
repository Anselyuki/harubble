// @vitest-environment jsdom

import { describe, expect, it, vi } from 'vitest';
import type { AppPreferences } from '$lib/types';
import { createThemePackageManager } from './themePackageManager.svelte';

const NO_OP_LISTEN: any = vi.fn(async () => () => {});

function makePrefs(
  overrides: Partial<AppPreferences['theme']> = {}
): AppPreferences {
  return {
    schemaVersion: 2,
    outputFormat: 'flac',
    outputDir: '/tmp',
    downloadLyrics: true,
    notifyOnDownloadComplete: true,
    notifyOnPlaybackChange: true,
    logLevel: 'error',
    locale: 'zh-CN',
    volume: 1,
    theme: {
      presetId: 'harubble-classic',
      customColors: {},
      colorScheme: 'auto',
      dynamicAlbumAccent: true,
      activePackageId: null,
      revision: 0,
      ...overrides,
    },
  } as AppPreferences;
}

describe('themePackageManager reducer 单调 revision', () => {
  it('接受首个 snapshot 并更新 activePackageId', () => {
    const mgr = createThemePackageManager({ listen: NO_OP_LISTEN });
    const accepted = mgr._applySnapshot(
      makePrefs({ activePackageId: 'acme', revision: 1 }),
      'command'
    );
    expect(accepted).toBe(true);
    expect(mgr.currentRevision).toBe(1);
    expect(mgr.activePackageId).toBe('acme');
  });

  it('丢弃 revision 更小的 snapshot（乱序事件保护）', () => {
    const mgr = createThemePackageManager({ listen: NO_OP_LISTEN });
    mgr._applySnapshot(
      makePrefs({ activePackageId: 'newer', revision: 5 }),
      'command'
    );
    const accepted = mgr._applySnapshot(
      makePrefs({ activePackageId: 'older', revision: 3 }),
      'event'
    );
    expect(accepted).toBe(false);
    expect(mgr.currentRevision).toBe(5);
    expect(mgr.activePackageId).toBe('newer');
  });

  it('接受相同 revision 的 snapshot（幂等重放安全）', () => {
    const mgr = createThemePackageManager({ listen: NO_OP_LISTEN });
    mgr._applySnapshot(
      makePrefs({ activePackageId: 'acme', revision: 2 }),
      'command'
    );
    const accepted = mgr._applySnapshot(
      makePrefs({ activePackageId: 'acme', revision: 2 }),
      'event'
    );
    expect(accepted).toBe(true);
    expect(mgr.currentRevision).toBe(2);
  });

  it('renderSeq 每次成功应用 snapshot 递增', () => {
    const mgr = createThemePackageManager({ listen: NO_OP_LISTEN });
    expect(mgr._getRenderSeq()).toBe(0);
    mgr._applySnapshot(makePrefs({ revision: 1 }), 'startup');
    expect(mgr._getRenderSeq()).toBe(1);
    mgr._applySnapshot(makePrefs({ revision: 2 }), 'command');
    expect(mgr._getRenderSeq()).toBe(2);
  });

  it('activePackageId=null 的 snapshot 会清空当前激活状态', () => {
    const mgr = createThemePackageManager({ listen: NO_OP_LISTEN });
    mgr._applySnapshot(
      makePrefs({ activePackageId: 'acme', revision: 1 }),
      'startup'
    );
    expect(mgr.activePackageId).toBe('acme');
    mgr._applySnapshot(
      makePrefs({ activePackageId: null, revision: 2 }),
      'event'
    );
    expect(mgr.activePackageId).toBeNull();
  });

  it('applySnapshot 检测到 activePackageId 变化时同步 DOM（P0 回归 guard）', async () => {
    // 应用启动 hydrate / 跨窗口事件路径必须触发 DOM 侧 applyPackageOverrides，
    // 不能只更新 $state 反应变量。见 themePackageManager::applySnapshot → syncDomToActive
    // 手动模拟"上次会话遗留的 CSS 覆盖"
    document.documentElement.dataset.themeFamily = 'terminal';
    document.documentElement.style.setProperty('--shape-md', '2px');
    const mgr = createThemePackageManager({ listen: NO_OP_LISTEN });
    // 第一次 applySnapshot：activePackageId 从初始 null 变为 'acme'。
    // 触发 syncDomToActive('acme')，在 test env 里 inspect 失败 → catch 返回 null →
    // applyPackageOverrides(null) → DOM 拉回默认 glass、清除 --shape-md inline style
    mgr._applySnapshot(
      makePrefs({ activePackageId: 'acme', revision: 1 }),
      'startup'
    );
    // 让 syncDomToActive 的 inspect promise + .catch + applyPackageOverrides flush
    await new Promise((r) => setTimeout(r, 10));
    expect(document.documentElement.dataset.themeFamily).toBe('glass');
    expect(document.documentElement.style.getPropertyValue('--shape-md')).toBe(
      ''
    );
  });

  it('保留 startSubscription / stopSubscription 幂等能力', async () => {
    const unlisten = vi.fn();
    const listen = vi.fn(async () => unlisten);
    const mgr = createThemePackageManager({ listen: listen as any });
    await mgr.startSubscription();
    expect(listen).toHaveBeenCalledTimes(1);
    // 幂等：重复 start 会先释放旧订阅
    await mgr.startSubscription();
    expect(unlisten).toHaveBeenCalledTimes(1);
    mgr.stopSubscription();
    expect(unlisten).toHaveBeenCalledTimes(2);
    // 再 stop 不应报错
    mgr.stopSubscription();
  });

  it('startSubscription pending 期间 stopSubscription 会作废 in-flight listen（epoch 竞态）', async () => {
    const unlisten = vi.fn();
    // 手动挂起 listen 的 resolve，模拟 await pending
    let resolveListen: (fn: () => void) => void = () => {};
    const listen = vi.fn(
      () =>
        new Promise<() => void>((r) => {
          resolveListen = r;
        })
    );
    const mgr = createThemePackageManager({ listen: listen as any });
    const startPromise = mgr.startSubscription();
    expect(listen).toHaveBeenCalledTimes(1);
    // 在 listen resolve 前，用户点隐藏 → stopSubscription
    mgr.stopSubscription();
    // 现在 listen resolve
    resolveListen(unlisten);
    await startPromise;
    // 关键断言：epoch 检测生效，主动 unlisten 了刚 resolve 的 fn
    expect(unlisten).toHaveBeenCalledTimes(1);
    // 再次 stopSubscription 不应重复 unlisten（订阅从未真正安装到 manager 上）
    mgr.stopSubscription();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });
});
