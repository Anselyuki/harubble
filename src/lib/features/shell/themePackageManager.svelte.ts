/**
 * 主题包前端状态管理器（Phase 1 Step 1.e）。
 *
 * 管理主题包相关的所有前端交互态：已安装列表、当前激活 id、revision、
 * 预览态、CAS 循环，以及通过 `preferences_snapshot` 事件的跨窗口同步。
 *
 * # 双序号 reducer（主方案 §5.2.1）
 *
 * - `intentSeq`：用户显式意图序号。每次用户调用 `setActive/preview/dismiss` 递增，
 *   保证乱序响应不会覆盖更新的意图。
 * - `renderSeq`：DOM/token 应用序号。用于在异步 Context Theme 派生中保护
 *   fullscreen 中切主题的视觉一致性（P0-8）。
 *
 * # CAS 语义
 *
 * `setActive` 携带上次读取到的 `revision` 调用后端。若后端返回 `RevisionMismatch`，
 * 前端先 `hydrate()` 拉取最新 preferences：若 active 包未变化，说明通常只是普通
 * 设置保存推进了共享 revision，可安全 rebase 一次；若 active 包已被其他窗口切换，
 * 或重试仍冲突，则保留远端状态并向调用方返回错误。
 *
 * 成功路径不需要客户端手动构造 snapshot——后端返回完整 `AppPreferences`
 * 直接作为新态（消除 P1-1 的 customColors 倒退问题）。
 *
 * # `preferences_snapshot` 归并
 *
 * 后端在 preferences 变更后广播完整快照，集中事件层调用
 * `applyPreferencesSnapshot`。reducer 严格按 `revision` 单调递增
 * 接受：老快照直接丢弃，避免网络乱序导致的态回滚。
 */

import type {
  AppPreferences,
  ThemePackageBlur,
  ThemePackageDensity,
  ThemePackageDocument,
  ThemePackageElevation,
  ThemePackageFontFamily,
  ThemePackageMotion,
  ThemePackageShape,
  ThemePackageSummary,
} from '$lib/types';
import {
  dismissThemePreview,
  installThemePackageFromFile,
  installThemePackageFromUrl,
  inspectThemePackage,
  listThemePackages,
  previewThemePackage,
  setActiveThemePackage,
  uninstallThemePackage,
  exportThemePackage,
  getPreferences,
} from '$lib/api';
import {
  applyBlurOverride,
  applyCssVariablesOverride,
  applyDensityOverride,
  applyElevationOverride,
  applyFontFamilyOverride,
  applyMotionOverride,
  applyShapeOverride,
  type BlurOverride,
  type DensityOverride,
  type ElevationOverride,
  type FontFamilyOverride,
  type MotionOverride,
  type ShapeOverride,
} from '$lib/design/gsap';
import { applyVisualContract } from '$lib/features/shell/visualContract.svelte';
import { setThemePackageRuntimeDocument } from '$lib/features/shell/themePackageRuntime.svelte';
import {
  cancelObsoleteThemePackageTransition,
  cancelThemePackageTransition,
  runThemePackageTransition,
  type ThemePackageTransitionReason,
} from '$lib/features/shell/themePackageTransition';

/**
 * 将主题包声明的稀疏 motion 字段转换为 gsap.ts 使用的 MotionOverride。
 *
 * ThemePackageMotion 字段名与 MOTION 档位常量的对应关系（camelCase → UPPER_CASE）。
 * 未声明的字段跳过，让 gsap.ts 保留默认值。
 */
function toMotionOverride(
  motion: ThemePackageMotion | undefined
): MotionOverride | null {
  if (!motion) return null;
  const mapped: MotionOverride = {};
  if (motion.micro !== undefined) mapped.MICRO = motion.micro;
  if (motion.fast !== undefined) mapped.FAST = motion.fast;
  if (motion.base !== undefined) mapped.BASE = motion.base;
  if (motion.slow !== undefined) mapped.SLOW = motion.slow;
  if (motion.page !== undefined) mapped.PAGE = motion.page;
  if (motion.baseOut !== undefined) mapped.BASE_OUT = motion.baseOut;
  if (motion.slowOut !== undefined) mapped.SLOW_OUT = motion.slowOut;
  if (motion.pageOut !== undefined) mapped.PAGE_OUT = motion.pageOut;
  if (motion.overlayIn !== undefined) mapped.OVERLAY_IN = motion.overlayIn;
  return Object.keys(mapped).length > 0 ? mapped : null;
}

/**
 * 将主题包 shape 稀疏字段转换为 gsap.ts 的 ShapeOverride。
 * 未声明字段跳过，让 CSS 变量保留默认。
 */
