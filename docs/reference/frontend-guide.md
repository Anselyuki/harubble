# 前端开发指南

> 前端架构、开发约定与设计规范。

## 1. 整体布局

```text
┌──────────────────────┬──────────────────────────────────────────────┐
│ AppSidebar           │ TopToolbar（macOS 拖拽区 + 工具入口）        │
│ ├ BrandLogo/BrandSlab ├─────────────────────────────────────────────┤
│ ├ SidebarNav         │ ViewRouter:                                  │
│ │  (Home / Search /  │ ├ HomeView                                   │
│ │   Library /        │ ├ SearchView                                 │
│ │   TagEditor)       │ ├ LibraryView                                │
│ └ Collections        │ ├ AlbumOverview                              │
│                      │ ├ CollectionDetailPanel                      │
│                      │ └ TagEditorView                              │
│                      ├──────────────────────────────────────────────┤
│                      │ PlayerFlyoutStack（Dock + 歌词/队列浮层）    │
└──────────────────────┴──────────────────────────────────────────────┘
                                    ▲                ▲
                                    │                │
                       AppSideSheets │                │  FullscreenPlayer
                       ├ SettingsSheet                │  （按需挂载）
                       └ DownloadTasksSheet           │
                                                     │
                                       StatusToastHost（全局 toast）
```

`OverlayScrollbars` 管理侧栏与主内容区滚动；macOS 下顶栏保留 `data-tauri-drag-region` 拖拽区。

## 2. 目录结构

```text
src/
├ App.svelte                        # 根装配：runtime 实例化、侧栏动画、shell 挂载
├ app.css                           # 全局 token、layer、reset 与全局语义 class
├ main.ts                           # Vite 入口
└ lib/
   ├ api.ts                         # 主 IPC bridge
   ├ settingsApi.ts                 # 设置面板专用 IPC bridge
   ├ collectionApi.ts               # 合集相关 IPC bridge
   ├ appEvents.ts                   # Tauri 事件名与载荷类型集中定义
   ├ types.ts                       # 前后端共享数据结构
   ├ theme.ts / themePresets.ts     # 主题切换与预设（旧版链路，部分函数已被 themeTokens 取代）
   ├ themeTokens.ts / monetPalette.ts # ThemeTokenSet 派生与 Monet 调色板
   ├ cache.ts / lazyLoad.ts / imageDataSrc.ts / downloadBadge.ts / utils.ts
   │
   ├ components/
   │  ├ ui/                         # shadcn-svelte / Bits UI primitive 包装
   │  ├ app/                        # 业务壳层组件（按域划分子目录）
   │  │  ├ shell/                   # 顶部工具栏、Sheet、Toast、Provider、Router
   │  │  │   └ settings/            # SettingsSheet 拆分出的分区组件
   │  │  ├ sidebar/                 # 侧栏框架与导航
   │  │  ├ home/                    # 首页区块
   │  │  ├ library/                 # 库视图
   │  │  ├ search/                  # 搜索视图
   │  │  ├ album/                   # 专辑舞台与详情
   │  │  ├ collection/              # 合集面板与表单
   │  │  ├ player/                  # 播放 Dock / 歌词 / 音量 / 全屏播放器
   │  │  └ tag-editor/              # Tag 编辑器视图与对话框
   │  ├ AlbumCard.svelte / SongRow.svelte / MetadataPopover.svelte
   │  ├ AudioPlayer.svelte / ViewTransition.svelte
   │  └ MotionSpinner.svelte / MotionPulseBlock.svelte / MotionMarqueeInner.svelte  # 通用动效原语
   │
   ├ features/                      # 业务域 controller / store / 纯函数
   │  ├ env/      store.svelte.ts
   │  ├ library/  controller + selectors + helpers
   │  ├ player/   controller + queue + lyrics + volume + formatUtils + miniPlayerBridge + playback-contract
   │  ├ download/ controller + presenters + formatters + guards
   │  ├ home/     controller + store
   │  ├ search/   controller + store
   │  ├ collection/ controller + resolvedSongs store
   │  ├ tagEditor/  controller + store + tagLibrary
   │  └ shell/    appRuntime + appRuntimeBootstrap + store + settings + albumStageMotion
   │              + navigation / navigationManager / selectionManager
   │              + themeManager / downloadBridge
   │
   ├ contexts/                      # Svelte context 键 + setter/getter（强类型）
   ├ design/                        # gsap 适配层 + 侧栏动画器 + 侧栏 resize 手柄 + view-transition 原语 + actions + variants
   ├ styles/                        # 字体声明（HarmonyOS Sans SC 等）
   ├ i18n/                          # locale state + formatters + types
   └ paraglide/                     # @inlang/paraglide-js 构建产物（messages）
```

