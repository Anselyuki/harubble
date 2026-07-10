# 技术决策记录

> 记录项目关键技术选型的背景、考量与结论。
>
> **状态说明（2026-04）**：本文以保留决策上下文和当时的取舍为主，不等于当前实现总览。若某项决策后续被调整，应优先在此追加注记或新增决策，而不是改写历史正文。
>
> 相关文档：[frontend-guide.md](../reference/frontend-guide.md)

## 决策 1：选择 `shadcn-svelte` 而非黑盒组件库

**背景**：项目需要一个 UI 组件基础层来支撑桌面应用的深度定制需求。

**考量**：

- 传统黑盒组件库（如 MUI、Ant Design）强迫页面长成统一模板风格，不适合需要独特视觉表达的桌面应用
- 项目需要快速补齐 `Sheet / Select / Switch / Progress / Tooltip / Toast` 等通用能力
- 无障碍和交互基础必须完整

**结论**：选择 `shadcn-svelte`，原因是：

1. 它不是黑盒组件库，源码直接复制到项目中，可深度定制
2. 基于 `Bits UI`，无障碍和交互基础较完整
3. 允许保留项目自己的视觉语言，不会强迫统一模板风格

## 决策 2：选择 `Tailwind CSS` 作为主样式层

**背景**：项目原有 `app.css` 超过 2000 行，样式重复较多，难以维护。

**考量**：

- 手写 CSS 在组件拆分后会让局部样式继续依赖超长全局文件
- 需要支持动态主题色和亮暗色模式
- spacing、radius、border、state 等基础样式需要收敛成稳定规则

**结论**：选择 `Tailwind CSS`，原因是：

1. 更适合把 spacing、radius、border、state 收敛成稳定规则
2. 业务组件拆分后，Tailwind 能让局部样式跟组件一起收口
3. 可以与项目级 CSS 变量结合，继续支持动态主题色和亮暗色模式

## 决策 3：设计系统 ≠ 组件库

**背景**：改造初期容易误认为"安装组件库 + 改 CSS"就是设计系统。

**结论**：设计系统不等于组件库。一个成熟设计系统至少包含：

1. 原则和最佳实践
2. foundations（color、spacing、typography、elevation、radius）
3. components
4. patterns
5. content / voice & tone
6. designers kit 或设计资产
7. source code
8. 发布、弃用、贡献和治理机制

这意味着本项目不能只做"安装 shadcn-svelte + 把旧 CSS 改成 Tailwind"，还必须补齐：设计令牌、模式层、内容规范、组件状态管理、文档和治理。

## 决策 4：先拆结构再换样式

**背景**：`App.svelte` 承担了大量业务编排，如果直接在上面堆 Tailwind class，只会让结构更加混乱。

**结论**：先解决 `App.svelte` 的职责堆积，再做分块 UI 重构。否则 Tailwind class 只会堆进旧结构里，无法真正收敛复杂度。

**执行策略**：采用"绞杀者模式"（Strangler Fig），先拿设置面板作为首次实弹演练目标，验证技术栈有效性后再全面铺开。

## 决策 5：组件迁移分类标准

**背景**：不是所有组件都适合直接用 shadcn-svelte 替换。

**结论**：采用 A/B/C 三类分类标准：

| 类别 | 策略                                | 适用条件                                                          | 示例                                                                        |
| ---- | ----------------------------------- | ----------------------------------------------------------------- | --------------------------------------------------------------------------- |
| A 类 | 直接使用 `shadcn-svelte`            | 通用交互、语义清晰、视觉差异不大、无复杂业务耦合                  | Button、Select、Switch、Sheet、Progress、Tooltip、Dialog、Skeleton          |
| B 类 | 基于 `shadcn-svelte` 包一层项目组件 | 底层交互通用、视觉状态较多、项目里重复出现                        | ToolbarIconButton、AppBadge、PanelSection、StatusToast、PlayerControlButton |
| C 类 | 保留定制组件，只消费令牌和原语      | 业务耦合强、状态密度高、布局/动画独特、需要动态主题色或复杂上下文 | AlbumStage、AlbumCard、SongRow、PlayerDock、LyricsFlyout、PlaylistFlyout    |