function toShapeOverride(
  shape: ThemePackageShape | undefined
): ShapeOverride | null {
  if (!shape) return null;
  const mapped: ShapeOverride = {};
  if (shape.xs !== undefined) mapped.xs = shape.xs;
  if (shape.sm !== undefined) mapped.sm = shape.sm;
  if (shape.md !== undefined) mapped.md = shape.md;
  if (shape.lg !== undefined) mapped.lg = shape.lg;
  if (shape.xl !== undefined) mapped.xl = shape.xl;
  if (shape['2xl'] !== undefined) mapped['2xl'] = shape['2xl'];
  if (shape.pill !== undefined) mapped.pill = shape.pill;
  return Object.keys(mapped).length > 0 ? mapped : null;
}

/**
 * 将主题包 density 稀疏字段转换为 gsap.ts 的 DensityOverride。
 */
function toDensityOverride(
  density: ThemePackageDensity | undefined
): DensityOverride | null {
  if (!density) return null;
  const mapped: DensityOverride = {};
  if (density.xs !== undefined) mapped.xs = density.xs;
  if (density.sm !== undefined) mapped.sm = density.sm;
  if (density.md !== undefined) mapped.md = density.md;
  if (density.lg !== undefined) mapped.lg = density.lg;
  if (density.xl !== undefined) mapped.xl = density.xl;
  return Object.keys(mapped).length > 0 ? mapped : null;
}

/**
 * 将主题包 elevation 稀疏字段转换为 gsap.ts 的 ElevationOverride。
 */
function toElevationOverride(
  elevation: ThemePackageElevation | undefined
): ElevationOverride | null {
  if (!elevation) return null;
  const mapped: ElevationOverride = {};
  if (elevation.none !== undefined) mapped.none = elevation.none;
  if (elevation.xs !== undefined) mapped.xs = elevation.xs;
  if (elevation.sm !== undefined) mapped.sm = elevation.sm;
  if (elevation.md !== undefined) mapped.md = elevation.md;
  if (elevation.lg !== undefined) mapped.lg = elevation.lg;
  if (elevation.xl !== undefined) mapped.xl = elevation.xl;
  return Object.keys(mapped).length > 0 ? mapped : null;
}

/**
 * 将主题包 blur 稀疏字段转换为 gsap.ts 的 BlurOverride。
 */
function toBlurOverride(
  blur: ThemePackageBlur | undefined
): BlurOverride | null {
  if (!blur) return null;
  const mapped: BlurOverride = {};
  if (blur.sm !== undefined) mapped.sm = blur.sm;
  if (blur.md !== undefined) mapped.md = blur.md;
  if (blur.lg !== undefined) mapped.lg = blur.lg;
  if (blur.xl !== undefined) mapped.xl = blur.xl;
  return Object.keys(mapped).length > 0 ? mapped : null;
}

/**
 * 将主题包稀疏 fontFamily 转换为 gsap.ts 的 FontFamilyOverride（Phase 4）。
 */
function toFontFamilyOverride(
  ff: ThemePackageFontFamily | undefined
): FontFamilyOverride | null {
  if (!ff) return null;
  const mapped: FontFamilyOverride = {};
  if (ff.body !== undefined) mapped.body = ff.body;
  if (ff.display !== undefined) mapped.display = ff.display;
  if (ff.mono !== undefined) mapped.mono = ff.mono;
  return Object.keys(mapped).length > 0 ? mapped : null;
}

export type EffectiveThemeScheme = 'light' | 'dark';

/** 合并主题包基础 CSS 变量与当前昼夜模式的稀疏覆盖。 */
export function resolveThemePackageCssVariables(
  doc: ThemePackageDocument | null,
  scheme: EffectiveThemeScheme
): Record<string, string> | null {
  if (!doc) return null;
  return {
    ...(doc.cssVariables ?? {}),
    ...(doc.cssVariableVariants?.[scheme] ?? {}),
  };
}

/** 仅重应用与 effective scheme 相关的自定义 CSS 变量。 */
export function applyThemePackageCssVariables(
  doc: ThemePackageDocument | null,
  scheme: EffectiveThemeScheme
): void {
  applyCssVariablesOverride(resolveThemePackageCssVariables(doc, scheme));
}

function currentDocumentScheme(): EffectiveThemeScheme {
  if (typeof document === 'undefined') return 'light';
  return document.documentElement.classList.contains('dark') ? 'dark' : 'light';
}

/**
 * 同步应用一个主题包的所有令牌覆盖到 GSAP 与 CSS 变量。
 *
 * 覆盖域：motion / shape / density / elevation / blur / visualContract /
 * fontFamily / cssVariables（Phase 4 追加）。
 * 传入 `null` 或缺失字段等价于卸载对应覆盖（恢复默认）。
 * setActive / preview / dismissPreview 通过它保持所有域的一致性，
 * 避免遗漏某个域导致视觉状态错乱。
 */