## 3. 域边界与依赖方向

| 域           | 职责                                                | 形态               |
| ------------ | --------------------------------------------------- | ------------------ |
| `env`        | 只读环境状态（isMacOS、prefersReducedMotion、视口） | store              |
| `library`    | 专辑列表/详情、切换竞态、库内搜索、封面预加载       | controller         |
| `player`     | 当前歌曲、队列、歌词加载与高亮、乱序/循环、音量     | controller         |
| `download`   | 任务列表与操作、下载设置、单曲/整专/多选入口、历史  | controller         |
| `home`       | 最新专辑、系列分组、收听历史、状态仪表盘            | controller + store |
| `search`     | 全局搜索 query/结果/最近输入与最近播放              | controller + store |
| `collection` | 合集 CRUD、歌曲管理、导入导出、表单对话框开关       | controller         |
| `tagEditor`  | Tag 双层编辑、三路合并、冲突解决                    | controller + store |
| `shell`      | runtime 编排、面板/视图开关、toast、跨域协调        | controller + store |

依赖方向（单向读）：

```text
env → library → player → download → home / search / collection / tagEditor → shell
```

`shell` 聚合其他域的结果，不反向写入业务状态。所有 controller 通过 `features/shell/appRuntime.svelte.ts` 注入到组件树。

## 4. 运行时架构

入口 `createAppRuntime()`（`features/shell/appRuntime.svelte.ts`）一次性：

1. 创建并持有各域 controller / store
2. 订阅 Tauri 事件（播放、下载、库存、偏好等）并分发给对应 controller
3. 通过 `shellStore.currentView` 切换视图（`AppView = 'home' | 'search' | 'overview' | 'library' | 'tagEditor' | 'collection'`）
4. 协调搜索定位、播放队列、下载面板、设置面板等跨域交互

`App.svelte` 仅作薄模板层：

- `AppProviders` 把 runtime 注入 Svelte context（见下）
- `ViewRouter` 根据 `runtime.currentView` 切换主区视图
- `AppSideSheets` 懒挂载设置 / 下载任务两块 Sheet
- `StatusToastHost` 渲染全局 toast
- `FullscreenPlayer` 按需挂载

### Context 注入

`src/lib/contexts/` 为各域提供强类型的 setter/getter：

| Context      | 注入位置                | 消费场景                             |
| ------------ | ----------------------- | ------------------------------------ |
| `shell`      | `AppProviders` 顶层注入 | 视图切换、面板开关、toast 等跨域协调 |
| `player`     | `AppProviders`          | 任何展示 / 触发播放状态的子组件      |
| `download`   | `AppProviders`          | 下载按钮、徽标、Sheet                |
| `library`    | `AppProviders`          | 专辑列表、详情、搜索结果定位         |
| `collection` | `AppProviders`          | 合集面板、添加到合集菜单、表单对话框 |

消费方式统一用 `getXxxContext()`，禁止业务组件自己 `setContext`。

### IPC 规则

