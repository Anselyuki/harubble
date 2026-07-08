use anyhow::{Context, Result};
use image::{imageops::FilterType, RgbaImage};
use serde::Serialize;
use std::collections::HashMap;

const DEFAULT_ACCENT_RGB: [u8; 3] = [250, 45, 72];
const MIN_ALPHA: u8 = 96;
const SAMPLE_SIZE: u32 = 64;
const QUANT_STEP: u8 = 24;

/// 从专辑封面中提取出的完整主题色卡。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemePalette {
    /// 主强调色的十六进制表示，例如 `#fa2d48`。
    pub accent_hex: String,
    /// 基于主强调色推导出的悬停态颜色。
    pub accent_hover_hex: String,
    /// 主强调色的 RGB 三通道字节值。
    pub accent_rgb: [u8; 3],
    /// 悬停态强调色的 RGB 三通道字节值。
    pub accent_hover_rgb: [u8; 3],
    /// 从封面中提取的多个代表色（用于波形可视化等场景），最多 4 个。
    pub wave_colors: Vec<[u8; 3]>,
    /// 表面/背景底色（十六进制）。
    pub surface_hex: String,
    /// 主要文本色（十六进制）。
    pub text_primary_hex: String,
    /// 次要文本色（十六进制）。
    pub text_secondary_hex: String,
    /// 淡色强调/辅助色（十六进制）。
    pub tint_hex: String,
    /// 危险/警告色（十六进制）。
    pub danger_hex: String,
}

#[derive(Clone, Copy, Default)]
struct BucketAccumulator {
    weight: f32,
    r_sum: f32,
    g_sum: f32,
    b_sum: f32,
}

impl BucketAccumulator {
    fn add(&mut self, rgb: [u8; 3], weight: f32) {
        self.weight += weight;
        self.r_sum += rgb[0] as f32 * weight;
        self.g_sum += rgb[1] as f32 * weight;
        self.b_sum += rgb[2] as f32 * weight;
    }

    fn average_rgb(&self) -> Option<[u8; 3]> {
        if self.weight <= f32::EPSILON {
            return None;
        }

        Some([
            (self.r_sum / self.weight).round().clamp(0.0, 255.0) as u8,
            (self.g_sum / self.weight).round().clamp(0.0, 255.0) as u8,
            (self.b_sum / self.weight).round().clamp(0.0, 255.0) as u8,
        ])
    }
}

/// 从原始图片字节中提取可用于界面的完整 6 槽色卡。
///
/// 该算法会先对图片降采样、忽略近乎透明的像素，偏向选择饱和度较高的中间色，
/// 再对结果做对比度归一化。从提取的色桶中推导出语义化的 6 色色卡：
/// accent（主强调）、surface（背景底色）、textPrimary（深色文本）、
/// textSecondary（浅色文本）、tint（辅助色调）、danger（暖警告色）。
pub fn extract_theme_palette(bytes: &[u8]) -> Result<ThemePalette> {
    let image = image::load_from_memory(bytes)
        .context("Failed to decode album artwork")?
        .to_rgba8();
    let sampled = image::imageops::resize(&image, SAMPLE_SIZE, SAMPLE_SIZE, FilterType::Triangle);
    let (accent_rgb, wave_colors, sorted_colors) = select_colors_extended(&sampled);
    let accent_rgb = accent_rgb.unwrap_or(DEFAULT_ACCENT_RGB);
    let accent_hover_rgb = derive_hover_color(accent_rgb);

    let slots = derive_full_slots(accent_rgb, &sorted_colors);

    Ok(ThemePalette {
        accent_hex: rgb_to_hex(accent_rgb),
        accent_hover_hex: rgb_to_hex(accent_hover_rgb),
        accent_rgb,
        accent_hover_rgb,
        wave_colors,
        surface_hex: rgb_to_hex(slots.surface),
        text_primary_hex: rgb_to_hex(slots.text_primary),
        text_secondary_hex: rgb_to_hex(slots.text_secondary),
        tint_hex: rgb_to_hex(slots.tint),
        danger_hex: rgb_to_hex(slots.danger),
    })
}

const WAVE_COLOR_COUNT: usize = 4;

struct DerivedSlots {
    surface: [u8; 3],
    text_primary: [u8; 3],
    text_secondary: [u8; 3],
    tint: [u8; 3],
    danger: [u8; 3],
}

