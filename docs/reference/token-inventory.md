# CSS Token 盘点清单

> Phase 0 产出物，作为 Monet 主题收敛的可执行依据。
> 基线：`src/app.css` + `src/lib/theme.ts` / `src/lib/themeTokens.ts` 运行时写入。
>
> **状态说明（2026-07）**：以 `ThemeTokenSet` 为核心的 App Theme + Context Theme 双链路已在 `themeTokens.ts` 落地，运行时写入者是 `applyAppThemeTokenSet` 与 `applyContextThemePalette`（由 `features/shell/themeManager.svelte.ts` 调用）；`theme.ts` 中残留的 `applyThemeColors` / `applyAlbumAccentPalette` 已无生产调用者，`applyThemePalette` 已从代码中移除。本清单的表头仍保留原来的旧函数名以便对照迁移前后差异；具体的现役调用链见 §5。

## 1. `:root` 全局变量（非 scheme 相关）

### 1.1 Accent 系列（运行时由 `applyThemeColors` 写入）

| 变量                                 | 默认值                         | 迁移归属                                      | 备注       |
| ------------------------------------ | ------------------------------ | --------------------------------------------- | ---------- |
| `--accent-rgb`                       | `250, 45, 72`                  | `ThemeTokenSet.accentRgb`                     | RGB 三通道 |
| `--accent-hover-rgb`                 | `255, 59, 92`                  | `ThemeTokenSet.accentHoverRgb`                |            |
| `--accent`                           | `rgb(var(--accent-rgb))`       | `ThemeTokenSet.accent`                        |            |
| `--accent-hover`                     | `rgb(var(--accent-hover-rgb))` | `ThemeTokenSet.accentHover`                   |            |
| `--accent-readable-foreground`       | `#390b10`                      | `ThemeTokenSet.accentReadableForeground`      |            |
| `--accent-hover-readable-foreground` | `#441019`                      | `ThemeTokenSet.accentHoverReadableForeground` |            |

### 1.2 Theme Surface / Text / Tint（运行时由 `applyThemeColors` 写入）

| 变量                     | 默认值                            | 迁移归属                    | 备注                                         |
| ------------------------ | --------------------------------- | --------------------------- | -------------------------------------------- |
| `--theme-surface`        | `#d1d6db`                         | 待废弃                      | preset surface 色，Monet 后不再需要独立 slot |
| `--theme-surface-rgb`    | `209, 214, 219`                   | 待废弃                      | 同上                                         |
| `--theme-text-primary`   | `#4a5056`                         | 待废弃                      | preset text 色，Monet 后由 tone 派生         |
| `--theme-text-secondary` | `#596066`                         | 待废弃                      | 同上                                         |
| `--theme-tint`           | `#899cb0`                         | 待废弃                      | preset tint 色                               |
| `--theme-tint-rgb`       | `137, 156, 176`                   | 待废弃                      | 同上                                         |
| `--destructive`          | `#c74f4f`（hardcoded in `:root`） | `ThemeTokenSet.destructive` | 兼容 shadcn `--destructive`                  |

### 1.3 Album Accent 系列（运行时由 `applyAlbumAccentPalette` 写入）

| 变量                                 | 默认值                              | 迁移归属           | 备注             |
| ------------------------------------ | ----------------------------------- | ------------------ | ---------------- |
| `--album-accent-rgb`                 | `var(--accent-rgb)`                 | Context Theme 专属 | 不进入 App Token |
| `--album-accent-hover-rgb`           | `var(--accent-hover-rgb)`           | Context Theme 专属 |                  |
| `--album-accent`                     | `var(--accent)`                     | Context Theme 专属 |                  |
| `--album-accent-hover`               | `var(--accent-hover)`               | Context Theme 专属 |                  |
| `--album-accent-readable-foreground` | `var(--accent-readable-foreground)` | Context Theme 专属 |                  |

### 1.4 Wave Colors（运行时由 `applyAlbumAccentPalette` 写入）