- **UI 展示组件禁止直接调用 `invoke` / `listen`**
- Rust command 注册统一维护在 `src-tauri/src/command_registry.rs`；新增或删除 command 时只修改这份注册表
- `src-tauri/src/command_scheduling.rs` 的 `COMMAND_SPECS` 是 command 调度元数据来源，必须与注册表保持覆盖一致
- 前端 command bridge 集中在 `lib/api.ts`、`lib/settingsApi.ts`、`lib/collectionApi.ts`
- `lib/appEvents.ts` 的 `AppEventMap` 统一维护事件名与载荷类型
- 事件订阅集中在 `appRuntime.svelte.ts` / `appRuntimeBootstrap.svelte.ts`；`features/player/miniPlayerBridge.ts` 是迷你播放器独立窗口的受控例外
- controller / shell / bridge 层承担 IPC 与事件转译

修改 command、事件名或载荷后，至少运行以下契约检查：

```bash
cargo test --manifest-path src-tauri/Cargo.toml command_scheduling::tests::command_registry_covers_all_tauri_commands -- --nocapture
bunx vitest run src/lib/features/contract/ipc-contract.test.ts
```

### 响应式粒度

- 高频 progress 数据单独 `$state`，与 jobs 结构体拆开
- 结构变更走 `jobs = [...]` 重建
- 高频 `Map` 状态使用 `SvelteMap`
- 派生数据优先 `$derived` / `$derived.by()`，避免在 effect 里手动同步

## 5. UI 系统

### 设计 token

核心维度：`surface`、`text`、`accent`、`motion`、`density`。

关键表面语义：`surface.window` / `.sidebar` / `.workspace` / `.sheet` / `.dock` / `.flyout` / `.state`。

阴影 / 边框 token：`--hero-card-shadow`、`--stage-shell-shadow`、`--sheet-border` 等在 `app.css` 顶部定义，深浅色自动切换。

Token 归属：

| 类型            | 内容                                          | 维护入口                                          |
| --------------- | --------------------------------------------- | ------------------------------------------------- |
| App theme       | 背景、文字、边框、surface                     | `applyAppThemeTokenSet`                           |
| Context theme   | accent、album accent、wave                    | `applyContextThemePalette`                        |
| Component alias | `toolbar-*`、`stage-*`、`player-*` 等组件语义 | `app.css`，不由 JS 运行时直接写入                 |
| 静态 token      | motion、easing、font                          | `app.css` / `design/gsap.ts` / `styles/fonts.css` |

`@theme inline` 只把 Tailwind token 映射到现有语义变量，不另建颜色来源。新增颜色时先确定所属层级，组件内不要复制主题值或直接写入根变量。

### 字体方案

全局字体使用 HarmonyOS Sans SC（本地 `@font-face`，不依赖 CDN）。西文展示场景额外提供 Geometos（品牌标识）和 NovecentoSansWide（宽体标签）。

CSS 变量：

| 变量             | 用途                     |
| ---------------- | ------------------------ |
| `--font-sans`    | 基础无衬线栈             |
| `--font-display` | 标题与展示文案           |
| `--font-body`    | 正文与 UI 文案           |
| `--font-mono`    | 等宽场景                 |
| `--font-brand`   | 品牌标识、Logo、大号英文 |
| `--font-wide`    | 英文分类标签、导航标题   |

规则：

- 组件不直接硬编码 `font-family`，统一通过 CSS 变量引用
- `--font-brand` / `--font-wide` 仅用于纯西文/数字内容
- 字体文件随应用打包，不引入外部 CDN

### Apple 化边界

视觉方向：`macOS 应用骨架 + Apple Music 的内容表达`。

- 玻璃材质只集中在 sheet / dock / flyout
- 主工作区保持干净，不做整页玻璃化
- 动态专辑色保留但降饱和、提亮、压对比
- 阴影、边框、高光保持轻量

### 动效规则

- 状态驱动的动画（入场 / 出场、位移、尺寸、布局、FLIP）统一通过 GSAP 控制，适配层位于 `src/lib/design/gsap.ts`
- 除下方「受控例外」小节列出的两类场景，**禁止**使用 CSS transition / animation、Svelte transition / animate、Web Animations API 等替代方案
- 缓动曲线统一使用 iOS 风格 CustomEase（已在 `gsap.ts` 注册）：