export function applyThemePackageDocument(
  doc: ThemePackageDocument | null,
  scheme: EffectiveThemeScheme = currentDocumentScheme()
): void {
  setThemePackageRuntimeDocument(doc);
  applyMotionOverride(toMotionOverride(doc?.motion));
  applyShapeOverride(toShapeOverride(doc?.shape));
  applyDensityOverride(toDensityOverride(doc?.density));
  applyElevationOverride(toElevationOverride(doc?.elevation));
  applyBlurOverride(toBlurOverride(doc?.blur));
  // Phase 3 Step 3.1：同步 visual contract（family + depth）到 $state 与 data-theme-* 属性
  applyVisualContract(doc?.visualContract);
  // Phase 4：字体族 + 自定义 CSS 变量
  applyFontFamilyOverride(toFontFamilyOverride(doc?.fontFamily));
  applyThemePackageCssVariables(doc, scheme);
  if (typeof document !== 'undefined') {
    if (doc) {
      document.documentElement.dataset.themePackageId = doc.manifest.id;
    } else {
      delete document.documentElement.dataset.themePackageId;
    }
  }
}

export interface ThemePackageRenderOptions {
  animate?: boolean;
  reason?: ThemePackageTransitionReason;
}

export function transitionThemePackageDocument(
  doc: ThemePackageDocument | null,
  options: ThemePackageRenderOptions = {}
): Promise<void> {
  return runThemePackageTransition(
    () => applyThemePackageDocument(doc, currentDocumentScheme()),
    { ...options, targetPackageId: doc?.manifest.id ?? null }
  );
}

/**
 * 事件来源标签。
 *
 * `command`：本会话主动 command 返回的 snapshot
 * `event`：Tauri 广播的 preferences_snapshot 事件（可能来自其他窗口）
 * `startup`：应用启动时 hydrate 拉取的初始 snapshot
 */
export type ThemeReducerSource = 'command' | 'event' | 'startup';

function isRevisionMismatchError(error: unknown): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    error.code === 'revisionMismatch'
  );
}

/** 主题包前端管理器的可测试 API 依赖。 */
export interface ThemePackageManagerDeps {
  getPreferences: typeof getPreferences;
  listPackages: typeof listThemePackages;
  inspectPackage: typeof inspectThemePackage;
  setActivePackage: typeof setActiveThemePackage;
  previewPackage: typeof previewThemePackage;
  dismissPreview: typeof dismissThemePreview;
  installFromFile?: typeof installThemePackageFromFile;
  installFromUrl?: typeof installThemePackageFromUrl;
  uninstallPackage?: typeof uninstallThemePackage;
  exportPackage?: typeof exportThemePackage;
  renderDocument?: (
    doc: ThemePackageDocument | null,
    options: ThemePackageRenderOptions
  ) => void | Promise<void>;
}