| 变量             | 默认值      | 迁移归属           | 备注         |
| ---------------- | ----------- | ------------------ | ------------ |
| `--wave-color-0` | 无 CSS 默认 | Context Theme 专属 | 仅运行时写入 |
| `--wave-color-1` | 无 CSS 默认 | Context Theme 专属 |              |
| `--wave-color-2` | 无 CSS 默认 | Context Theme 专属 |              |
| `--wave-color-3` | 无 CSS 默认 | Context Theme 专属 |              |

### 1.5 Surface 组合变量

| 变量                  | 表达式                                                         | 迁移归属           |
| --------------------- | -------------------------------------------------------------- | ------------------ |
| `--surface-window`    | `color-mix(in srgb, var(--bg-primary) 100%, transparent)`      | → `surfaceBase`    |
| `--surface-sidebar`   | `color-mix(in srgb, var(--bg-secondary) 92%, white 8%)`        | → `surfaceSidebar` |
| `--surface-workspace` | `color-mix(in srgb, var(--bg-primary) 96%, white 4%)`          | → `surfaceBase`    |
| `--surface-sheet`     | `color-mix(in srgb, var(--bg-secondary) 76%, transparent 24%)` | → `surfaceOverlay` |
| `--surface-dock`      | `color-mix(in srgb, var(--player-shell-bg) 88%, white 12%)`    | → `surfaceOverlay` |
| `--surface-flyout`    | `color-mix(in srgb, var(--player-shell-bg) 80%, transparent)`  | → `surfaceOverlay` |
| `--surface-state`     | `rgba(var(--accent-rgb), 0.08)`                                | → `surfaceState`   |

### 1.6 Motion / Easing / Font（保持不变，不进入 ThemeTokenSet）

| 变量                                  | 用途                             | 迁移归属   |
| ------------------------------------- | -------------------------------- | ---------- |
| `--motion-fast`                       | 140ms                            | 保留，不变 |
| `--motion-base`                       | 180ms                            | 保留，不变 |
| `--motion-slow`                       | 260ms                            | 保留，不变 |
| `--motion-page`                       | 320ms                            | 保留，不变 |
| `--motion-spinner`                    | 900ms                            | 保留，不变 |
| `--motion-pulse`                      | 1800ms                           | 保留，不变 |
| `--ease-standard`                     | cubic-bezier(0.2, 0, 0, 1)       | 保留，不变 |
| `--ease-decelerate`                   | cubic-bezier(0.16, 1, 0.3, 1)    | 保留，不变 |
| `--ease-linear`                       | linear                           | 保留，不变 |
| `--ease-ios`                          | cubic-bezier(0.25, 0.1, 0.25, 1) | 保留，不变 |
| `--motion-hover`                      | 多属性 transition                | 保留，不变 |
| `--font-sans`                         | HarmonyOS Sans SC                | 保留，不变 |
| `--font-display/body/mono/brand/wide` | 字体栈                           | 保留，不变 |

### 1.7 shadcn-svelte 桥接 Token（当前在 `:root` 中定义）