| 曲线         | 参数                   | 用途               |
| ------------ | ---------------------- | ------------------ |
| `ios`        | `0.25, 0.1, 0.25, 1.0` | 标准 ease-in-out   |
| `ios-in`     | `0.42, 0, 1, 1`        | 入场前置 / 收起    |
| `ios-out`    | `0, 0, 0.58, 1`        | 出场 / 释放        |
| `ios-spring` | `0.22, 0.61, 0.36, 1`  | 主要位移与布局动画 |

- 不使用 bounce 类夸张反馈
- 不使用 GSAP 内置的 `power2.out` / `power3.out` 等曲线
- `reduced motion` 开启时按 `getMotionDuration` 降级
- 通用 helper：`animateIn` / `animateOut` / `gsapScrollIntoView` / `killTweens`
- **响应式派生 tween**：当动画目标属性受 media query 约束（例如同一元素在不同断点下 base/active 尺寸不同）时，优先让 GSAP tween 一个进度型 CSS 变量（如 `--lyric-progress` 从 0 到 1），再由 CSS 用 `calc` 从该变量派生 `font-size` / `transform` / `color`；断点覆盖派生基础变量即可自动生效，JS 侧不需感知 media query。参考 `design/actions.ts` 中的 `lyricActiveTween`

#### 时长令牌

动画时长统一从 `gsap.ts` 导出的 `MOTION` 常量取值，作为 JS 侧单一真相来源，主档位与 `app.css` 的 `--motion-*` CSS 变量一一对应；不要在组件内硬编码毫秒数，也不要在 GSAP 调用里直接写秒制字面量（`duration: 0.2`）——一律走 `getMotionDuration(MOTION.*)` 以同时获得 reduced-motion 降级。出场普遍略短于入场，以营造“快速让位、从容入场”的 iOS 观感，故常规档位提供独立的 `*_OUT` 值。

| 令牌                | 入场  | 出场（`*_OUT`） | 用途                                                                           |
| ------------------- | ----- | --------------- | ------------------------------------------------------------------------------ |
| `MOTION.MICRO`      | 100ms | —               | 极快微交互：按下回弹、小浮层关闭                                               |
| `MOTION.FAST`       | 140ms | —               | hover / 细粒度状态反馈                                                         |
| `MOTION.BASE`       | 180ms | 150ms           | 常规元素进出                                                                   |
| `MOTION.SLOW`       | 260ms | 200ms           | 覆盖层 / 较大元素进出                                                          |
| `MOTION.PAGE`       | 320ms | 280ms           | 主视图页面级转场                                                               |
| `MOTION.OVERLAY_IN` | 200ms | —               | 浮层统一入场（dialog/select/tooltip 等），与浮层出场 `BASE_OUT` / `MICRO` 配对 |

此外，`app.css` 还定义了两个仅供 CSS 循环装饰使用的时长档，不进入 `MOTION.*`：

| CSS 变量           | 时长   | 用途                                                             |
| ------------------ | ------ | ---------------------------------------------------------------- |
| `--motion-spinner` | 900ms  | `motion-spin` keyframe 一圈耗时（Spinner / 单元格 loading 指示） |
| `--motion-pulse`   | 1800ms | `motion-pulse` keyframe 一次呼吸周期（骨架屏 / 占位块）          |

这两个档只允许在受控例外 ① 覆盖的循环装饰动画中通过 CSS `animation` 引用，不参与 GSAP 时间线。

少数经权衡的有意特例不并入令牌：如音量胶囊「展开 400ms / 收缩 799ms」的非对称节奏、列表 stagger 的起步 `delay` 与气泡内容的错位 `delay`（这些是编排偏移而非元素时长）。这类点保留就地数值，但须在调用处以注释说明为何是特例。

#### 转场原语

页面级与分层进入的转场统一走 `src/lib/design/view-transition.ts`，不要在组件内各自拼时间线：

