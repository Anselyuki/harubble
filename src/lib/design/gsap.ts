import gsap from 'gsap';
import { Flip } from 'gsap/Flip';
import { CustomEase } from 'gsap/CustomEase';
import { ScrollToPlugin } from 'gsap/ScrollToPlugin';

gsap.registerPlugin(Flip, CustomEase, ScrollToPlugin);

CustomEase.create('ios', '0.25, 0.1, 0.25, 1.0');
CustomEase.create('ios-in', '0.42, 0, 1, 1');
CustomEase.create('ios-out', '0, 0, 0.58, 1');
CustomEase.create('ios-spring', '0.22, 0.61, 0.36, 1');

export { gsap, Flip };

export const reducedMotionQuery =
  typeof window !== 'undefined'
    ? window.matchMedia('(prefers-reduced-motion: reduce)')
    : null;

/**
 * 当前是否处于 reduced-motion 模式。
 *
 * 模块加载后监听 `matchMedia('change')`；用户在系统偏好中切换动效开关时
 * 该值实时更新，飞行中的 tween 不会被强制截断（GSAP 已经在运行），
 * 但下一次调用 `getMotionDuration` / `shouldSkipMotion` 会立即反映新态。
 */
let reducedMotionActive = reducedMotionQuery?.matches ?? false;

if (
  reducedMotionQuery &&
  typeof reducedMotionQuery.addEventListener === 'function'
) {
  reducedMotionQuery.addEventListener('change', (event) => {
    reducedMotionActive = event.matches;
  });
}

/**
 * 主题包可选择覆盖 MOTION 档位。null 表示未覆盖，落回默认档位。
 *
 * Phase 2 Step 2.c：主题包激活时应用 `motion.durations` 到此对象，
 * 卸载 / 切回默认时置回 null。
 */
export type MotionOverride = Partial<Record<keyof typeof MOTION, number>>;
let motionOverride: MotionOverride | null = null;

/**
 * 是否应完全跳过动画（reduced-motion 开启）。
 *
 * 调用点应据此在 tween 之前直接同步落到终态，而不是让 tween 以 0 秒运行——
 * `duration: 0` 的 tween 仍会调度到下一帧，可能引入闪烁。
 */
export function shouldSkipMotion(): boolean {
  return reducedMotionActive;
}

/**
 * 根据档位毫秒数返回 GSAP 使用的秒数。
 *
 * reduced-motion 时返回 0；主题包 override 命中时按覆盖值换算。
 * 老调用点无需修改（0 秒 tween 等价于瞬时跳到终态）；
 * 新代码若需要区分"跳过 tween"与"0 秒 tween"，请配合 `shouldSkipMotion` 使用。
 */
export function getMotionDuration(baseMs: number): number {
  if (reducedMotionActive) return 0;
  const override = motionOverride;
  if (override) {
    for (const [key, value] of Object.entries(override) as Array<
      [keyof typeof MOTION, number]
    >) {
      if (MOTION[key] === baseMs) {
        return value / 1000;
      }
    }
  }
  return baseMs / 1000;
}

/**
 * 应用主题包声明的 motion 覆盖到 MOTION 与 CSS 变量。
 *
 * 入参 `overrides` 是从主题包读取的档位映射（毫秒），例如
 * `{ FAST: 120, BASE: 200 }`。调用后：
 * 1. `getMotionDuration` 立即反映新档位
 * 2. `document.documentElement` 上写入对应的 `--motion-fast` 等 CSS 变量
 *
 * 传 `null` 恢复默认。该函数是 Phase 2 Step 2.c 的运行时同步入口，
 * 让 GSAP / CSS transitions / 主题包三方保持一致。
 */
export function applyMotionOverride(overrides: MotionOverride | null): void {
  motionOverride = overrides;
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  const cssMap: Record<keyof typeof MOTION, string | null> = {
    MICRO: null, // 未映射到 CSS 变量
    FAST: '--motion-fast',
    BASE: '--motion-base',
    SLOW: '--motion-slow',
    PAGE: '--motion-page',
    BASE_OUT: null,
    SLOW_OUT: null,
    PAGE_OUT: null,
    OVERLAY_IN: null,
  };
  for (const [key, cssVar] of Object.entries(cssMap) as Array<
    [keyof typeof MOTION, string | null]
  >) {
    if (!cssVar) continue;
    const value = overrides?.[key];
    if (value !== undefined) {
      root.style.setProperty(cssVar, `${value}ms`);
    } else {
      root.style.removeProperty(cssVar);
    }
  }
}

