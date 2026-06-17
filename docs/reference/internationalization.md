# i18n 架构参考

本文档记录项目国际化的架构决策、技术选型和开发规范。

## 支持语言

- `zh-CN`：默认语言
- `en-US`：第二语言

## 品牌标识

"Harubble" 是产品品牌标识，不参与 i18n 翻译，在所有语言中保持原文不变。

## 技术选型

| 层   | 方案                        | 职责                                         |
| ---- | --------------------------- | -------------------------------------------- |
| 前端 | `@inlang/paraglide-js`      | message 编译、类型安全函数生成、tree-shaking |
| 后端 | `fluent-templates` (Fluent) | 系统通知、偏好校验、后端用户错误本地化       |

前后端异步解耦：各自管理文案资源，不共享 message 文件，不互相调用 i18n runtime。

## 单一语言来源

```text
后端偏好文件 / 默认值
  → get_preferences / set_preferences
  → AppPreferences.locale
  → 前端 Svelte locale mirror
  → Paraglide runtime + document.documentElement.lang
```

约束：

1. `AppPreferences.locale` 是唯一语言来源。
2. 前端不读取 `navigator.language`、不使用 localStorage/IndexedDB 保存语言。
3. 语言切换通过 `set_preferences` 提交，以后端返回的 `AppPreferences.locale` 为准。
4. 保存失败时保持当前语言不变，不回退到浏览器系统语言。

## 目录结构

### 前端

```text
messages/
├── zh-CN.json
└── en-US.json

src/lib/paraglide/          # Paraglide 生成目录，不手写
src/lib/i18n/
├── index.ts                # 对外入口
├── locale.svelte.ts        # Svelte 5 响应式语言状态
├── formatters.ts           # byte / speed / duration
└── types.ts                # Locale 类型定义
```

调用方式：

```ts
import * as m from '$lib/paraglide/messages';

m.download_job_status_running({ current: 2, total: 12 });
formatByteSize(bytes, locale);
```

### 后端

```text
src-tauri/locales/
├── zh-CN/main.ftl
└── en-US/main.ftl

src-tauri/src/i18n/
└── mod.rs                  # Locale 类型、Fluent loader、翻译入口
```

Fluent 约定：

- message id 使用 kebab-case，按业务域前缀：`notification-*`、`preferences-*`、`search-*`
- `.ftl` 只存用户可见文案
- `tr()` 统一负责参数注入和 fallback
- 缺 key 时回退 `zh-CN`；仍缺失时返回 message id，不 panic

## 开发规范

1. 新增前端用户可见文案必须通过 Paraglide message。
2. 新增 `zh-CN` message 时必须同步新增 `en-US` message。
3. 动态文案必须使用参数化模板，不手写字符串拼接。
4. 数量文案必须考虑单复数。
5. 新增后端用户错误优先给出 code/key，不优先给裸字符串。
6. 新增系统通知文案必须在 `.ftl` 文件中登记。
7. 当前仅支持 LTR 语言。
8. "Harubble" 品牌标识不翻译。

## 不参与 i18n 的内容

- 专辑名、歌曲名、艺术家名、歌词（上游 API 返回的内容数据）
- 日志 key、内部错误 key、Rust/TS 类型名、Tauri command 名称
- rustdoc、开发文档、README
- "Harubble" 品牌标识

## 验证

```bash
bun run check        # Paraglide 生成 + 类型检查 + 前端构建
cargo test --workspace
```

手动验收：切换语言后 UI 即时更新，重启后保持选择，保存失败不丢失当前语言。