- `runViewTransition`：主视图“推进 / 后退”转场。入场视图整屏滑入并保持不透明，离场快照同向小幅位移并淡出，避免重叠期重影。仅由 `ViewTransition.svelte` 使用
- `freezeViewSnapshot`：把离场视图的真实 DOM 节点 `cloneNode(true)` 冻结为静态快照并同步 `scrollTop`，保留已渲染图片与滚动位置，规避重新加载导致的闪白
- `runLayeredIn`：为一组层叠元素编排统一节奏的“分层进入”（封面 → 标题 → 列表），用于详情面板与骨架屏，保证加载态与就绪态观感同构

#### 受控例外

仅以下两类场景允许使用 CSS 动画能力，且必须满足对应约束：

1. **无限循环的 loading / 装饰动画**（spinner、骨架屏 pulse、不确定进度条、marquee）允许 CSS keyframes：
   - 优先复用 Motion 原语（`MotionSpinner` / `MotionPulseBlock` / `MotionMarqueeInner`）或 `app.css` 中的全局 keyframes（`motion-spin` / `motion-pulse` / `motion-progress-slide` / `motion-marquee`），不要在组件内复制同类 keyframes
   - 缓动使用 `var(--ease-ios)`；匀速旋转 / 匀速位移可用 `linear`
   - 必须提供 reduced-motion 降级：Motion 原语走 `reducedMotion` prop，直接写 keyframes 的场景用 `prefers-reduced-motion` media query 关闭动画
2. **纯状态颜色反馈**允许 CSS transition，统一写 `transition: var(--motion-hover)`：
   - 覆盖场景包括：`:hover` / `:active` / `:focus-visible` 等指针交互状态、以及应用状态驱动的 class 切换（例如当前活动歌词行的 `.active`、勾选项的高亮）——凡是只涉及 color / background / 边框 / 阴影 等非位移、非尺寸、非布局属性的状态反馈都在此列
   - 该 token 仅覆盖 background-color / color / border-color / opacity / box-shadow；位移、尺寸、布局变化仍必须走 GSAP
   - 时长与曲线由 token 统一（`--motion-fast` + `--ease-ios`），不要自行写 `transition: all …` 或自定义时长 / 曲线
   - reduced-motion 下该 token 全局置为 `none`，组件无需单独处理

### 组件分层

| 层级        | 说明                                  | 示例                                             |
| ----------- | ------------------------------------- | ------------------------------------------------ |
| Primitive   | 基础交互原语（shadcn-svelte/Bits UI） | Button、Dialog、Sheet、Tabs、Sonner、Skeleton    |
| App Variant | 项目视觉与状态约束包装                | AlbumCard、SongRow、MetadataPopover              |
| Composite   | 面向单个业务区域的复合组件            | TopToolbar、PlayerFlyoutStack、SettingsSheet     |
| View        | 域顶层视图（由 `ViewRouter` 路由）    | HomeView、SearchView、LibraryView、TagEditorView |
| Pattern     | 跨视图复用的结构模式                  | 侧栏列表、右侧 Sheet、空状态、骨架屏             |

### Primitive 库

`src/lib/components/ui/` 已落地的 primitive 目录：

```
alert-dialog · badge · button · collapsible-group · dialog · input
progress · scroll-area · select · separator · sheet · skeleton
slider · sonner · switch · tabs · tooltip
```

新增 primitive 须遵循 shadcn-svelte 的 slot/data-attribute 约定，并通过项目语义化 class（如 `.app-dialog`、`.sheet-*`、`.settings-field`）承接视觉。

### CSS reset 与语义 class 的分工

`src/app.css` 顶部的通配符 reset 已经被包进 `@layer base`：

```css
@layer base {
  * {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }
}
```

Tailwind v4 的 `theme / base / components / utilities` 四个 layer 中，`utilities` 优先级高于 `base`，因此 shadcn `Input` / `Button` 等组件自带的 `px-2.5` / `py-1` / `gap-*` 现在正常生效；直接写在组件上的 `class="px-3"` 也会正确覆盖默认 padding。

规约：

