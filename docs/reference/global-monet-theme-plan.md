# 全局 Monet 取色与主题收敛方案

> 状态：设计方案草案  
> 目标：评估“全局 Monet 取色”可行性，并给出一条可分阶段落地的主题系统改造路径，统一当前分散的颜色主题设置。

## 1. 背景

当前颜色系统存在三套并行逻辑：

1. 用户主题偏好：`src/lib/themePresets.ts` 定义 6 个手动色槽（`accent / surface / textPrimary / textSecondary / tint / danger`），`src/lib/theme.ts` 把它们展开为 `--accent`、`--theme-*`、`--destructive` 等 CSS 变量。
2. 专辑封面取色：`src-tauri/src/theme.rs` 从封面提取 `ThemePalette`，前端 `src/lib/features/shell/themeManager.svelte.ts` 只把它应用到 `--album-*` 与 `--wave-color-*`，主要影响播放器、全屏和库详情。
3. 全局基础 token：`src/app.css` 手写了亮/暗模式下的 `--bg-*`、`--text-*`、`--surface-*`、`--player-*`、shadcn token 与 Sheet/Dialog token。Tag Editor 里还残留多处 `--color-*` fallback。

因此，用户在设置里改主题时，实际只影响部分品牌/强调色；基础背景、文字、控件语义色和部分 Tag Editor 旧变量仍由其他地方决定。全局 Monet 的价值不只是“再取一个颜色”，而是建立单一颜色生成模型，让所有 UI token 都从同一份主题结果派生。

## 2. 多智能体讨论纪要

### Agent A：前端架构

现状的好处是运行时入口已经存在：`createThemeManager()` 统一监听偏好、选中专辑与全屏状态，并负责调用 `applyThemeColors()` / `applyAlbumAccentPalette()`。这意味着新增全局 Monet 不需要从组件层散点接入，应该在 `themeManager` 继续收口。

关键问题是 `theme.ts` 现在只派生强调色和品牌色，不派生完整 UI token。建议新增一个 `ResolvedTheme` 层，把“偏好 + scheme + Monet 源 + 专辑上下文”解析成完整 token map，再一次性写入 `document.documentElement`。组件继续消费 CSS 变量，不直接理解 Monet。

### Agent B：设计系统

“全局 Monet”不应等同于“整页跟着封面变色”。音乐应用已经有专辑上下文色，若全局背景也随选中专辑频繁变化，会破坏导航和设置页稳定性。建议区分两个色彩域：

- App Theme：全局、稳定、由用户选择或显式开启 Monet 源生成。
- Context Theme：上下文、临时、由专辑封面生成，只影响播放器、专辑舞台、波形等沉浸区域。

设置页应把当前“6 个颜色输入框”收敛成更少的主题控制：外观模式、主题来源、种子色/图片、动态专辑色开关、高级覆盖。普通用户不应该维护 6 个互相影响不清晰的色槽。

### Agent C：后端 / IPC

Rust 侧已经依赖 `image`，并已有 `extract_image_theme` command。扩展为 Monet 只需要在 `src-tauri/src/theme.rs` 增加完整 palette 生成函数，或新增独立模块，成本可控。更大的契约变化在返回结构：当前 `ThemePalette` 只有 accent/hover/wave，无法表达 Material You 所需的 tonal roles。

建议新增返回类型而不是改坏旧类型，例如 `MonetPalette` / `ThemeTokenSet`。旧 `extract_image_theme` 保留给专辑上下文；全局 Monet 可新增 `extract_global_theme` 或前端纯 TS 生成。若目标是跨平台一致，推荐 Rust 生成核心 palette，前端只做 CSS token 映射。

### Agent D：无障碍与测试

完整 Monet 必须把对比度作为生成阶段的不变量，而不是在单个按钮上补丁式修正。至少要验证：主文本对背景 >= 4.5，次级文本 >= 3，accent 上文字 >= 4.5，危险色可读。

测试策略应覆盖纯函数，而不是只做视觉截图：palette 生成、CSS token 映射、偏好迁移、亮/暗模式切换、专辑上下文色与全局主题不互相覆盖。最后再用 Playwright 或手工截图回归关键页面。

### 讨论结论

可行，但不建议把当前专辑取色直接升级成“全局跟随专辑”。正确路径是先建立统一主题解析层，再把 Monet 作为一种 App Theme 来源接入，同时保留 Context Theme 的专辑沉浸色。

