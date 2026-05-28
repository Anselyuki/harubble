# 前端开发指南

> 前端架构、开发约定与设计规范。

## 1. 布局与主要组件

```text
┌──────────────┬──────────────────────────────────────────────┐
│ 专辑侧栏     │ 主内容区                                     │
│ AlbumCard ×N │ 顶部工具栏                                   │
│              │ 专辑舞台（封面大图）                         │
│              │ 专辑信息 / 操作按钮 / 曲目列表 / 内容滚动区  │
│              │                                              │
│              │ 底部播放器 Dock                              │
│              │ ├─ 传输控制 / 进度 / 乱序 / 循环 / 下载      │
│              │ └─ 歌词面板 / 播放队列面板                   │
└──────────────┴──────────────────────────────────────────────┘
                         ▲                    ▲
                         └── 设置 Sheet        └── 下载任务 Sheet
```

左侧和主内容区使用 `OverlayScrollbars` 管理滚动，macOS 下顶部保留拖拽区域。

### 主要组件

| 组件                           | 职责                                            |
| ------------------------------ | ----------------------------------------------- |
| `App.svelte`                   | 根装配层，controller 初始化、事件订阅、壳层编排 |
| `AlbumSidebarContainer.svelte` | 左侧专辑侧栏装配容器                            |
| `TopToolbar.svelte`            | 顶部工具栏（刷新、下载入口、设置入口）          |
| `AlbumWorkspaceContent.svelte` | 专辑舞台、详情、骨架屏和曲目区组合容器          |
| `SongRow.svelte`               | 曲目行（默认点击播放，多选模式点击勾选）        |
| `PlayerFlyoutStack.svelte`     | 底部播放器 Dock 与歌词/队列浮层                 |
| `FullscreenPlayer.svelte`      | 全屏播放器视图                                  |
| `SettingsSheet.svelte`         | 设置面板                                        |
| `DownloadTasksSheet.svelte`    | 下载任务面板                                    |
| `CollectionDetailPanel.svelte` | 合集详情面板                                    |
| `TagEditorView.svelte`         | Tag 编辑器主视图                                |

## 2. 域边界与依赖方向

| 域           | 职责                                                    | 形态               |
| ------------ | ------------------------------------------------------- | ------------------ |
| `env`        | 只读环境状态（isMacOS、prefersReducedMotion、视口信号） | store              |
| `library`    | 专辑列表/详情、库内搜索、切换竞态、封面预加载           | controller         |
| `player`     | 当前歌曲、播放队列、歌词加载与高亮、乱序/循环           | controller         |
| `download`   | 任务列表与操作、下载设置、单曲/整专/多选入口、历史筛选  | controller         |
| `home`       | 首页数据：最新专辑、系列分组、收听历史、状态仪表盘      | controller + store |
| `shell`      | 面板开关、视图切换、toast、全局交互协调                 | store + controller |
| `collection` | 合集列表与详情、歌曲管理、导入导出                      | controller         |
| `tagEditor`  | Tag 双层编辑、三路合并与冲突解决                        | controller + store |

依赖方向（单向读）：

```text
env → library → player → download → home → collection → tagEditor → shell
```

`shell` 只读取其他域的聚合结果，不反向写入业务状态。

## 3. 运行时架构

运行时以 `createAppRuntime()`（`features/shell/appRuntime.svelte.ts`）为核心：

1. 创建并持有各域 controller
2. 订阅 Tauri 事件并分发给对应 controller
3. 通过 `shellStore.currentView`（`'home' | 'library'`）切换视图
4. 协调搜索定位、播放队列、下载面板与设置面板的跨域交互

`App.svelte` 仅作为薄模板层消费 runtime 返回的响应式属性。

### IPC 规则

- **UI 展示组件禁止直接调用 `invoke` 或 `listen`**
- controller / shell / bridge 层承担 IPC 与事件接入
- 领域服务封装在 `api.ts` / `settingsApi.ts` / `features/*/service.ts`

### 响应式粒度

- 高频 progress 数据单独 `$state`，与 jobs 结构体拆开
- 结构变更走 `jobs = [...]` 重建
- 高频 `Map` 状态使用 `SvelteMap`

## 4. UI 系统

### 设计 token

核心维度：`surface`、`text`、`accent`、`motion`、`density`

关键表面语义：`surface.window` / `.sidebar` / `.workspace` / `.sheet` / `.dock` / `.flyout` / `.state`

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

视觉方向：`macOS 应用骨架 + Apple Music 的内容表达`

- 玻璃材质只集中在 sheet / dock / flyout
- 主工作区保持干净，不做整页玻璃化
- 动态专辑色保留但降饱和、提亮、压对比
- 阴影、边框、高光保持轻量

### 动效规则

- 不使用 bounce 类夸张反馈
- `reduced motion` 开启时降级
- 动效参数统一来自 `motion.ts`

### 组件分层

| 层级        | 说明                                  | 示例                                   |
| ----------- | ------------------------------------- | -------------------------------------- |
| Primitive   | 基础交互原语（shadcn-svelte/Bits UI） | Button、Badge、Sheet、Dialog、Skeleton |
| App Variant | 项目视觉和状态约束包装                | ToolbarIconButton、AppBadge            |
| Composite   | 面向单个业务区域的复合组件            | TopToolbar、PlayerDock、SettingsSheet  |
| Pattern     | 跨组件复用的结构模式                  | 侧栏列表、右侧 Sheet、空状态模式       |

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

**根治路径**（如未来重构 Tailwind 集成时考虑）：把通配符 reset 包进 `@layer base`，让 utilities 层重新可达，但需要逐一回归现有页面。背景见 `docs/history/decisions.md` 决策 9。

## 5. 国际化（i18n）

语言来源：`AppPreferences.locale` 是唯一来源，前端只镜像后端偏好。

前端翻译层使用 `@inlang/paraglide-js`，构建期生成类型安全 message 函数。

规则：

1. 用户可见文案必须通过 Paraglide message，不得硬编码
2. 新增 `zh-CN` message 时必须同步新增 `en-US` message
3. 动态文案使用参数化模板，不做字符串拼接
4. 上游内容数据（专辑名、歌曲名、歌词等）不翻译

响应式更新：组件文案必须显式依赖 `localeState.current` 建立响应式依赖。高频组件使用聚合 `$derived.by()` 模式，低频面板可用 `{#key localeState.current}`。

## 6. 交互模式

### 下载标记消费

- 前端统一以后端内容接口返回的 `download` 字段为准，不自行推导
- `downloadStatus` 枚举：`detected` / `verified` / `partial` / `unverifiable` / `mismatch`
- `mismatch` 按异常态处理

### 曲目点击

- 默认：点击播放
- 多选模式：点击切换选中状态

### 播放状态流

前端通过 Tauri command 拉起播放，通过 event 持续同步状态（`player-state-changed` / `player-progress`）。

## 7. 内容与反馈规范

- 句子短、少解释、不营销
- 面板标题像系统功能名
- 按钮动词优先，6 字内
- 错误先说失败对象，再说可恢复动作
- toast 只保留结果和必要下一步
- 空状态说当前没有什么 + 引导动作
- Loading 优先骨架

## 8. 相关文档

- Rust rustdoc（`cargo doc`）：后端类型、命令、事件的接口文档
- [roadmap.md](../history/roadmap.md)：后端路线图
- [decisions.md](../history/decisions.md)：技术选型决策记录
- [internationalization.md](./internationalization.md)：国际化架构参考
- [release-process.md](../process/release-process.md)：CI 与发布流程
