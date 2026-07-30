// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from 'vitest';
import { applyMotionOverride } from '$lib/design/gsap';
import type {
  AppPreferences,
  ThemePackageDocument,
  ThemePackageSummary,
} from '$lib/types';
import {
  applyThemePackageDocument,
  createThemePackageManager,
  resolveThemePackageCssVariables,
  type ThemePackageRenderOptions,
} from './themePackageManager.svelte';
import { cancelThemePackageTransition } from './themePackageTransition';

function makePackage(id: string, family = 'ark'): ThemePackageDocument {
  return {
    schemaVersion: 1,
    manifest: { id, name: id, version: '1.0.0' },
    slots: {},
    visualContract: { family, depth: 'moderate' },
  };
}

function makeDeps(
  overrides: Partial<Parameters<typeof createThemePackageManager>[0]> = {}
): Parameters<typeof createThemePackageManager>[0] {
  return {
    getPreferences: vi.fn(async () => makePrefs()),
    listPackages: vi.fn(async (): Promise<ThemePackageSummary[]> => []),
    inspectPackage: vi.fn(async () => null),
    setActivePackage: vi.fn(async (id, revision) =>
      makePrefs({ activePackageId: id, revision: revision + 1 })
    ),
    previewPackage: vi.fn(async (id) => makePackage(id)),
    dismissPreview: vi.fn(async () => {}),
    renderDocument: vi.fn(async (doc) => applyThemePackageDocument(doc)),
    ...overrides,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

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

afterEach(() => {
  cancelThemePackageTransition();
  applyMotionOverride(null);
  applyThemePackageDocument(null);
});

describe('theme package CSS variable scheme variants', () => {
  it('按 base < 当前 scheme variant 合并，且切换模式会清理旧变量', () => {
    const pkg: ThemePackageDocument = {
      ...makePackage('adaptive'),
      cssVariables: {
        '--theme-custom-panel': '#base',
        '--theme-custom-static': '10px',
      },
      cssVariableVariants: {
        light: {
          '--theme-custom-panel': '#light',
          '--theme-custom-light-only': '1',
        },
        dark: { '--theme-custom-panel': '#dark' },
      },
    };

    expect(resolveThemePackageCssVariables(pkg, 'light')).toEqual({
      '--theme-custom-panel': '#light',
      '--theme-custom-static': '10px',
      '--theme-custom-light-only': '1',
    });

    applyThemePackageDocument(pkg, 'light');
    expect(
      document.documentElement.style.getPropertyValue('--theme-custom-panel')
    ).toBe('#light');
    expect(
      document.documentElement.style.getPropertyValue(
        '--theme-custom-light-only'
      )
    ).toBe('1');

    applyThemePackageDocument(pkg, 'dark');
    expect(
      document.documentElement.style.getPropertyValue('--theme-custom-panel')
    ).toBe('#dark');
    expect(
      document.documentElement.style.getPropertyValue('--theme-custom-static')
    ).toBe('10px');
    expect(
      document.documentElement.style.getPropertyValue(
        '--theme-custom-light-only'
      )
    ).toBe('');

    applyThemePackageDocument(null, 'light');
  });

  it('兼容仅声明基础 cssVariables 的旧主题包', () => {
    const legacy: ThemePackageDocument = {
      ...makePackage('legacy'),
      cssVariables: { '--theme-custom-panel': '#legacy' },
    };

    expect(resolveThemePackageCssVariables(legacy, 'light')).toEqual(
      legacy.cssVariables
    );
    expect(resolveThemePackageCssVariables(legacy, 'dark')).toEqual(
      legacy.cssVariables
    );
  });
});

describe('themePackageManager reducer 单调 revision', () => {
  it('接受首个 snapshot 并更新 activePackageId', () => {
    const mgr = createThemePackageManager(makeDeps());
    const accepted = mgr._applySnapshot(
      makePrefs({ activePackageId: 'acme', revision: 1 }),
      'command'
    );
    expect(accepted).toBe(true);
    expect(mgr.currentRevision).toBe(1);
    expect(mgr.activePackageId).toBe('acme');
  });

  it('丢弃 revision 更小的 snapshot（乱序事件保护）', () => {
    const mgr = createThemePackageManager(makeDeps());
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
    const mgr = createThemePackageManager(makeDeps());
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

  it('丢弃相同 revision 但 active id 冲突的 snapshot', () => {
    const mgr = createThemePackageManager(makeDeps());
    mgr._applySnapshot(
      makePrefs({ activePackageId: 'first', revision: 2 }),
      'command'
    );

    const accepted = mgr._applySnapshot(
      makePrefs({ activePackageId: 'stale', revision: 2 }),
      'event'
    );

    expect(accepted).toBe(false);
    expect(mgr.activePackageId).toBe('first');
    expect(mgr.currentRevision).toBe(2);
  });

  it('renderSeq 每次成功应用 snapshot 递增', () => {
    const mgr = createThemePackageManager(makeDeps());
    expect(mgr._getRenderSeq()).toBe(0);
    mgr._applySnapshot(makePrefs({ revision: 1 }), 'startup');
    expect(mgr._getRenderSeq()).toBe(1);
    mgr._applySnapshot(makePrefs({ revision: 2 }), 'command');
    expect(mgr._getRenderSeq()).toBe(2);
  });

  it('activePackageId=null 的 snapshot 会清空当前激活状态', () => {
    const mgr = createThemePackageManager(makeDeps());
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
    const mgr = createThemePackageManager(makeDeps());
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

  it('预览期间的持久化事件只更新 active，不覆盖预览 DOM', async () => {
    const inspect = vi.fn(async (id: string) => makePackage(id, 'corporate'));
    const mgr = createThemePackageManager(
      makeDeps({ inspectPackage: inspect })
    );

    await mgr.preview('previewed');
    expect(document.documentElement.dataset.themeFamily).toBe('ark');

    mgr._applySnapshot(
      makePrefs({ activePackageId: 'persisted', revision: 2 }),
      'event'
    );
    await Promise.resolve();

    expect(mgr.activePackageId).toBe('persisted');
    expect(mgr.previewingId).toBe('previewed');
    expect(inspect).not.toHaveBeenCalled();
    expect(document.documentElement.dataset.themeFamily).toBe('ark');
  });

  it('旧 active inspect 在 preview 之后完成也不会覆盖预览', async () => {
    const pendingInspect = deferred<ThemePackageDocument | null>();
    const mgr = createThemePackageManager(
      makeDeps({ inspectPackage: vi.fn(() => pendingInspect.promise) })
    );

    mgr._applySnapshot(
      makePrefs({ activePackageId: 'persisted', revision: 1 }),
      'startup'
    );
    await mgr.preview('previewed');
    pendingInspect.resolve(makePackage('persisted', 'corporate'));
    await Promise.resolve();

    expect(document.documentElement.dataset.themeFamily).toBe('ark');
  });

  it('预览请求失败后恢复当前持久化主题', async () => {
    const inspect = vi.fn(async (id: string) => makePackage(id, 'corporate'));
    const mgr = createThemePackageManager(
      makeDeps({
        inspectPackage: inspect,
        previewPackage: vi.fn(async () => {
          throw new Error('preview failed');
        }),
      })
    );
    mgr._applySnapshot(
      makePrefs({ activePackageId: 'persisted', revision: 1 }),
      'startup'
    );
    await Promise.resolve();

    await expect(mgr.preview('missing')).rejects.toThrow('preview failed');

    expect(mgr.previewingId).toBeNull();
    expect(inspect).toHaveBeenLastCalledWith('persisted');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
  });

  it('公开快照入口按 revision 同步跨窗口 active 状态', () => {
    const mgr = createThemePackageManager(makeDeps());

    const accepted = mgr.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'ark-ui-exa', revision: 4 })
    );

    expect(accepted).toBe(true);
    expect(mgr.currentRevision).toBe(4);
    expect(mgr.activePackageId).toBe('ark-ui-exa');
  });

  it('旧 dismiss inspect 结果不会覆盖之后的新预览', async () => {
    const pendingInspect = deferred<ThemePackageDocument | null>();
    const inspect = vi.fn(() => pendingInspect.promise);
    const mgr = createThemePackageManager(
      makeDeps({ inspectPackage: inspect })
    );
    mgr._applySnapshot(
      makePrefs({ activePackageId: 'persisted', revision: 1 }),
      'startup'
    );
    await Promise.resolve();
    await mgr.preview('first');

    const dismissPromise = mgr.dismissPreview();
    await Promise.resolve();
    await mgr.preview('second');
    pendingInspect.resolve(makePackage('persisted', 'corporate'));
    await dismissPromise;

    expect(mgr.previewingId).toBe('second');
    expect(document.documentElement.dataset.themeFamily).toBe('ark');
  });

  it('成功激活会清理预览状态并显示新持久化包', async () => {
    const inspect = vi.fn(async (id: string) => makePackage(id, 'corporate'));
    const dismissPreview = vi.fn(async () => {});
    const mgr = createThemePackageManager(
      makeDeps({ inspectPackage: inspect, dismissPreview })
    );

    await mgr.preview('previewed');
    await mgr.setActive('persisted');

    expect(mgr.previewingId).toBeNull();
    expect(mgr.activePackageId).toBe('persisted');
    expect(dismissPreview).toHaveBeenCalled();
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
  });

  it('应用当前预览包时跳过二次 inspect 与重复转场', async () => {
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const inspectPackage = vi.fn(async () => {
      throw new Error('transient inspect failure');
    });
    const manager = createThemePackageManager(
      makeDeps({
        inspectPackage,
        previewPackage: vi.fn(async (id: string) =>
          makePackage(id, 'corporate')
        ),
        renderDocument,
      })
    );

    await manager.preview('previewed');
    renderDocument.mockClear();
    await manager.setActive('previewed');

    expect(manager.activePackageId).toBe('previewed');
    expect(manager.previewingId).toBeNull();
    expect(inspectPackage).not.toHaveBeenCalled();
    expect(renderDocument).not.toHaveBeenCalled();
    expect(document.documentElement.dataset.themePackageId).toBe('previewed');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
  });

  it('当前预览包的激活事件先于关闭清理返回时不播放转场', async () => {
    const activationResult = deferred<AppPreferences>();
    const dismissGate = deferred<void>();
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const setActivePackage = vi.fn(() => activationResult.promise);
    const manager = createThemePackageManager(
      makeDeps({
        setActivePackage,
        dismissPreview: vi.fn(() => dismissGate.promise),
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, id === 'previewed' ? 'corporate' : 'terminal')
        ),
        renderDocument,
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themeFamily).toBe('terminal')
    );
    await manager.preview('previewed');
    renderDocument.mockClear();

    const activation = manager.setActive('previewed');
    await vi.waitFor(() => expect(setActivePackage).toHaveBeenCalledOnce());
    const closingDismiss = manager.dismissPreview();
    const persisted = makePrefs({
      activePackageId: 'previewed',
      revision: 2,
    });
    manager.applyPreferencesSnapshot(persisted);
    activationResult.resolve(persisted);

    await activation;
    expect(renderDocument).not.toHaveBeenCalled();
    dismissGate.resolve(undefined);
    await closingDismiss;

    expect(renderDocument).not.toHaveBeenCalled();
  });

  it('预览包应用清理期间收到远端主题时保留远端激活动画', async () => {
    const dismissGate = deferred<void>();
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const manager = createThemePackageManager(
      makeDeps({
        dismissPreview: vi.fn(() => dismissGate.promise),
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, id === 'remote' ? 'terminal' : 'corporate')
        ),
        renderDocument,
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await manager.preview('previewed');
    renderDocument.mockClear();

    const activation = manager.setActive('previewed');
    await vi.waitFor(() => expect(manager.activePackageId).toBe('previewed'));
    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'remote', revision: 3 })
    );
    await vi.waitFor(() =>
      expect(
        renderDocument.mock.calls.filter(
          ([doc]) => doc?.manifest.id === 'remote'
        )
      ).toHaveLength(1)
    );
    dismissGate.resolve(undefined);
    await activation;

    expect(
      renderDocument.mock.calls.filter(([doc]) => doc?.manifest.id === 'remote')
    ).toEqual([
      [
        expect.objectContaining({
          manifest: expect.objectContaining({ id: 'remote' }),
        }),
        { animate: true, reason: 'activate' },
      ],
    ]);
  });

  it('远端主题在预览包激活命令响应前到达时重绘远端主题', async () => {
    const activationResult = deferred<AppPreferences>();
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const setActivePackage = vi.fn(() => activationResult.promise);
    const manager = createThemePackageManager(
      makeDeps({
        setActivePackage,
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, id === 'remote' ? 'terminal' : 'corporate')
        ),
        renderDocument,
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await manager.preview('previewed');
    renderDocument.mockClear();

    const activation = manager.setActive('previewed');
    await vi.waitFor(() => expect(setActivePackage).toHaveBeenCalledOnce());
    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'remote', revision: 3 })
    );
    activationResult.resolve(
      makePrefs({ activePackageId: 'previewed', revision: 2 })
    );
    await activation;

    expect(manager.activePackageId).toBe('remote');
    expect(renderDocument).toHaveBeenCalledOnce();
    expect(renderDocument).toHaveBeenCalledWith(
      expect.objectContaining({
        manifest: expect.objectContaining({ id: 'remote' }),
      }),
      { animate: true, reason: 'activate' }
    );
  });

  it('权威快照激活当前预览包时退出预览态且不重复渲染', async () => {
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const manager = createThemePackageManager(
      makeDeps({
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, 'corporate')
        ),
        renderDocument,
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await manager.preview('previewed');
    renderDocument.mockClear();

    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'previewed', revision: 2 })
    );
    await Promise.resolve();

    expect(manager.activePackageId).toBe('previewed');
    expect(manager.previewingId).toBeNull();
    expect(renderDocument).not.toHaveBeenCalled();
  });

  it('新预览等待期间同包被远端激活时按真实视觉目标切换', async () => {
    const nextPreview = deferred<ThemePackageDocument>();
    const dismissPreview = vi.fn(async () => {});
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const manager = createThemePackageManager(
      makeDeps({
        previewPackage: vi.fn((id: string) =>
          id === 'next' ? nextPreview.promise : Promise.resolve(makePackage(id))
        ),
        dismissPreview,
        renderDocument,
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await manager.preview('first');
    renderDocument.mockClear();

    const pending = manager.preview('next');
    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'next', revision: 2 })
    );
    expect(renderDocument).not.toHaveBeenCalled();
    nextPreview.resolve(makePackage('next', 'corporate'));
    await pending;

    expect(manager.activePackageId).toBe('next');
    expect(manager.previewingId).toBeNull();
    expect(dismissPreview).toHaveBeenCalledOnce();
    expect(renderDocument).toHaveBeenCalledWith(
      expect.objectContaining({
        manifest: expect.objectContaining({ id: 'next' }),
      }),
      { animate: true, reason: 'activate' }
    );
  });

  it('新预览等待期间激活旧预览包时立即清除旧预览态', async () => {
    const nextPreview = deferred<ThemePackageDocument>();
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const manager = createThemePackageManager(
      makeDeps({
        previewPackage: vi.fn((id: string) =>
          id === 'next'
            ? nextPreview.promise
            : Promise.resolve(makePackage(id, 'corporate'))
        ),
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, id === 'remote' ? 'terminal' : 'corporate')
        ),
        renderDocument,
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await manager.preview('previewed');
    renderDocument.mockClear();

    const pending = manager.preview('next');
    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'previewed', revision: 2 })
    );

    expect(manager.activePackageId).toBe('previewed');
    expect(manager.previewingId).toBeNull();
    nextPreview.reject(new Error('next preview failed'));
    await expect(pending).rejects.toThrow('next preview failed');
    expect(manager.previewingId).toBeNull();
    expect(document.documentElement.dataset.themePackageId).toBe('previewed');

    renderDocument.mockClear();
    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'remote', revision: 3 })
    );
    await vi.waitFor(() => expect(renderDocument).toHaveBeenCalledOnce());
    expect(renderDocument).toHaveBeenCalledWith(
      expect.objectContaining({
        manifest: expect.objectContaining({ id: 'remote' }),
      }),
      { animate: true, reason: 'activate' }
    );
  });

  it('待处理预览失败后用动画恢复期间激活的权威主题', async () => {
    const pendingPreview = deferred<ThemePackageDocument>();
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const manager = createThemePackageManager(
      makeDeps({
        previewPackage: vi.fn(() => pendingPreview.promise),
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, id === 'activated' ? 'corporate' : 'terminal')
        ),
        renderDocument,
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themePackageId).toBe('previous')
    );
    renderDocument.mockClear();

    const pending = manager.preview('activated');
    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'activated', revision: 2 })
    );
    pendingPreview.reject(new Error('preview failed'));
    await expect(pending).rejects.toThrow('preview failed');
    await vi.waitFor(() => expect(renderDocument).toHaveBeenCalledOnce());

    expect(manager.activePackageId).toBe('activated');
    expect(manager.previewingId).toBeNull();
    expect(renderDocument).toHaveBeenCalledWith(
      expect.objectContaining({
        manifest: expect.objectContaining({ id: 'activated' }),
      }),
      { animate: true, reason: 'activate' }
    );
  });

  it('预览遮罩中点前收到同包激活时保持遮罩并延后提交', async () => {
    applyMotionOverride({ BASE: 120, BASE_OUT: 120 });
    const manager = createThemePackageManager(
      makeDeps({
        renderDocument: undefined,
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, id === 'previewed' ? 'corporate' : 'terminal')
        ),
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themePackageId).toBe('previous')
    );

    const preview = manager.preview('previewed');
    await vi.waitFor(() =>
      expect(
        document.querySelector<HTMLElement>('.theme-package-transition')
          ?.dataset.phase
      ).toBe('cover')
    );
    expect(document.documentElement.dataset.themePackageId).toBe('previous');

    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'previewed', revision: 2 })
    );
    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(document.documentElement.dataset.themePackageId).toBe('previous');
    expect(
      document.querySelector<HTMLElement>('.theme-package-transition')?.dataset
        .phase
    ).toBe('cover');
    await vi.waitFor(
      () =>
        expect(document.documentElement.dataset.themePackageId).toBe(
          'previewed'
        ),
      { timeout: 600 }
    );
    await preview;
  });

  it('切回已渲染主题时取消尚未提交的旧目标转场', async () => {
    applyMotionOverride({ BASE: 120, BASE_OUT: 120 });
    const manager = createThemePackageManager(
      makeDeps({
        renderDocument: undefined,
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, id === 'rendered' ? 'corporate' : 'terminal')
        ),
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'rendered', revision: 1 }),
      'startup'
    );
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themePackageId).toBe('rendered')
    );

    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'incoming', revision: 2 })
    );
    await vi.waitFor(() =>
      expect(
        document.querySelector<HTMLElement>('.theme-package-transition')
          ?.dataset.phase
      ).toBe('cover')
    );
    expect(document.documentElement.dataset.themePackageId).toBe('rendered');

    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'rendered', revision: 3 })
    );
    await new Promise((resolve) => setTimeout(resolve, 320));

    expect(manager.activePackageId).toBe('rendered');
    expect(document.documentElement.dataset.themePackageId).toBe('rendered');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
    expect(document.querySelector('.theme-package-transition')).toBeNull();
  });

  it('同包激活不会截断已提交目标的 reveal 动画', async () => {
    applyMotionOverride({ BASE: 120, BASE_OUT: 700 });
    const manager = createThemePackageManager(
      makeDeps({
        renderDocument: undefined,
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, id === 'previewed' ? 'corporate' : 'terminal')
        ),
        previewPackage: vi.fn(async (id: string) =>
          makePackage(id, 'corporate')
        ),
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themePackageId).toBe('previous')
    );

    const preview = manager.preview('previewed');
    await vi.waitFor(
      () => {
        expect(document.documentElement.dataset.themePackageId).toBe(
          'previewed'
        );
        expect(
          document.querySelector<HTMLElement>('.theme-package-transition')
            ?.dataset.phase
        ).toBe('reveal');
      },
      { timeout: 600 }
    );

    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'previewed', revision: 2 })
    );
    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(manager.activePackageId).toBe('previewed');
    expect(manager.previewingId).toBeNull();
    expect(
      document.querySelector<HTMLElement>('.theme-package-transition')?.dataset
        .phase
    ).toBe('reveal');
    await preview;
    expect(document.querySelector('.theme-package-transition')).toBeNull();
  });

  it('退出预览中点后收到预览包激活时重新播放完整切换', async () => {
    applyMotionOverride({ BASE: 120, BASE_OUT: 120 });
    const manager = createThemePackageManager(
      makeDeps({
        renderDocument: undefined,
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, id === 'previewed' ? 'corporate' : 'terminal')
        ),
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themePackageId).toBe('previous')
    );
    await manager.preview('previewed');
    expect(document.documentElement.dataset.themePackageId).toBe('previewed');

    const dismissal = manager.dismissPreview();
    await vi.waitFor(
      () => {
        expect(document.documentElement.dataset.themePackageId).toBe(
          'previous'
        );
        expect(
          document.querySelector<HTMLElement>('.theme-package-transition')
            ?.dataset.phase
        ).toBe('reveal');
      },
      { timeout: 600 }
    );

    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'previewed', revision: 2 })
    );
    await new Promise((resolve) => setTimeout(resolve, 20));

    expect(document.documentElement.dataset.themePackageId).toBe('previous');
    expect(
      document.querySelector<HTMLElement>('.theme-package-transition')?.dataset
        .phase
    ).toBe('cover');
    await vi.waitFor(
      () =>
        expect(document.documentElement.dataset.themePackageId).toBe(
          'previewed'
        ),
      { timeout: 600 }
    );
    await dismissal;
  });

  it('恢复渲染提交后报错不会抑制后续真实主题切换', async () => {
    const restoreFailure = new Error('restore render failed');
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        options: ThemePackageRenderOptions
      ) => {
        applyThemePackageDocument(doc);
        if (options.reason === 'dismiss-preview') throw restoreFailure;
      }
    );
    const manager = createThemePackageManager(
      makeDeps({
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, id === 'previewed' ? 'corporate' : 'terminal')
        ),
        renderDocument,
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themePackageId).toBe('previous')
    );
    await manager.preview('previewed');

    await expect(manager.dismissPreview()).rejects.toBe(restoreFailure);
    expect(document.documentElement.dataset.themePackageId).toBe('previous');
    renderDocument.mockClear();

    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'previewed', revision: 2 })
    );
    await vi.waitFor(() => expect(renderDocument).toHaveBeenCalledOnce());

    expect(renderDocument).toHaveBeenCalledWith(
      expect.objectContaining({
        manifest: expect.objectContaining({ id: 'previewed' }),
      }),
      { animate: true, reason: 'activate' }
    );
  });

  it('预览中的激活在设置关闭后返回仍同步已持久化主题', async () => {
    const activationResult = deferred<AppPreferences>();
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const inspectPackage = vi.fn(async (id: string) =>
      makePackage(id, id === 'next' ? 'corporate' : 'terminal')
    );
    const manager = createThemePackageManager(
      makeDeps({
        setActivePackage: vi.fn(() => activationResult.promise),
        inspectPackage,
        renderDocument,
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themeFamily).toBe('terminal')
    );
    await manager.preview('previewed');

    const activation = manager.setActive('next');
    await manager.dismissPreview();
    expect(document.documentElement.dataset.themeFamily).toBe('terminal');

    const persisted = makePrefs({ activePackageId: 'next', revision: 2 });
    activationResult.resolve(persisted);
    await activation;
    manager.applyPreferencesSnapshot(persisted);

    expect(manager.activePackageId).toBe('next');
    expect(manager.previewingId).toBeNull();
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
    expect(renderDocument).toHaveBeenLastCalledWith(
      expect.objectContaining({
        manifest: expect.objectContaining({ id: 'next' }),
      }),
      { animate: true, reason: 'activate' }
    );
  });

  it('设置关闭后事件先同步激活主题时命令回应不重启动画', async () => {
    const activationResult = deferred<AppPreferences>();
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const manager = createThemePackageManager(
      makeDeps({
        setActivePackage: vi.fn(() => activationResult.promise),
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, id === 'next' ? 'corporate' : 'terminal')
        ),
        renderDocument,
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'previous', revision: 1 }),
      'startup'
    );
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themeFamily).toBe('terminal')
    );
    await manager.preview('previewed');

    const activation = manager.setActive('next');
    await manager.dismissPreview();
    const persisted = makePrefs({ activePackageId: 'next', revision: 2 });
    manager.applyPreferencesSnapshot(persisted);
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themeFamily).toBe('corporate')
    );
    const rendersBeforeCommandResponse = renderDocument.mock.calls.length;

    activationResult.resolve(persisted);
    await activation;

    expect(renderDocument).toHaveBeenCalledTimes(rendersBeforeCommandResponse);
    expect(
      renderDocument.mock.calls.filter(([doc]) => doc?.manifest.id === 'next')
    ).toHaveLength(1);
  });

  it('串行化快速连续激活并让最新意图使用前一次返回的 revision', async () => {
    const firstResult = deferred<AppPreferences>();
    const setActivePackage = vi.fn(
      async (id: string | null, revision: number) => {
        if (id === 'first') return firstResult.promise;
        return makePrefs({ activePackageId: id, revision: revision + 1 });
      }
    );
    const inspect = vi.fn(async (id: string) =>
      makePackage(id, id === 'second' ? 'corporate' : 'terminal')
    );
    const mgr = createThemePackageManager(
      makeDeps({ setActivePackage, inspectPackage: inspect })
    );
    mgr._applySnapshot(
      makePrefs({ activePackageId: null, revision: 1 }),
      'startup'
    );

    const first = mgr.setActive('first');
    const second = mgr.setActive('second');
    await vi.waitFor(() => expect(setActivePackage).toHaveBeenCalledTimes(1));
    expect(setActivePackage).toHaveBeenLastCalledWith('first', 1);

    firstResult.resolve(makePrefs({ activePackageId: 'first', revision: 2 }));
    await Promise.all([first, second]);

    expect(setActivePackage).toHaveBeenCalledTimes(2);
    expect(setActivePackage).toHaveBeenLastCalledWith('second', 2);
    expect(mgr.currentRevision).toBe(3);
    expect(mgr.activePackageId).toBe('second');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
  });

  it('普通设置保存推进 revision 后自动 rebase 一次主题包激活', async () => {
    const setActivePackage = vi
      .fn<(id: string | null, revision: number) => Promise<AppPreferences>>()
      .mockRejectedValueOnce({
        code: 'revisionMismatch',
        detail: { currentRevision: 2, expectedRevision: 1 },
      })
      .mockImplementationOnce(async (id, revision) =>
        makePrefs({ activePackageId: id, revision: revision + 1 })
      );
    const manager = createThemePackageManager(
      makeDeps({
        getPreferences: vi.fn(async () => makePrefs({ revision: 2 })),
        setActivePackage,
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, 'corporate')
        ),
      })
    );
    manager._applySnapshot(makePrefs({ revision: 1 }), 'startup');

    await manager.setActive('persisted');

    expect(setActivePackage).toHaveBeenNthCalledWith(1, 'persisted', 1);
    expect(setActivePackage).toHaveBeenNthCalledWith(2, 'persisted', 2);
    expect(manager.activePackageId).toBe('persisted');
    expect(manager.currentRevision).toBe(3);
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
  });

  it('不为已被后续激活取代的旧意图执行 CAS 重试', async () => {
    const mismatch = Object.assign(new Error('revision mismatch'), {
      code: 'revisionMismatch',
      detail: { currentRevision: 2, expectedRevision: 1 },
    });
    const setActivePackage = vi
      .fn<(id: string | null, revision: number) => Promise<AppPreferences>>()
      .mockRejectedValueOnce(mismatch)
      .mockImplementationOnce(async (id, revision) =>
        makePrefs({ activePackageId: id, revision: revision + 1 })
      );
    const manager = createThemePackageManager(
      makeDeps({
        getPreferences: vi.fn(async () => makePrefs({ revision: 2 })),
        setActivePackage,
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, 'corporate')
        ),
      })
    );
    manager._applySnapshot(makePrefs({ revision: 1 }), 'startup');

    const first = manager.setActive('first');
    const second = manager.setActive('second');

    await expect(first).rejects.toBe(mismatch);
    await expect(second).resolves.toMatchObject({
      theme: expect.objectContaining({ activePackageId: 'second' }),
    });
    expect(setActivePackage).toHaveBeenCalledTimes(2);
    expect(setActivePackage).toHaveBeenNthCalledWith(1, 'first', 1);
    expect(setActivePackage).toHaveBeenNthCalledWith(2, 'second', 2);
    expect(manager.activePackageId).toBe('second');
  });

  it('有界 CAS 重试再次失败时重新同步最新权威主题', async () => {
    const firstMismatch = Object.assign(new Error('first mismatch'), {
      code: 'revisionMismatch',
      detail: { currentRevision: 2, expectedRevision: 1 },
    });
    const retryMismatch = Object.assign(new Error('retry mismatch'), {
      code: 'revisionMismatch',
      detail: { currentRevision: 3, expectedRevision: 2 },
    });
    const getPreferences = vi
      .fn<() => Promise<AppPreferences>>()
      .mockResolvedValueOnce(makePrefs({ revision: 2 }))
      .mockResolvedValueOnce(
        makePrefs({ activePackageId: 'remote', revision: 3 })
      );
    const manager = createThemePackageManager(
      makeDeps({
        getPreferences,
        listPackages: vi.fn(async () => [
          {
            id: 'remote',
            name: 'remote',
            version: '1.0.0',
            status: 'committed' as const,
            builtin: false,
          },
        ]),
        setActivePackage: vi
          .fn<
            (id: string | null, revision: number) => Promise<AppPreferences>
          >()
          .mockRejectedValueOnce(firstMismatch)
          .mockRejectedValueOnce(retryMismatch),
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, 'corporate')
        ),
      })
    );
    manager._applySnapshot(makePrefs({ revision: 1 }), 'startup');

    await expect(manager.setActive('local')).rejects.toBe(retryMismatch);

    expect(getPreferences).toHaveBeenCalledTimes(2);
    expect(manager.currentRevision).toBe(3);
    expect(manager.activePackageId).toBe('remote');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
  });

  it('远端已切换主题包时不自动覆盖 revision 冲突', async () => {
    const mismatch = Object.assign(new Error('revision mismatch'), {
      code: 'revisionMismatch',
      detail: { currentRevision: 2, expectedRevision: 1 },
    });
    const setActivePackage = vi.fn(async () => {
      throw mismatch;
    });
    const manager = createThemePackageManager(
      makeDeps({
        getPreferences: vi.fn(async () =>
          makePrefs({ activePackageId: 'remote', revision: 2 })
        ),
        listPackages: vi.fn(async () => [
          {
            id: 'remote',
            name: 'remote',
            version: '1.0.0',
            status: 'committed' as const,
            builtin: false,
          },
        ]),
        setActivePackage,
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, 'corporate')
        ),
      })
    );
    manager._applySnapshot(makePrefs({ revision: 1 }), 'startup');

    await expect(manager.setActive('local')).rejects.toBe(mismatch);

    expect(setActivePackage).toHaveBeenCalledTimes(1);
    expect(manager.activePackageId).toBe('remote');
    expect(manager.currentRevision).toBe(2);
  });

  it('CAS 失败且期间收到更高 revision 事件时重绘权威 active 包', async () => {
    const gate = deferred<void>();
    const inspect = vi.fn(async (id: string) =>
      makePackage(id, id === 'remote' ? 'corporate' : 'terminal')
    );
    const getPreferences = vi.fn(async () =>
      makePrefs({ activePackageId: 'remote', revision: 2 })
    );
    const mgr = createThemePackageManager(
      makeDeps({
        inspectPackage: inspect,
        getPreferences,
        setActivePackage: vi.fn(async () => {
          await gate.promise;
          throw new Error('revision mismatch');
        }),
      })
    );
    mgr._applySnapshot(
      makePrefs({ activePackageId: 'local', revision: 1 }),
      'startup'
    );
    await new Promise((resolve) => setTimeout(resolve, 10));
    expect(document.documentElement.dataset.themeFamily).toBe('terminal');

    const pending = mgr.setActive('next');
    mgr.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'remote', revision: 2 })
    );
    gate.resolve(undefined);

    await expect(pending).rejects.toThrow('revision mismatch');
    expect(mgr.activePackageId).toBe('remote');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
    expect(inspect).toHaveBeenLastCalledWith('remote');
  });

  it('激活成功后预览清理失败不会把持久化结果报成失败', async () => {
    const inspect = vi.fn(async (id: string) => makePackage(id, 'corporate'));
    const mgr = createThemePackageManager(
      makeDeps({
        inspectPackage: inspect,
        dismissPreview: vi.fn(async () => {
          throw new Error('cleanup failed');
        }),
      })
    );

    const snapshot = await mgr.setActive('persisted');

    expect(snapshot.theme?.activePackageId).toBe('persisted');
    expect(mgr.activePackageId).toBe('persisted');
    expect(mgr.latestError).toBe('cleanup failed');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
  });

  it('覆盖当前激活包后重新 inspect 并应用新文档', async () => {
    const inspect = vi.fn(async (id: string) => makePackage(id, 'corporate'));
    const installFromFile = vi.fn(async () => ({
      id: 'persisted',
      name: 'persisted',
      version: '2.0.0',
      status: 'committed' as const,
      builtin: false,
    }));
    const mgr = createThemePackageManager(
      makeDeps({ inspectPackage: inspect, installFromFile })
    );
    mgr._applySnapshot(
      makePrefs({ activePackageId: 'persisted', revision: 1 }),
      'startup'
    );
    await Promise.resolve();
    await mgr.importFromFile('/tmp/persisted.json');

    expect(installFromFile).toHaveBeenCalledWith('/tmp/persisted.json');
    expect(inspect).toHaveBeenLastCalledWith('persisted');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
  });

  it('覆盖当前预览包后保留预览态并应用最新文档', async () => {
    const inspect = vi.fn(async (id: string) => makePackage(id, 'corporate'));
    const installFromFile = vi.fn(async () => ({
      id: 'previewed',
      name: 'previewed',
      version: '2.0.0',
      status: 'committed' as const,
      builtin: false,
    }));
    const mgr = createThemePackageManager(
      makeDeps({ inspectPackage: inspect, installFromFile })
    );
    await mgr.preview('previewed');
    await mgr.importFromFile('/tmp/previewed.json');

    expect(mgr.previewingId).toBe('previewed');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
  });

  it('覆盖导入 inspect 等待期间远端切换后不重绘旧 active 包', async () => {
    const overwrittenInspect = deferred<ThemePackageDocument>();
    let activeInspectCount = 0;
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const manager = createThemePackageManager(
      makeDeps({
        installFromUrl: vi.fn(async () => ({
          id: 'active',
          name: 'active',
          version: '2.0.0',
          status: 'committed' as const,
          builtin: false,
        })),
        inspectPackage: vi.fn((id: string) => {
          if (id === 'active' && activeInspectCount++ > 0) {
            return overwrittenInspect.promise;
          }
          return Promise.resolve(
            makePackage(id, id === 'remote' ? 'terminal' : 'ark')
          );
        }),
        renderDocument,
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'active', revision: 1 }),
      'startup'
    );
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themePackageId).toBe('active')
    );
    renderDocument.mockClear();

    const importing = manager.importFromUrl('https://example.com/active.json');
    await vi.waitFor(() => expect(activeInspectCount).toBe(2));
    manager.applyPreferencesSnapshot(
      makePrefs({ activePackageId: 'remote', revision: 2 })
    );
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themePackageId).toBe('remote')
    );
    overwrittenInspect.resolve(makePackage('active', 'corporate'));
    await importing;

    expect(manager.activePackageId).toBe('remote');
    expect(document.documentElement.dataset.themePackageId).toBe('remote');
    expect(document.documentElement.dataset.themeFamily).toBe('terminal');
    expect(renderDocument.mock.calls.at(-1)?.[0]?.manifest.id).toBe('remote');
  });

  it('卸载预览包会清除预览态并恢复持久化 DOM', async () => {
    const uninstallPackage = vi.fn(async () => {});
    const dismissPreview = vi.fn(async () => {});
    const mgr = createThemePackageManager(
      makeDeps({ uninstallPackage, dismissPreview })
    );
    await mgr.preview('previewed');
    await mgr.uninstall('previewed');

    expect(uninstallPackage).toHaveBeenCalledWith('previewed');
    expect(dismissPreview).toHaveBeenCalled();
    expect(mgr.previewingId).toBeNull();
    expect(document.documentElement.dataset.themeFamily).toBe('glass');
  });

  it('卸载非预览包时保留当前预览状态与 DOM', async () => {
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const manager = createThemePackageManager(
      makeDeps({
        getPreferences: vi.fn(async () =>
          makePrefs({ activePackageId: 'active', revision: 1 })
        ),
        listPackages: vi.fn(async () => [
          {
            id: 'active',
            name: 'active',
            version: '1.0.0',
            status: 'committed' as const,
            builtin: false,
          },
        ]),
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, id === 'previewed' ? 'corporate' : 'terminal')
        ),
        previewPackage: vi.fn(async (id: string) =>
          makePackage(id, 'corporate')
        ),
        uninstallPackage: vi.fn(async () => {}),
        renderDocument,
      })
    );
    manager._applySnapshot(
      makePrefs({ activePackageId: 'active', revision: 1 }),
      'startup'
    );
    await manager.preview('previewed');
    renderDocument.mockClear();

    await manager.uninstall('other');

    expect(manager.activePackageId).toBe('active');
    expect(manager.previewingId).toBe('previewed');
    expect(document.documentElement.dataset.themePackageId).toBe('previewed');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
    expect(renderDocument).not.toHaveBeenCalled();
  });

  it('激活包卸载失败但后端已回滚引用时恢复权威 DOM', async () => {
    const inspect = vi.fn(async (id: string) => makePackage(id, 'corporate'));
    const managerHolder: {
      current?: ReturnType<typeof createThemePackageManager>;
    } = {};
    const uninstallPackage = vi.fn(async () => {
      // The backend persists activePackageId=null before the filesystem move;
      // emulate that event before reporting the move failure.
      managerHolder.current?.applyPreferencesSnapshot(
        makePrefs({ activePackageId: null, revision: 2 })
      );
      throw new Error('move failed');
    });
    const manager = createThemePackageManager(
      makeDeps({
        inspectPackage: inspect,
        uninstallPackage,
        getPreferences: vi.fn(async () =>
          makePrefs({ activePackageId: null, revision: 2 })
        ),
      })
    );
    managerHolder.current = manager;

    manager._applySnapshot(
      makePrefs({ activePackageId: 'active', revision: 1 }),
      'startup'
    );
    await new Promise((resolve) => setTimeout(resolve, 10));
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');

    await expect(manager.uninstall('active')).rejects.toThrow('move failed');

    expect(manager.activePackageId).toBeNull();
    expect(document.documentElement.dataset.themeFamily).toBe('glass');
    expect(manager.latestError).toBe('move failed');
  });

  it('卸载非 active 包期间收到跨窗口切换后仍重绘最新 active', async () => {
    const gate = deferred<void>();
    const inspect = vi.fn(async (id: string) =>
      makePackage(id, id === 'remote' ? 'corporate' : 'terminal')
    );
    const uninstallPackage = vi.fn(async () => {
      manager.applyPreferencesSnapshot(
        makePrefs({ activePackageId: 'remote', revision: 2 })
      );
      await gate.promise;
    });
    const manager = createThemePackageManager(
      makeDeps({
        inspectPackage: inspect,
        uninstallPackage,
        getPreferences: vi.fn(async () =>
          makePrefs({ activePackageId: 'remote', revision: 2 })
        ),
        listPackages: vi.fn(async () => [
          {
            id: 'remote',
            name: 'remote',
            version: '1.0.0',
            status: 'committed' as const,
            builtin: false,
          },
        ]),
      })
    );

    manager._applySnapshot(
      makePrefs({ activePackageId: 'local', revision: 1 }),
      'startup'
    );
    await new Promise((resolve) => setTimeout(resolve, 10));
    expect(document.documentElement.dataset.themeFamily).toBe('terminal');

    const pending = manager.uninstall('other');
    await Promise.resolve();
    gate.resolve(undefined);
    await pending;

    expect(manager.activePackageId).toBe('remote');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
    expect(inspect).toHaveBeenLastCalledWith('remote');
  });

  it('卸载非 active 包失败时仍重绘期间收到的跨窗口 active', async () => {
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const inspect = vi.fn(async (id: string) =>
      makePackage(id, id === 'remote' ? 'corporate' : 'terminal')
    );
    const uninstallPackage = vi.fn(async () => {
      manager.applyPreferencesSnapshot(
        makePrefs({ activePackageId: 'remote', revision: 2 })
      );
      throw new Error('move failed');
    });
    const manager = createThemePackageManager(
      makeDeps({ inspectPackage: inspect, uninstallPackage, renderDocument })
    );

    manager._applySnapshot(
      makePrefs({ activePackageId: 'local', revision: 1 }),
      'startup'
    );
    await vi.waitFor(() =>
      expect(document.documentElement.dataset.themeFamily).toBe('terminal')
    );
    renderDocument.mockClear();

    await expect(manager.uninstall('other')).rejects.toThrow('move failed');

    expect(manager.activePackageId).toBe('remote');
    expect(document.documentElement.dataset.themePackageId).toBe('remote');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
    expect(inspect).toHaveBeenLastCalledWith('remote');
    expect(renderDocument).toHaveBeenLastCalledWith(
      expect.objectContaining({
        manifest: expect.objectContaining({ id: 'remote' }),
      }),
      { animate: true, reason: 'activate' }
    );
    expect(manager.latestError).toBe('move failed');
  });

  it('dismissPreview 在 IPC 完成前就作废本地预览态', async () => {
    const gate = deferred<void>();
    const dismissPreview = vi.fn(() => gate.promise);
    const mgr = createThemePackageManager(makeDeps({ dismissPreview }));
    await mgr.preview('previewed');

    const pending = mgr.dismissPreview();
    expect(mgr.previewingId).toBeNull();
    gate.resolve(undefined);
    await pending;
  });

  it('退出预览早于预览 IPC 返回时再次清理后端态', async () => {
    const previewGate = deferred<ThemePackageDocument>();
    const dismissPreview = vi.fn(async () => {});
    const renderDocument = vi.fn(async () => {});
    const mgr = createThemePackageManager(
      makeDeps({
        previewPackage: vi.fn(() => previewGate.promise),
        dismissPreview,
        renderDocument,
      })
    );

    const pendingPreview = mgr.preview('late-preview');
    await mgr.dismissPreview();
    previewGate.resolve(makePackage('late-preview'));
    await pendingPreview;

    expect(dismissPreview).toHaveBeenCalledTimes(2);
    expect(mgr.previewingId).toBeNull();
    expect(renderDocument).toHaveBeenCalledTimes(1);
    expect(renderDocument).toHaveBeenCalledWith(null, {
      animate: false,
      reason: 'dismiss-preview',
    });
  });

  it('旧预览先返回时不会清理仍在等待的较新预览', async () => {
    const firstGate = deferred<ThemePackageDocument>();
    const secondGate = deferred<ThemePackageDocument>();
    const dismissPreview = vi.fn(async () => {});
    const renderDocument = vi.fn(async () => {});
    const previewPackage = vi.fn((id: string) =>
      id === 'first' ? firstGate.promise : secondGate.promise
    );
    const mgr = createThemePackageManager(
      makeDeps({ previewPackage, dismissPreview, renderDocument })
    );

    const firstPreview = mgr.preview('first');
    const secondPreview = mgr.preview('second');
    firstGate.resolve(makePackage('first'));
    await firstPreview;

    expect(dismissPreview).not.toHaveBeenCalled();
    expect(mgr.previewingId).toBeNull();

    secondGate.resolve(makePackage('second'));
    await secondPreview;

    expect(dismissPreview).not.toHaveBeenCalled();
    expect(mgr.previewingId).toBe('second');
    expect(renderDocument).toHaveBeenCalledOnce();
    expect(renderDocument).toHaveBeenCalledWith(
      expect.objectContaining({
        manifest: expect.objectContaining({ id: 'second' }),
      }),
      { animate: true, reason: 'preview' }
    );
  });

  it('旧预览返回后较新预览失败会清理后端预览态', async () => {
    const firstGate = deferred<ThemePackageDocument>();
    const secondGate = deferred<ThemePackageDocument>();
    const dismissPreview = vi.fn(async () => {});
    const renderDocument = vi.fn(async () => {});
    const previewPackage = vi.fn((id: string) =>
      id === 'first' ? firstGate.promise : secondGate.promise
    );
    const mgr = createThemePackageManager(
      makeDeps({ previewPackage, dismissPreview, renderDocument })
    );

    const firstPreview = mgr.preview('first');
    const secondPreview = mgr.preview('second');
    firstGate.resolve(makePackage('first'));
    await firstPreview;
    secondGate.reject(new Error('second preview failed'));

    await expect(secondPreview).rejects.toThrow('second preview failed');
    expect(dismissPreview).toHaveBeenCalledOnce();
    expect(mgr.previewingId).toBeNull();
    expect(renderDocument).not.toHaveBeenCalled();
  });

  it('预览、激活与退出预览分别使用对应的主题转场原因', async () => {
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const manager = createThemePackageManager(
      makeDeps({
        renderDocument,
        inspectPackage: vi.fn(async (id: string) =>
          makePackage(id, 'corporate')
        ),
      })
    );

    await manager.preview('previewed');
    await manager.setActive('persisted');
    await manager.preview('previewed');
    await manager.dismissPreview();

    expect(renderDocument.mock.calls.map(([, options]) => options)).toEqual([
      { animate: true, reason: 'preview' },
      { animate: true, reason: 'activate' },
      { animate: true, reason: 'preview' },
      { animate: true, reason: 'dismiss-preview' },
    ]);
  });

  it('首次 hydrate 即时提交主题文档，不播放遮罩动画', async () => {
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const manager = createThemePackageManager(
      makeDeps({
        renderDocument,
        getPreferences: vi.fn(async () =>
          makePrefs({ activePackageId: 'persisted', revision: 1 })
        ),
        listPackages: vi.fn(async () => [
          {
            id: 'persisted',
            name: 'persisted',
            version: '1.0.0',
            status: 'committed' as const,
            builtin: false,
          },
        ]),
        inspectPackage: vi.fn(async () =>
          makePackage('persisted', 'corporate')
        ),
      })
    );

    await manager.hydrate();
    await vi.waitFor(() => expect(renderDocument).toHaveBeenCalledTimes(1));

    expect(renderDocument).toHaveBeenCalledWith(
      expect.objectContaining({
        manifest: expect.objectContaining({ id: 'persisted' }),
      }),
      { animate: false, reason: 'activate' }
    );
  });

  it('首次 hydrate 的 inspect 临时失败后会在下次 hydrate 重试', async () => {
    const renderDocument = vi.fn(
      async (
        doc: ThemePackageDocument | null,
        _options: ThemePackageRenderOptions
      ) => applyThemePackageDocument(doc)
    );
    const inspectPackage = vi
      .fn<(id: string) => Promise<ThemePackageDocument | null>>()
      .mockRejectedValueOnce(new Error('transient inspect failure'))
      .mockResolvedValueOnce(makePackage('persisted', 'corporate'));
    const manager = createThemePackageManager(
      makeDeps({
        renderDocument,
        getPreferences: vi.fn(async () =>
          makePrefs({ activePackageId: 'persisted', revision: 1 })
        ),
        listPackages: vi.fn(async () => [
          {
            id: 'persisted',
            name: 'persisted',
            version: '1.0.0',
            status: 'committed' as const,
            builtin: false,
          },
        ]),
        inspectPackage,
      })
    );

    await manager.hydrate();
    await vi.waitFor(() => expect(inspectPackage).toHaveBeenCalledTimes(1));
    await vi.waitFor(() =>
      expect(manager.latestError).toContain('transient inspect failure')
    );
    expect(document.documentElement.dataset.themePackageId).toBeUndefined();

    await manager.hydrate();

    expect(inspectPackage).toHaveBeenCalledTimes(2);
    expect(document.documentElement.dataset.themePackageId).toBe('persisted');
    expect(document.documentElement.dataset.themeFamily).toBe('corporate');
    expect(manager.latestError).toBeNull();
    expect(renderDocument).toHaveBeenLastCalledWith(
      expect.objectContaining({
        manifest: expect.objectContaining({ id: 'persisted' }),
      }),
      { animate: false, reason: 'activate' }
    );
  });
});
