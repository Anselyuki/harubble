//! 主题包核心数据结构定义。
//!
//! # 模块职责
//!
//! 定义主题包 JSON 文档模型（`ThemePackageDocument`）、清单（`ThemePackageManifest`）、
//! 摘要（`ThemePackageSummary`）与磁盘状态（`ThemePackageStatus`）。这些类型在
//! 后端子系统内部使用，同时通过 Tauri command 的 serde 序列化契约暴露给前端；
//! 前端 `src/lib/types.ts` 的 `ThemePackageDocument` 等类型形状必须与此保持一致。
//!
//! # 稳定性
//!
//! Phase 1 MVP 阶段，字段级契约尚未冻结；新增字段需带 `#[serde(default)]` 以
//! 保证向后兼容。破坏性变更（重命名 / 移除字段）需伴随 `schemaVersion` bump。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 主题包完整 JSON 文档（v1 schema）。
///
/// 一个主题包的最小语义单位。用户从文件或 URL 导入的 `.json` 反序列化为本结构，
/// 经 sanitizer 校验后落入 `PackageStore` 的 committed 目录。
///
/// # 字段说明
///
/// - `schema_version`：主题包 schema 版本号，当前仅 `1` 有效
/// - `manifest`：清单元数据（id、name、version、作者等）
/// - `slots`：6 个颜色 slot 的 hex 定义（`accent/surface/textPrimary/textSecondary/tint/danger`）
/// - `variants`：可选的 scheme 变体（light/dark 独立 slot），缺失时走全局派生
/// - `motion`：可选的 motion 档位覆盖（Phase 2 Step 2.c），主题包激活时前端应用到
///   GSAP `MOTION` 与 CSS `--motion-*` 变量；缺失时走内置默认档位
/// - `warnings`：sanitize 阶段收集的字段级降级说明；导入 UI 会展示给用户
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThemePackageDocument {
    pub schema_version: u32,
    pub manifest: ThemePackageManifest,
    #[serde(default)]
    pub slots: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variants: Option<ThemePackageVariants>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<ThemePackageMotion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<ThemePackageShape>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub density: Option<ThemePackageDensity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elevation: Option<ThemePackageElevation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blur: Option<ThemePackageBlur>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_contract: Option<ThemePackageVisualContract>,
    /// Phase 4：字体族声明（可选）。
    ///
    /// 允许主题包声明要使用的字体名，前端通过 `@font-face` + Tauri asset 协议加载。
    /// sanitizer 只校验字符串长度与允许字符集，不验证字体文件是否存在（运行时降级）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_family: Option<ThemePackageFontFamily>,
    /// Phase 4：自定义 CSS 变量声明（可选）。
    ///
    /// 允许主题包声明超出预定义 5 组 token 的 CSS 变量。key 必须以 `--theme-custom-`
    /// 开头（命名空间隔离，防止覆盖 app 内部变量），value 经过白名单 sanitize。
    /// 前端注入到 `:root` 下，所有组件均可通过 `var(--theme-custom-xxx)` 消费。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub css_variables: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// 主题包字体族声明（Phase 4）。
///
/// 通过声明 `body` / `display` / `mono` 三个语义角色的字体名，前端覆盖
/// `--font-body` / `--font-display` / `--font-mono` CSS 变量。
///
/// # 安全约束
///
/// - 字体名只允许 `[a-zA-Z0-9 \-_,]` 字符集（32-127 ASCII 可打印，排除引号与括号）
/// - 最长 256 字节
/// - 不包含 `url()` / `@import` / `expression()` 等危险关键字
/// - 应用于 CSS 时值会被单引号包裹，防止 CSS 注入
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThemePackageFontFamily {
    /// body 文本字体（对应 `--font-body`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// 标题 / 品牌字体（对应 `--font-display`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// 等宽字体（对应 `--font-mono`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mono: Option<String>,
}

/// 主题包 visual contract 声明（Phase 3 Step 3.1）。
///
/// 用于选择整体视觉族与深度层级：
///
/// - `family`：视觉语言族。schema 层只要求是字符串，运行时通过
///   `resolve_visual_contract` 校验是否在当前 app 版本的支持集内；不在则
///   fallback 到 `glass` 并累积 warning。
/// - `depth`：视觉深度。同上，fallback 到 `balanced`。
///
/// **注意**：schema 校验和"当前 app 版本已实现的支持集"是两回事。
/// 作者可以声明 `family: 'terminal'`，但 Phase 3.1 只实现了 `glass / material`；
/// 未支持的 family 会 fallback 并 warn，而不是拒绝安装（保持前向兼容）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThemePackageVisualContract {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<String>,
}