## 3. 可行性判断

结论：高可行性，中等改造量。

支撑条件：

- 现有 Rust 取色能力已经可解码 JPEG/PNG/WebP，并能从图片中选取代表色。
- 前端已有 `theme.ts` 负责 CSS 变量写入和 GSAP 过渡。
- 偏好系统已经能保存 `theme`，并有前后端验证链路。
- shadcn token 已经通过 `@theme inline` 映射到 CSS 变量，具备接入统一 token 的基础。

主要风险：

- 现有 `app.css` 中大量 token 是手写常量，直接动态化会带来较大视觉回归。
- Tag Editor 仍有 `--color-primary`、`--color-border`、`--color-chip-bg` 等旧变量 fallback，需要迁移或提供别名。
- `ThemePreferences` 目前字段较窄，扩展后需要兼容老配置和导入导出。
- Monet palette 生成如果只凭单个 accent 推导，容易出现低对比或单色化问题。

## 4. 目标架构

### 4.1 单一主题流水线

```text
ThemePreferences
  ├─ mode: preset | custom | monetSeed | monetImage | system
  ├─ colorScheme: auto | light | dark
  ├─ seedColor / sourceImage / customOverrides
  └─ dynamicAlbumAccentEnabled
        ↓
resolveAppTheme(preferences, effectiveScheme)
        ↓
ResolvedAppTheme
  ├─ seed / source / generatedAt
  ├─ lightTokens
  ├─ darkTokens
  └─ semanticTokens
        ↓
applyThemeTokenSet(tokens)
        ↓
:root CSS variables
```

专辑封面上下文色保持独立：

```text
selectedAlbum.coverUrl
        ↓
extractImageTheme()
        ↓
ContextThemePalette
        ↓
--album-* / --wave-color-*
```

### 4.2 Token 分层

建议把颜色 token 分成 4 层，避免继续混用：

| 层级            | 示例                                                       | 来源            | 说明                           |
| --------------- | ---------------------------------------------------------- | --------------- | ------------------------------ |
| Seed            | `seedColor`、`sourceImage`                                 | 偏好或图片      | Monet 输入，不直接给组件用     |
| Tonal Palette   | `primary40`、`neutral98`、`error40`                        | 算法生成        | 中间层，供映射使用             |
| Semantic Token  | `--bg-primary`、`--text-secondary`、`--border`、`--accent` | 主题解析        | 组件直接消费                   |
| Component Token | `--player-shell-bg`、`--sheet-border`                      | 语义 token 派生 | 特定区域消费，不保存为用户偏好 |

### 4.3 保留与替换关系

保留：

- `--album-*`：上下文色，仅用于专辑/播放沉浸区域。
- `--wave-color-*`：可视化颜色，继续由专辑 palette 提供。
- `--accent`：全局强调色，但由统一主题结果输出。
- `--destructive`：语义危险色，改为从 error tonal palette 或覆盖值输出。

逐步替换：

- `--theme-surface`、`--theme-text-primary`、`--theme-text-secondary`、`--theme-tint`：保留迁移期别名，长期并入语义 token。
- `--color-primary`、`--color-border`、`--color-chip-bg`、`--color-text-*`：在 `app.css` 提供别名，Tag Editor 逐步改用 `--accent` / `--border` / `--surface-state` / `--text-*`。
- 组件内硬编码 fallback：改成统一 token fallback，避免视觉脱离主题。

## 5. Monet 生成策略

### 5.1 输入源

第一阶段建议支持两种输入：

1. Seed Color：用户选择一个颜色，前端或后端生成完整 palette。
2. Preset：现有预设继续存在，但预设不再保存 6 个终端色槽，而是保存 seed + 少量品牌覆盖。

第二阶段再支持：

1. Image Source：用户选择本地图片或指定封面作为全局主题源。
2. Current Album as Global：可作为显式选项，不默认启用，避免全局 UI 频繁跳色。

### 5.2 输出角色

不必完整复制 Android 所有角色，但需要覆盖桌面应用实际用到的语义：

```ts
interface ThemeTokenSet {
  accent: string;
  accentHover: string;
  accentReadableForeground: string;
  bgPrimary: string;
  bgSecondary: string;
  bgTertiary: string;
  bgElevated: string;
  textPrimary: string;
  textSecondary: string;
  textTertiary: string;
  border: string;
  ring: string;
  destructive: string;
  surfaceState: string;
  surfaceWindow: string;
  surfaceSidebar: string;
  surfaceWorkspace: string;
  surfaceSheet: string;
  surfaceDock: string;
  surfaceFlyout: string;
}
```

