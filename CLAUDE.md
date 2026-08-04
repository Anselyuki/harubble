# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## 项目概览

- 技术栈：Rust + Tauri 2 + Vite + Svelte 5
- 形态：跨平台桌面应用（当前发布产物覆盖 macOS / Windows / Linux）
- 当前状态：下载、播放、库存、搜索、合集和 Tag Editor 等核心领域已落地；进行中的任务以当前代码、Issues 和 PR 为准，仓库文档不维护阶段完成清单

## 常用命令

```bash
bun install
bun run tauri:dev
bun run format
bun run format:check
bun run lint
bun run check
bun run build
bun run tauri:build

cargo fmt --all
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace

# 文档
cargo doc -p harubble_core --no-deps
cargo doc -p harubble --lib --no-deps --document-private-items
cargo doc -p harubble --bin harubble --no-deps --document-private-items
```

## 关键入口

- `src-tauri/src/main.rs`：Tauri 二进制入口与应用启动 wiring
- `src-tauri/src/command_registry.rs`：Tauri command 注册单一事实源
- `src-tauri/src/command_scheduling.rs`：command 调度元数据与资源域约束
- `src-tauri/src/app_state/mod.rs`：后端共享状态组合，聚合播放器、下载、库存、偏好、日志与搜索服务。已拆分为 `api_clients` / `download_subsystem` / `preferences` / `playback` / `media_controls` 子模块
- `src/App.svelte`：前端壳层入口，实例化 runtime，并装配侧栏、路由、播放器和侧边面板
- `src/lib/features/shell/appRuntime.svelte.ts`：controller 初始化、Tauri 事件订阅与跨域状态协调
- `src/lib/features/shell/appRuntimeBootstrap.svelte.ts` / `appRuntimeComposites.svelte.ts`：启动订阅与跨域回调组合
- `src/lib/api.ts`：主 Tauri command bridge
- `src/lib/settingsApi.ts`：设置面板专用 IPC bridge
- `src/lib/collectionApi.ts`：合集专用 IPC bridge
- `src/lib/types.ts`：前后端共享数据结构
- `src/lib/features/`：按 `env / library / player / download / home / search / shell / collection / tagEditor` 划分的领域目录；`contract/` 保存跨前后端 IPC 与主题包契约测试
- `src/lib/components/app/`：前端壳层组件目录，按业务域划分子目录：
  - `sidebar/`：侧栏框架（AppSidebar、SidebarNav、BrandLogo 等）
  - `player/`：播放控制（PlayerFlyoutStack、FullscreenPlayer、VolumeCapsule、LyricsBubble 等）
  - `home/`：首页视图（HomeView 及各 Home\* 子组件）
  - `library/`：库存主视图（LibraryView）
  - `search/`：全局搜索视图（SearchView、SearchBar、SearchRecentQueries、SearchRecentlyPlayed 等）
  - `album/`：专辑与库存（AlbumOverview、AlbumWorkspace、AlbumDetailPanel 等）
  - `collection/`：合集（CollectionDetailPanel、CollectionFormDialog、AddToCollectionMenu 等）
  - `download/`：下载域对话框（ClearDownloadHistoryDialog 等）
  - `tag-editor/`：标签编辑器（TagEditorView、TagEditorPanel、TagEditorConflict\* 等）
  - `shell/`：应用壳层（TopToolbar、AppSideSheets、SettingsSheet、DownloadTasksSheet）
- **主题包系统**（Phase 0-3 与 Phase 4 JSON 最小安全子集已落地；任意 CSS stylesheet、`.hbtheme` ZIP 与包内 assets 尚未实现）：
  - `src-tauri/src/theme_packages/`：builtin / service / store / sanitizer / downloader / types，五套编译期内置主题包 + 三态磁盘管理（staging / committed / pending-delete）+ SSRF 白名单 + 字体栈和 `--theme-custom-*` 变量清洗 + 真实 SHA-256 sidecar
  - `src-tauri/src/commands/theme_packages.rs`：9 条 IPC command（list / inspect / install-file / install-url / uninstall / set-active / preview / dismiss / export）
  - `src-tauri/src/preferences.rs`：v1 → v2 schema migration（追加 `active_package_id` + `revision` CAS 字段）
  - `src/lib/themeTokens.ts` / `src/lib/themePresets.ts`：`deriveGlobalTokensFromSlots` 派生函数 + SYSTEM_LIGHT/DARK_SLOTS
  - `src/lib/design/gsap.ts`：五组运行时 token 覆盖入口，以及 `applyFontFamilyOverride` / `applyCssVariablesOverride` 安全子集入口
  - `src/lib/features/shell/themePackageManager.svelte.ts` / `themePackageRuntime.svelte.ts`：共享主题包状态机 + App Theme slots/variant 响应态 + 字体栈和昼夜 CSS 变量覆盖 + preferences_snapshot 订阅 + CAS 循环 + 悬挂 activePackageId 自愈
  - `src/lib/features/shell/visualContract.svelte.ts`：family × depth 状态 + resolve + `data-theme-*` / `data-ark-*` HTML 属性写入
  - `src/lib/features/player/controllers/`：`playToggleController` / `volumeCapsuleController` 承担业务逻辑，view 只消费共享交互契约
  - `src/lib/components/app/player/glass/` / `material/` / `family/`：legacy primitive 视觉族拆分与五个 Ark UI family 的共享语义 view；WaveGlassPanel 仅 glass 使用