| 变量                       | 当前值                              | 迁移方向                                      |
| -------------------------- | ----------------------------------- | --------------------------------------------- |
| `--background`             | `var(--bg-primary)`                 | 改引 `ThemeTokenSet.bgPrimary`                |
| `--foreground`             | `var(--text-primary)`               | 改引 `ThemeTokenSet.textPrimary`              |
| `--popover`                | `var(--bg-secondary)`               | 改引 `ThemeTokenSet.bgSecondary`              |
| `--popover-foreground`     | `var(--text-primary)`               | 改引 `ThemeTokenSet.textPrimary`              |
| `--card`                   | `var(--bg-secondary)`               | 改引 `ThemeTokenSet.bgSecondary`              |
| `--card-foreground`        | `var(--text-primary)`               | 改引 `ThemeTokenSet.textPrimary`              |
| `--primary`                | `var(--accent)`                     | 改引 `ThemeTokenSet.accent`                   |
| `--primary-foreground`     | `var(--accent-readable-foreground)` | 改引 `ThemeTokenSet.accentReadableForeground` |
| `--secondary`              | `var(--bg-tertiary)`                | 改引 `ThemeTokenSet.bgTertiary`               |
| `--secondary-foreground`   | `var(--text-primary)`               | 改引 `ThemeTokenSet.textPrimary`              |
| `--muted`                  | `var(--bg-tertiary)`                | 改引 `ThemeTokenSet.bgTertiary`               |
| `--muted-foreground`       | `var(--text-secondary)`             | 改引 `ThemeTokenSet.textSecondary`            |
| `--destructive`            | `#c74f4f`                           | 改引 `ThemeTokenSet.destructive`              |
| `--destructive-foreground` | `white`                             | 派生值，保留硬编码                            |
| `--input`                  | `var(--bg-tertiary)`                | 改引 `ThemeTokenSet.bgTertiary`               |
| `--ring`                   | `rgba(var(--accent-rgb), 0.3)`      | 改引 `ThemeTokenSet.ring`                     |
| `--radius`                 | `0.5rem`                            | 保留，不变                                    |

---

## 2. `:root.light` 变量

### 2.1 基础色

| 变量               | 值                             | 迁移归属                      |
| ------------------ | ------------------------------ | ----------------------------- |
| `--bg-primary`     | `#ffffff`                      | `ThemeTokenSet.bgPrimary`     |
| `--bg-secondary`   | `#f5f5f7`                      | `ThemeTokenSet.bgSecondary`   |
| `--bg-tertiary`    | `#e8e8ed`                      | `ThemeTokenSet.bgTertiary`    |
| `--bg-elevated`    | `rgba(255,255,255,0.8)`        | `ThemeTokenSet.bgElevated`    |
| `--text-primary`   | `#1d1d1f`                      | `ThemeTokenSet.textPrimary`   |
| `--text-secondary` | `#6e6e73`                      | `ThemeTokenSet.textSecondary` |
| `--text-tertiary`  | `#86868b`                      | `ThemeTokenSet.textTertiary`  |
| `--border`         | `rgba(0,0,0,0.08)`             | `ThemeTokenSet.border`        |
| `--accent-light`   | `rgba(var(--accent-rgb), 0.1)` | 派生值                        |

### 2.2 Surface Tint

| 变量                    | 值                                  | 迁移归属         |
| ----------------------- | ----------------------------------- | ---------------- |
| `--surface-tint`        | `rgba(var(--theme-tint-rgb), 0.04)` | 待评估，可能废弃 |
| `--surface-tint-strong` | `rgba(var(--theme-tint-rgb), 0.08)` | 待评估，可能废弃 |

### 2.3 Toolbar 系列（Component Alias，不进入 ThemeTokenSet）

| 变量                  | 值                       | 迁移归属              |
| --------------------- | ------------------------ | --------------------- |
| `--toolbar-bg`        | `rgba(255,255,255,0.78)` | Component alias，保留 |
| `--toolbar-surface`   | `rgba(255,255,255,0.72)` | Component alias，保留 |
| `--toolbar-segment`   | `rgba(17,17,17,0.05)`    | Component alias，保留 |
| `--toolbar-separator` | `rgba(17,17,17,0.08)`    | Component alias，保留 |
| `--toolbar-highlight` | `rgba(255,255,255,0.82)` | Component alias，保留 |
| `--toolbar-shadow`    | 多值 box-shadow          | Component alias，保留 |

### 2.4 Stage 系列（Component Alias）

| 变量                   | 值                                | 迁移归属              |
| ---------------------- | --------------------------------- | --------------------- |
| `--stage-shell-fill`   | `rgba(248,249,251,0.96)`          | Component alias，保留 |
| `--stage-shell-border` | `rgba(15,23,42,0.08)`             | Component alias，保留 |
| `--stage-shell-shadow` | 多值                              | Component alias，保留 |
| `--stage-media-shadow` | `0 18px 38px rgba(15,23,42,0.12)` | Component alias，保留 |
| `--stage-top-scrim`    | `rgba(7,10,18,0.18)`              | Component alias，保留 |
| `--stage-edge-scrim`   | `rgba(7,10,18,0.1)`               | Component alias，保留 |

