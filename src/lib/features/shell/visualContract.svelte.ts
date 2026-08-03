/**
 * Visual Contract 状态源（Phase 3 Step 3.1）。
 *
 * 提供 family × depth 的响应式状态，供 view 层组件通过 `$derived` 消费；
 * 同时把值写到 `<html>` 的 `data-theme-family` / `data-theme-depth` 属性，
 * 让 CSS 侧的 `:root[data-theme-family='material']` 选择器生效。
 *
 * # 双真相源同步策略（主方案 §2.5）
 *
 * - **JS 侧**：`getVisualContract()` 返回 `$state` 引用，`$derived` 自动重求值
 * - **CSS 侧**：`data-theme-family` / `data-theme-depth` HTML 属性
 *
 * 两者由 `applyVisualContract` 一次写入保证同步，不允许分离修改任一侧。
 *
 * # 版本化支持集
 *
 * `SUPPORTED_THEME_FAMILIES` / `SUPPORTED_THEME_DEPTHS` 声明当前 app 版本
 * 已实现的 family / depth 值。主题包声明超出支持集的 family 会 fallback 到
 * `glass` / `balanced`，同时通过 sanitize warnings 累积告警。
 */

/**
 * 视觉语言族。字面量联合，编译期约束支持集。
 */
export type ThemeFamily =
  | 'glass'
  | 'material'
  | 'terminal'
  | 'ark'
  | 'endfield'
  | 'exa'
  | 'popucom'
  | 'corporate';

/**
 * 视觉深度层级。
 */
export type ThemeDepth =
  | 'flat'
  | 'balanced'
  | 'deep'
  | 'minimal'
  | 'moderate'
  | 'complex'
  | 'maximal';

/**
 * 当前 app 版本已实现的 family 支持集。
 *
 * Phase 3.1：`glass`（视觉零变化的默认族）+ `material`（POC 阶段）
 * Phase 3.3：追加 `terminal`（monochrome + 直角 + 无动画）
 */
export const SUPPORTED_THEME_FAMILIES: readonly ThemeFamily[] = [
  'glass',
  'material',
  'terminal',
  'ark',
  'endfield',
  'exa',
  'popucom',
  'corporate',
];

/**
 * 当前 app 版本已实现的 depth 支持集。
 */
export const SUPPORTED_THEME_DEPTHS: readonly ThemeDepth[] = [
  'flat',
  'balanced',
  'deep',
  'minimal',
  'moderate',
  'complex',
  'maximal',
];

export const ARK_UI_THEME_FAMILIES = [
  'ark',
  'endfield',
  'exa',
  'popucom',
  'corporate',
] as const satisfies readonly ThemeFamily[];

export type ArkUiThemeFamily = (typeof ARK_UI_THEME_FAMILIES)[number];

export function isArkUiThemeFamily(
  family: ThemeFamily
): family is ArkUiThemeFamily {
  return (ARK_UI_THEME_FAMILIES as readonly ThemeFamily[]).includes(family);
}

const STRUCTURED_CONTROL_FAMILIES: readonly ThemeFamily[] = [
  'material',
  'terminal',
  'ark',
  'endfield',
  'corporate',
];

/** 选择现有的硬边/elevation 控件实现；颜色和几何仍由 package token 决定。 */
export function usesStructuredControls(family: ThemeFamily): boolean {
  return STRUCTURED_CONTROL_FAMILIES.includes(family);
}

/**
 * visualContract 解析结果，一定在支持集内。
 */
export interface ResolvedVisualContract {
  family: ThemeFamily;
  depth: ThemeDepth;
  /** 解析过程中产生的降级告警（fallback 时非空） */
  warnings: string[];
}

/**
 * 把主题包声明的 visualContract 解析到支持集内。
 *
 * 未声明 / 未知 family → fallback 到 `glass` 并 warn；未声明 / 未知 depth → `balanced`。
 */
export function resolveVisualContract(
  raw:
    | {
        family?: string;
        depth?: string;
      }
    | null
    | undefined
): ResolvedVisualContract {
  const warnings: string[] = [];
  const rawFamily = raw?.family;
  const family =
    rawFamily &&
    (SUPPORTED_THEME_FAMILIES as readonly string[]).includes(rawFamily)
      ? (rawFamily as ThemeFamily)
      : 'glass';
  if (rawFamily && family !== rawFamily) {
    warnings.push(
      `family=${rawFamily} not supported by current app version, fell back to glass`
    );
  }
  const rawDepth = raw?.depth;
  const depth =
    rawDepth && (SUPPORTED_THEME_DEPTHS as readonly string[]).includes(rawDepth)
      ? (rawDepth as ThemeDepth)
      : 'balanced';
  if (rawDepth && depth !== rawDepth) {
    warnings.push(`depth=${rawDepth} not supported, fell back to balanced`);
  }
  return { family, depth, warnings };
}

/**
 * 响应式 visual contract 状态。
 *
 * 使用 Svelte 5 `$state` 让 `$derived(getVisualContract())` 在主题包切换时
 * 自动重求值；不直接暴露原始对象引用给外部，防止未经 apply 就被改。
 */
const visualContractState = $state<{
  family: ThemeFamily;
  depth: ThemeDepth;
}>({
  family: 'glass',
  depth: 'balanced',
});

/**
 * 获取当前 visual contract 状态引用。
 *
 * 返回的对象是响应式的 $state，可被 $derived 追踪；
 * 只读消费，写入必须通过 `applyVisualContract`。
 */
export function getVisualContract(): {
  readonly family: ThemeFamily;
  readonly depth: ThemeDepth;
} {
  return visualContractState;
}

/**
 * 应用主题包声明的 visualContract 到 JS state 与 DOM 属性。
 *
 * 传 `null` 或缺失字段等价于 fallback 到默认 `glass` / `balanced`。
 * 该函数是主题包激活 / 预览 / 卸载三处的唯一入口，保证 JS 侧 $state
 * 与 CSS 侧 `data-theme-family` 属性同步更新。
 *
 * 返回 resolve 时产生的 warnings（fallback 事件），供 UI 层展示给用户。
 */
export function applyVisualContract(
  raw: { family?: string; depth?: string } | null | undefined
): string[] {
  const resolved = resolveVisualContract(raw);
  visualContractState.family = resolved.family;
  visualContractState.depth = resolved.depth;
  if (typeof document !== 'undefined') {
    document.documentElement.dataset.themeFamily = resolved.family;
    document.documentElement.dataset.themeDepth = resolved.depth;
    document.documentElement.dataset.arkTheme = resolved.family;
    document.documentElement.dataset.arkDepth = resolved.depth;
  }
  return resolved.warnings;
}
