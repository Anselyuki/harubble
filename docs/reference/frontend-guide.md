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
   │  │  ├ download/                # 下载历史等下载域对话框
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
   │  └ shell/    appRuntime + appRuntimeBootstrap + appRuntimeComposites
   │              + store + settings + albumStageMotion + eventSequence + menuCommands
   │              + navigation / navigationManager / selectionManager
   │              + themeManager / themePackageManager / visualContract / downloadBridge
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

依赖方向：

```text
shared types / api bridges / env
              ↓
library  player  download  home  search  collection  tagEditor
              ↓
shell runtime composition
```

业务 controller 彼此默认并列，通过构造参数接收所需能力；例如 Home 只读取 album catalog 的窄接口，不直接持有 Library 全域状态。`shell` 负责创建 controller、注入跨域回调并把结果映射到 context，不越过 controller 直接改业务内部状态。

当前各业务域仍会复用 `features/shell/domainErrors.ts` 的错误格式化函数，这是已知的兼容例外，不代表允许业务域普遍依赖 shell。新增共享纯函数应放入独立基础模块，避免扩大这条反向依赖。

## 4. 运行时架构

入口 `createAppRuntime()`（`features/shell/appRuntime.svelte.ts`）一次性：

1. 创建并持有各域 controller / store
2. 通过 `appRuntimeBootstrap` 订阅 Tauri 事件（播放、下载、库存、偏好等），并分发给对应 controller
3. 通过 `shellStore.currentView` 切换视图（`AppView = 'home' | 'search' | 'overview' | 'library' | 'tagEditor' | 'collection'`）
4. 协调搜索定位、播放队列、下载面板、设置面板等跨域交互

跨域回调组合集中在 `appRuntimeComposites.svelte.ts`，事件顺序防护集中在 `eventSequence.svelte.ts`，原生菜单命令分发集中在 `menuCommands.ts`。这些模块仍由 `createAppRuntime()` 统一装配，不在展示组件中另建全局状态源。

`App.svelte` 是根装配与窗口级 shell 几何层，不持有业务域状态，但负责侧栏动画/resize、根级 overlay 和 runtime wiring：

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
- Rust Tauri command 的 handler、名称、domain、priority 与 cancel policy 统一维护在 `src-tauri/src/command_registry.rs` 的同一条目；`command_scheduling.rs` 的 `TAURI_COMMAND_SPECS` 由该宏自动生成
- 非 Tauri 的后台入口才单独维护在 `command_scheduling.rs` 的 `INTERNAL_COMMAND_SPECS`；新增或删除 Tauri command 时仍须同步前端 bridge/type 与契约测试
- 前端 command bridge 以 `lib/api.ts` 为主入口，设置与合集域分别由 `lib/settingsApi.ts`、`lib/collectionApi.ts` 收窄；新增 bridge 必须保持域边界，并纳入 IPC 契约测试
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

| 类型             | 内容                                                                              | 维护入口                                                  |
| ---------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------- |
| App theme        | 背景、文字、边框、surface                                                         | `applyAppThemeTokenSet`                                   |
| Context theme    | accent、album accent、wave                                                        | `applyContextThemePalette`                                |
| Component alias  | `toolbar-*`、`stage-*`、`player-*` 等组件语义                                     | `app.css`，不由 JS 运行时直接写入                         |
| 主题包运行时覆盖 | motion / shape / density / elevation / blur、语义字体栈与 `--theme-custom-*` 变体 | `themePackageManager` / `design/gsap.ts`                  |
| 静态设计输入     | iOS easing、打包字体与 `brand` / `wide` 字体角色                                  | `app.css` / `design/gsap.ts` / `src/lib/styles/fonts.css` |

`@theme inline` 只把 Tailwind token 映射到现有语义变量，不另建颜色来源。新增颜色时先确定所属层级，组件内不要复制主题值或直接写入根变量。

### 字体方案

全局字体使用 HarmonyOS Sans SC（本地 `@font-face`，不依赖 CDN）。西文展示场景额外提供 Geometos（品牌标识）和 NovecentoSansWide（宽体标签）。主题包的 `fontFamily` 只覆盖 `body / display / mono` 三个语义字体栈，依赖系统已有字体或应用已经打包并加载的字体；JSON 主题包不会下载远程字体，也不能携带字体资产。

CSS 变量：