### 2.5 Hero Card 系列（Component Alias）

| 变量                    | 值                       | 迁移归属              |
| ----------------------- | ------------------------ | --------------------- |
| `--hero-card-bg`        | `rgba(255,255,255,0.78)` | Component alias，保留 |
| `--hero-card-border`    | `rgba(15,23,42,0.08)`    | Component alias，保留 |
| `--hero-card-highlight` | `rgba(255,255,255,0.7)`  | Component alias，保留 |
| `--hero-card-shadow`    | 多值                     | Component alias，保留 |

### 2.6 Player 系列（Component Alias，引用 `--album-accent-*`）

| 变量                         | 迁移归属                                |
| ---------------------------- | --------------------------------------- |
| `--player-shell-bg`          | Component alias，保留                   |
| `--player-shell-border`      | Component alias，保留                   |
| `--player-shell-highlight`   | Component alias，保留                   |
| `--player-cover-start`       | Component alias（Context Theme 消费者） |
| `--player-cover-end`         | Component alias（Context Theme 消费者） |
| `--player-placeholder-color` | Component alias，保留                   |
| `--player-title`             | Component alias，保留                   |
| `--player-subtitle`          | Component alias，保留                   |
| `--player-track-bg`          | Component alias，保留                   |
| `--player-track-fill-end`    | Component alias（Context Theme 消费者） |
| `--player-thumb-border`      | Component alias，保留                   |
| `--player-thumb-bg`          | Component alias（Context Theme 消费者） |
| `--player-thumb-shadow`      | Component alias（Context Theme 消费者） |
| `--player-control-color`     | Component alias，保留                   |
| `--player-control-hover-bg`  | Component alias（Context Theme 消费者） |
| `--player-time`              | Component alias，保留                   |
| `--player-play-text`         | Component alias，保留                   |
| `--player-play-shadow`       | Component alias（Context Theme 消费者） |
| `--player-play-shadow-hover` | Component alias（Context Theme 消费者） |

### 2.7 其他

| 变量                  | 值                 | 迁移归属 |
| --------------------- | ------------------ | -------- |
| `--scrollbar-thumb`   | `rgba(0,0,0,0.2)`  | 保留     |
| `--hover-bg`          | `rgba(0,0,0,0.04)` | 保留     |
| `--hover-bg-elevated` | `rgba(0,0,0,0.06)` | 保留     |

---

## 3. `:root.dark` 变量

与 `:root.light` 对称，仅列出值差异：

### 3.1 基础色

| 变量               | 暗色值                          | 迁移归属                      |
| ------------------ | ------------------------------- | ----------------------------- |
| `--bg-primary`     | `#000000`                       | `ThemeTokenSet.bgPrimary`     |
| `--bg-secondary`   | `#1c1c1e`                       | `ThemeTokenSet.bgSecondary`   |
| `--bg-tertiary`    | `#2c2c2e`                       | `ThemeTokenSet.bgTertiary`    |
| `--bg-elevated`    | `rgba(28,28,30,0.8)`            | `ThemeTokenSet.bgElevated`    |
| `--text-primary`   | `#ffffff`                       | `ThemeTokenSet.textPrimary`   |
| `--text-secondary` | `#8e8e93`                       | `ThemeTokenSet.textSecondary` |
| `--text-tertiary`  | `#636366`                       | `ThemeTokenSet.textTertiary`  |
| `--border`         | `rgba(255,255,255,0.08)`        | `ThemeTokenSet.border`        |
| `--accent-light`   | `rgba(var(--accent-rgb), 0.18)` | 派生值                        |

### 3.2 暗色 Component Alias

结构与 light 对称；变量名相同，值适配暗色背景。不再重复列出。

---

## 4. `@theme inline` 注册项（Tailwind v4 Token）

