//! 主题包字段级 sanitizer。
//!
//! # 模块职责
//!
//! 承担主题包 JSON 反序列化后的字段级校验与清洗：
//!
//! - **颜色 slot 白名单**：只允许运行时可无损消费的 `#RRGGBB`
//! - **CSS 值黑名单**：拒绝 `url(...)`、`expression(...)`、`@import`、`javascript:` 等易被滥用的形式
//! - **manifest 长度限制**：id ≤ 64、name ≤ 128、version ≤ 32、其它文本 ≤ 512
//! - **motion 数值范围**：所有档位 clamp 到 `[0, 5000]` ms，超出区间以警告降级
//!
//! # warn-而非-reject 语义
//!
//! 主方案 §5 明确 sanitizer 的原则是"补齐而非报错"：单个 slot 非法只会：
//!
//! 1. 从最终 slots 中移除该字段（sanitize_document 内 mut Vec）
//! 2. 追加一条 warning 到 `document.warnings`，前端在导入 UI 上展示给用户
//!
//! 仅当**完整性契约破坏**（如 manifest.id 为空、schemaVersion 不合法）时才拒绝安装。

use crate::theme_packages::types::ThemePackageDocument;
use std::collections::BTreeMap;

/// 主题包 slot 白名单，与前端 `ThemeColorSlots` 完全对齐。
const KNOWN_SLOTS: &[&str] = &[
    "accent",
    "surface",
    "textPrimary",
    "textSecondary",
    "tint",
    "danger",
];

/// manifest 字段长度上限。
const MAX_ID_LEN: usize = 64;
const MAX_NAME_LEN: usize = 128;
const MAX_VERSION_LEN: usize = 32;
const MAX_TEXT_LEN: usize = 512;

/// Validate the stable identifier used by newly imported packages.
pub(crate) fn validate_package_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > MAX_ID_LEN {
        return Err(format!(
            "theme package id must contain 1 to {MAX_ID_LEN} ASCII characters"
        ));
    }
    let bytes = id.as_bytes();
    if !bytes[0].is_ascii_lowercase() || !bytes[bytes.len() - 1].is_ascii_alphanumeric() {
        return Err("theme package id must use lowercase kebab-case".to_string());
    }
    if bytes
        .iter()
        .any(|byte| !(byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-'))
        || id.split('-').any(str::is_empty)
    {
        return Err("theme package id must use lowercase kebab-case".to_string());
    }
    Ok(())
}

/// Validate an identifier read from the committed directory.
///
/// Older releases allowed uppercase letters and underscores in package ids.  We
/// keep those packages addressable, but the value still has to be a conservative
/// single path component so it can never escape `committed/` when used as a file
/// stem.  New imports continue to use the stricter kebab-case validator above.
pub(crate) fn validate_stored_package_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > MAX_ID_LEN {
        return Err(format!(
            "stored theme package id must contain 1 to {MAX_ID_LEN} ASCII characters"
        ));
    }
    if id == "." || id == ".." {
        return Err("stored theme package id cannot be a path component".to_string());
    }
    let bytes = id.as_bytes();
    if !bytes[0].is_ascii_alphanumeric() || !bytes[bytes.len() - 1].is_ascii_alphanumeric() {
        return Err("stored theme package id must be a safe file stem".to_string());
    }
    if bytes.iter().any(|byte| {
        !(byte.is_ascii_alphanumeric() || *byte == b'-' || *byte == b'_' || *byte == b'.')
    }) {
        return Err("stored theme package id must be a safe file stem".to_string());
    }
    // Do not create or address Windows device names even when running on Unix;
    // package directories can be copied between platforms.
    let upper = id.to_ascii_uppercase();
    let stem = upper.split('.').next().unwrap_or(upper.as_str());
    if matches!(stem, "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && matches!(stem.as_bytes().first(), Some(b'C' | b'L'))
            && stem.as_bytes()[1].is_ascii_digit()
            && stem.as_bytes()[2].is_ascii_digit()
            && stem.as_bytes()[3].is_ascii_digit())
    {
        return Err("stored theme package id uses a reserved device name".to_string());
    }
    Ok(())
}

/// motion 档位允许的最大毫秒数（超出视为异常，clamp）。
const MAX_MOTION_MS: u32 = 5000;
/// shape 档位允许的最大像素数（除 pill 外）。pill 允许到 65535。
const MAX_SHAPE_PX: u32 = 200;
/// shape pill 档位允许的最大像素数（用于超大圆角）。
const MAX_PILL_PX: u32 = 65535;
/// density 档位允许的最大像素数。
const MAX_DENSITY_PX: u32 = 128;
/// blur 档位允许的最大像素数。
const MAX_BLUR_PX: u32 = 128;
/// elevation box-shadow 字符串允许的最大字节数。
const MAX_ELEVATION_LEN: usize = 512;
/// fontFamily 单个字体名允许的最大字节数。
const MAX_FONT_NAME_LEN: usize = 256;
/// cssVariables 允许的最大条目数（防止注入超量 CSS 变量拖慢浏览器）。
const MAX_CSS_VARIABLES: usize = 64;
/// cssVariables 单个值允许的最大字节数。
const MAX_CSS_VAR_VALUE_LEN: usize = 256;
/// cssVariables key 的必须前缀（命名空间隔离，防止覆盖 app 内部变量）。
const CSS_VAR_KEY_PREFIX: &str = "--theme-custom-";

/// CSS 值黑名单关键字。遇到即拒绝该字段（不区分大小写）。
const CSS_BLACKLIST: &[&str] = &[
    "url(",
    "expression(",
    "@import",
    "javascript:",
    "<script",
    "behavior:",
    "-moz-binding",
    "vbscript:",
    "data:text/html",
];

