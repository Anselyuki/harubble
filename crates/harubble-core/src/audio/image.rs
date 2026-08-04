//! 封面图片 MIME 类型识别与 JPEG 编码工具。
//!
//! 提供 [`detect_image_mime`]（根据魔数推断图片 MIME 类型）与 [`encode_cover_as_jpeg`]
//! （将封面统一转换为 JPEG 以提升 FLAC 播放器兼容性）。

use anyhow::{Context, Result};
use image::codecs::jpeg::JpegEncoder;

/// 根据图片魔数推断 MIME 类型。
///
/// # 示例
///
/// ```
/// use harubble_core::audio::detect_image_mime;
///
/// assert_eq!(detect_image_mime(&[0xFF, 0xD8, 0xFF, 0x00]), Some("image/jpeg"));
/// assert_eq!(detect_image_mime(b"GIF89a"), Some("image/gif"));
/// ```
pub fn detect_image_mime(data: &[u8]) -> Option<&'static str> {
    if data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WEBP") {
        Some("image/webp")
    } else {
        None
    }
}

/// 将嵌入封面统一编码为 JPEG，以提升 FLAC 播放器兼容性。
pub fn encode_cover_as_jpeg(data: &[u8]) -> Result<Vec<u8>> {
    if detect_image_mime(data) == Some("image/jpeg") {
        return Ok(data.to_vec());
    }

    let image = image::load_from_memory(data).context("Failed to decode cover image")?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut rgb = Vec::with_capacity((width as usize) * (height as usize) * 3);

    for pixel in rgba.pixels() {
        let [red, green, blue, alpha] = pixel.0;
        let alpha = alpha as u16;
        let inv_alpha = 255_u16.saturating_sub(alpha);
        rgb.push(((red as u16 * alpha + 255 * inv_alpha) / 255) as u8);
        rgb.push(((green as u16 * alpha + 255 * inv_alpha) / 255) as u8);
        rgb.push(((blue as u16 * alpha + 255 * inv_alpha) / 255) as u8);
    }

    let mut jpeg = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut jpeg, 92);
    encoder
        .encode(&rgb, width, height, image::ColorType::Rgb8.into())
        .context("Failed to encode cover image as JPEG")?;
    Ok(jpeg)
}