| `@theme inline` 变量             | 当前引用                        | `ThemeTokenSet` 对应         |
| -------------------------------- | ------------------------------- | ---------------------------- |
| `--color-background`             | `var(--background)`             | → `bgPrimary`                |
| `--color-foreground`             | `var(--foreground)`             | → `textPrimary`              |
| `--color-card`                   | `var(--card)`                   | → `bgSecondary`              |
| `--color-card-foreground`        | `var(--card-foreground)`        | → `textPrimary`              |
| `--color-popover`                | `var(--popover)`                | → `bgSecondary`              |
| `--color-popover-foreground`     | `var(--popover-foreground)`     | → `textPrimary`              |
| `--color-primary`                | `var(--primary)`                | → `accent`                   |
| `--color-primary-foreground`     | `var(--primary-foreground)`     | → `accentReadableForeground` |
| `--color-secondary`              | `var(--secondary)`              | → `bgTertiary`               |
| `--color-secondary-foreground`   | `var(--secondary-foreground)`   | → `textPrimary`              |
| `--color-muted`                  | `var(--muted)`                  | → `bgTertiary`               |
| `--color-muted-foreground`       | `var(--muted-foreground)`       | → `textSecondary`            |
| `--color-accent`                 | `var(--surface-state)`          | → `surfaceState`             |
| `--color-accent-foreground`      | `var(--foreground)`             | → `textPrimary`              |
| `--color-destructive`            | `var(--destructive)`            | → `destructive`              |
| `--color-destructive-foreground` | `var(--destructive-foreground)` | 保留硬编码                   |
| `--color-border`                 | `var(--border)`                 | → `border`                   |
| `--color-input`                  | `var(--input)`                  | → `bgTertiary`               |
| `--color-ring`                   | `var(--ring)`                   | → `ring`                     |

迁移后方向反转：`@theme inline` 内的值将直接引用 `ThemeTokenSet` 导出的语义 token，中间 shadcn 桥接变量变为纯别名。

---

## 5. 运行时写入分析

### 5.1 `applyAppThemeTokenSet()`（App Theme 链路，现役）

来源：`src/lib/themeTokens.ts:154`；调用点：`features/shell/themeManager.svelte.ts`。

写入变量（覆盖除 accent 组以外的 `ThemeTokenSet` 全部字段）：

```
--bg-primary, --bg-secondary, --bg-tertiary, --bg-elevated,
--text-primary, --text-secondary, --text-tertiary,
--border, --ring, --destructive, --destructive-rgb,
--surface-state, --surface-base, --surface-sidebar, --surface-overlay,
--surface-secondary, --surface-tertiary
```

（accent 组交由 `applyContextThemePalette` 与 App Theme 共同写入。）

### 5.2 `applyContextThemePalette()`（Context Theme 链路，现役）

来源：`src/lib/themeTokens.ts:176`；调用点同上。

写入变量：

```
--accent, --accent-hover, --accent-rgb, --accent-hover-rgb,
--accent-readable-foreground, --accent-hover-readable-foreground,
--album-accent, --album-accent-hover, --album-accent-rgb,
--album-accent-hover-rgb, --album-accent-readable-foreground,
--wave-color-0, --wave-color-1, --wave-color-2, --wave-color-3,
--theme-surface, --theme-surface-rgb,
--theme-text-primary, --theme-text-secondary,
--theme-tint, --theme-tint-rgb
```

`--theme-*` 变量在 §1.2 表格中被列为「待废弃」，但当前仍由 `applyContextThemePalette` 主动写入 — Monet 语义 token 完全接管前保持兼容。

### 5.3 `applyThemeColors()` / `applyAlbumAccentPalette()`（旧版链路，无生产调用者）

`theme.ts` 中仍保留这两个 export 及其派生函数 `deriveThemeCssVariables`，但生产代码已切换到 §5.1 / §5.2 的新链路，`applyThemeColors` / `applyAlbumAccentPalette` 在 `src/` 内除自身与测试外无调用者。原「遗留 `applyThemePalette`」函数已从 `theme.ts` 中移除。