export function createThemePackageManager(deps: ThemePackageManagerDeps) {
  let intentSeq = 0;
  let renderSeq = 0;
  let renderEpoch = 0;
  let activationQueue: Promise<void> = Promise.resolve();
  let currentRevision = $state(0);
  let hasSnapshot = false;
  let activePackageId = $state<string | null>(null);
  let previewingId = $state<string | null>(null);
  let displayMode: 'persisted' | 'preview' | 'pending-persist' = 'persisted';
  let pendingPreviewId: string | null = null;
  let installedPackages = $state<ThemePackageSummary[]>([]);
  let latestError = $state<string | null>(null);
  let failedInspectPackageId: string | null = null;
  // Sync 纪元号：syncDomToActive 入口 ++epoch 后异步 await inspect；
  // 若 await 期间又有新的 syncDomToActive 调用（例如快速切主题包），
  // async continuation 检测到 epoch mismatch 会丢弃陈旧结果。

  function currentRenderedPackageId(): string | null | undefined {
    if (typeof document === 'undefined') return undefined;
    return document.documentElement.dataset.themePackageId ?? null;
  }

  function renderDocument(
    doc: ThemePackageDocument | null,
    options: ThemePackageRenderOptions = { animate: false }
  ): Promise<void> {
    const renderedPackageId = currentRenderedPackageId();
    const targetPackageId = doc?.manifest.id ?? null;
    const resolvedOptions =
      options.animate &&
      renderedPackageId !== undefined &&
      renderedPackageId === targetPackageId
        ? { ...options, animate: false }
        : options;
    if (deps.renderDocument) {
      return Promise.resolve(deps.renderDocument(doc, resolvedOptions));
    }
    return transitionThemePackageDocument(doc, resolvedOptions);
  }

  function hasCurrentPreviewIntent(): boolean {
    return displayMode === 'preview';
  }

  /**
   * 从后端拉取最新 preferences 与 packages 列表（初始化 / 手动 rebase 用）。
   *
   * # 悬挂 activePackageId 自愈
   *
   * 若 `preferences.theme.activePackageId` 引用的包已被卸载或未导入（例如
   * Phase 3.2→3.3 flag opt-in→opt-out 切换后的用户，或用户在关闭 UI 期间通过
   * 其他手段修改了 preferences），静默清空 activePackageId 避免应用启动时
   * 尝试激活不存在的包导致的错误。清空通过 setActive(null) 走后端 CAS 流程，
   * 保证与其它窗口的一致性。
   */
  async function hydrate(): Promise<void> {
    try {
      const [prefs, packages] = await Promise.all([
        deps.getPreferences(),
        deps.listPackages(),
      ]);
      const previousActivePackageId = activePackageId;
      applySnapshot(prefs, 'startup');
      installedPackages = packages;
      const shouldRetryFailedInspect =
        displayMode === 'persisted' &&
        activePackageId !== null &&
        activePackageId === previousActivePackageId &&
        activePackageId === failedInspectPackageId;
      latestError = null;
      await selfHealDanglingActive(prefs, packages);
      if (shouldRetryFailedInspect && activePackageId !== null) {
        await syncDomToActive(activePackageId, {
          animate: false,
          reason: 'activate',
        });
      }
    } catch (err) {
      latestError = err instanceof Error ? err.message : String(err);
    }
  }

  /**
   * 悬挂 activePackageId 自愈：
   * - 成功路径立即以返回 snapshot 更新本地态（不依赖后端事件广播——
   *   RevisionMismatch 是错误路径，后端不广播 preferences_snapshot）
   * - CAS 冲突路径 bounded 重试一次：重新 getPreferences 拿最新 revision；
   *   若他人已把 active 改成合法值则跳过；仍冲突则记录警告
   */
  async function selfHealDanglingActive(
    prefs: AppPreferences,
    packages: ThemePackageSummary[]
  ): Promise<void> {
    const active = prefs.theme?.activePackageId ?? null;
    if (!active || packages.some((p) => p.id === active)) return;

    async function tryClear(expectedRevision: number): Promise<boolean> {
      try {
        const snap = await deps.setActivePackage(null, expectedRevision);
        applySnapshot(snap, 'command');
        await syncDomToActive(null, { animate: false });
        return true;
      } catch {
        return false;
      }
    }

    if (await tryClear(prefs.theme?.revision ?? 0)) return;

    try {
      const [latestPrefs, latestPackages] = await Promise.all([
        deps.getPreferences(),
        deps.listPackages(),
      ]);
      installedPackages = latestPackages;
      applySnapshot(latestPrefs, 'startup');
      const latestActive = latestPrefs.theme?.activePackageId ?? null;
      if (!latestActive || latestPackages.some((p) => p.id === latestActive))
        return;
      const ok = await tryClear(latestPrefs.theme?.revision ?? 0);
      if (!ok) {
        latestError = '无法修复失效的主题包引用，下次刷新时将重试';
      }
    } catch (error) {
      latestError = error instanceof Error ? error.message : String(error);
    }
  }

  /**
   * 手动刷新主题包列表（导入 / 卸载后调用）。
   */
  async function refreshList(): Promise<ThemePackageSummary[]> {
    try {
      const packages = await deps.listPackages();
      installedPackages = packages;
      return packages;
    } catch (error) {
      latestError = error instanceof Error ? error.message : String(error);
      throw error;
    }
  }

  /**
   * 应用来自任意来源的 preferences 快照到本地态。
   *
   * reducer 严格按 revision 单调递增接受：
   * - 新 revision > 当前 → 应用
   * - 相同 revision 仅接受同一 active id 的幂等重放
   * - 更小 revision，或相同 revision 携带不同 active id → 丢弃
   *
   * `source` 用于日志与调试，不影响 reducer 决策逻辑。
   */
  function applySnapshot(
    snapshot: AppPreferences,
    source: ThemeReducerSource
  ): boolean {
    const theme = snapshot.theme;
    if (!theme) return false;
    const incoming = theme.revision ?? 0;
    if (incoming < currentRevision) {
      // 老 snapshot，丢弃
      return false;
    }
    const incomingActive = theme.activePackageId ?? null;
    if (
      hasSnapshot &&
      incoming === currentRevision &&
      incomingActive !== activePackageId
    ) {
      // A revision identifies the complete theme write. Accepting a conflicting
      // payload at the same revision could roll back an active package while
      // still passing the monotonicity check.
      return false;
    }
    const prevActive = activePackageId;
    const previewResolvedByActivation =
      previewingId !== null && incomingActive === previewingId;
    currentRevision = incoming;
    activePackageId = incomingActive;
    if (previewResolvedByActivation) {
      previewingId = null;
      if (displayMode === 'preview' && pendingPreviewId === null) {
        displayMode = 'persisted';
      }
    }
    hasSnapshot = true;
    renderSeq += 1;
    // 若 activePackageId 变化（含启动首次赋值），异步同步 DOM 侧的 5 组 token +
    // visualContract 到当前包。command 路径会在 backend preview cleanup 完成后
    // 显式同步，避免同一次用户操作启动两段重叠动画。
    if (
      source !== 'command' &&
      prevActive !== activePackageId &&
      displayMode === 'persisted'
    ) {
      void syncDomToActive(activePackageId, {
        animate: source === 'event',
        reason: 'activate',
      });
    }
    return true;
  }

  /**
   * 根据 activePackageId 拉取文档并应用 5 组 token + visualContract。
   *
   * - `id === null`：清空所有覆盖（恢复 app 默认）
   * - `id === some`：inspect 该包，失败或不存在则视为清空（避免应用陈旧文档）
   *
   * 与 setActive 命令路径不同，此函数由事件 / 启动 / self-heal 路径调用，
   * 不做 CAS 检查，因为 revision 已由 applySnapshot 更新。
   *
   * # Sync epoch 竞态防护
   *
   * inspect 是异步的：连续两次 applySnapshot 可能触发两次 syncDomToActive，
   * 若 resolve 顺序颠倒，陈旧 doc 会覆盖新态。用 `syncEpoch` 计数器捕获入口
   * 纪元号，await 完成后比对；纪元过期则直接丢弃结果。
   */
  async function syncDomToActive(
    id: string | null,
    options: ThemePackageRenderOptions = { animate: false }
  ): Promise<void> {
    const myEpoch = ++renderEpoch;
    const renderedPackageId = currentRenderedPackageId();
    if (
      renderedPackageId !== undefined &&
      renderedPackageId === id &&
      displayMode === 'persisted' &&
      myEpoch === renderEpoch
    ) {
      // The transition commit marker is written only after every package token
      // reaches the DOM. Re-inspecting the same visible package adds no value and
      // could incorrectly replace a successfully previewed-and-applied package
      // with defaults if a transient inspect fails.
      // This request still supersedes any transition whose midpoint has not
      // committed yet; otherwise an older target can overwrite this same-ID
      // authoritative state after the early return.
      cancelObsoleteThemePackageTransition(id);
      if (failedInspectPackageId === id) failedInspectPackageId = null;
      return;
    }
    if (id === null) {
      if (displayMode === 'persisted' && myEpoch === renderEpoch) {
        await renderDocument(null, options);
        failedInspectPackageId = null;
      }
      return;
    }
    // inspect 失败区分两类：dangling ref 由 selfHealDanglingActive 兜底闭环，
    // 但事件同步 / 瞬时 IO 错误路径下用户会看到主题"无声消失"却无反馈。
    // 这里写 latestError + console.warn 留痕，UI 可通过 latestError 决定是否
    // toast；仍 fallback 到默认覆盖避免冻结 UI。
    let doc: ThemePackageDocument | null;
    let inspectFailed = false;
    try {
      doc = await deps.inspectPackage(id);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      latestError = `无法加载主题包 ${id}: ${message}`;
      failedInspectPackageId = id;
      inspectFailed = true;
      doc = null;
    }
    if (myEpoch !== renderEpoch || displayMode !== 'persisted') {
      // 有更新的 syncDomToActive 请求在飞或已完成，丢弃陈旧结果
      return;
    }
    await renderDocument(doc, options);
    if (!inspectFailed && failedInspectPackageId === id) {
      failedInspectPackageId = null;
    }
  }

  /**
   * 通过 CAS 激活指定主题包（或传 null 清空激活状态）。
   *
   * @returns 新的 preferences 快照；若发生 RevisionMismatch 会先 hydrate，
   *   仅在 active 包未变化时自动 rebase 一次，否则向调用方返回冲突。
   */
  async function commitActiveIntent(
    id: string | null,
    localIntent: number
  ): Promise<AppPreferences> {
    const activePackageIdAtAttempt = activePackageId;
    let snapshot: AppPreferences;
    try {
      snapshot = await deps.setActivePackage(id, currentRevision);
    } catch (err) {
      if (localIntent === intentSeq) {
        displayMode = previewingId ? 'preview' : 'persisted';
      }
      // CAS 冲突或其他错误：拉取最新态，让上层重试
      await hydrate();
      if (
        localIntent === intentSeq &&
        isRevisionMismatchError(err) &&
        activePackageId === activePackageIdAtAttempt
      ) {
        // An ordinary settings save advances the shared snapshot revision too.
        // If the persisted package selection itself did not change, rebasing
        // once is safe and prevents a nearby settings auto-save from making the
        // Apply button appear to fail. A real cross-window package switch still
        // falls through to the conflict path below.
        if (localIntent === intentSeq) displayMode = 'pending-persist';
        try {
          snapshot = await deps.setActivePackage(id, currentRevision);
        } catch (retryError) {
          if (localIntent === intentSeq) {
            displayMode = previewingId ? 'preview' : 'persisted';
          }
          await hydrate();
          if (localIntent === intentSeq && displayMode === 'persisted') {
            await syncDomToActive(activePackageId);
          }
          throw retryError;
        }
      } else {
        if (localIntent === intentSeq && displayMode === 'persisted') {
          // A preferences_snapshot received while this command was pending is
          // intentionally state-only.  Its revision is equal to the subsequent
          // hydrate response, so the reducer will not schedule another render;
          // explicitly reconcile the DOM after the failed CAS instead.
          await syncDomToActive(activePackageId);
        }
        throw err;
      }
    }

    // Even a superseded local command is an authoritative backend write. Apply
    // its revision so the next queued activation rebases instead of repeating
    // the stale CAS token.
    const activeBeforeSnapshot = activePackageId;
    const snapshotAccepted = applySnapshot(snapshot, 'command');
    const commandChangedActive =
      snapshotAccepted && activePackageId !== activeBeforeSnapshot;
    const renderEpochAfterCommandSnapshot = renderEpoch;

    if (
      localIntent !== intentSeq &&
      displayMode === 'persisted' &&
      commandChangedActive
    ) {
      // Closing the settings sheet can dismiss an existing preview while this
      // activation is still in flight. If the dismiss finishes first, it paints
      // the previous active package; the later successful command must then
      // reconcile the DOM even though its local intent was superseded. A newer
      // preview or activation retains ownership through the other display modes.
      await syncDomToActive(activePackageId, {
        animate: true,
        reason: 'activate',
      });
    }

    // 到这里持久化已经成功。后续 preview cleanup 失败不能把整次激活伪装成失败。
    if (localIntent === intentSeq) {
      previewingId = null;
      pendingPreviewId = null;
      displayMode = 'persisted';
      try {
        await deps.dismissPreview();
      } catch (error) {
        latestError = error instanceof Error ? error.message : String(error);
      }
      if (
        localIntent === intentSeq &&
        renderEpoch === renderEpochAfterCommandSnapshot
      ) {
        await syncDomToActive(activePackageId, {
          animate: true,
          reason: 'activate',
        });
      }
    }
    return snapshot;
  }

  function setActive(id: string | null): Promise<AppPreferences> {
    const localIntent = ++intentSeq;
    pendingPreviewId = null;
    latestError = null;
    displayMode = 'pending-persist';
    renderEpoch += 1;

    // Serialize local activations so every command uses the revision returned
    // by the previous one. This guarantees that a rapid second click is the
    // final persisted intent instead of failing CAS against the first click.
    const operation = activationQueue.then(() =>
      commitActiveIntent(id, localIntent)
    );
    activationQueue = operation.then(
      () => undefined,
      () => undefined
    );
    return operation;
  }

  /**
   * 进入预览态。前端消费返回的 document 派生 token 应用到 DOM。
   *
   * 覆盖 pending-persist 分支（P1-2）：无论此前是否有 pending-persist 的
   * setActive 请求，preview 都会将 previewingId 设为新值，并递增 intentSeq
   * 使 pending 请求返回时被丢弃。
   *
   * # visualContract 一致性
   *
   * preview 与 setActive 都走 `applyThemePackageDocument`，内部调用 `applyVisualContract`
   * 写入 `data-theme-family` / `data-theme-depth` 到 `<html>`。因此 Router 组件
   * （PlayToggleGlyph / VolumeCapsule）在预览与激活两种状态下读到的 `contract.family`
   * 一致，视觉表现严格对齐，不会出现"预览 terminal 但看到 glass"的错乱。
   */
  async function preview(id: string): Promise<ThemePackageDocument> {
    const localIntent = ++intentSeq;
    pendingPreviewId = id;
    latestError = null;
    displayMode = 'preview';
    const myRenderEpoch = ++renderEpoch;
    let doc: ThemePackageDocument;
    try {
      doc = await deps.previewPackage(id);
    } catch (error) {
      if (localIntent === intentSeq) {
        pendingPreviewId = null;
        displayMode = previewingId ? 'preview' : 'persisted';
        if (!previewingId) {
          try {
            // An older preview command may have registered after this newer
            // request started. Since the newest preview failed, no backend
            // preview should survive when there is no valid local preview.
            await deps.dismissPreview();
          } catch (cleanupError) {
            if (localIntent === intentSeq) {
              latestError =
                cleanupError instanceof Error
                  ? cleanupError.message
                  : String(cleanupError);
            }
          }
          if (localIntent === intentSeq) {
            await syncDomToActive(activePackageId, {
              animate: true,
              reason: 'activate',
            });
          }
        }
      }
      throw error;
    }
    if (localIntent !== intentSeq) {
      // A dismiss can reach the backend before this slower preview command
      // registers its state. Clear once more after the stale response so
      // closing the sheet cannot leave a server-side preview behind. Preserve
      // a newer preview intent: its command owns the final backend state.
      if (!hasCurrentPreviewIntent()) {
        try {
          await deps.dismissPreview();
        } catch {
          // This intent is already obsolete; the current operation owns any
          // user-visible error and reconciliation state.
        }
      }
      return doc;
    }
    pendingPreviewId = null;
    if (activePackageId === id) {
      previewingId = null;
      displayMode = 'persisted';
      if (myRenderEpoch === renderEpoch) {
        await renderDocument(doc, {
          animate: true,
          reason: 'activate',
        });
      }
      try {
        await deps.dismissPreview();
      } catch (error) {
        latestError = error instanceof Error ? error.message : String(error);
      }
      return doc;
    }
    previewingId = id;
    // 预览态同步全部 5 组 override（motion/shape/density/elevation/blur）+ visualContract，
    // 让 GSAP + CSS + Router 立即反映主题包节奏与家族切换
    if (myRenderEpoch === renderEpoch) {
      await renderDocument(doc, { animate: true, reason: 'preview' });
    }
    return doc;
  }

  /**
   * 关闭预览态。
   *
   * 覆盖 pending-persist 分支（P1-2）：即使此前有 setActive 请求在飞，
   * 也会作废其影响 preview 的能力（intent bump），并主动清空预览 id，
   * 避免用户按 ESC 后 UI 仍显示预览状态导致视觉冻结。
   */
  async function dismissPreview(): Promise<void> {
    const localIntent = ++intentSeq;
    cancelThemePackageTransition();
    pendingPreviewId = null;
    latestError = null;
    const myRenderEpoch = ++renderEpoch;
    // Invalidate local preview state before awaiting IPC.  Closing the settings
    // sheet must be enough to stop a late preview response from repainting DOM.
    previewingId = null;
    displayMode = 'persisted';
    let dismissError: unknown = null;
    try {
      await deps.dismissPreview();
    } catch (error) {
      dismissError = error;
      latestError = error instanceof Error ? error.message : String(error);
    }
    if (localIntent === intentSeq) {
      // 退出预览：如果 activePackageId 存在则恢复其覆盖，否则彻底清空
      if (activePackageId) {
        let doc: ThemePackageDocument | null;
        try {
          doc = await deps.inspectPackage(activePackageId);
        } catch (error) {
          const message =
            error instanceof Error ? error.message : String(error);
          latestError = `无法恢复主题包 ${activePackageId}: ${message}`;
          doc = null;
        }
        if (localIntent === intentSeq && myRenderEpoch === renderEpoch) {
          await renderDocument(doc, {
            animate: true,
            reason: 'dismiss-preview',
          });
        }
      } else if (myRenderEpoch === renderEpoch) {
        await renderDocument(null, {
          animate: true,
          reason: 'dismiss-preview',
        });
      }
    }
    if (dismissError !== null && localIntent === intentSeq) {
      throw dismissError instanceof Error
        ? dismissError
        : new Error(String(dismissError));
    }
  }

  /**
   * 从文件路径导入并安装主题包，刷新列表。
   */
  async function importFromFile(path: string): Promise<ThemePackageSummary> {
    const localIntent = ++intentSeq;
    pendingPreviewId = null;
    latestError = null;
    if (displayMode === 'preview' && previewingId === null) {
      displayMode = 'persisted';
      renderEpoch += 1;
      void syncDomToActive(activePackageId);
    }
    const install = deps.installFromFile ?? installThemePackageFromFile;
    const summary = await install(path);
    await refreshList();
    await reapplyInstalledPackage(summary.id, localIntent);
    return summary;
  }

  /**
   * 从 https URL 下载并安装主题包，刷新列表。
   *
   * 后端做 SSRF 校验 + sanitize；前端仅负责传入原始 URL 与刷新态。
   */
  async function importFromUrl(url: string): Promise<ThemePackageSummary> {
    const localIntent = ++intentSeq;
    pendingPreviewId = null;
    latestError = null;
    if (displayMode === 'preview' && previewingId === null) {
      displayMode = 'persisted';
      renderEpoch += 1;
      void syncDomToActive(activePackageId);
    }
    const install = deps.installFromUrl ?? installThemePackageFromUrl;
    const summary = await install(url);
    await refreshList();
    await reapplyInstalledPackage(summary.id, localIntent);
    return summary;
  }

  /** Re-read an overwritten document so active/preview DOM never keeps stale data. */
  async function reapplyInstalledPackage(
    id: string,
    localIntent: number
  ): Promise<void> {
    if (localIntent !== intentSeq) return;
    const isPreviewing = displayMode === 'preview' && previewingId === id;
    const isActive = displayMode === 'persisted' && activePackageId === id;
    if (!isPreviewing && !isActive) return;
    try {
      const doc = await deps.inspectPackage(id);
      if (localIntent !== intentSeq) return;
      const stillPreviewing = displayMode === 'preview' && previewingId === id;
      const stillActive = displayMode === 'persisted' && activePackageId === id;
      if (!stillPreviewing && !stillActive) return;
      await renderDocument(doc);
    } catch (error) {
      latestError = error instanceof Error ? error.message : String(error);
      throw error;
    }
  }

  /**
   * 检查指定主题包的完整文档。
   */
  async function inspect(id: string): Promise<ThemePackageDocument | null> {
    return deps.inspectPackage(id);
  }

  /**
   * 卸载指定主题包，刷新列表。
   * 后端在卸载激活包时会广播 preferences_snapshot 事件，reducer 自动接收。
   */
  async function uninstall(id: string): Promise<void> {
    const localIntent = ++intentSeq;
    pendingPreviewId = null;
    latestError = null;
    const wasPreviewing = previewingId === id;
    const wasActive = activePackageId === id;
    const preservedPreviewId =
      previewingId !== null && previewingId !== id ? previewingId : null;
    displayMode = 'pending-persist';
    renderEpoch += 1;
    const uninstallPackage = deps.uninstallPackage ?? uninstallThemePackage;
    try {
      await uninstallPackage(id);
      if (localIntent !== intentSeq) return;
      // The backend clears its in-memory preview state as part of uninstall;
      // clear the local state before refreshing so no stale badge/DOM survives.
      if (wasPreviewing) {
        previewingId = null;
        displayMode = 'persisted';
        if (activePackageId) {
          await syncDomToActive(activePackageId);
        } else {
          await renderDocument(null);
        }
        if (localIntent !== intentSeq) return;
        try {
          // Keep this explicit for older backends and for a failed in-flight
          // preview command; the service operation is intentionally idempotent.
          await deps.dismissPreview();
        } catch (error) {
          latestError = error instanceof Error ? error.message : String(error);
        }
      }
      displayMode =
        preservedPreviewId !== null && previewingId === preservedPreviewId
          ? 'preview'
          : 'persisted';
      await refreshList();
      await hydrate();
      if (localIntent !== intentSeq) return;
      // `hydrate()` may receive the same revision that was already accepted
      // while the uninstall was pending.  Reconcile explicitly even when the
      // removed package was neither active nor previewed, because another
      // window may have changed the active package during this operation.
      if (displayMode === 'persisted') {
        if (activePackageId) {
          await syncDomToActive(activePackageId);
        } else {
          await renderDocument(null);
        }
      }
    } catch (error) {
      if (localIntent === intentSeq) {
        displayMode = previewingId ? 'preview' : 'persisted';
        latestError = error instanceof Error ? error.message : String(error);
        // `uninstall_theme_package` persists an active-reference rollback
        // before attempting the filesystem move.  If that move fails, its
        // snapshot is still authoritative even though the command rejects;
        // refresh it before returning so the old package document cannot stay
        // painted after activePackageId has already been cleared.
        if (!previewingId) {
          const failureMessage = latestError;
          if (wasActive) await hydrate();
          if (localIntent === intentSeq) {
            if (activePackageId) {
              await syncDomToActive(
                activePackageId,
                wasActive
                  ? { animate: false }
                  : { animate: true, reason: 'activate' }
              );
            } else {
              await renderDocument(
                null,
                wasActive
                  ? { animate: false }
                  : { animate: true, reason: 'activate' }
              );
            }
            latestError = failureMessage;
          }
        }
      }
      throw error;
    }
  }

  /**
   * 导出指定主题包原始 JSON 到本地路径。
   */
  async function exportPackage(id: string, outputPath: string): Promise<void> {
    const exportTheme = deps.exportPackage ?? exportThemePackage;
    return exportTheme(id, outputPath);
  }

  return {
    // Reactive state
    get currentRevision() {
      return currentRevision;
    },
    get activePackageId() {
      return activePackageId;
    },
    get previewingId() {
      return previewingId;
    },
    get installedPackages() {
      return installedPackages;
    },
    get latestError() {
      return latestError;
    },
    // Actions
    hydrate,
    refreshList,
    setActive,
    preview,
    dismissPreview,
    importFromFile,
    importFromUrl,
    inspect,
    uninstall,
    exportPackage,
    applyPreferencesSnapshot: (snapshot: AppPreferences) =>
      applySnapshot(snapshot, 'event'),
    // Testing hooks
    _applySnapshot: applySnapshot,
    _getIntentSeq: () => intentSeq,
    _getRenderSeq: () => renderSeq,
  };
}

export type ThemePackageManager = ReturnType<typeof createThemePackageManager>;

let sharedThemePackageManager: ThemePackageManager | null = null;

/** 主窗口唯一的主题包管理器；设置抽屉与 app runtime 共享同一订阅和预览态。 */
export function getThemePackageManager(): ThemePackageManager {
  sharedThemePackageManager ??= createThemePackageManager({
    getPreferences,
    listPackages: listThemePackages,
    inspectPackage: inspectThemePackage,
    setActivePackage: setActiveThemePackage,
    previewPackage: previewThemePackage,
    dismissPreview: dismissThemePreview,
    installFromFile: installThemePackageFromFile,
    installFromUrl: installThemePackageFromUrl,
    uninstallPackage: uninstallThemePackage,
    exportPackage: exportThemePackage,
  });
  return sharedThemePackageManager;
}