| 变量             | 用途                     |
| ---------------- | ------------------------ |
| `--font-sans`    | 基础无衬线栈             |
| `--font-display` | 标题与展示文案           |
| `--font-body`    | 正文与 UI 文案           |
| `--font-mono`    | 等宽场景                 |
| `--font-brand`   | 品牌标识、Logo、数字读数 |
| `--font-wide`    | 分类标签、导航标题       |

规则：

- 组件不直接硬编码 `font-family`，统一通过 CSS 变量引用
- `--font-brand` / `--font-wide` 用于品牌和宽体标签角色；内置西文字体不含中文时，缺失字形按字体栈回退到 `--font-sans`
- 应用内置字体文件随应用打包，不引入外部 CDN；主题包声明字体名不等于加载字体文件

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

少数经权衡的有意特例不并入主题包令牌：Ark UI 音量胶囊是一个局部 `minimal` depth 的直接操控，五个 family 固定共享 `180ms` reveal / `120ms` close，不随主题包 motion override 改变交互节奏；列表 stagger 的起步 `delay` 与气泡内容的错位 `delay` 仍属于编排偏移而非元素时长。这类点保留就地数值，但须在调用处以注释说明为何是特例。

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

国际化的语言来源、前后端资源结构和文案规则统一见 [internationalization.md](./internationalization.md)。主窗口前端只镜像 `AppPreferences.locale`，不另行持久化语言。Windows Mini Player 当前只同步偏好中的主题字段，尚未把 `locale` 应用到它自己的 Paraglide runtime 与 `document.lang`，因此仍使用启动基线 `zh-CN`；这是实现缺口，不是新的语言来源。

响应式更新：主窗口组件文案必须显式依赖 `localeState.current` 建立响应式依赖。高频组件使用聚合 `$derived.by()` 模式，低频面板可用 `{#key localeState.current}`。Windows Mini Player 目前不适用此契约；它的 locale hydration / 订阅仍待实现。

格式化辅助：`src/lib/i18n/formatters.ts` 提供 `formatByteSize` / `formatSpeed` / `formatDuration`。

## 7. 交互模式

### 下载标记消费

- 前端统一以后端内容接口返回的 `download` 字段为准，不自行推导
- `downloadStatus` 枚举：`missing` / `detected` / `verified` / `partial` / `unverifiable` / `mismatch` / `unknown`
- `mismatch` 按异常态处理；`missing` 表示库存中未匹配到本地文件，`unknown` 表示尚未完成扫描或状态无法判定，UI 层需保留兜底分支

### 曲目点击

- 默认：行内播放按钮是键盘与辅助技术的主操作；整行单击仅作为指针便利操作
- 多选模式：显式选择按钮负责切换选中状态，使用 `aria-pressed` 暴露当前状态
- 不给包含下载、合集等子按钮的整行容器添加 `role="button"`，避免嵌套交互语义

### 破坏性操作

- 清空收听历史、清空下载历史、删除合集和删除标签维度必须使用应用级 `AlertDialog` 二次确认，不调用浏览器原生 `confirm()` / `alert()`
- 原生菜单与页面按钮必须汇合到同一个 request/confirm 入口，不能绕过确认或空状态反馈
- 异步确认期间禁用确认与取消按钮，避免重复提交；对话框文案必须说明不会受影响的数据或任务

### 排序与拖拽

- 拖拽只是一种增强路径；合集歌曲和标签值必须同时提供可聚焦的拖拽手柄、方向键/Home/End 操作或独立上下移动按钮
- 排序完成后用 `aria-live="polite"` 公布新位置；首尾不可移动操作保持禁用
- 不能把 `draggable` 放在承载多个操作的整行容器上

### 选择器、菜单与滚动

- 二元/多选模式使用 `aria-pressed` 按钮组；只有实现完整焦点与面板关系时才使用 tabs/menu 语义
- 普通操作列表使用原生列表和按钮，打开浮层后把焦点移到首个可用操作
- 横向滚动区保留可见滚动条；仅在仍可沿目标方向滚动时消费纵向滚轮，首尾必须把滚动交还页面

### 通知权限

- 测试通知只能在权限已经是 `granted` 时发送
- 未授权的原生菜单命令打开通知设置并给出反馈，不隐式弹出系统授权请求；授权只能由设置页中的明确用户操作触发

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

`SUPPORTED_THEME_FAMILIES` 当前包含 `glass / material / terminal / ark / endfield / exa / popucom / corporate`；`SUPPORTED_THEME_DEPTHS` 同时兼容 legacy 的 `flat / balanced / deep` 与 Ark UI 的 `minimal / moderate / complex / maximal`。Rust sanitizer 只校验 visual contract 的长度与字符集，保留格式合法的未知值；前端 `resolveVisualContract` 再 fallback 到 `glass` / `balanced` 并返回 warning。当前 `themePackageManager` 只应用解析结果，不持久化或展示这组 resolver warning。