/**
 * 主题包 shape 档位覆盖（Phase 2 Step 2.d）。
 *
 * 与 `--shape-xs/sm/md/lg/xl/2xl/pill` CSS 变量一一对应。传 null 恢复默认。
 * 传入未声明的字段会被跳过（稀疏语义）。
 */
export type ShapeOverride = Partial<
  Record<'xs' | 'sm' | 'md' | 'lg' | 'xl' | '2xl' | 'pill', number>
>;

export function applyShapeOverride(overrides: ShapeOverride | null): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  const map: Record<keyof ShapeOverride, string> = {
    xs: '--shape-xs',
    sm: '--shape-sm',
    md: '--shape-md',
    lg: '--shape-lg',
    xl: '--shape-xl',
    '2xl': '--shape-2xl',
    pill: '--shape-pill',
  };
  for (const [key, cssVar] of Object.entries(map) as Array<
    [keyof ShapeOverride, string]
  >) {
    const value = overrides?.[key];
    if (value !== undefined) {
      root.style.setProperty(cssVar, `${value}px`);
    } else {
      root.style.removeProperty(cssVar);
    }
  }
}

/**
 * 主题包 density 档位覆盖（Phase 2 Step 2.d）。
 *
 * 与 `--density-xs/sm/md/lg/xl` CSS 变量一一对应。传 null 恢复默认。
 */
export type DensityOverride = Partial<
  Record<'xs' | 'sm' | 'md' | 'lg' | 'xl', number>
>;

export function applyDensityOverride(overrides: DensityOverride | null): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  const map: Record<keyof DensityOverride, string> = {
    xs: '--density-xs',
    sm: '--density-sm',
    md: '--density-md',
    lg: '--density-lg',
    xl: '--density-xl',
  };
  for (const [key, cssVar] of Object.entries(map) as Array<
    [keyof DensityOverride, string]
  >) {
    const value = overrides?.[key];
    if (value !== undefined) {
      root.style.setProperty(cssVar, `${value}px`);
    } else {
      root.style.removeProperty(cssVar);
    }
  }
}

/**
 * 主题包 elevation 档位覆盖（Phase 2 Step 2.b）。
 *
 * 每档为完整的 box-shadow 字符串。传 null 恢复默认。
 * 假设后端 sanitizer 已经拒绝含 url/expression 等的值；前端不再二次校验。
 */
export type ElevationOverride = Partial<
  Record<'none' | 'xs' | 'sm' | 'md' | 'lg' | 'xl', string>
>;

export function applyElevationOverride(
  overrides: ElevationOverride | null
): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  const map: Record<keyof ElevationOverride, string> = {
    none: '--elevation-none',
    xs: '--elevation-xs',
    sm: '--elevation-sm',
    md: '--elevation-md',
    lg: '--elevation-lg',
    xl: '--elevation-xl',
  };
  for (const [key, cssVar] of Object.entries(map) as Array<
    [keyof ElevationOverride, string]
  >) {
    const value = overrides?.[key];
    if (value !== undefined) {
      root.style.setProperty(cssVar, value);
    } else {
      root.style.removeProperty(cssVar);
    }
  }
}

/**
 * 主题包 blur 档位覆盖（Phase 2 Step 2.b）。
 *
 * 与 `--blur-sm/md/lg/xl` CSS 变量一一对应，单位像素。传 null 恢复默认。
 * 用于玻璃拟态 backdrop-filter 半径；传 0 可关闭模糊（Material 风格）。
 */
export type BlurOverride = Partial<Record<'sm' | 'md' | 'lg' | 'xl', number>>;