**2026-07 注记（示例组件命名调整）**：C 类中 `LyricsFlyout` / `PlaylistFlyout` 的职责最终由 `PlayerFlyoutStack` + `LyricsBubble` 承担，未落地为原名；B 类中 `StatusToast` 实际落地为 `StatusToastHost`；`AppBadge` / `PanelSection` / `PlayerControlButton` / `ToolbarIconButton` 未按具名 B 类包装落地，历史上多以内联组件承接。分类标准本身不变。

## 决策 6：UI 组件禁止直接调用 Tauri IPC

**背景**：UI 组件与 Tauri `invoke` / `listen` 强耦合会导致重构期间事件监听丢失、状态同步混乱。

**结论**：建立"通信网关层"（Gateway Layer），具体规则：

- UI 组件中**严禁**直接调用 `invoke` 或 `listen`
- 创建领域服务文件（如 `src/lib/features/player/player-service.ts`），内部封装 `invoke` 和事件监听
- UI 仅绑定服务层暴露的响应式状态

这样在调整 UI 结构时，不用担心丢掉底层的事件监听。

**2026-07 注记（受控例外）**：secondary window `MiniPlayerWindow` 无法直接共享主窗口 `appRuntime`，其 Tauri IPC 通过 `src/lib/features/player/miniPlayerBridge.ts` 集中承载（`invoke` / `listen` 均在 bridge 层），组件本体仍不直接调用 IPC。当前可接受的 bridge 形态包括：`src/lib/api.ts` / `src/lib/settingsApi.ts` / `src/lib/collectionApi.ts` 以及 `src/lib/features/*/xxxBridge.ts`。

## 决策 7：前后端异步解耦 i18n 方案

**背景**：项目原始 UI 全部硬编码中文，需要支持中英双语切换。前端是 Svelte 5 + Vite（非 SvelteKit），后端是 Rust + Tauri 2。

**考量**：

- 前端可选方案：`@inlang/paraglide-js`、`svelte-i18n`、`i18next`、自研 runtime
- 后端可选方案：Rust Fluent（`fluent-templates`）、自研静态映射表、零依赖 match/format
- 两端需要共享语言来源，但不应共享同一套文案资源

**结论**：

1. 前端采用 `@inlang/paraglide-js`，构建期生成类型安全 message 函数，编译期捕获缺 key、参数类型错误和未使用文案
2. 后端采用 `fluent-templates`，使用 Fluent `.ftl` 文件管理系统通知、偏好校验和用户错误文案
3. 两端异步解耦实现：各自独立落地，不互相调用运行时，不等待对方文案迁移完成
4. 只通过 `AppPreferences.locale` 契约同步当前语言，不共享同一套文案资源

**选择 Paraglide 的原因**：

- 与 Vite 项目贴合，构建期生成，不需要运行时字符串查找
- TypeScript 参数类型安全，比 `t('key')` 字符串方式更可靠
- 本项目不使用 SvelteKit，Paraglide 纯 Vite 模式足够；`svelte-i18n` 和 `i18next` 的运行时开销不必要

**选择 Fluent 的原因**：

- 避免先做静态映射表、后迁移 `.ftl` 的二次成本
- Fluent 语法支持复数、参数和条件选择，适合通知和错误场景
- `fluent-templates` 支持编译期嵌入资源，部署时不需要额外文件

**Svelte 5 响应式集成约束**：

- Paraglide runtime 语言切换不会自动触发 Svelte 5 组件重新渲染
- 必须通过项目侧 `localeState.current`（`$state`）显式建立响应式依赖
- 高频组件使用聚合 `$derived.by()` 模式，低频面板允许 `{#key localeState.current}` 简化迁移

## 决策 8：Tag Registry 远程注册表 + 本地缓存方案

**背景**：需要为专辑和歌曲附加自定义元数据标签（流派、阵营、时代等），上游 API 不提供此类数据。

**考量**：