- **glass**：iOS 液态玻璃（默认族，视觉零变化）
- **material**：Material 3 elevation + emphasized easing
- **terminal**：monochrome + 直角 + 无光晕，Router 复用 Material view，通过 `:root[data-theme-family='terminal']` CSS baseline 把 shape/elevation/blur token 全部归零实现风格切换
- **ark / endfield / exa / popucom / corporate**：Harubble 内置的 Ark UI inspired 原创家族。包负责 tokens，`src/app.css` 按包声明的 depth 提供壳层签名；其中 ark / exa / popucom / corporate 为 `moderate`，Field Signal（`endfield`）为 `complex`。内置包不包含官方 logo、游戏素材或远程字体。

内置包源码位于 `src/lib/theme-packages/builtins/`，由 Rust `builtin.rs` 在编译期嵌入。内置包可预览、激活和导出，不能直接卸载或由新的同 ID 导入包覆盖。兼容例外是：若升级前已经存在同 ID 用户包，它会继续遮蔽新加入的内置包，并允许被同 ID 用户包替换；卸载该用户包后才重新露出内置包。

主题包与昼夜模式是独立轴，运行时遵循以下所有权：

- `slots` 提供包的基础色，`variants.light / variants.dark` 稀疏覆盖当前 effective scheme；`auto` 只负责把系统模式解析为 light 或 dark。
- `cssVariables` 提供家族公共变量，`cssVariableVariants.light / dark` 只覆盖依赖昼夜模式的变量。切换 scheme 时必须重新合并并清理上一模式的 inline 变量。
- 内容画布的 panel / rule 随昼夜模式变化；Ark UI 家族的固定 rail / toolbar 属于 shell grammar，使用独立壳层 token，不能复用 scheme-aware panel。
- 激活或预览主题包时，不应用 legacy preset 的单套 `customColors`；这些值保留在偏好中，停用主题包后恢复。设置页在此期间仅锁定 preset 与六个旧色槽，昼夜模式和动态专辑色仍可调整。
- 专辑 Context Theme 继续只通过 `CONTEXT_TOKEN_ALLOWLIST` 覆盖专辑语义色，不得写入 App Theme 的全局背景、正文或主题包家族变量。

### 9.2 家族切换契约（重要）

**家族运行时切换应保留语义 DOM 与交互连续性**，具体契约：

- `PlayToggleGlyph` 保留 glass / structured-control 分发。`VolumeCapsule` 的 `ark / endfield / exa / popucom / corporate` 共享同一个 semantic view 和 HCI 交互模型；`glass`、`material / terminal` 仍沿用原 view。
- Ark UI 音量胶囊的局部 depth 固定为 `minimal`：展开统一为 `180ms`，收起统一为 `120ms`；family 只通过静态 tokens、geometry 和 marker 表达身份，不改变动效通道、操作语义或自动收起规则。`prefers-reduced-motion` 下直接落到同一终态。
- 音量图标按钮始终是稳定的 mute / unmute toggle，以 `aria-pressed` 表达状态；hover 或 focus-within 只负责 reveal，不改变按钮含义。整数百分比在轨道内常驻，slider value、progress fill 与 `aria-valuetext` 共享同一 position 并直接更新，不对精密操控值做补间。
- reveal 由正常布局提供 `200px` 空间并推动左侧 controls，不得以绝对浮层覆盖相邻命中区；离开 hover / focus / dragging 后使用固定 `799ms` 操作宽限，这一 HCI 延迟不跟随 family motion token。粗指针环境常驻展开，父级同时预留 `200 × 40px`，slider 保持可聚焦和可直接触控。
- 颜色所有权保持分层：App Theme 负责胶囊结构、family marker 与 focus signal；album Context Theme 只负责 progress 和 thumb，不反向覆盖 App Theme 结构色。
- 五个 Ark UI family 之间切换时不使用 keyed remount；同一 DOM、当前焦点和 open 状态原位保留，CSS 只更新 family 的静态视觉契约，不重放 reveal。
- `LyricsBubble`、`FullscreenPlayer` 家族切换继续使用 CSS 域覆盖（如 `:root[data-theme-family='material']` 选择器），DOM 保留，不重置瞬时状态。

### 9.3 添加新 primitive 的判断