同时生成 shadcn 别名：

```css
--background: var(--bg-primary);
--foreground: var(--text-primary);
--popover: var(--bg-secondary);
--primary: var(--accent);
--primary-foreground: var(--accent-readable-foreground);
--muted: var(--bg-tertiary);
--muted-foreground: var(--text-secondary);
--color-primary: var(--primary);
--color-border: var(--border);
```

### 5.3 算法选择

推荐路径：

- 短期：基于现有 HSL / luminance helper，使用 seed hue 派生 `accent`，用 neutral hue 派生背景和表面，并强制对比度。
- 中期：实现简化 Material You tonal palette：primary / secondary / tertiary / neutral / neutralVariant / error，每组按 tone 0-100 生成。
- 长期：如引入第三方库，优先评估 Rust crate 或小型 TS 实现，但要避免给 Tauri 包体引入过重依赖。

短期算法示例：

```text
seed rgb -> hsl
primary = clamp(saturation 0.42..0.78, lightness by scheme)
secondary = hue ± 24, lower saturation
neutral = seed hue, saturation 0.04..0.10
light bg = neutral tone 98 / 95 / 91
dark bg = neutral tone 6 / 12 / 18
accent foreground = binary search to 4.5 contrast
```

## 6. 偏好模型改造

现有模型：

```ts
interface ThemePreferences {
  presetId: string;
  customColors: Partial<ThemeColorSlots>;
  colorScheme?: ColorScheme;
}
```

建议新增 v2，并保留兼容读取：

```ts
type ThemeSourceMode = 'preset' | 'custom' | 'monetSeed' | 'monetImage';

type DynamicAlbumAccentMode = 'off' | 'playerOnly' | 'libraryAndPlayer';

interface ThemePreferencesV2 {
  version: 2;
  colorScheme: ColorScheme;
  sourceMode: ThemeSourceMode;
  presetId: string;
  seedColor: string | null;
  sourceImagePath: string | null;
  customOverrides: Partial<ThemeTokenOverrides>;
  dynamicAlbumAccent: DynamicAlbumAccentMode;
}
```

兼容规则：

- 旧 `presetId` 映射到 v2 `sourceMode: 'preset'`。
- 旧 `customColors.accent` 映射到 `seedColor` 或 `customOverrides.accent`，具体取决于是否只改了 accent。
- 旧 6 色槽继续读取，但设置 UI 默认隐藏到“高级覆盖”。
- 后端反序列化时接受 v1/v2，写回时统一写 v2，或先保持 v1 写回直到 UI 改造完成。

## 7. 设置面板改造

建议把当前 Theme 区块改为：

1. 外观：自动 / 浅色 / 深色。
2. 主题来源：预设 / Monet 取色 / 自定义。
3. 预设选择：Harubble Classic、Clear Aqua、Night Console 等。
4. Monet 种子：颜色选择器 + “从当前封面生成”按钮。
5. 动态专辑色：关闭 / 仅播放器 / 库详情与播放器。
6. 高级颜色覆盖：折叠区，显示少量语义项，而不是暴露所有底层 token。

当前 6 个颜色输入框不应作为主流程继续展示。它们适合迁移为高级覆盖，且文案需要解释覆盖的是语义角色，而不是底层 UI 色。

## 8. 分阶段实施计划

### Phase 1：盘点与别名收敛

- 在 `app.css` 顶部补齐旧变量到新语义 token 的别名表。
- 清点 `--color-*`、`--surface-secondary`、`--accent-surface`、`--bg-input` 等未定义或散落 fallback。
- 为 Tag Editor 提供兼容别名，避免后续切换 palette 时局部掉色。
- 增加 `docs/reference/frontend-guide.md` 的颜色 token 规则，明确组件只能消费语义 token 或组件 token。

验收：全局搜索不再出现未定义 token fallback 作为主要路径；`bun run check:types`、`bun run lint:eslint` 通过。

### Phase 2：新增主题解析核心