/// 主题包 elevation 档位覆盖（完整 CSS box-shadow 字符串）。
///
/// 每个档位是一整段合法的 `box-shadow` 值，支持多层阴影（逗号分隔）。
/// 未声明的字段保留 app.css 中的默认值。sanitizer 会拒绝含 `url(`、`expression(`
/// 等危险关键字的值。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThemePackageElevation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub none: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xs: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub md: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lg: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xl: Option<String>,
}

/// 主题包 blur 档位覆盖（backdrop-filter 模糊半径，像素）。
///
/// 未声明字段保留 app.css 默认。传 0 可关闭玻璃拟态。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThemePackageBlur {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sm: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub md: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lg: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xl: Option<u32>,
}

/// 主题包声明的 shape 档位覆盖（像素）。
///
/// 与 CSS 变量 `--shape-*` 一一对应，用于整体切换圆角风格：
/// - `xs / sm / md / lg / xl / 2xl`：递增台阶（数值可以覆写为任意 px）
/// - `pill`：胶囊 / 完全圆形（数值一般 9999px 或更大）
///
/// 未声明的字段保留 app.css 中的默认值。前端 `applyShapeOverride` 同步到
/// `document.documentElement` 上的 CSS 变量。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThemePackageShape {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sm: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub md: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lg: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xl: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "2xl")]
    pub xxl: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pill: Option<u32>,
}

/// 主题包声明的 density 档位覆盖（像素）。
///
/// 与 CSS 变量 `--density-*` 一一对应，用于整体切换 UI 密度（紧凑 / 舒适 / 松弛）。
/// 未声明字段保留 app.css 中默认值。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThemePackageDensity {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xs: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sm: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub md: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lg: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xl: Option<u32>,
}

/// 主题包声明的 motion 档位覆盖（毫秒）。
///
/// 每个字段对应 `MOTION` 常量中的一个档位；未声明的字段保留内置默认。
/// 稀疏语义与 `slots` 一致，允许主题包只覆盖其中一两档。
///
/// 前端在激活主题包时通过 `applyMotionOverride({ FAST, BASE, SLOW, PAGE, ... })`
/// 同步到 GSAP MOTION 与 CSS `--motion-*` 变量，保持四路真相源统一。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ThemePackageMotion {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub micro: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fast: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slow: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_out: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slow_out: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_out: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay_in: Option<u32>,
}

/// 主题包清单元数据。
///
/// 承载主题包身份识别与展示所需的最小字段集。`id` 是 committed 目录中的文件名主体，
/// 亦是 `ThemePreferences.active_package_id` 的引用值。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThemePackageManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_app_version: Option<String>,
}

/// scheme 变体覆盖（稀疏语义）。
///
/// `light` 与 `dark` 均为可选，且内部字段亦稀疏——只有作者显式声明的 slot 会被应用，
/// 未声明的走顶层 `slots` 或系统默认派生。这一稀疏语义与前端
/// `ThemeVariantOverride = DeepPartial<...>` 类型对齐（P0-3）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThemePackageVariants {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light: Option<BTreeMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dark: Option<BTreeMap<String, String>>,
}

/// 主题包磁盘状态。
///
/// 反映 `PackageStore` 三态状态机中某个具体主题包所处的目录：
/// - `Staging`：临时目录，正在写入或校验中
/// - `Committed`：稳定的、可被激活的主题包
/// - `PendingDelete`：已卸载，等待启动扫描时清理
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThemePackageStatus {
    Staging,
    Committed,
    PendingDelete,
}

/// 主题包展示摘要（`list_theme_packages` 命令返回值元素）。
///
/// 前端主题列表 UI 只需要 id/name/version + 是否为内置/是否激活即可渲染卡片，
/// 完整 slots 通过 `inspect_theme_package` 按需获取。这样列表命令返回体积最小化。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThemePackageSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub status: ThemePackageStatus,
    #[serde(default)]
    pub builtin: bool,
    #[serde(default)]
    pub sha256: Option<String>,
}
