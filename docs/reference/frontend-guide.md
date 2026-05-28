# 前端开发指南

> 前端架构、开发约定与设计规范。

## 1. 整体布局

```text
┌──────────────────────┬──────────────────────────────────────────────┐
│ AppSidebar           │ TopToolbar（macOS 拖拽区 + 工具入口）        │
│ ├ BrandLogo          ├──────────────────────────────────────────────┤
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
   ├ types.ts                       # 前后端共享数据结构
   ├ theme.ts                       # 主题切换
   ├ cache.ts / lazyLoad.ts / imageDataSrc.ts / downloadBadge.ts
   │
   ├ components/
   │  ├ ui/                         # shadcn-svelte / Bits UI primitive 包装
   │  ├ app/                        # 业务壳层组件（按域划分子目录）
   │  │  ├ shell/                   # 顶部工具栏、Sheet、Toast、Provider、Router
   │  │  ├ sidebar/                 # 侧栏框架与导航
   │  │  ├ home/                    # 首页区块
   │  │  ├ library/                 # 库视图
   │  │  ├ search/                  # 搜索视图
   │  │  ├ album/                   # 专辑舞台与详情
   │  │  ├ collection/              # 合集面板与表单
   │  │  ├ player/                  # 播放 Dock / 歌词 / 音量 / 全屏播放器
   │  │  └ tag-editor/              # Tag 编辑器视图与对话框
   │  ├ AlbumCard.svelte / SongRow.svelte / MetadataPopover.svelte
   │  └ Motion*.svelte              # 通用动效原语
   │
   ├ features/                      # 业务域 controller / store / 纯函数
   │  ├ env/      store.svelte.ts
   │  ├ library/  controller + selectors + helpers
   │  ├ player/   controller + queue + lyrics + volume
   │  ├ download/ controller + presenters + formatters + guards
   │  ├ home/     controller + store
   │  ├ search/   controller + store
   │  ├ collection/ controller
   │  ├ tagEditor/  controller + store + tagLibrary
   │  └ shell/    appRuntime + appRuntimeBootstrap + store + settings + albumStageMotion
   │
   ├ contexts/                      # Svelte context 键 + setter/getter（强类型）
   ├ design/                        # gsap 适配层 + 侧栏动画器 + variants
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
- IPC 入口集中在 `lib/api.ts`、`lib/settingsApi.ts`、`lib/collectionApi.ts`
- 事件订阅集中在 `appRuntime.svelte.ts` / `appRuntimeBootstrap.svelte.ts`
- controller / shell / bridge 层承担 IPC 与事件转译

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

- 所有动画统一通过 GSAP 控制，适配层位于 `src/lib/design/gsap.ts`
- **禁止**使用 CSS transition / animation、Svelte transition / animate、Web Animations API 等替代方案
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

### CSS 陷阱：全局 reset 屏蔽 Tailwind padding utility

`src/app.css` 顶部存在 unlayered 的通配符 reset：

```css
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}
```

Tailwind v4 通过 `@import 'tailwindcss'` 把所有 utility 注入 `@layer utilities`。按 CSS 规范，**unlayered 样式始终胜过 layered 样式**（与 specificity 无关），所以上面这条通配符 reset 会**完整屏蔽**所有 layered 的 `px-*` / `py-*` / `p-*` utility，包括：

- shadcn `Input` 自带的 `px-2.5 py-1`
- shadcn `Button` 各 size variant 的 `px-2.5` / `px-2` / `gap-*`
- 任何直接写在组件里的 Tailwind padding utility

**症状**：`<Input />` / `<Button />` 视觉上文字紧贴边框，明明源码写了 `px-2.5` 却完全没生效。

**已采用的对策**：在范围明确的局部 class（如 `.app-dialog`、`.sheet-section`、`.settings-field`）里**用 unlayered 普通 CSS 显式声明 padding**，依靠更高 specificity 胜过通配符 reset。

```css
/* 示例：dialog 内 input/button 留白 */
.app-dialog input[data-slot='input'] {
  padding-inline: 12px;
}
.app-dialog .dialog-footer [data-slot='button'],
.app-dialog .dialog-body [data-slot='button'] {
  padding-inline: 10px;
}
```

**不要**这样做：

- 在组件 prop 上加 `class="px-3"` 期望它覆盖默认 padding —— layered utility 仍然吃不过通配符 reset
- 用 `!` 重要标记（`px-3!`）硬刚 —— 视觉债务而非根治
- 单独删 `*` reset 的 `padding: 0` —— 大量页面在视觉上依赖它，回归面积过大

**根治路径**（如未来重构 Tailwind 集成时考虑）：把通配符 reset 包进 `@layer base`，让 utilities 层重新可达，但需要逐一回归现有页面。背景见 `docs/history/decisions.md` 决策 9，关联上游 issue Anselyuki/harubble#47。

## 6. 国际化（i18n）

语言来源：`AppPreferences.locale` 是唯一来源，前端只镜像后端偏好。

前端翻译层使用 `@inlang/paraglide-js`，构建期生成类型安全 message 函数到 `src/lib/paraglide/`。

规则：

1. 用户可见文案必须通过 Paraglide message，不得硬编码
2. 新增 `zh-CN` message 时必须同步新增 `en-US` message
3. 动态文案使用参数化模板，不做字符串拼接
4. 上游内容数据（专辑名、歌曲名、歌词等）不翻译

响应式更新：组件文案必须显式依赖 `localeState.current` 建立响应式依赖。高频组件使用聚合 `$derived.by()` 模式，低频面板可用 `{#key localeState.current}`。

格式化辅助：`src/lib/i18n/formatters.ts` 提供 `formatByteSize` / `formatSpeed` / `formatDuration`。

## 7. 交互模式

### 下载标记消费

- 前端统一以后端内容接口返回的 `download` 字段为准，不自行推导
- `downloadStatus` 枚举：`detected` / `verified` / `partial` / `unverifiable` / `mismatch`
- `mismatch` 按异常态处理

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

## 9. 相关文档

- Rust rustdoc（`cargo doc`）：后端类型、命令、事件的接口文档
- [roadmap.md](../history/roadmap.md)：后端路线图
- [decisions.md](../history/decisions.md)：技术选型决策记录
- [internationalization.md](./internationalization.md)：国际化架构参考
- [release-process.md](../process/release-process.md)：CI 与发布流程