- 新建 `src/lib/themeTokens.ts` 或扩展 `src/lib/theme.ts`，提供 `resolveAppTheme()` / `deriveThemeTokenSet()`。
- 把 `deriveThemeCssVariables()` 从 6 色槽输出扩展为完整 token set。
- 保留 `applyAlbumAccentPalette()`，但确保它只写 `--album-*` 和 `--wave-color-*`。
- 补充 `src/lib/theme.test.ts`：对比度、亮暗 token、旧预设兼容、album token 隔离。

验收：现有 UI 行为保持，主题切换仍有 GSAP 颜色过渡；专辑取色不会覆盖全局 `--accent`。

### Phase 3：实现 Monet seed palette

- 增加 seed color 生成完整 light/dark token 的函数。
- 先用 TS 纯函数实现，便于测试和快速迭代；若后续需要从图片提取全局源，再复用 Rust。
- 在 `ThemePreferences` 前端类型中引入 v2 草案，但后端可先兼容 v1。
- 设置面板新增“Monet 取色”来源和 seed color 控件。

验收：用户选择一个种子色后，背景、表面、文字、控件、shadcn primitive、播放器外壳都从同一 palette 派生。

### Phase 4：后端偏好与 IPC 契约升级

- Rust `ThemePreferences` 增加 v2 字段和反序列化兼容。
- 更新 `src/lib/types.ts`、`src/lib/themePresets.ts`、`src/lib/features/shell/settings.svelte.ts`。
- 更新 rustdoc：偏好模型、迁移规则、错误场景。
- 添加 Rust 偏好反序列化测试：v1 缺省、非法 seed、未知 sourceMode、导入导出。

验收：旧偏好文件可正常启动；导入旧配置后设置页显示合理；保存后数据结构稳定。

### Phase 5：图片源 Monet 与上下文色策略

- 新增全局图片源取色 command，或复用 `extract_image_theme` 后在前端生成 full palette。
- 增加缓存 key：图片路径 / URL + mtime 或 hash，避免频繁解码。
- 设置“从当前封面生成全局主题”必须是显式用户操作，不随选中专辑自动保存。
- 动态专辑色保留 runtime-only，不写入全局偏好。

验收：从图片生成主题不会阻塞主线程；选中专辑变化只影响上下文 token，除非用户主动保存为全局主题。

### Phase 6：组件迁移与视觉回归

- Tag Editor 全量迁移到 `--text-*`、`--border`、`--accent`、`--surface-*`。
- Home 组件中 `--surface-secondary`、`--surface-tertiary`、`--accent-surface` 统一改为已定义 token。
- Mini Player 和 Player Dock 明确只消费全局 player token + `--album-*`。
- 对 Home / Search / Library / Collection / Tag Editor / Settings / Player 做亮暗模式截图回归。

验收：不再依赖组件局部 fallback 来补全主题；所有主要视图在预设、Monet seed、深色模式下可读。

## 9. 测试清单

前端单元测试：

- `resolveThemeColors()` 兼容旧预设。
- `deriveThemeTokenSet(seed, 'light')` / `deriveThemeTokenSet(seed, 'dark')` 输出完整 token。
- `accentReadableForeground` 对比度 >= 4.5。
- `applyAlbumAccentPalette()` 不写 `--accent`。
- v1 preferences 迁移到 v2 的纯函数测试。

Rust 测试：

- `ThemePreferences` 反序列化旧配置。
- 非法 hex、未知 slot、未知 sourceMode 降级。
- 图片取色失败返回明确错误，不破坏偏好保存。

集成/视觉回归：

- 启动后 `:root` 同时只有一个有效 color scheme class。
- 设置页切换主题来源即时预览并可落盘。
- 专辑切换时 `--album-accent` 更新，`--accent` 不变。
- Tag Editor 不再出现固定紫色主按钮或灰色 fallback 脱离主题。

## 10. 推荐决策

建议采用“全局 App Theme + 专辑 Context Theme”双域模型。

短期不要把全局主题直接绑定当前专辑封面。先做 token 收敛和 seed-based Monet，解决当前颜色设置混乱的问题；再把图片源作为显式的主题生成入口接进来。这样能复用现有封面取色基础，又不会让全局导航、设置页和列表区域因为播放上下文变化而失去稳定性。

优先落地顺序：

1. 统一 token 命名和别名，清理未定义 fallback。
2. 新增完整 `ThemeTokenSet` 解析层。
3. 接入 seed-based Monet。
4. 升级偏好模型和设置 UI。
5. 再做图片源全局主题。