**清理路径：** 待 Monet 语义 token 完成收敛后，与 `--theme-*` 一起在 `theme.ts` 移除。

---

## 6. Tag Editor 使用的 `--color-*` 变量（已收敛）

原「盘点段」发现的 7 个未定义变量，现已全部在 `src/app.css` 的 `:root` 中提供别名，Tag Editor 中 `var(--color-*, #fallback)` 会命中真值，fallback 只在极端降级路径下才生效：

| 变量                     | 当前值                  | 状态                         |
| ------------------------ | ----------------------- | ---------------------------- |
| `--color-text-primary`   | `var(--text-primary)`   | 已别名                       |
| `--color-text-secondary` | `var(--text-secondary)` | 已别名                       |
| `--color-info`           | `var(--accent)`         | 已别名（复用 accent 语义）   |
| `--color-success`        | `#16a34a`（硬编码）     | 已定义，暗色主题下尚未做适配 |
| `--color-warning`        | `#f59e0b`（硬编码）     | 已定义，暗色主题下尚未做适配 |
| `--color-danger`         | `var(--destructive)`    | 已别名                       |
| `--color-chip-bg`        | `var(--bg-tertiary)`    | 已别名                       |

以及通过 `@theme inline` 间接生效的 3 个：

| 变量                  | 实际值                             | 状态   |
| --------------------- | ---------------------------------- | ------ |
| `--color-primary`     | `var(--primary)` → `var(--accent)` | 已覆盖 |
| `--color-border`      | `var(--border)`                    | 已覆盖 |
| `--color-destructive` | `var(--destructive)`               | 已覆盖 |

**后续待办**：`--color-success` / `--color-warning` 目前仍为硬编码浅色调，暗色主题下对比度不足，需要在 Monet 收敛阶段一并处理（或提升为独立语义 token）。

---

## 7. Token 归属总结

### 进入 `ThemeTokenSet`（由 Monet 或 preset 派生，写入 `:root`）

```
accent, accentHover, accentRgb, accentHoverRgb,
accentReadableForeground, accentHoverReadableForeground,
bgPrimary, bgSecondary, bgTertiary, bgElevated,
textPrimary, textSecondary, textTertiary,
border, ring, destructive, destructiveRgb,
surfaceState, surfaceBase, surfaceSidebar, surfaceOverlay
```

### 保留为 Context Theme 专属（写入 `:root` 但由专辑色控制）

```
--album-accent, --album-accent-hover, --album-accent-rgb,
--album-accent-hover-rgb, --album-accent-readable-foreground,
--wave-color-0..3
```

### 保留为 Component Alias（从语义 token 派生，不由 JS 运行时写入）

```
--surface-window, --surface-sidebar, --surface-workspace,
--surface-sheet, --surface-dock, --surface-flyout,
--toolbar-*, --stage-*, --hero-card-*, --player-*,
--scrollbar-thumb, --hover-bg, --hover-bg-elevated,
--accent-light, --surface-tint, --surface-tint-strong
```

### 保留不变（非颜色系统）

```
--motion-*, --ease-*, --font-*, --radius, --macos-titlebar-height, --safe-area-top
```

### 待废弃（Phase 1 移除或重映射）

| 变量                                              | 理由                                 |
| ------------------------------------------------- | ------------------------------------ |
| `--theme-surface` / `--theme-surface-rgb`         | Monet 后用 surface tone 替代         |
| `--theme-text-primary` / `--theme-text-secondary` | Monet 后用 tone 派生                 |
| `--theme-tint` / `--theme-tint-rgb`               | Monet 后用 neutral-variant tone 替代 |

---

## 8. 迁移后 shadcn token 引用方向

当前链路（二级间接）：

```
@theme inline → --color-background: var(--background) → --background: var(--bg-primary)
```

迁移后（一级直引）：

```
@theme inline → --color-background: var(--bg-primary)
```

shadcn 桥接变量（`--background`、`--foreground`、`--primary` 等）变为可选的向下兼容别名，最终可逐步移除。