- 嵌入二进制：更新 tag 数据需要发版，灵活性差
- 远程 JSON + 本地缓存：tag 数据独立于应用版本更新，离线时仍可使用缓存
- 数据库存储：对于只读注册表而言引入不必要的复杂度

**结论**：采用远程 JSON 注册表 + 本地原子文件缓存方案，原因是：

1. tag 数据由 GitHub 托管的 JSON 文件定义，可独立于应用版本更新
2. 应用启动时异步拉取远程 JSON 并原子写入本地缓存；网络不可达时回退到本地缓存
3. 注册表包含多 locale 标签（zh-CN / en-US / ja-JP），按用户偏好 locale 解析展示名，支持 locale 回退

**Tag 注入模式**：

- tag 数据在 command 层注入，不存入 API 响应缓存，避免缓存与注册表更新不一致
- `get_albums`、`get_album_detail`、`get_song_detail`、`get_latest_albums`、`get_albums_by_series`、`get_albums_by_tag_dimension` 在库存注入后额外注入 tag 数据
- 歌曲 tag 继承所属专辑 tag 并与自身 tag 合并去重

**搜索索引策略**：

- 搜索索引包含所有 locale 下的 tag 值（不只是当前 locale），确保搜索不受用户语言切换影响
- tag 值同时生成拼音全拼和首字母变体，与既有拼音召回机制一致
- tag 命中通过 `matchedFields` 中的 `tagValues` 枚举值表达

## 决策 9：保留 `* { padding: 0 }` 全局 reset，局部 unlayered CSS 覆盖

**背景**：在 2026-05-28 重构 Dialog 视觉时发现，`<Input />` 与 `<Button />` 的 shadcn 默认 `px-2.5` 完全没有生效，文字紧贴边框。

**根因**：`src/app.css` 顶部有 unlayered 通配符 reset：

```css
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}
```

Tailwind v4 把所有 utility 放进 `@layer utilities`。CSS cascade layer 规范规定 **unlayered 样式优先级高于任何 layer**（与 specificity 无关），所以 unlayered 的 `* { padding: 0 }` 会屏蔽 layered 的全部 `p-*` / `px-*` / `py-*` utility。

**考量**：

- 彻底删除 reset：大量自研页面在视觉上依赖它（依赖 `margin/padding: 0` 的默认值），回归面积难以评估
- 把 reset 包进 `@layer base`：可让 utilities 重新可达，但同样需要全量回归测试
- 局部 unlayered CSS 覆盖：在需要 padding 的复合组件 / pattern 层用 specificity 更高的普通 CSS 显式声明，scope 收敛

**结论**：采用第三种方案，原因是：

1. harubble 的样式体系已经大量依赖局部语义 class（`.sheet-*`、`.app-dialog`、`.settings-field` 等），而非直接消费 Tailwind utility
2. shadcn primitive 的视觉默认值在本项目里通常被业务层重新定义，padding utility 失效在大多数路径上没有可见副作用
3. 真正需要 padding 的范围（dialog、sheet section、settings field）可以在 unlayered 局部 class 里显式声明，specificity 自然胜过通配符 reset

**实践规约**：

- 不要靠 `<Input class="px-3" />` 这类 layered utility 覆盖默认 padding —— 它吃不过通配符 reset
- 不要随手加 `!` 重要标记硬刚通配符
- 需要 padding 的局部范围，统一在对应的语义 class（如 `.app-dialog input[data-slot='input']`）里用普通 CSS 声明

**未来重构方向**：如果引入更深度的 Tailwind 集成或更多 shadcn 默认视觉依赖，再统一评估是否把 reset 迁入 `@layer base`。届时需要走完整的视觉回归。

**2026-07 注记（决策 9 已调整）**：全局 reset 已经迁入 `@layer base`（见 `src/app.css` 顶部），utilities 层的 `p-*` / `px-*` / `py-*` 恢复可达；上面"实践规约"第 1、2 条（"不要靠 layered utility 覆盖默认 padding"、"不要随手加 `!` 硬刚"）随之作废；第 3 条（"需要 padding 的局部范围仍统一在语义 class 里显式声明"）仍作为组织性建议保留。历史结论保留在正文中，方便回溯当时的取舍。