## 真相来源

- **后端契约真相**：Rust rustdoc（`cargo doc`）、`src-tauri/src/command_registry.rs` 与前端 bridge/type 定义
- **前端架构真相**：`docs/reference/frontend-guide.md`
- **发布流程真相**：`.github/workflows/ci.yml`、`.github/workflows/distribute.yml` 与 `docs/process/release-process.md`
- **文档目录**：`docs/README.md`

## 代码层约定

### 前端

- 前端相关实现一律以 Svelte 5 为最高优先级；除非用户明确要求，否则不要为了延续旧习惯而主动回退到旧版写法或保守兼容模式
- UI 展示组件不要直接调用 `invoke` / `listen`；统一走 bridge、controller 或具备明确边界的 shell 层
- 组件的 `font-family` 统一通过 `--font-sans` / `--font-body` / `--font-display` / `--font-mono` / `--font-brand` / `--font-wide` CSS 变量引用，不直接硬编码字体名；`brand` / `wide` 用于品牌和宽体标签角色，中文等缺失字形由其字体栈回退到 `--font-sans`，字体方案详见 `docs/reference/frontend-guide.md` 的「字体方案」小节
- 如果改了歌词、下载设置或播放器交互，同时检查 `src/App.svelte`、`src/lib/components/AudioPlayer.svelte`、`src/lib/components/app/player/PlayerTimeline.svelte` 与对应 controller/bridge 的状态同步
- **动画编排**：所有前端动画统一使用 GSAP 控制，适配层位于 `src/lib/design/gsap.ts`；不要新增或使用 CSS transitions / animations、Svelte transition / animate、Web Animations API 或其他动画方案。仅有两个受控例外：① 无限循环 loading / 装饰动画可用 CSS keyframes，须复用 Motion\* 原语或 `app.css` 全局 keyframes（`motion-spin` / `motion-progress-slide`）并做 reduced-motion 降级；② hover / active 纯状态颜色反馈统一使用 `transition: var(--motion-hover)`。详见 `docs/reference/frontend-guide.md` 的「动效规则」小节
- **动画曲线**：所有 GSAP 动画统一使用 iOS 风格的缓动曲线（已在 `src/lib/design/gsap.ts` 中注册为 CustomEase）：
  - `ios`：标准 ease-in-out（`0.25, 0.1, 0.25, 1.0`）
  - `ios-in`：ease-in（`0.42, 0, 1, 1`）
  - `ios-out`：ease-out（`0, 0, 0.58, 1`）
  - `ios-spring`：弹性出场（`0.22, 0.61, 0.36, 1`），用于主要的位移和布局动画
  - 不要使用 GSAP 内置的 `power2.out` / `power3.out` 等曲线，统一使用上述 iOS 曲线（Material family view 也遵循此约束）
- **设计 token 五组**（Phase 2 已注册）：使用 CSS 变量而非硬编码：
  - **shape**：`--shape-xs/sm/md/lg/xl/2xl/pill/circle`（`border-radius` 走 var）
  - **density**：`--density-xs/sm/md/lg/xl`（间距 / 内边距）
  - **elevation**：`--elevation-none/xs/sm/md/lg/xl`（`box-shadow` 完整字符串）
  - **blur**：`--blur-sm/md/lg/xl`（`backdrop-filter` 半径）
  - **motion**：`--motion-fast/base/slow/page` 等（已由 gsap.ts MOTION 常量镜像同步）
  - 新组件的样式默认从这些变量开始；非标准尺寸（如 7px / 22px）需组件专属时可保留硬编码，但需注释理由
- **悬浮 chrome 契约**：任何"漂浮"在内容之上的容器（TopToolbar 胶囊、悬浮工具栏、玻璃卡片……）**禁止**使用 Tailwind 的 `bg-white/*` / `border-white/*` / `text-white` / `shadow-[…rgba(…)]` 一类硬编码颜色，也不要用 `rounded-full` / `backdrop-blur-xl` 一类硬编码 Tailwind 尺寸类。要求：
  - 背景/边框/阴影统一走 `--toolbar-surface` / `--toolbar-highlight` / `--toolbar-shadow`（或语义相近的 `--surface-*` token），它们在 `:root.light` / `:root.dark` 各有一份，跟随 `effectiveScheme` 变化
  - 圆角/模糊走 `var(--shape-pill)` / `var(--blur-lg)` 等 token，让主题包 `shape` / `blur` 覆盖生效
  - icon 颜色走 `var(--text-primary)` / `var(--icon-*)`；chrome 背景与 icon 颜色必须绑定到同一 scheme 轴，避免"深色主题包 + 白底胶囊 → 白 icon 消失"的失效外观
  - 如果需要 chrome 只在某个 family 下变形，用 `:root[data-theme-family='xxx']` 域覆盖 token，而不是给组件写 family 分支