/// Normalize a legacy color spelling to the six-digit form consumed by the UI.
///
/// The alpha channel is intentionally discarded: theme slots are opaque in the
/// current runtime contract.  The boolean in the return value indicates that a
/// legacy/alpha spelling was accepted and should produce a diagnostic warning.
fn normalize_color_value(raw: &str) -> Option<(String, bool)> {
    let value = raw.trim();
    if let Some(hex) = value.strip_prefix('#') {
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let rgb = match hex.len() {
            3 => {
                let mut expanded = String::with_capacity(6);
                for ch in hex.chars() {
                    expanded.push(ch);
                    expanded.push(ch);
                }
                expanded
            }
            4 => hex[..3].chars().flat_map(|ch| [ch, ch]).collect(),
            6 => hex.to_string(),
            8 => hex[..6].to_string(),
            _ => return None,
        };
        return Some((format!("#{rgb}").to_ascii_lowercase(), hex.len() != 6));
    }

    let inner = value.strip_suffix(')')?;
    let open = inner.find('(')?;
    if inner[open + 1..].contains(')') {
        return None;
    }
    let function = inner[..open].trim().to_ascii_lowercase();
    let args = &inner[open + 1..];
    let expected = match function.as_str() {
        "rgb" => 3,
        "rgba" => 4,
        _ => return None,
    };
    let parts = args.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() != expected || parts.iter().any(|part| part.is_empty()) {
        return None;
    }
    let mut channels = [0u8; 3];
    for (index, part) in parts.iter().take(3).enumerate() {
        let channel = if let Some(percent) = part.strip_suffix('%') {
            let value = percent.parse::<f32>().ok()?;
            if !value.is_finite() || !(0.0..=100.0).contains(&value) {
                return None;
            }
            (value * 2.55).round() as u8
        } else {
            let value = part.parse::<u16>().ok()?;
            if value > 255 {
                return None;
            }
            value as u8
        };
        channels[index] = channel;
    }
    if expected == 4 {
        let alpha = parts[3];
        let valid_alpha = if let Some(percent) = alpha.strip_suffix('%') {
            let value = percent.parse::<f32>().ok()?;
            value.is_finite() && (0.0..=100.0).contains(&value)
        } else {
            let value = alpha.parse::<f32>().ok()?;
            value.is_finite() && (0.0..=1.0).contains(&value)
        };
        if !valid_alpha {
            return None;
        }
    }
    Some((
        format!("#{:02x}{:02x}{:02x}", channels[0], channels[1], channels[2]),
        true,
    ))
}

/// 判断字符串是否为可兼容的颜色值（包括旧格式）。
pub(crate) fn is_valid_color_value(raw: &str) -> bool {
    normalize_color_value(raw).is_some()
}

/// 单个 slot BTreeMap 清洗结果。
struct SlotCleanResult {
    cleaned: BTreeMap<String, String>,
    warnings: Vec<String>,
}

/// 清洗 slot 集合：移除未知 key 与非法 value，累积警告。
fn sanitize_slot_map(raw: &BTreeMap<String, String>, scope: &str) -> SlotCleanResult {
    let mut cleaned = BTreeMap::new();
    let mut warnings = Vec::new();
    for (key, value) in raw {
        if !KNOWN_SLOTS.contains(&key.as_str()) {
            warnings.push(format!("{scope}: unknown slot '{key}' dropped"));
            continue;
        }
        let Some((normalized, legacy)) = normalize_color_value(value) else {
            warnings.push(format!(
                "{scope}: slot '{key}' has invalid color value, dropped"
            ));
            continue;
        };
        if legacy {
            warnings.push(format!(
                "{scope}: slot '{key}' normalized to opaque #RRGGBB"
            ));
        }
        cleaned.insert(key.clone(), normalized);
    }
    SlotCleanResult { cleaned, warnings }
}

/// 截断超长文本字段，超长视为警告。
fn clamp_text(field: &str, raw: &mut String, limit: usize, warnings: &mut Vec<String>) {
    if raw.chars().count() > limit {
        warnings.push(format!("manifest.{field} truncated to {limit} chars"));
        *raw = raw.chars().take(limit).collect();
    }
}

/// 对 motion 档位值做 clamp，超出 `[0, MAX_MOTION_MS]` 视为异常并 clamp。
fn clamp_motion(field: &str, value: &mut Option<u32>, warnings: &mut Vec<String>) {
    if let Some(v) = *value {
        if v > MAX_MOTION_MS {
            warnings.push(format!(
                "motion.{field}={v}ms exceeds cap {MAX_MOTION_MS}ms, clamped"
            ));
            *value = Some(MAX_MOTION_MS);
        }
    }
}