fn select_colors_extended(image: &RgbaImage) -> (Option<[u8; 3]>, Vec<[u8; 3]>, Vec<[u8; 3]>) {
    let mut buckets: HashMap<(u8, u8, u8), BucketAccumulator> = HashMap::new();
    let mut fallback = BucketAccumulator::default();

    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;
        if a < MIN_ALPHA {
            continue;
        }

        let rgb = [r, g, b];
        fallback.add(rgb, 1.0);

        let (_, saturation, lightness) = rgb_to_hsl(rgb);
        let luminance = relative_luminance(rgb);
        if saturation < 0.16 || !(0.06..=0.94).contains(&luminance) {
            continue;
        }

        let vibrancy = 0.35 + saturation * 0.9;
        let light_focus = 1.0 - ((lightness - 0.52).abs() * 1.85).min(0.85);
        let weight = vibrancy * (0.45 + light_focus);
        let key = (
            quantize_component(r),
            quantize_component(g),
            quantize_component(b),
        );

        buckets.entry(key).or_default().add(rgb, weight);
    }

    let mut sorted_buckets: Vec<&BucketAccumulator> = buckets.values().collect();
    sorted_buckets.sort_by(|a, b| b.weight.total_cmp(&a.weight));

    let accent = sorted_buckets
        .first()
        .and_then(|b| b.average_rgb())
        .or_else(|| fallback.average_rgb())
        .map(normalize_accent);

    let wave_colors: Vec<[u8; 3]> = sorted_buckets
        .iter()
        .take(WAVE_COLOR_COUNT)
        .filter_map(|b| b.average_rgb())
        .collect();

    let wave_colors = if wave_colors.is_empty() {
        vec![accent.unwrap_or(DEFAULT_ACCENT_RGB)]
    } else {
        wave_colors
    };

    let sorted_colors: Vec<[u8; 3]> = sorted_buckets
        .iter()
        .take(12)
        .filter_map(|b| b.average_rgb())
        .collect();

    (accent, wave_colors, sorted_colors)
}

fn derive_full_slots(accent: [u8; 3], palette_colors: &[[u8; 3]]) -> DerivedSlots {
    let (accent_hue, _, _) = rgb_to_hsl(accent);

    let palette_hsl: Vec<([u8; 3], f32, f32, f32)> = palette_colors
        .iter()
        .map(|&c| {
            let (h, s, l) = rgb_to_hsl(c);
            (c, h, s, l)
        })
        .collect();

    let surface = derive_surface(accent_hue, &palette_hsl);
    let text_primary = derive_text_primary(accent_hue);
    let text_secondary = derive_text_secondary(accent_hue);
    let tint = derive_tint(accent_hue, &palette_hsl);
    let danger = derive_danger(accent_hue, &palette_hsl);

    DerivedSlots {
        surface,
        text_primary,
        text_secondary,
        tint,
        danger,
    }
}

fn derive_surface(accent_hue: f32, palette_hsl: &[([u8; 3], f32, f32, f32)]) -> [u8; 3] {
    for &(color, _, sat, light) in palette_hsl {
        if sat < 0.3 && light > 0.7 {
            return color;
        }
    }
    hsl_to_rgb(accent_hue, 0.08, 0.88)
}

fn derive_text_primary(accent_hue: f32) -> [u8; 3] {
    hsl_to_rgb(accent_hue, 0.08, 0.22)
}

fn derive_text_secondary(accent_hue: f32) -> [u8; 3] {
    hsl_to_rgb(accent_hue, 0.06, 0.38)
}

fn derive_tint(accent_hue: f32, palette_hsl: &[([u8; 3], f32, f32, f32)]) -> [u8; 3] {
    for &(color, hue, sat, light) in palette_hsl.iter().skip(1) {
        let hue_diff = ((hue - accent_hue).abs()).min(1.0 - (hue - accent_hue).abs());
        if hue_diff > 0.05 && sat > 0.15 && (0.4..=0.7).contains(&light) {
            return color;
        }
    }
    hsl_to_rgb(accent_hue, 0.2, 0.6)
}

fn derive_danger(accent_hue: f32, palette_hsl: &[([u8; 3], f32, f32, f32)]) -> [u8; 3] {
    for &(color, hue, sat, light) in palette_hsl {
        let is_warm = hue < 0.08 || hue > 0.92;
        if is_warm && sat > 0.4 && (0.3..=0.55).contains(&light) {
            return color;
        }
    }

    let danger_hue = if (accent_hue - 0.0).abs() < 0.1 || (accent_hue - 1.0).abs() < 0.1 {
        0.08
    } else {
        0.0
    };
    hsl_to_rgb(danger_hue, 0.6, 0.42)
}

