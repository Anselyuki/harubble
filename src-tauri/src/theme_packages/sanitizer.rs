//! 主题包字段级 sanitizer（Phase 1 遗留 · A）。
//!
//! # 模块职责
//!
//! 承担主题包 JSON 反序列化后的字段级校验与清洗：
//!
//! - **颜色 slot 白名单**：只允许 `#RRGGBB` / `#RRGGBBAA` hex 与 `rgb()` / `rgba()` 三种形式
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

/// 主题包 slot 白名单（Phase 1 MVP 与 `ThemeColorSlots` 完全对齐）。
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

/// 判断字符串是否合法的颜色值（hex / rgb / rgba）。
///
/// 允许：
/// - `#[0-9a-fA-F]{3,4}` / `#[0-9a-fA-F]{6,8}`（3/4/6/8 字符）
/// - `rgb(r, g, b)` / `rgba(r, g, b, a)`（数值范围检查交给浏览器，本层仅做形态校验）
pub(crate) fn is_valid_color_value(raw: &str) -> bool {
    let s = raw.trim();
    if s.is_empty() {
        return false;
    }
    // 黑名单短路：任何危险关键字直接拒
    let lower = s.to_ascii_lowercase();
    for bad in CSS_BLACKLIST {
        if lower.contains(bad) {
            return false;
        }
    }
    // hex 形式
    if let Some(rest) = s.strip_prefix('#') {
        let len = rest.len();
        return (len == 3 || len == 4 || len == 6 || len == 8)
            && rest.chars().all(|c| c.is_ascii_hexdigit());
    }
    // rgb() / rgba() 形式
    if lower.starts_with("rgb(") || lower.starts_with("rgba(") {
        let open = s.find('(').unwrap_or(0);
        let close = s.rfind(')');
        if close != Some(s.len() - 1) {
            return false;
        }
        let inner = &s[open + 1..s.len() - 1];
        let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
        if parts.len() < 3 || parts.len() > 4 {
            return false;
        }
        // 前三个数值 0-255，第四个 0-1（若存在）
        for (i, p) in parts.iter().enumerate() {
            if i < 3 {
                if p.parse::<u16>().map(|n| n <= 255).unwrap_or(false) {
                    continue;
                }
                return false;
            } else if let Ok(a) = p.parse::<f32>() {
                if !(0.0..=1.0).contains(&a) {
                    return false;
                }
            } else {
                return false;
            }
        }
        return true;
    }
    false
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
        if !is_valid_color_value(value) {
            warnings.push(format!(
                "{scope}: slot '{key}' has invalid color value, dropped"
            ));
            continue;
        }
        cleaned.insert(key.clone(), value.trim().to_string());
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

/// 完整清洗一份主题包文档（in-place）。
///
/// 返回 `Err` 表示遭遇不可恢复的完整性错误（schemaVersion / manifest.id 缺失等）；
/// 否则文档被就地修补，`document.warnings` 累积所有降级说明。
pub(crate) fn sanitize_document(document: &mut ThemePackageDocument) -> Result<(), String> {
    // 硬门槛：schemaVersion / id 缺失直接拒（不 warn）
    if document.schema_version != 1 {
        return Err(format!(
            "unsupported theme package schemaVersion: {}",
            document.schema_version
        ));
    }
    if document.manifest.id.trim().is_empty() {
        return Err("theme package id must be non-empty".to_string());
    }
    if document.manifest.name.trim().is_empty() {
        return Err("theme package name must be non-empty".to_string());
    }
    if document.manifest.version.trim().is_empty() {
        return Err("theme package version must be non-empty".to_string());
    }

    let mut warnings: Vec<String> = std::mem::take(&mut document.warnings);

    // manifest 长度截断
    clamp_text("id", &mut document.manifest.id, MAX_ID_LEN, &mut warnings);
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

    document.warnings = warnings;
    Ok(())
}

/// 校验单个 elevation box-shadow 字符串。
///
/// - 长度超上限 → 截断
/// - 命中 CSS 黑名单（url/expression/javascript/等）→ 直接置 None 并 warn
/// - 允许多层逗号分隔的 shadow；不做 CSS AST 层校验（保持 warn-而非-reject 语义）
/// 清洗 visualContract 单字段：长度 ≤ 32 + 允许字符集 [a-z0-9\-_]，非法则清空并 warn。
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme_packages::types::{
        ThemePackageBlur, ThemePackageDensity, ThemePackageElevation, ThemePackageManifest,
        ThemePackageMotion, ThemePackageShape, ThemePackageVariants, ThemePackageVisualContract,
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
            warnings: Vec::new(),
        }
    }

    #[test]
    fn is_valid_color_value_accepts_hex_rgb_rgba() {
        assert!(is_valid_color_value("#fff"));
        assert!(is_valid_color_value("#abcd"));
        assert!(is_valid_color_value("#7c3aed"));
        assert!(is_valid_color_value("#7c3aedff"));
        assert!(is_valid_color_value("rgb(255, 0, 0)"));
        assert!(is_valid_color_value("rgba(0, 0, 0, 0.5)"));
    }

    #[test]
    fn is_valid_color_value_rejects_invalid_forms() {
        assert!(!is_valid_color_value("red"));
        assert!(!is_valid_color_value("#zz"));
        assert!(!is_valid_color_value("#12345"));
        assert!(!is_valid_color_value("rgb(300, 0, 0)"));
        assert!(!is_valid_color_value("rgba(0,0,0,2.5)"));
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
}