判断某个 primitive 需要"DOM 拆分"（router + glass/material view）还是"CSS 域覆盖"：

- **DOM 拆分**：语义结构、信息层级或关键交互真正不同（如 glass 的 `WaveGlassPanel` 与 structured-control view）；仅 easing、边框或 marker 不足以构成拆分理由。
- **CSS 域覆盖**：DOM 结构和交互 family-无关，差异仅位于 tokens / geometry / marker / chrome（背景、shadow、blur）。五个 Ark UI `VolumeCapsule` family 属于这一类。

不确定时优先 CSS 域覆盖，成本更低、不引入运行时状态重置。

### 9.4 主题包库折叠状态（`theme_packages_v1`）

主题包库沿用灰度阶段的 localStorage key `theme_packages_v1`，当前仅控制详细管理区的展开状态：

- **Phase 1–3.2**：opt-in（`localStorage['theme_packages_v1'] === '1'` 才显示）
- **Phase 3.3 起**：改为 opt-out（默认显示；`localStorage['theme_packages_v1'] === '0'` 隐藏）
- **当前**：`'0'` 只收起主题包详情，原位置始终保留“展开主题包库”入口；收起不会清除 activePackageId 或 previewingId

后端 IPC（`list_theme_packages` 等 9 条命令）、preferences v2 schema 与主窗口启动 hydration 不受此状态控制，始终生效。折叠只改变设置页布局，不会让已经激活或正在预览的主题失效。

#### 悬挂 activePackageId 自愈

Phase 3.2→3.3 opt-in→opt-out 切换的 legacy 用户可能在关闭 UI 期间导入过主题包并留下 `preferences.theme.activePackageId`。翻转 flag 后 UI 恢复显示，若该包被卸载或未导入，会触发悬挂引用。

共享 `themePackageManager` 在主窗口启动时先订阅 `preferences_snapshot`，再执行 `hydrate()` 并校验 activePackageId 是否在 installed + built-in 列表内；不在则通过 `setActiveThemePackage(null)` CAS 清空。该流程不依赖用户首次打开设置页。

#### 状态排查

`localStorage.getItem('theme_packages_v1')` 返回 `null` 或 `'1'` 时详情默认展开，返回 `'0'` 时详情收起。无论该值为何，标题与展开/收起按钮都必须存在；主题包激活状态以 preferences v2 为准。

### 9.5 Phase 4 · JSON 最小安全子集与完整 CSS 边界

Phase 4 JSON 最小安全子集已经落地，仍使用单个 JSON 文档，不执行作者提供的 stylesheet：

- `fontFamily.body / display / mono` 覆盖三个语义字体栈。sanitizer 限制单项长度、字符集和 CSS 黑名单；运行时只通过 `style.setProperty` 写入变量，不创建 `@font-face`，也不加载主题包资产。
- `cssVariables` 只接受 `--theme-custom-*` 命名空间；每个 map 清洗后最多保留 64 项，并拒绝声明/块结构字符、HTML 逃逸字符、CSS 转义字符和黑名单关键字。值在原始 UTF-8 长度超过 256 字节时触发按字符截断；因为截断单位不是字节，当前实现不保证非 ASCII 结果仍小于等于 256 字节。
- `cssVariableVariants.light / dark` 使用同一清洗规则。运行时先合并基础 map 与当前 effective scheme 的稀疏覆盖；切换 scheme 或主题包时先清理上一组 inline key，避免残留。
- 五套内置主题包均使用字体栈与昼夜变量覆盖，相关字段已经纳入 Rust sanitizer、前端类型和契约测试。

完整的用户自定义 CSS 路线仍未启动：当前不支持 `.hbtheme` ZIP、任意 selector / stylesheet、布局覆写或包内字体与图片 assets。该路线继续采用触发式决策，至少需要社区对完整布局/装饰定制的明确需求，并先完成以下安全前置：

- 完整 CSP 与隔离策略，阻断 `expression()`、`@import url()` 等执行或外联面
- ZIP 威胁模型，包括 zipbomb、路径穿越、符号链接、文件数量上限与 magic bytes 校验
- Assets 白名单，只允许经校验的字体和图片，禁止 JS / HTML / iframe

在需求阈值和安全前置未满足前，保持现有 JSON 最小安全子集，不扩展为任意 CSS 执行环境。

## 10. 相关文档

- Rust rustdoc（`cargo doc`）：后端类型、命令、事件的接口文档
- [internationalization.md](./internationalization.md)：国际化架构参考
- [release-process.md](../process/release-process.md)：CI 与发布流程