/// 对 shape / density 单档位值做 clamp，超出上限时记录警告。
fn clamp_scalar(
    domain: &str,
    field: &str,
    value: &mut Option<u32>,
    max: u32,
    warnings: &mut Vec<String>,
) {
    if let Some(v) = *value {
        if v > max {
            warnings.push(format!(
                "{domain}.{field}={v}px exceeds cap {max}px, clamped"
            ));
            *value = Some(max);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PackageIdPolicy {
    Canonical,
    Stored,
}

/// 完整清洗一份主题包文档（in-place）。
///
/// 返回 `Err` 表示遭遇不可恢复的完整性错误（schemaVersion / manifest.id 缺失等）；
/// 否则文档被就地修补，`document.warnings` 累积所有降级说明。
fn sanitize_document_with_policy(
    document: &mut ThemePackageDocument,
    id_policy: PackageIdPolicy,
    preserve_warnings: bool,
) -> Result<(), String> {
    // 硬门槛：schemaVersion / id 缺失直接拒（不 warn）
    if document.schema_version != 1 {
        return Err(format!(
            "unsupported theme package schemaVersion: {}",
            document.schema_version
        ));
    }
    match id_policy {
        PackageIdPolicy::Canonical => validate_package_id(&document.manifest.id)?,
        PackageIdPolicy::Stored => validate_stored_package_id(&document.manifest.id)?,
    }
    if document.manifest.name.trim().is_empty() {
        return Err("theme package name must be non-empty".to_string());
    }
    if document.manifest.version.trim().is_empty() {
        return Err("theme package version must be non-empty".to_string());
    }

    let mut warnings: Vec<String> = if preserve_warnings {
        std::mem::take(&mut document.warnings)
    } else {
        Vec::new()
    };

    // manifest 长度截断（id 已在上方硬校验，不可截断改变身份）
    clamp_text(
        "name",
        &mut document.manifest.name,
        MAX_NAME_LEN,
        &mut warnings,
    );
    clamp_text(
        "version",
        &mut document.manifest.version,
        MAX_VERSION_LEN,
        &mut warnings,
    );
    if let Some(ref mut desc) = document.manifest.description {
        clamp_text("description", desc, MAX_TEXT_LEN, &mut warnings);
    }
    if let Some(ref mut author) = document.manifest.author {
        clamp_text("author", author, MAX_TEXT_LEN, &mut warnings);
    }
    if let Some(ref mut license) = document.manifest.license {
        clamp_text("license", license, MAX_TEXT_LEN, &mut warnings);
    }
    if let Some(ref mut min_ver) = document.manifest.min_app_version {
        clamp_text("minAppVersion", min_ver, MAX_VERSION_LEN, &mut warnings);
    }

    // 顶层 slots 清洗
    let cleaned = sanitize_slot_map(&document.slots, "slots");
    document.slots = cleaned.cleaned;
    warnings.extend(cleaned.warnings);

    // variants 稀疏清洗
    if let Some(ref mut variants) = document.variants {
        if let Some(ref light) = variants.light {
            let cleaned = sanitize_slot_map(light, "variants.light");
            variants.light = if cleaned.cleaned.is_empty() {
                None
            } else {
                Some(cleaned.cleaned)
            };
            warnings.extend(cleaned.warnings);
        }
        if let Some(ref dark) = variants.dark {
            let cleaned = sanitize_slot_map(dark, "variants.dark");
            variants.dark = if cleaned.cleaned.is_empty() {
                None
            } else {
                Some(cleaned.cleaned)
            };
            warnings.extend(cleaned.warnings);
        }
    }

    // motion 数值范围检查
    if let Some(ref mut motion) = document.motion {
        clamp_motion("micro", &mut motion.micro, &mut warnings);
        clamp_motion("fast", &mut motion.fast, &mut warnings);
        clamp_motion("base", &mut motion.base, &mut warnings);
        clamp_motion("slow", &mut motion.slow, &mut warnings);
        clamp_motion("page", &mut motion.page, &mut warnings);
        clamp_motion("baseOut", &mut motion.base_out, &mut warnings);
        clamp_motion("slowOut", &mut motion.slow_out, &mut warnings);
        clamp_motion("pageOut", &mut motion.page_out, &mut warnings);
        clamp_motion("overlayIn", &mut motion.overlay_in, &mut warnings);
    }

    // shape 数值范围检查
    if let Some(ref mut shape) = document.shape {
        clamp_scalar("shape", "xs", &mut shape.xs, MAX_SHAPE_PX, &mut warnings);
        clamp_scalar("shape", "sm", &mut shape.sm, MAX_SHAPE_PX, &mut warnings);
        clamp_scalar("shape", "md", &mut shape.md, MAX_SHAPE_PX, &mut warnings);
        clamp_scalar("shape", "lg", &mut shape.lg, MAX_SHAPE_PX, &mut warnings);
        clamp_scalar("shape", "xl", &mut shape.xl, MAX_SHAPE_PX, &mut warnings);
        clamp_scalar("shape", "2xl", &mut shape.xxl, MAX_SHAPE_PX, &mut warnings);
        clamp_scalar("shape", "pill", &mut shape.pill, MAX_PILL_PX, &mut warnings);
    }

    // density 数值范围检查
    if let Some(ref mut density) = document.density {
        clamp_scalar(
            "density",
            "xs",
            &mut density.xs,
            MAX_DENSITY_PX,
            &mut warnings,
        );
        clamp_scalar(
            "density",
            "sm",
            &mut density.sm,
            MAX_DENSITY_PX,
            &mut warnings,
        );
        clamp_scalar(
            "density",
            "md",
            &mut density.md,
            MAX_DENSITY_PX,
            &mut warnings,
        );
        clamp_scalar(
            "density",
            "lg",
            &mut density.lg,
            MAX_DENSITY_PX,
            &mut warnings,
        );
        clamp_scalar(
            "density",
            "xl",
            &mut density.xl,
            MAX_DENSITY_PX,
            &mut warnings,
        );
    }

    // blur 数值范围检查
    if let Some(ref mut blur) = document.blur {
        clamp_scalar("blur", "sm", &mut blur.sm, MAX_BLUR_PX, &mut warnings);
        clamp_scalar("blur", "md", &mut blur.md, MAX_BLUR_PX, &mut warnings);
        clamp_scalar("blur", "lg", &mut blur.lg, MAX_BLUR_PX, &mut warnings);
        clamp_scalar("blur", "xl", &mut blur.xl, MAX_BLUR_PX, &mut warnings);
    }

    // visualContract：仅做长度截断与允许字符集校验；支持集校验在运行时（前端 resolveVisualContract）
    if let Some(ref mut vc) = document.visual_contract {
        clamp_visual_contract_field("family", &mut vc.family, &mut warnings);
        clamp_visual_contract_field("depth", &mut vc.depth, &mut warnings);
    }

    // elevation 字符串校验：长度截断 + 黑名单短路
    if let Some(ref mut elevation) = document.elevation {
        sanitize_elevation_field("none", &mut elevation.none, &mut warnings);
        sanitize_elevation_field("xs", &mut elevation.xs, &mut warnings);
        sanitize_elevation_field("sm", &mut elevation.sm, &mut warnings);
        sanitize_elevation_field("md", &mut elevation.md, &mut warnings);
        sanitize_elevation_field("lg", &mut elevation.lg, &mut warnings);
        sanitize_elevation_field("xl", &mut elevation.xl, &mut warnings);
    }

    // fontFamily 字体名清洗
    if let Some(ref mut ff) = document.font_family {
        sanitize_font_name_field("fontFamily.body", &mut ff.body, &mut warnings);
        sanitize_font_name_field("fontFamily.display", &mut ff.display, &mut warnings);
        sanitize_font_name_field("fontFamily.mono", &mut ff.mono, &mut warnings);
    }

    // cssVariables 条目数 + 命名空间 + 值清洗
    sanitize_css_variables(&mut document.css_variables, "cssVariables", &mut warnings);

    // cssVariableVariants 与基础变量共享完全相同的安全边界；每套 map 独立限额。
    if let Some(ref mut variants) = document.css_variable_variants {
        if let Some(ref mut light) = variants.light {
            sanitize_css_variables(light, "cssVariableVariants.light", &mut warnings);
            if light.is_empty() {
                variants.light = None;
            }
        }
        if let Some(ref mut dark) = variants.dark {
            sanitize_css_variables(dark, "cssVariableVariants.dark", &mut warnings);
            if dark.is_empty() {
                variants.dark = None;
            }
        }
        if variants.light.is_none() && variants.dark.is_none() {
            document.css_variable_variants = None;
        }
    }

    document.warnings = warnings;
    Ok(())
}

/// Sanitize a newly imported or built-in package using the canonical id policy.
pub(crate) fn sanitize_document(document: &mut ThemePackageDocument) -> Result<(), String> {
    sanitize_document_with_policy(document, PackageIdPolicy::Canonical, true)
}

/// Sanitize a document that already lives in committed storage.
///
/// This compatibility entry point accepts the safe legacy id grammar while
/// retaining trusted warnings that were previously generated by Harubble.
pub(crate) fn sanitize_stored_document(document: &mut ThemePackageDocument) -> Result<(), String> {
    sanitize_document_with_policy(document, PackageIdPolicy::Stored, true)
}

/// Sanitize a committed document whose sidecar is missing. Without the
/// integrity marker, serialized warnings are treated as untrusted package
/// input and replaced with diagnostics generated by this pass.
pub(crate) fn sanitize_untrusted_stored_document(
    document: &mut ThemePackageDocument,
) -> Result<(), String> {
    sanitize_document_with_policy(document, PackageIdPolicy::Stored, false)
}

/// Sanitize untrusted imported data and ignore package-authored warnings.
pub(crate) fn sanitize_import_document(document: &mut ThemePackageDocument) -> Result<(), String> {
    sanitize_document_with_policy(document, PackageIdPolicy::Canonical, false)
}

/// 校验单个 elevation box-shadow 字符串。
///
/// - 长度超上限 → 截断
/// - 命中 CSS 黑名单（url/expression/javascript/等）→ 直接置 None 并 warn
/// - 允许多层逗号分隔的 shadow；不做 CSS AST 层校验（保持 warn-而非-reject 语义）
///   清洗 visualContract 单字段：长度 ≤ 32 + 允许字符集 [a-z0-9\-_]，非法则清空并 warn。
///
/// 支持集校验（是否在当前 app 版本已实现的 family/depth 内）不在此层执行，
/// 由前端 `resolveVisualContract` 在应用主题包时做 fallback + warning。
fn clamp_visual_contract_field(
    field: &str,
    value: &mut Option<String>,
    warnings: &mut Vec<String>,
) {
    let Some(raw) = value else { return };
    if raw.is_empty() {
        *value = None;
        return;
    }
    if raw.len() > 32 {
        warnings.push(format!("visualContract.{field} exceeds 32 chars, dropped"));
        *value = None;
        return;
    }
    let allowed = raw
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
    if !allowed {
        warnings.push(format!(
            "visualContract.{field} contains invalid characters, dropped"
        ));
        *value = None;
    }
}

fn sanitize_elevation_field(field: &str, value: &mut Option<String>, warnings: &mut Vec<String>) {
    let Some(raw) = value else { return };
    if raw.len() > MAX_ELEVATION_LEN {
        warnings.push(format!(
            "elevation.{field} exceeds {MAX_ELEVATION_LEN} chars, dropped"
        ));
        *value = None;
        return;
    }
    // 结构字符拦截（defense-in-depth）：box-shadow value 内不应含 CSS 声明分隔符、
    // HTML 逃逸符或 CSS 转义符。与 cssVariables 共享同一套结构字符集。
    if raw.contains(';')
        || raw.contains('{')
        || raw.contains('}')
        || raw.contains('<')
        || raw.contains('>')
        || raw.contains('\\')
    {
        warnings.push(format!(
            "elevation.{field} contains structural chars (;{{}}<>\\), dropped"
        ));
        *value = None;
        return;
    }
    let lower = raw.to_ascii_lowercase();
    for banned in CSS_BLACKLIST {
        if lower.contains(banned) {
            warnings.push(format!(
                "elevation.{field} contains disallowed keyword '{banned}', dropped"
            ));
            *value = None;
            return;
        }
    }
}

/// 校验单个字体名字段（Phase 4）。
///
/// 允许字符集：`[a-zA-Z0-9 ,\-_\.]`（字体名常见字符）。
/// 命中 CSS 黑名单或超长则清空并 warn。
fn sanitize_font_name_field(field: &str, value: &mut Option<String>, warnings: &mut Vec<String>) {
    let Some(raw) = value else { return };
    if raw.is_empty() {
        *value = None;
        return;
    }
    if raw.len() > MAX_FONT_NAME_LEN {
        warnings.push(format!(
            "{field} exceeds {MAX_FONT_NAME_LEN} chars, dropped"
        ));
        *value = None;
        return;
    }
    let lower = raw.to_ascii_lowercase();
    for banned in CSS_BLACKLIST {
        if lower.contains(banned) {
            warnings.push(format!(
                "{field} contains disallowed keyword '{banned}', dropped"
            ));
            *value = None;
            return;
        }
    }
    let allowed = raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || " ,\\-_.".contains(c));
    if !allowed {
        warnings.push(format!(
            "{field} contains invalid characters for a font name, dropped"
        ));
        *value = None;
    }
}

/// 校验一个自定义 CSS 变量 map（Phase 4）。
///
/// - key 必须以 `--theme-custom-` 开头（命名空间隔离）
/// - key 除前缀外只允许 `[a-z0-9\-]`（合法 CSS 自定义属性名）
/// - value 经 CSS 黑名单过滤 + 长度截断
/// - 条目数超 MAX_CSS_VARIABLES 时截断并 warn
fn sanitize_css_variables(
    vars: &mut std::collections::BTreeMap<String, String>,
    scope: &str,
    warnings: &mut Vec<String>,
) {
    let mut to_remove: Vec<String> = Vec::new();
    let mut count = 0;
    for (key, value) in vars.iter_mut() {
        count += 1;
        if count > MAX_CSS_VARIABLES {
            warnings.push(format!(
                "{scope} exceeds {MAX_CSS_VARIABLES} entries; extra entries dropped"
            ));
            to_remove.push(key.clone());
            continue;
        }
        // key 命名空间校验
        if !key.starts_with(CSS_VAR_KEY_PREFIX) {
            warnings.push(format!(
                "{scope} key '{key}' must start with '{CSS_VAR_KEY_PREFIX}', dropped"
            ));
            to_remove.push(key.clone());
            continue;
        }
        let suffix = &key[CSS_VAR_KEY_PREFIX.len()..];
        let key_valid = !suffix.is_empty()
            && suffix
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !key_valid {
            warnings.push(format!(
                "{scope} key '{key}' suffix must be [a-z0-9-], dropped"
            ));
            to_remove.push(key.clone());
            continue;
        }
        // value 校验
        if value.len() > MAX_CSS_VAR_VALUE_LEN {
            warnings.push(format!(
                "{scope}['{key}'] value truncated to {MAX_CSS_VAR_VALUE_LEN} chars"
            ));
            *value = value.chars().take(MAX_CSS_VAR_VALUE_LEN).collect();
        }
        // 结构字符拦截（defense-in-depth）：
        // - `;` `{` `}`：CSS 声明与块结构分隔面，setProperty 会拒绝，
        //   但显式丢弃避免值经过其他 sink（如 <style> innerHTML）成为注入面
        // - `<` `>`：防止 `</style>` HTML 上下文逃逸
        // - `\`：CSS 转义可绕过 keyword 黑名单（`url\28 evil\29` 等价 `url(evil)`）
        if value.contains(';')
            || value.contains('{')
            || value.contains('}')
            || value.contains('<')
            || value.contains('>')
            || value.contains('\\')
        {
            warnings.push(format!(
                "{scope}['{key}'] contains structural chars (;{{}}<>\\), dropped"
            ));
            to_remove.push(key.clone());
            continue;
        }
        let lower = value.to_ascii_lowercase();
        let value_ok = !CSS_BLACKLIST.iter().any(|b| lower.contains(b));
        if !value_ok {
            warnings.push(format!(
                "{scope}['{key}'] contains disallowed keyword, dropped"
            ));
            to_remove.push(key.clone());
        }
    }
    for k in to_remove {
        vars.remove(&k);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme_packages::types::{
        ThemePackageBlur, ThemePackageCssVariableVariants, ThemePackageDensity,
        ThemePackageElevation, ThemePackageFontFamily, ThemePackageManifest, ThemePackageMotion,
        ThemePackageShape, ThemePackageVariants, ThemePackageVisualContract,
    };

    fn base_document() -> ThemePackageDocument {
        ThemePackageDocument {
            schema_version: 1,
            manifest: ThemePackageManifest {
                id: "acme".to_string(),
                name: "Acme".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                license: None,
                min_app_version: None,
            },
            slots: {
                let mut m = BTreeMap::new();
                m.insert("accent".to_string(), "#7c3aed".to_string());
                m.insert("surface".to_string(), "#ffffff".to_string());
                m
            },
            variants: None,
            motion: None,
            shape: None,
            density: None,
            elevation: None,
            blur: None,
            visual_contract: None,
            font_family: None,
            css_variables: BTreeMap::new(),
            css_variable_variants: None,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn is_valid_color_value_accepts_exact_six_digit_hex() {
        assert!(is_valid_color_value("#7c3aed"));
        assert!(is_valid_color_value(" #ABCDEF "));
    }

    #[test]
    fn is_valid_color_value_accepts_legacy_opaque_forms() {
        for value in [
            "#fff",
            "#abcd",
            "#7c3aedff",
            "rgb(255, 0, 0)",
            "rgba(0,0,0,0.5)",
            "rgb(100%, 0%, 0%)",
        ] {
            assert!(is_valid_color_value(value), "expected valid color: {value}");
        }
    }

    #[test]
    fn is_valid_color_value_rejects_invalid_forms() {
        assert!(!is_valid_color_value("red"));
        assert!(!is_valid_color_value("#zz"));
        assert!(!is_valid_color_value("#ab"));
        assert!(!is_valid_color_value("#12345"));
        assert!(!is_valid_color_value("rgb(300, 0, 0)"));
        assert!(!is_valid_color_value("rgba(0,0,0,2.5)"));
        assert!(!is_valid_color_value("rgb(1,2,3,4)"));
        assert!(!is_valid_color_value("rgba(1,2,3)"));
        assert!(!is_valid_color_value(""));
        assert!(!is_valid_color_value("   "));
    }

    #[test]
    fn is_valid_color_value_rejects_blacklisted_keywords() {
        assert!(!is_valid_color_value("url(evil.com)"));
        assert!(!is_valid_color_value("expression(alert(1))"));
        assert!(!is_valid_color_value("@import 'evil'"));
        assert!(!is_valid_color_value("javascript:void(0)"));
        assert!(!is_valid_color_value("<script>xss</script>"));
        assert!(!is_valid_color_value("VBScript:oops"));
    }

    #[test]
    fn sanitize_drops_unknown_slot_with_warning() {
        let mut doc = base_document();
        doc.slots.insert("evil".to_string(), "#000000".to_string());
        sanitize_document(&mut doc).unwrap();
        assert!(!doc.slots.contains_key("evil"));
        assert!(doc
            .warnings
            .iter()
            .any(|w| w.contains("unknown slot 'evil'")));
    }

    #[test]
    fn sanitize_drops_invalid_color_with_warning() {
        let mut doc = base_document();
        doc.slots
            .insert("accent".to_string(), "url(bad.jpg)".to_string());
        sanitize_document(&mut doc).unwrap();
        assert!(!doc.slots.contains_key("accent"));
        assert!(doc
            .warnings
            .iter()
            .any(|w| w.contains("invalid color value")));
    }

    #[test]
    fn sanitize_normalizes_legacy_slot_colors_without_losing_rgb() {
        let mut doc = base_document();
        doc.slots.insert("accent".to_string(), "#abc".to_string());
        doc.slots
            .insert("surface".to_string(), "rgba(1, 2, 3, 0.25)".to_string());
        sanitize_document(&mut doc).unwrap();
        assert_eq!(doc.slots.get("accent").map(String::as_str), Some("#aabbcc"));
        assert_eq!(
            doc.slots.get("surface").map(String::as_str),
            Some("#010203")
        );
        assert!(
            doc.warnings
                .iter()
                .filter(|warning| warning.contains("normalized"))
                .count()
                >= 2
        );
    }

    #[test]
    fn stored_id_policy_accepts_legacy_safe_stems_and_rejects_paths() {
        for valid in ["Legacy_Theme", "Ark-UI", "theme.v1", "theme_2"] {
            assert!(validate_stored_package_id(valid).is_ok(), "{valid}");
        }
        for invalid in ["../escape", "a/b", "a\\b", "CON", "", "a b", "a\0b"] {
            assert!(validate_stored_package_id(invalid).is_err(), "{invalid:?}");
        }
    }

    #[test]
    fn sanitize_rejects_unsupported_schema_version() {
        let mut doc = base_document();
        doc.schema_version = 2;
        assert!(sanitize_document(&mut doc).is_err());
    }

    #[test]
    fn sanitize_rejects_empty_id() {
        let mut doc = base_document();
        doc.manifest.id = "  ".to_string();
        assert!(sanitize_document(&mut doc).is_err());
    }

    #[test]
    fn sanitize_rejects_ids_that_are_not_lowercase_kebab_case() {
        for invalid in [
            "../../escape",
            "/tmp/x",
            "a/b",
            "Ark-UI",
            "ark_ui",
            "ark--ui",
            "-ark-ui",
            "ark-ui-",
        ] {
            let mut doc = base_document();
            doc.manifest.id = invalid.to_string();
            assert!(
                sanitize_document(&mut doc).is_err(),
                "id should be rejected: {invalid}"
            );
        }
    }

    #[test]
    fn sanitize_accepts_lowercase_kebab_case_id() {
        let mut doc = base_document();
        doc.manifest.id = "ark-ui-endfield-2".to_string();
        sanitize_document(&mut doc).unwrap();
        assert_eq!(doc.manifest.id, "ark-ui-endfield-2");
    }

    #[test]
    fn sanitize_truncates_overlong_text_fields() {
        let mut doc = base_document();
        doc.manifest.name = "x".repeat(200);
        sanitize_document(&mut doc).unwrap();
        assert_eq!(doc.manifest.name.chars().count(), MAX_NAME_LEN);
        assert!(doc.warnings.iter().any(|w| w.contains("truncated")));
    }

    #[test]
    fn sanitize_clamps_extreme_motion_values() {
        let mut doc = base_document();
        doc.motion = Some(ThemePackageMotion {
            fast: Some(20_000),
            base: Some(500),
            ..Default::default()
        });
        sanitize_document(&mut doc).unwrap();
        assert_eq!(doc.motion.as_ref().unwrap().fast, Some(MAX_MOTION_MS));
        assert_eq!(doc.motion.as_ref().unwrap().base, Some(500));
        assert!(doc.warnings.iter().any(|w| w.contains("clamped")));
    }

    #[test]
    fn sanitize_variants_cleans_sparse_map() {
        let mut doc = base_document();
        let mut dark = BTreeMap::new();
        dark.insert("accent".to_string(), "#ffffff".to_string());
        dark.insert("unknown".to_string(), "#000000".to_string());
        doc.variants = Some(ThemePackageVariants {
            light: None,
            dark: Some(dark),
        });
        sanitize_document(&mut doc).unwrap();
        let cleaned_dark = doc.variants.as_ref().unwrap().dark.as_ref().unwrap();
        assert!(cleaned_dark.contains_key("accent"));
        assert!(!cleaned_dark.contains_key("unknown"));
    }

    #[test]
    fn sanitize_preserves_existing_warnings_and_appends() {
        let mut doc = base_document();
        doc.warnings.push("pre-existing".to_string());
        doc.slots.insert("evil".to_string(), "#000".to_string());
        sanitize_document(&mut doc).unwrap();
        assert!(doc.warnings.contains(&"pre-existing".to_string()));
        assert!(doc.warnings.len() >= 2);
    }

    #[test]
    fn sanitize_clamps_shape_values_within_range() {
        let mut doc = base_document();
        doc.shape = Some(ThemePackageShape {
            md: Some(999),
            xxl: Some(12),
            pill: Some(80_000),
            ..Default::default()
        });
        sanitize_document(&mut doc).unwrap();
        let shape = doc.shape.as_ref().unwrap();
        assert_eq!(shape.md, Some(MAX_SHAPE_PX));
        assert_eq!(shape.xxl, Some(12));
        assert_eq!(shape.pill, Some(MAX_PILL_PX));
        assert!(doc.warnings.iter().any(|w| w.contains("shape.md")));
        assert!(doc.warnings.iter().any(|w| w.contains("shape.pill")));
    }

    #[test]
    fn sanitize_clamps_density_values_within_range() {
        let mut doc = base_document();
        doc.density = Some(ThemePackageDensity {
            md: Some(500),
            sm: Some(4),
            ..Default::default()
        });
        sanitize_document(&mut doc).unwrap();
        let density = doc.density.as_ref().unwrap();
        assert_eq!(density.md, Some(MAX_DENSITY_PX));
        assert_eq!(density.sm, Some(4));
        assert!(doc.warnings.iter().any(|w| w.contains("density.md")));
    }

    #[test]
    fn sanitize_clamps_blur_values_within_range() {
        let mut doc = base_document();
        doc.blur = Some(ThemePackageBlur {
            md: Some(999),
            xl: Some(8),
            ..Default::default()
        });
        sanitize_document(&mut doc).unwrap();
        let blur = doc.blur.as_ref().unwrap();
        assert_eq!(blur.md, Some(MAX_BLUR_PX));
        assert_eq!(blur.xl, Some(8));
        assert!(doc.warnings.iter().any(|w| w.contains("blur.md")));
    }

    #[test]
    fn sanitize_accepts_valid_elevation_shadow_strings() {
        let mut doc = base_document();
        doc.elevation = Some(ThemePackageElevation {
            md: Some("0 4px 12px rgba(0, 0, 0, 0.12)".to_string()),
            lg: Some("0 8px 24px rgba(0, 0, 0, 0.16), 0 2px 4px rgba(0, 0, 0, 0.08)".to_string()),
            ..Default::default()
        });
        sanitize_document(&mut doc).unwrap();
        let elevation = doc.elevation.as_ref().unwrap();
        assert!(elevation.md.is_some());
        assert!(elevation.lg.is_some());
    }

    #[test]
    fn sanitize_rejects_elevation_containing_url_or_expression() {
        let mut doc = base_document();
        doc.elevation = Some(ThemePackageElevation {
            md: Some("0 4px url(evil.png)".to_string()),
            lg: Some("expression(alert(1))".to_string()),
            ..Default::default()
        });
        sanitize_document(&mut doc).unwrap();
        let elevation = doc.elevation.as_ref().unwrap();
        assert!(elevation.md.is_none());
        assert!(elevation.lg.is_none());
        assert_eq!(
            doc.warnings
                .iter()
                .filter(|w| w.contains("elevation"))
                .count(),
            2
        );
    }

    #[test]
    fn sanitize_accepts_valid_visual_contract_values() {
        let mut doc = base_document();
        doc.visual_contract = Some(ThemePackageVisualContract {
            family: Some("glass".to_string()),
            depth: Some("balanced".to_string()),
        });
        sanitize_document(&mut doc).unwrap();
        let vc = doc.visual_contract.as_ref().unwrap();
        assert_eq!(vc.family.as_deref(), Some("glass"));
        assert_eq!(vc.depth.as_deref(), Some("balanced"));
    }

    #[test]
    fn sanitize_drops_invalid_visual_contract_characters() {
        let mut doc = base_document();
        doc.visual_contract = Some(ThemePackageVisualContract {
            family: Some("GLASS!!".to_string()),
            depth: Some("BALANCED".to_string()),
        });
        sanitize_document(&mut doc).unwrap();
        let vc = doc.visual_contract.as_ref().unwrap();
        assert!(vc.family.is_none());
        assert!(vc.depth.is_none());
        assert!(doc
            .warnings
            .iter()
            .any(|w| w.contains("visualContract.family")));
    }

    #[test]
    fn sanitize_truncates_overlong_visual_contract_strings() {
        let mut doc = base_document();
        doc.visual_contract = Some(ThemePackageVisualContract {
            family: Some("a".repeat(50)),
            depth: None,
        });
        sanitize_document(&mut doc).unwrap();
        let vc = doc.visual_contract.as_ref().unwrap();
        assert!(vc.family.is_none());
        assert!(doc
            .warnings
            .iter()
            .any(|w| w.contains("visualContract.family") && w.contains("exceeds")));
    }

    #[test]
    fn sanitize_truncates_overlong_elevation_strings() {
        let mut doc = base_document();
        let huge = "0 1px 2px black,".repeat(200); // ~3400 chars
        doc.elevation = Some(ThemePackageElevation {
            md: Some(huge),
            ..Default::default()
        });
        sanitize_document(&mut doc).unwrap();
        let elevation = doc.elevation.as_ref().unwrap();
        assert!(elevation.md.is_none());
        assert!(doc
            .warnings
            .iter()
            .any(|w| w.contains("elevation.md") && w.contains("exceeds")));
    }

    #[test]
    fn sanitize_accepts_valid_font_names() {
        let mut doc = base_document();
        doc.font_family = Some(ThemePackageFontFamily {
            body: Some("HarmonyOS Sans SC".to_string()),
            display: Some("Geometos, sans-serif".to_string()),
            mono: Some("SF Mono".to_string()),
        });
        sanitize_document(&mut doc).unwrap();
        let ff = doc.font_family.as_ref().unwrap();
        assert_eq!(ff.body.as_deref(), Some("HarmonyOS Sans SC"));
        assert_eq!(ff.display.as_deref(), Some("Geometos, sans-serif"));
        assert!(doc.warnings.is_empty());
    }

    #[test]
    fn sanitize_rejects_font_with_css_injection() {
        let mut doc = base_document();
        doc.font_family = Some(ThemePackageFontFamily {
            body: Some("url(evil.woff)".to_string()),
            display: None,
            mono: None,
        });
        sanitize_document(&mut doc).unwrap();
        let ff = doc.font_family.as_ref().unwrap();
        assert!(ff.body.is_none());
        assert!(doc.warnings.iter().any(|w| w.contains("fontFamily.body")));
    }

    #[test]
    fn sanitize_rejects_font_with_invalid_chars() {
        let mut doc = base_document();
        doc.font_family = Some(ThemePackageFontFamily {
            body: Some("Font<Name>".to_string()),
            display: None,
            mono: None,
        });
        sanitize_document(&mut doc).unwrap();
        let ff = doc.font_family.as_ref().unwrap();
        assert!(ff.body.is_none());
    }

    #[test]
    fn sanitize_css_variables_accepts_valid_entries() {
        let mut doc = base_document();
        doc.css_variables
            .insert("--theme-custom-bg".to_string(), "#ff0000".to_string());
        doc.css_variables
            .insert("--theme-custom-radius".to_string(), "8px".to_string());
        sanitize_document(&mut doc).unwrap();
        assert_eq!(
            doc.css_variables
                .get("--theme-custom-bg")
                .map(String::as_str),
            Some("#ff0000")
        );
        assert!(doc.warnings.is_empty());
    }

    #[test]
    fn sanitize_css_variable_variants_uses_the_same_rules_for_both_schemes() {
        let mut light = BTreeMap::new();
        light.insert(
            "--theme-custom-panel".to_string(),
            "rgba(255, 255, 255, 0.9)".to_string(),
        );
        light.insert("--bg-primary".to_string(), "#ffffff".to_string());

        let mut dark = BTreeMap::new();
        dark.insert(
            "--theme-custom-panel".to_string(),
            "rgba(0, 0, 0, 0.9)".to_string(),
        );
        dark.insert(
            "--theme-custom-injected".to_string(),
            "url(evil.png)".to_string(),
        );

        let mut doc = base_document();
        doc.css_variable_variants = Some(ThemePackageCssVariableVariants {
            light: Some(light),
            dark: Some(dark),
        });
        sanitize_document(&mut doc).unwrap();

        let variants = doc.css_variable_variants.as_ref().unwrap();
        let light = variants.light.as_ref().unwrap();
        let dark = variants.dark.as_ref().unwrap();
        assert_eq!(
            light.get("--theme-custom-panel").map(String::as_str),
            Some("rgba(255, 255, 255, 0.9)")
        );
        assert!(!light.contains_key("--bg-primary"));
        assert_eq!(
            dark.get("--theme-custom-panel").map(String::as_str),
            Some("rgba(0, 0, 0, 0.9)")
        );
        assert!(!dark.contains_key("--theme-custom-injected"));
        assert!(doc
            .warnings
            .iter()
            .any(|warning| warning.contains("cssVariableVariants.light")));
        assert!(doc
            .warnings
            .iter()
            .any(|warning| warning.contains("cssVariableVariants.dark")));
    }

    #[test]
    fn deserialize_legacy_document_without_css_variable_variants() {
        let source = r##"{
            "schemaVersion": 1,
            "manifest": { "id": "legacy", "name": "Legacy", "version": "1.0.0" },
            "slots": { "accent": "#7c3aed" },
            "cssVariables": { "--theme-custom-panel": "#ffffff" }
        }"##;

        let mut doc: ThemePackageDocument = serde_json::from_str(source).unwrap();
        sanitize_document(&mut doc).unwrap();

        assert!(doc.css_variable_variants.is_none());
        assert_eq!(
            doc.css_variables
                .get("--theme-custom-panel")
                .map(String::as_str),
            Some("#ffffff")
        );
    }

    #[test]
    fn sanitize_css_variables_rejects_bad_prefix() {
        let mut doc = base_document();
        doc.css_variables
            .insert("--bg-primary".to_string(), "#ff0000".to_string()); // wrong prefix
        sanitize_document(&mut doc).unwrap();
        assert!(doc.css_variables.is_empty());
        assert!(doc.warnings.iter().any(|w| w.contains("--bg-primary")));
    }

    #[test]
    fn sanitize_css_variables_rejects_injection_value() {
        let mut doc = base_document();
        doc.css_variables.insert(
            "--theme-custom-test".to_string(),
            "url(evil.png)".to_string(),
        );
        sanitize_document(&mut doc).unwrap();
        assert!(doc.css_variables.is_empty());
        assert!(doc
            .warnings
            .iter()
            .any(|w| w.contains("--theme-custom-test")));
    }

    #[test]
    fn sanitize_css_variables_enforces_max_entries() {
        let mut doc = base_document();
        for i in 0..=MAX_CSS_VARIABLES {
            doc.css_variables
                .insert(format!("--theme-custom-v{i}"), "#000".to_string());
        }
        sanitize_document(&mut doc).unwrap();
        assert!(doc.css_variables.len() <= MAX_CSS_VARIABLES);
        assert!(doc.warnings.iter().any(|w| w.contains("cssVariables")));
    }

    #[test]
    fn sanitize_rejects_css_variable_values_with_structural_chars() {
        let mut doc = base_document();
        doc.css_variables.insert(
            "--theme-custom-x".to_string(),
            "red; z-index: 999".to_string(),
        );
        doc.css_variables
            .insert("--theme-custom-y".to_string(), "{ color: red }".to_string());
        doc.css_variables
            .insert("--theme-custom-z".to_string(), "#7c3aed".to_string());
        sanitize_document(&mut doc).unwrap();
        // 含 ';' 或 '{}' 的两个被丢弃，正常十六进制颜色保留
        assert!(!doc.css_variables.contains_key("--theme-custom-x"));
        assert!(!doc.css_variables.contains_key("--theme-custom-y"));
        assert_eq!(
            doc.css_variables
                .get("--theme-custom-z")
                .map(|s| s.as_str()),
            Some("#7c3aed")
        );
        assert!(
            doc.warnings
                .iter()
                .filter(|w| w.contains("structural chars"))
                .count()
                >= 2
        );
    }

    #[test]
    fn sanitize_rejects_elevation_with_structural_chars() {
        let mut doc = base_document();
        // 覆盖 ';' '{' '}' 三种结构字符（与 cssVariables 字符集对称）
        for bad_val in [
            "0 8px 24px black; color: red",
            "0 8px 24px { color: red }",
            "0 8px 24px }",
        ] {
            let mut d = base_document();
            d.elevation = Some(ThemePackageElevation {
                md: Some(bad_val.to_string()),
                ..Default::default()
            });
            sanitize_document(&mut d).unwrap();
            assert!(
                d.elevation.as_ref().unwrap().md.is_none(),
                "should reject: {bad_val}"
            );
            assert!(
                d.warnings
                    .iter()
                    .any(|w| w.contains("elevation.md") && w.contains("structural chars")),
                "should warn for: {bad_val}"
            );
        }
        // 合法的多层 shadow 不受误伤
        doc.elevation = Some(ThemePackageElevation {
            md: Some("0 4px 12px rgba(0, 0, 0, 0.12), 0 2px 4px rgba(0, 0, 0, 0.08)".to_string()),
            ..Default::default()
        });
        sanitize_document(&mut doc).unwrap();
        assert!(doc.elevation.as_ref().unwrap().md.is_some());
    }

    #[test]
    fn sanitize_rejects_css_variable_values_with_html_escape_and_backslash() {
        let mut doc = base_document();
        doc.css_variables
            .insert("--theme-custom-lt".to_string(), "</style>".to_string());
        doc.css_variables.insert(
            "--theme-custom-bs".to_string(),
            "url\\28evil\\29".to_string(),
        );
        doc.css_variables
            .insert("--theme-custom-ok".to_string(), "#7c3aed".to_string());
        sanitize_document(&mut doc).unwrap();
        assert!(!doc.css_variables.contains_key("--theme-custom-lt"));
        assert!(!doc.css_variables.contains_key("--theme-custom-bs"));
        assert!(doc.css_variables.contains_key("--theme-custom-ok"));
        assert!(
            doc.warnings
                .iter()
                .filter(|w| w.contains("structural chars"))
                .count()
                >= 2
        );
    }
}