- **视觉族切换**（Phase 3 已落地）：新增 primitive 若需支持 family × depth 差异，两种模式二选一：
  - **DOM 拆分**：视觉本质不同（如 iOS spring vs Material fade），在 `src/lib/components/app/player/{glass,material}/` 建对应 view；Router 组件保留原路径读 `getVisualContract().family` 分发；业务逻辑收敛到 `src/lib/features/player/controllers/` 由 view 共享
  - **CSS 域覆盖**：DOM 结构 / 交互 family-无关，只有 chrome（背景 / shadow / blur）差异，在 `src/app.css` 用 `:root[data-theme-family='material']` / `[data-theme-family='terminal']` 域选择器覆盖
  - 详细决策矩阵与家族切换视觉状态重置契约见 `docs/reference/frontend-guide.md` §9.3
- **主题包库折叠状态**：`theme_packages_v1` localStorage 仅控制设置页主题包详情的展开/收起（`'0'` 为收起），恢复入口始终可见，且不会清除激活/预览状态；**后端 IPC、preferences v2 schema 与启动主题 hydration 始终生效**。兼容说明见 `docs/reference/frontend-guide.md` §9.4

### 后端与文档

- 后端”端点”指的是 Tauri command，不是 HTTP server route
- 共享数据结构优先在 Rust 侧定义，再让前端 `types.ts` 保持形状一致
- 涉及并发、异步或后台任务时，不跨 `await` 持有锁，不改变 cancel / stop / worker 生命周期，也不改变资源清理顺序
- 新增或删除 Tauri command 时，在 `src-tauri/src/command_registry.rs` 的同一条目维护 handler、名称、domain、priority 与 cancel policy；`command_scheduling.rs` 的 `TAURI_COMMAND_SPECS` 会由宏自动生成，只有非 Tauri 后台入口单独维护在 `INTERNAL_COMMAND_SPECS`。同时同步前端 bridge/type 与契约测试
- `src-tauri/src/main.rs` 顶部必须保留 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`；新增 GUI 二进制入口时同步设置，`@tauri-apps/api` 与 Rust `tauri` crate 保持 minor 版本一致
- 所有对外暴露的 API 都必须编写函数文档，且文档内容统一使用中文：
  - 至少说明用途、入参语义、出参/返回值语义以及关键副作用或错误场景
  - 层级较高、承担入口职责的 API，还应补充适用场景、使用注意事项与调用约束
  - 涉及明确契约边界时，写清前置条件、状态约束、不变量、是否幂等、是否允许重试
  - 从调用者视角出发，在有必要时补充返回数据的稳定性/兼容性预期、常见调用顺序与最小可用示例
  - 新增或修改对外 API 时同步补齐或更新对应文档；在可行时尽量补充文档测试
- 所有公开模块（尤其会进入 rustdoc 模块列表的 `pub mod`）都必须补充模块级 rustdoc，且文档内容统一使用中文：
  - 至少概括该模块当前公开职责、主要暴露能力与典型使用场景
  - 模块职责发生变化时，同步更新模块级 rustdoc，保证 rustdoc 首页、模块页与实际导出能力一致
- 如果改了 command 参数、返回值或事件载荷，要同步更新：
  - `src/lib/api.ts`
  - `src/lib/types.ts`
  - `src-tauri` / `harubble_core` 中对应的 rustdoc

### 格式化与质量

- 前端代码与 Markdown 文档默认使用 Prettier 统一格式化；前端静态规则检查默认使用 ESLint；Rust 代码格式化默认使用 `cargo fmt --all`
- `bun run check` 默认包含格式、lint、类型、前端构建与 `cargo check --workspace`，`cargo test --workspace` 需单独执行
- 结构性重构、测试整理与文档补充默认视为行为保持变更；不要改业务分支语义、状态流转顺序、事件顺序、错误语义或日志 key
- 行为保持类变更应保持单一目的和清晰边界，通过相关测试后再合入，不把命名整理、业务修正和契约变更混在同一批 diff 中

### 测试

- 测试整理优先按”内联单元测试 / crate 级场景测试 / 契约测试 / 前端测试”分层理解：依赖私有 helper、私有状态或内部执行态的测试继续保留内联；只有通过公开 API 就能稳定表达的行为场景，才适合迁到 `crates/<crate>/tests/`
- 不要为了测试迁移放大生产代码可见性；若外移测试会迫使 private / `pub(crate)` 边界继续外扩，应优先保留原地测试或单独设计高层测试 seam
- 涉及文件系统路径、缓存路径、下载输出路径或持久化路径的测试，不要写死平台分隔符；优先比较 `Path` / `PathBuf` 语义，或先做统一规范化后再比较，并避免只在 macOS 本地成立的断言
- 新增或整理测试时，优先按行为域、规则域、场景域分组，避免为了 DRY 过度抽象测试代码，也不要改变原有断言语义

### Git 与协作

- 未经用户明确指示，不要新建分支；默认在当前分支上工作，涉及分支切换、新建分支、基于分支的推送或 PR 准备时先确认
- 所有提交、PR 及相关 git / GitHub 协作文案一律使用中文