- 局部大范围的语义视觉（如 `.app-dialog`、`.sheet-section`、`.settings-field`）继续用普通 CSS 显式声明 padding / spacing / 排版，避免堆砌重复的 utility 组合；这些语义 class 优先级足以覆盖 utility 层。
- 组件内一次性、局部的间距调整优先用 Tailwind utility 或组件 variant，不再需要 `!important` / `px-3!` 之类的硬刚写法。

## 6. 国际化（i18n）

国际化的语言来源、前后端资源结构和文案规则统一见 [internationalization.md](./internationalization.md)。前端只镜像 `AppPreferences.locale`，不另行持久化语言。

响应式更新：组件文案必须显式依赖 `localeState.current` 建立响应式依赖。高频组件使用聚合 `$derived.by()` 模式，低频面板可用 `{#key localeState.current}`。

格式化辅助：`src/lib/i18n/formatters.ts` 提供 `formatByteSize` / `formatSpeed` / `formatDuration`。

## 7. 交互模式

### 下载标记消费

- 前端统一以后端内容接口返回的 `download` 字段为准，不自行推导
- `downloadStatus` 枚举：`missing` / `detected` / `verified` / `partial` / `unverifiable` / `mismatch` / `unknown`
- `mismatch` 按异常态处理；`missing` 表示库存中未匹配到本地文件，`unknown` 表示尚未完成扫描或状态无法判定，UI 层需保留兜底分支

### 曲目点击

- 默认：点击播放
- 多选模式：点击切换选中状态

### 播放状态流

前端通过 Tauri command 拉起播放，通过 event 持续同步状态（`player-state-changed` / `player-progress`）。

### 视图切换

视图切换由 `shellStore.currentView` 单一来源驱动，由 `SidebarNav` 与 `ViewRouter` 共同消费，不在业务组件里直接 mutate `currentView` 之外的视图状态。

## 8. 内容与反馈规范

- 句子短、少解释、不营销
- 面板标题像系统功能名
- 按钮动词优先，6 字内
- 错误先说失败对象，再说可恢复动作
- toast 只保留结果和必要下一步
- 空状态说当前没有什么 + 引导动作
- Loading 优先骨架

## 9. Visual Contract · family × depth 视觉族

### 9.1 支持集与 fallback

`SUPPORTED_THEME_FAMILIES = ['glass', 'material', 'terminal']`，`SUPPORTED_THEME_DEPTHS = ['flat', 'balanced', 'deep']`。主题包声明超出支持集的值 fallback 到 `glass` / `balanced` 并生成 `sanitizeWarnings`；不 reject 保证前向兼容。

- **glass**：iOS 液态玻璃（默认族，视觉零变化）
- **material**：Material 3 elevation + emphasized easing
- **terminal**：monochrome + 直角 + 无光晕，Router 复用 Material view，通过 `:root[data-theme-family='terminal']` CSS baseline 把 shape/elevation/blur token 全部归零实现风格切换

### 9.2 家族切换契约（重要）

**家族运行时切换会重置组件的瞬时视觉状态**，具体表现：

- `PlayToggleGlyph`、`VolumeCapsule` 使用 `{#if}{:else}` 分发到 `glass/` / `material/` 子 view。切换 family 时旧 view 被 unmount，触发 `controller.destroy()` / `animator.destroy()`，新 view 从初始态（Closed / play）mount。
- 用户在音量胶囊 open 状态下切主题包，胶囊会瞬间收起（不是 bug）。
- `LyricsBubble`、`FullscreenPlayer` 家族切换用 CSS 域覆盖（`:root[data-theme-family='material']` 选择器），DOM 保留，不重置瞬时状态。

**为什么不做状态迁移**：跨家族转移 open/pending/dragging 等瞬时态需要 Router 持有额外状态镜像，增加复杂度且家族切换本身是低频操作。可接受的取舍。

### 9.3 添加新 primitive 的判断

判断某个 primitive 需要"DOM 拆分"（router + glass/material view）还是"CSS 域覆盖"：

- **DOM 拆分**：有 family 特有的 tween 曲线 / 显隐层数 / 关键交互（如 WaveGlassPanel、iOS spring 与 Material fade 的动画本质不同）
- **CSS 域覆盖**：DOM 结构和交互 family-无关，只有 chrome（背景、shadow、blur）差异