export function applyBlurOverride(overrides: BlurOverride | null): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  const map: Record<keyof BlurOverride, string> = {
    sm: '--blur-sm',
    md: '--blur-md',
    lg: '--blur-lg',
    xl: '--blur-xl',
  };
  for (const [key, cssVar] of Object.entries(map) as Array<
    [keyof BlurOverride, string]
  >) {
    const value = overrides?.[key];
    if (value !== undefined) {
      root.style.setProperty(cssVar, `${value}px`);
    } else {
      root.style.removeProperty(cssVar);
    }
  }
}

/**
 * 主题包字体族覆盖（Phase 4）。
 *
 * 覆盖 `--font-body` / `--font-display` / `--font-mono` CSS 变量。
 * 值应已经 sanitizer 过滤，此处不做二次校验；不需要 `@font-face`
 * 声明（依赖用户系统字体或已通过 Tauri asset 加载的字体）。
 */
export type FontFamilyOverride = Partial<
  Record<'body' | 'display' | 'mono', string>
>;

export function applyFontFamilyOverride(
  overrides: FontFamilyOverride | null
): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  const map: Record<keyof FontFamilyOverride, string> = {
    body: '--font-body',
    display: '--font-display',
    mono: '--font-mono',
  };
  for (const [key, cssVar] of Object.entries(map) as Array<
    [keyof FontFamilyOverride, string]
  >) {
    const value = overrides?.[key];
    if (value && value.trim() !== '') {
      root.style.setProperty(cssVar, value);
    } else {
      root.style.removeProperty(cssVar);
    }
  }
}

/**
 * 主题包自定义 CSS 变量覆盖（Phase 4）。
 *
 * 把主题包声明的 `--theme-custom-*` key-value 直接写入 `documentElement`。
 * key 已由 sanitizer 强制以 `--theme-custom-` 前缀开头（命名空间隔离）；
 * value 已经 sanitizer 过滤 CSS 黑名单。此处不做二次校验。
 *
 * # 状态清理
 *
 * 应用新的覆盖前，先移除所有之前设置的 `--theme-custom-*` 变量（追踪
 * 上一次注入的 key 集合），保证切换主题包时不残留旧变量。
 */
let previousCustomKeys = new Set<string>();

export function applyCssVariablesOverride(
  overrides: Record<string, string> | null
): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  // 移除上一次注入的 keys（防止残留）
  for (const key of previousCustomKeys) {
    root.style.removeProperty(key);
  }
  const nextKeys = new Set<string>();
  if (overrides) {
    for (const [key, value] of Object.entries(overrides)) {
      if (key.startsWith('--theme-custom-')) {
        root.style.setProperty(key, value);
        nextKeys.add(key);
      }
    }
  }
  previousCustomKeys = nextKeys;
}

/**
 * 统一动效时长令牌（毫秒）。
 *
 * 作为前端 GSAP 动画时长的单一真相来源，主档位与 `src/app.css` 中的
 * `--motion-fast / --motion-base / --motion-slow / --motion-page` CSS 变量
 * 一一对应；新增动画请从这里取值，不要在组件内硬编码毫秒数。
 *
 * 进出语义档位：
 * - `MICRO`：极快微交互（按下回弹、小浮层关闭），快于 `FAST`
 * - `FAST`：hover / 细粒度状态反馈（对应 `--motion-fast`）
 * - `BASE`：常规元素进出（对应 `--motion-base`）
 * - `SLOW`：覆盖层 / 较大元素进出（对应 `--motion-slow`）
 * - `PAGE`：主视图页面级转场（对应 `--motion-page`）
 *
 * 出场普遍略短于入场，以营造“快速让位、从容入场”的 iOS 观感，
 * 因此常规档位提供独立的 `*_OUT` 值。
 *
 * 专用入场档：
 * - `OVERLAY_IN`：浮层（dialog / select / tooltip / popover）的统一入场时长，
 *   与浮层出场（`BASE_OUT` / `MICRO`）配对使用，使各类浮层进出节奏一致。
 *
 * 注：少数经权衡的有意特例（如音量胶囊收缩的非对称时长）不走令牌，
 * 在调用点就地以注释说明，不强行并入档位。
 */
