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
 * `setActive` 携带上次读取到的 `revision` 调用后端。若后端返回 `RevisionMismatch`：
 * 1. 前端应展示"版本已过期"提示
 * 2. 用户重试时先 `hydrate()` 拉取最新 preferences 再调用 `setActive`
 *
 * 成功路径不需要客户端手动构造 snapshot——后端返回完整 `AppPreferences`
 * 直接作为新态（消除 P1-1 的 customColors 倒退问题）。
 *
 * # `preferences_snapshot` 订阅
 *
 * 后端在 preferences 变更后广播完整快照。reducer 严格按 `revision` 单调递增
 * 接受：老快照直接丢弃，避免网络乱序导致的态回滚。
 */

import type { listen as tauriListen } from '@tauri-apps/api/event';
import type {
  AppPreferences,
  ThemePackageBlur,
  ThemePackageDensity,
  ThemePackageDocument,
  ThemePackageElevation,
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
  applyDensityOverride,
  applyElevationOverride,
  applyMotionOverride,
  applyShapeOverride,
  type BlurOverride,
  type DensityOverride,
  type ElevationOverride,
  type MotionOverride,
  type ShapeOverride,
} from '$lib/design/gsap';
import { applyVisualContract } from '$lib/features/shell/visualContract.svelte';

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
 * 同步应用一个主题包的所有令牌覆盖到 GSAP 与 CSS 变量。
 *
 * 覆盖域：motion / shape / density / elevation / blur。
 * 传入 `null` 或缺失字段等价于卸载对应覆盖（恢复默认）。
 * setActive / preview / dismissPreview 通过它保持所有域的一致性，
 * 避免遗漏某个域导致视觉状态错乱。
 */
function applyPackageOverrides(doc: ThemePackageDocument | null): void {
  applyMotionOverride(toMotionOverride(doc?.motion));
  applyShapeOverride(toShapeOverride(doc?.shape));
  applyDensityOverride(toDensityOverride(doc?.density));
  applyElevationOverride(toElevationOverride(doc?.elevation));
  applyBlurOverride(toBlurOverride(doc?.blur));
  // Phase 3 Step 3.1：同步 visual contract（family + depth）到 $state 与 data-theme-* 属性
  applyVisualContract(doc?.visualContract);
}

/**
 * 事件来源标签。
 *
 * `command`：本会话主动 command 返回的 snapshot
 * `event`：Tauri 广播的 preferences_snapshot 事件（可能来自其他窗口）
 * `startup`：应用启动时 hydrate 拉取的初始 snapshot
 */
export type ThemeReducerSource = 'command' | 'event' | 'startup';

/**
 * 主题包前端管理器依赖。
 *
 * `listen` 参数注入让测试可以模拟事件系统，无需依赖 Tauri runtime。
 */
export interface ThemePackageManagerDeps {
  listen: typeof tauriListen;
}