不确定时优先 CSS 域覆盖，成本更低、不引入运行时状态重置。

### 9.4 灰度上线状态（`theme_packages_v1` flag）

主题包库 UI（`ThemePackageLibrarySection`）由 localStorage flag `theme_packages_v1` 控制显隐：

- **Phase 1–3.2**：opt-in（`localStorage['theme_packages_v1'] === '1'` 才显示）
- **Phase 3.3 起（当前）**：**opt-out**（默认显示；`localStorage['theme_packages_v1'] === '0'` 隐藏）
- 稳定 4 个 minor 版本后移除 flag 检查

后端 IPC（`list_theme_packages` 等 9 条命令）与 preferences v2 schema 不受此 flag 控制，始终生效。flag 只是设置页 UI 入口的最后一道回退开关。

#### 悬挂 activePackageId 自愈

Phase 3.2→3.3 opt-in→opt-out 切换的 legacy 用户可能在关闭 UI 期间导入过主题包并留下 `preferences.theme.activePackageId`。翻转 flag 后 UI 恢复显示，若该包被卸载或未导入，会触发悬挂引用。

`themePackageManager.hydrate()` 在启动时校验 activePackageId 是否在 `installedPackages` 列表内；不在则通过 `setActiveThemePackage(null)` CAS 清空。用户体感 = "首次打开设置页时主题静默回到内置 preset"，无需手工介入。

#### 运维排查清单

1. **单用户状态排查**：让用户在浏览器 devtools 执行 `localStorage.getItem('theme_packages_v1')`。返回 `null` = opt-out 默认启用；返回 `'1'` = 显式启用（legacy opt-in 用户）；返回 `'0'` = 显式禁用。
2. **全局回滚路径**：本 flag 是**客户端本地开关**，无远端 kill switch。回滚必须发版：把 `featureFlagEnabled` 的默认值从 `!== '0'` 改回 `=== '1'`（回到 opt-in），或临时把 `<section>` 整段注释掉。
3. **结束灰度的判据**：连续 2 个 minor 版本内主题包相关支持工单 ≤ 1；主题包 preview→apply 转化率 ≥ 30%；`RevisionMismatch` 错误率 ≤ 0.1%。三项同时满足才移除 flag。

### 9.5 Phase 4 · CSS 覆盖层（未启动 · 触发式）

主方案 Phase 4 引入用户自定义 CSS 覆盖层（`.hbtheme` 打包含 CSS + assets）。**当前未启动**，因为它是**触发式**而非里程碑式，需要以下条件同时满足才应开工：

**触发条件**

- 用户社区提出"完全改 layout / 换字体 / 加自定义装饰"的呼声 ≥ 5 个 GitHub issue 或社区讨论线索
- Phase 3.3 灰度稳定至少 2 个 minor 版本

**必需前置（在动工前完成）**

- 完整 CSP 策略：`sandbox` iframe 隔离 CSS 执行环境 + 禁用 `unsafe-eval` 阻止 `expression()` / `@import url()` 攻击面
- ZIP 威胁模型：zipbomb 大小限制、路径穿越（`..` / 符号链接）、文件数量上限、magic bytes 校验
- Assets 白名单：仅允许字体（woff2 / ttf）与图片（png / webp / svg 需 sanitize），禁 JS / HTML / iframe

**默认立场**：不主动实现。触发条件与前置项其中一项未达标，就"沿用可选定位"（迁移方案原文）。当前 Phase 0-3 的 6 slot + 5 组 token + family × depth 已覆盖 95% 场景，尚未收到社区呼声。

**首次评估节点**：Phase 3.3 opt-out flag 稳定 3 个 minor 版本后。

## 10. 相关文档

- Rust rustdoc（`cargo doc`）：后端类型、命令、事件的接口文档
- [internationalization.md](./internationalization.md)：国际化架构参考
- [release-process.md](../process/release-process.md)：CI 与发布流程