export const MOTION = {
  MICRO: 100,
  FAST: 140,
  BASE: 180,
  SLOW: 260,
  PAGE: 320,
  BASE_OUT: 150,
  SLOW_OUT: 200,
  PAGE_OUT: 280,
  OVERLAY_IN: 200,
} as const;

/**
 * 页面级转场的水平位移量（百分比）。
 *
 * 入场视图从屏幕外整屏滑入（`100`），离场视图反向小幅位移（`18`）并淡出，
 * 形成方向一致、互不遮挡的“推进 / 后退”观感。
 */
export const VIEW_SHIFT = {
  IN: 100,
  OUT: 18,
} as const;

export function killTweens(targets: gsap.TweenTarget): void {
  gsap.killTweensOf(targets);
}

export function animateIn(
  target: gsap.TweenTarget,
  fromVars: gsap.TweenVars,
  toVars: gsap.TweenVars,
  durationMs: number,
  ease?: string
): gsap.core.Tween {
  killTweens(target);
  // reduced-motion：直接同步落到终态，不排 tween，避免 0 秒 tween 引入的额外一帧
  if (shouldSkipMotion()) {
    gsap.set(target, toVars);
    return gsap.to(target, { duration: 0 });
  }
  return gsap.fromTo(target, fromVars, {
    ...toVars,
    duration: getMotionDuration(durationMs),
    ease: ease ?? 'ios-spring',
  });
}

export function animateOut(
  target: gsap.TweenTarget,
  toVars: gsap.TweenVars,
  durationMs: number,
  opts?: { ease?: string; onComplete?: () => void }
): gsap.core.Tween {
  killTweens(target);
  if (shouldSkipMotion()) {
    gsap.set(target, toVars);
    opts?.onComplete?.();
    return gsap.to(target, { duration: 0 });
  }
  return gsap.to(target, {
    ...toVars,
    duration: getMotionDuration(durationMs),
    ease: opts?.ease ?? 'ios-in',
    onComplete: opts?.onComplete,
  });
}

/**
 * 等待文档字体加载完成。
 *
 * 用于动画启动前确保字体度量稳定，避免 scrollWidth 等测量值因字体 fallback 而偏差。
 * 内置 500ms 超时保护，超时后静默 resolve，不阻塞动画流程。
 */
export function awaitFontsReady(): Promise<void> {
  if (typeof document === 'undefined') return Promise.resolve();
  return Promise.race([
    document.fonts.ready.then(() => {}),
    new Promise<void>((resolve) => setTimeout(resolve, 500)),
  ]);
}

/**
 * 等待浏览器完成首次 paint（双帧 rAF）+ 字体就绪。
 *
 * 用于 FLIP 动画首次触发前，确保 getBoundingClientRect() 返回准确坐标。
 * 合并双帧等待与字体加载，取两者中较晚完成的时机。
 */
export function awaitPaintReady(): Promise<void> {
  if (typeof window === 'undefined') return Promise.resolve();
  const frameDone = new Promise<void>((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve());
    });
  });
  return Promise.all([frameDone, awaitFontsReady()]).then(() => {});
}

export function gsapScrollIntoView(
  container: HTMLElement,
  target: HTMLElement,
  block: 'center' | 'nearest' = 'nearest'
): void {
  const containerRect = container.getBoundingClientRect();
  const targetRect = target.getBoundingClientRect();
  const currentScrollTop = container.scrollTop;

  const targetTop = currentScrollTop + (targetRect.top - containerRect.top);
  const targetBottom = targetTop + targetRect.height;

  let scrollTarget: number;

  if (block === 'center') {
    scrollTarget = targetTop - (container.clientHeight - targetRect.height) / 2;
  } else {
    const isAbove = targetRect.top < containerRect.top;
    const isBelow = targetRect.bottom > containerRect.bottom;

    if (!isAbove && !isBelow) return;

    if (isAbove) {
      scrollTarget = targetTop;
    } else {
      scrollTarget = targetBottom - container.clientHeight;
    }
  }

  if (shouldSkipMotion()) {
    container.scrollTop = scrollTarget;
    return;
  }
  gsap.to(container, {
    scrollTo: { y: scrollTarget },
    duration: getMotionDuration(MOTION.PAGE),
    ease: 'ios-out',
  });
}