fn normalize_accent(rgb: [u8; 3]) -> [u8; 3] {
    let (hue, saturation, lightness) = rgb_to_hsl(rgb);

    let normalized = if saturation < 0.12 {
        hsl_to_rgb(hue, saturation, lightness.clamp(0.26, 0.48))
    } else {
        hsl_to_rgb(
            hue,
            saturation.clamp(0.42, 0.8),
            lightness.clamp(0.32, 0.54),
        )
    };

    ensure_contrast_with_white(normalized, 4.2)
}

fn ensure_contrast_with_white(mut rgb: [u8; 3], min_contrast: f32) -> [u8; 3] {
    let (hue, saturation, mut lightness) = rgb_to_hsl(rgb);

    while contrast_ratio(rgb, [255, 255, 255]) < min_contrast && lightness > 0.18 {
        lightness = (lightness - 0.04).max(0.18);
        rgb = hsl_to_rgb(hue, saturation, lightness);
    }

    rgb
}

fn derive_hover_color(rgb: [u8; 3]) -> [u8; 3] {
    let lighter = mix_rgb(rgb, [255, 255, 255], 0.08);
    if contrast_ratio(lighter, [255, 255, 255]) >= 4.2 {
        lighter
    } else {
        rgb
    }
}

fn quantize_component(value: u8) -> u8 {
    value / QUANT_STEP
}

fn mix_rgb(base: [u8; 3], target: [u8; 3], amount: f32) -> [u8; 3] {
    let amount = amount.clamp(0.0, 1.0);
    [
        mix_channel(base[0], target[0], amount),
        mix_channel(base[1], target[1], amount),
        mix_channel(base[2], target[2], amount),
    ]
}

fn mix_channel(base: u8, target: u8, amount: f32) -> u8 {
    (base as f32 + (target as f32 - base as f32) * amount)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn rgb_to_hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn contrast_ratio(left: [u8; 3], right: [u8; 3]) -> f32 {
    let left_lum = relative_luminance(left);
    let right_lum = relative_luminance(right);
    let (bright, dark) = if left_lum >= right_lum {
        (left_lum, right_lum)
    } else {
        (right_lum, left_lum)
    };

    (bright + 0.05) / (dark + 0.05)
}

fn relative_luminance(rgb: [u8; 3]) -> f32 {
    fn linearize(channel: u8) -> f32 {
        let value = channel as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    let r = linearize(rgb[0]);
    let g = linearize(rgb[1]);
    let b = linearize(rgb[2]);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

fn rgb_to_hsl(rgb: [u8; 3]) -> (f32, f32, f32) {
    let r = rgb[0] as f32 / 255.0;
    let g = rgb[1] as f32 / 255.0;
    let b = rgb[2] as f32 / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let lightness = (max + min) * 0.5;
    let delta = max - min;

    if delta <= f32::EPSILON {
        return (0.0, 0.0, lightness);
    }

    let saturation = delta / (1.0 - (2.0 * lightness - 1.0).abs());
    let hue = if (max - r).abs() <= f32::EPSILON {
        ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() <= f32::EPSILON {
        ((b - r) / delta) + 2.0
    } else {
        ((r - g) / delta) + 4.0
    } / 6.0;

    (hue, saturation, lightness)
}

fn hsl_to_rgb(hue: f32, saturation: f32, lightness: f32) -> [u8; 3] {
    if saturation <= f32::EPSILON {
        let channel = (lightness * 255.0).round().clamp(0.0, 255.0) as u8;
        return [channel, channel, channel];
    }

    let q = if lightness < 0.5 {
        lightness * (1.0 + saturation)
    } else {
        lightness + saturation - lightness * saturation
    };
    let p = 2.0 * lightness - q;

    [
        hue_to_channel(p, q, hue + 1.0 / 3.0),
        hue_to_channel(p, q, hue),
        hue_to_channel(p, q, hue - 1.0 / 3.0),
    ]
}

fn hue_to_channel(p: f32, q: f32, mut t: f32) -> u8 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }

    let value = if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    };

    (value * 255.0).round().clamp(0.0, 255.0) as u8
}
