use anyhow::{Context, Result};
use image::{imageops::FilterType, RgbaImage};
use serde::Serialize;
use std::collections::HashMap;

const DEFAULT_ACCENT_RGB: [u8; 3] = [250, 45, 72];
const MIN_ALPHA: u8 = 96;
const SAMPLE_SIZE: u32 = 64;
const QUANT_STEP: u8 = 24;

/// 从专辑封面中提取出的主题强调色集合。
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

/// 从原始图片字节中提取可用于界面的强调色方案。
///
/// 该算法会先对图片降采样、忽略近乎透明的像素，偏向选择饱和度较高的中间色，
/// 再对结果做对比度归一化，以保证在应用的浅色播放器表面上仍具备可读性。
pub fn extract_theme_palette(bytes: &[u8]) -> Result<ThemePalette> {
    let image = image::load_from_memory(bytes)
        .context("Failed to decode album artwork")?
        .to_rgba8();
    let sampled = image::imageops::resize(&image, SAMPLE_SIZE, SAMPLE_SIZE, FilterType::Triangle);
    let accent_rgb = select_accent_color(&sampled).unwrap_or(DEFAULT_ACCENT_RGB);
    let accent_hover_rgb = derive_hover_color(accent_rgb);

    Ok(ThemePalette {
        accent_hex: rgb_to_hex(accent_rgb),
        accent_hover_hex: rgb_to_hex(accent_hover_rgb),
        accent_rgb,
        accent_hover_rgb,
    })
}

fn select_accent_color(image: &RgbaImage) -> Option<[u8; 3]> {
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

    let selected = buckets
        .values()
        .max_by(|left, right| left.weight.total_cmp(&right.weight))
        .and_then(BucketAccumulator::average_rgb)
        .or_else(|| fallback.average_rgb())?;

    Some(normalize_accent(selected))
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