export function createThemePackageManager(deps: ThemePackageManagerDeps) {
  let intentSeq = 0;
  let renderSeq = 0;
  let currentRevision = $state(0);
  let activePackageId = $state<string | null>(null);
  let previewingId = $state<string | null>(null);
  let installedPackages = $state<ThemePackageSummary[]>([]);
  let latestError = $state<string | null>(null);
  let unlistenSnapshot: (() => void) | null = null;
  // 订阅纪元号：startSubscription 入口 ++epoch 后异步 await deps.listen；
  // 若 await 期间 stopSubscription 再次 ++epoch，async continuation 检测到
  // epoch mismatch 会 unlisten 刚 resolve 的 fn，防止订阅泄漏与幻影事件。
  let subscriptionEpoch = 0;

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
        getPreferences(),
        listThemePackages(),
      ]);
      applySnapshot(prefs, 'startup');
      installedPackages = packages;
      latestError = null;
      await selfHealDanglingActive(prefs, packages);
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
        const snap = await setActiveThemePackage(null, expectedRevision);
        applySnapshot(snap, 'command');
        return true;
      } catch {
        return false;
      }
    }

    if (await tryClear(prefs.theme?.revision ?? 0)) return;

    try {
      const [latestPrefs, latestPackages] = await Promise.all([
        getPreferences(),
        listThemePackages(),
      ]);
      installedPackages = latestPackages;
      applySnapshot(latestPrefs, 'startup');
      const latestActive = latestPrefs.theme?.activePackageId ?? null;
      if (!latestActive || latestPackages.some((p) => p.id === latestActive))
        return;
      const ok = await tryClear(latestPrefs.theme?.revision ?? 0);
      if (!ok) {
        console.warn(
          '[themePackageManager] dangling activePackageId self-heal failed after retry; will retry on next hydrate'
        );
      }
    } catch (err) {
      console.warn(
        '[themePackageManager] dangling self-heal retry failed:',
        err
      );
    }
  }

  /**
   * 手动刷新主题包列表（导入 / 卸载后调用）。
   */
  async function refreshList(): Promise<ThemePackageSummary[]> {
    const packages = await listThemePackages();
    installedPackages = packages;
    return packages;
  }

  /**
   * 应用来自任意来源的 preferences 快照到本地态。
   *
   * reducer 严格按 revision 单调递增接受：
   * - 新 revision > 当前 → 应用
   * - 新 revision <= 当前 → 丢弃（可能是乱序事件或过期响应）
   *
   * `source` 用于日志与调试，不影响 reducer 决策逻辑。
   */
  function applySnapshot(
    snapshot: AppPreferences,
    _source: ThemeReducerSource
  ): boolean {
    const theme = snapshot.theme;
    if (!theme) return false;
    const incoming = theme.revision ?? 0;
    if (incoming < currentRevision) {
      // 老 snapshot，丢弃
      return false;
    }
    currentRevision = incoming;
    activePackageId = theme.activePackageId ?? null;
    renderSeq += 1;
    return true;
  }

  /**
   * 通过 CAS 激活指定主题包（或传 null 清空激活状态）。
   *
   * @returns 新的 preferences 快照；若发生 RevisionMismatch 会先 hydrate
   *   再抛出原始错误，让调用者决定是否重试。
   */
  async function setActive(id: string | null): Promise<AppPreferences> {
    const localIntent = ++intentSeq;
    try {
      const snapshot = await setActiveThemePackage(id, currentRevision);
      // 只有最新的意图才能改变本地态（防止乱序响应覆盖后续操作）
      if (localIntent === intentSeq) {
        applySnapshot(snapshot, 'command');
        // 同步 motion 覆盖：激活主题包时读取其 motion 字段并应用，
        // 清空激活状态（id=null）时恢复默认档位
        if (id === null) {
          applyPackageOverrides(null);
        } else {
          const doc = await inspectThemePackage(id).catch(() => null);
          applyPackageOverrides(doc);
        }
      }
      return snapshot;
    } catch (err) {
      // CAS 冲突或其他错误：拉取最新态，让上层重试
      await hydrate();
      throw err;
    }
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
   * preview 与 setActive 都走 `applyPackageOverrides`，内部调用 `applyVisualContract`
   * 写入 `data-theme-family` / `data-theme-depth` 到 `<html>`。因此 Router 组件
   * （PlayToggleGlyph / VolumeCapsule）在预览与激活两种状态下读到的 `contract.family`
   * 一致，视觉表现严格对齐，不会出现"预览 terminal 但看到 glass"的错乱。
   */
  async function preview(id: string): Promise<ThemePackageDocument> {
    const localIntent = ++intentSeq;
    const doc = await previewThemePackage(id);
    if (localIntent === intentSeq) {
      previewingId = id;
      // 预览态同步全部 5 组 override（motion/shape/density/elevation/blur）+ visualContract，
      // 让 GSAP + CSS + Router 立即反映主题包节奏与家族切换
      applyPackageOverrides(doc);
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
    await dismissThemePreview();
    if (localIntent === intentSeq) {
      previewingId = null;
      // 退出预览：如果 activePackageId 存在则恢复其覆盖，否则彻底清空
      if (activePackageId) {
        const doc = await inspectThemePackage(activePackageId).catch(
          () => null
        );
        applyPackageOverrides(doc);
      } else {
        applyPackageOverrides(null);
      }
    }
  }

  /**
   * 从文件路径导入并安装主题包，刷新列表。
   */
  async function importFromFile(path: string): Promise<ThemePackageSummary> {
    const summary = await installThemePackageFromFile(path);
    await refreshList();
    return summary;
  }

  /**
   * 从 https URL 下载并安装主题包，刷新列表。
   *
   * 后端做 SSRF 校验 + sanitize；前端仅负责传入原始 URL 与刷新态。
   */
  async function importFromUrl(url: string): Promise<ThemePackageSummary> {
    const summary = await installThemePackageFromUrl(url);
    await refreshList();
    return summary;
  }

  /**
   * 检查指定主题包的完整文档。
   */
  async function inspect(id: string): Promise<ThemePackageDocument | null> {
    return inspectThemePackage(id);
  }

  /**
   * 卸载指定主题包，刷新列表。
   * 后端在卸载激活包时会广播 preferences_snapshot 事件，reducer 自动接收。
   */
  async function uninstall(id: string): Promise<void> {
    await uninstallThemePackage(id);
    await refreshList();
  }

  /**
   * 导出指定主题包原始 JSON 到本地路径。
   */
  async function exportPackage(id: string, outputPath: string): Promise<void> {
    return exportThemePackage(id, outputPath);
  }

  /**
   * 启动 preferences_snapshot 事件订阅，并返回释放函数。
   *
   * 幂等：重复调用会先释放旧订阅再建立新订阅。
   *
   * # Pending-listen 竞态防护
   *
   * `deps.listen` 是异步的：await 期间用户可能通过 stopSubscription 关闭订阅。
   * 用 `subscriptionEpoch` 计数器捕获入口时刻的纪元号，await 完成后比对：
   * - 纪元一致：正常写入 unlistenSnapshot
   * - 纪元过期：说明 stop 已发生，主动 unlisten 刚 resolve 的 fn 后返回
   *
   * 避免"start 起飞、stop 到达、listen resolve、订阅泄漏"的场景。
   */
  async function startSubscription(): Promise<void> {
    if (unlistenSnapshot) {
      unlistenSnapshot();
      unlistenSnapshot = null;
    }
    const myEpoch = ++subscriptionEpoch;
    const unlisten = await deps.listen<AppPreferences>(
      'preferences_snapshot',
      (event) => {
        applySnapshot(event.payload, 'event');
      }
    );
    if (myEpoch !== subscriptionEpoch) {
      // stopSubscription 已经发生（epoch 递增过），释放刚 resolve 的 fn 后放弃
      unlisten();
      return;
    }
    unlistenSnapshot = unlisten;
  }

  function stopSubscription(): void {
    // 递增 epoch 使 in-flight start 的 continuation 作废
    subscriptionEpoch += 1;
    if (unlistenSnapshot) {
      unlistenSnapshot();
      unlistenSnapshot = null;
    }
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
    startSubscription,
    stopSubscription,
    // Testing hooks
    _applySnapshot: applySnapshot,
    _getIntentSeq: () => intentSeq,
    _getRenderSeq: () => renderSeq,
  };
}

export type ThemePackageManager = ReturnType<typeof createThemePackageManager>;
