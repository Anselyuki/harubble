//! 音频文件写盘、原子替换与磁盘空间检查工具。
//!
//! 核心能力：[`save_audio`]（将音频字节写盘并按需执行 WAV→FLAC 转码）、
//! [`write_file_atomically`]（通过临时文件原子替换避免写盘中断留下残缺文件）、
//! [`ensure_available_space`]（写盘前磁盘空间预检）。

use anyhow::{Context, Result};
use std::borrow::Cow;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::format::{AudioFormat, OutputFormat};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);
const FLOAT_FLAC_BITS_PER_SAMPLE: usize = 24;
const FLOAT_FLAC_SCALE: f64 = (1_u32 << (FLOAT_FLAC_BITS_PER_SAMPLE - 1)) as f64;

struct TpdfDither {
    state: u32,
}

impl TpdfDither {
    fn new() -> Self {
        Self { state: 0x6D2B_79F5 }
    }

    fn uniform(&mut self) -> f64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.state = value;
        value as f64 / (u32::MAX as f64 + 1.0)
    }

    fn next(&mut self) -> f64 {
        self.uniform() - self.uniform()
    }
}

fn quantize_float_sample(sample: f32, dither: &mut TpdfDither) -> Result<i32> {
    anyhow::ensure!(sample.is_finite(), "WAV contains a non-finite float sample");

    let clamped = f64::from(sample).clamp(-1.0, 1.0);
    if clamped <= -1.0 {
        return Ok(-(1_i32 << (FLOAT_FLAC_BITS_PER_SAMPLE - 1)));
    }
    if clamped >= 1.0 {
        return Ok((1_i32 << (FLOAT_FLAC_BITS_PER_SAMPLE - 1)) - 1);
    }

    let quantized = (clamped * FLOAT_FLAC_SCALE + dither.next()).round() as i32;
    Ok(quantized.clamp(
        -(1_i32 << (FLOAT_FLAC_BITS_PER_SAMPLE - 1)),
        (1_i32 << (FLOAT_FLAC_BITS_PER_SAMPLE - 1)) - 1,
    ))
}

fn normalize_oversized_ieee_float_fmt_chunk(data: &[u8]) -> Result<Cow<'_, [u8]>> {
    if data.len() < 12 || &data[..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Ok(Cow::Borrowed(data));
    }

    let mut chunk_offset = 12_usize;
    while chunk_offset.saturating_add(8) <= data.len() {
        let chunk_len = u32::from_le_bytes(
            data[chunk_offset + 4..chunk_offset + 8]
                .try_into()
                .expect("chunk length slice"),
        ) as usize;
        let payload_offset = chunk_offset + 8;
        let payload_end = payload_offset
            .checked_add(chunk_len)
            .context("WAV chunk length overflow")?;
        anyhow::ensure!(payload_end <= data.len(), "WAV chunk exceeds file length");

        if &data[chunk_offset..chunk_offset + 4] == b"fmt " && chunk_len == 40 {
            anyhow::ensure!(chunk_len >= 18, "WAV fmt chunk is too short");
            let format_tag = u16::from_le_bytes(
                data[payload_offset..payload_offset + 2]
                    .try_into()
                    .expect("format tag slice"),
            );
            let cb_size = u16::from_le_bytes(
                data[payload_offset + 16..payload_offset + 18]
                    .try_into()
                    .expect("cbSize slice"),
            );
            if format_tag == 3 && cb_size == 0 {
                const STANDARD_FLOAT_FMT_LEN: usize = 18;
                let removed_len = chunk_len - STANDARD_FLOAT_FMT_LEN;
                let mut normalized = Vec::with_capacity(data.len() - removed_len);
                normalized.extend_from_slice(&data[..chunk_offset + 4]);
                normalized.extend_from_slice(&(STANDARD_FLOAT_FMT_LEN as u32).to_le_bytes());
                normalized.extend_from_slice(
                    &data[payload_offset..payload_offset + STANDARD_FLOAT_FMT_LEN],
                );
                normalized.extend_from_slice(&data[payload_end..]);
                let riff_len = u32::try_from(normalized.len().saturating_sub(8))
                    .context("normalized WAV is too large")?;
                normalized[4..8].copy_from_slice(&riff_len.to_le_bytes());
                return Ok(Cow::Owned(normalized));
            }
        }

        let padded_len = chunk_len
            .checked_add(chunk_len % 2)
            .context("WAV padded chunk length overflow")?;
        chunk_offset = payload_offset
            .checked_add(padded_len)
            .context("WAV chunk offset overflow")?;
    }

    Ok(Cow::Borrowed(data))
}

/// 将音频字节写入磁盘，并按需要执行 WAV → FLAC 转码。
pub fn save_audio(
    data: &[u8],
    out_dir: &Path,
    base_name: &str,
    output_format: OutputFormat,
) -> Result<PathBuf> {
    use super::format::sanitize_filename;
    fs::create_dir_all(out_dir)?;
    let detected = AudioFormat::detect(data);
    let safe_name = sanitize_filename(base_name);

    let out_ext = match (detected, output_format) {
        (AudioFormat::Wav, OutputFormat::Flac) => "flac",
        (fmt, _) => fmt.extension(),
    };

    let out_path = out_dir.join(format!("{safe_name}.{out_ext}"));
    ensure_available_space(out_dir, data.len() as u64)?;

    if detected == AudioFormat::Wav && output_format == OutputFormat::Flac {
        use flacenc::component::BitRepr;
        use flacenc::error::Verify;
        let normalized_wav = normalize_oversized_ieee_float_fmt_chunk(data)?;
        let cursor = std::io::Cursor::new(normalized_wav.as_ref());
        let mut reader = hound::WavReader::new(cursor).context("Failed to read WAV data")?;
        let spec = reader.spec();

        let (samples, bits_per_sample) = match spec.sample_format {
            hound::SampleFormat::Int => (
                reader
                    .samples::<i32>()
                    .collect::<Result<Vec<_>, _>>()
                    .context("Failed to read integer WAV samples")?,
                spec.bits_per_sample as usize,
            ),
            hound::SampleFormat::Float => {
                let mut dither = TpdfDither::new();
                let samples = reader
                    .samples::<f32>()
                    .enumerate()
                    .map(|(index, sample)| {
                        let sample = sample.context("Failed to read float WAV sample")?;
                        quantize_float_sample(sample, &mut dither).with_context(|| {
                            format!("Failed to quantize float WAV sample at index {index}")
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                (samples, FLOAT_FLAC_BITS_PER_SAMPLE)
            }
        };

        let config = flacenc::config::Encoder::default()
            .into_verified()
            .map_err(|e| anyhow::anyhow!("FLAC encoder config error: {:?}", e))?;
        let source = flacenc::source::MemSource::from_samples(
            &samples,
            spec.channels as usize,
            bits_per_sample,
            spec.sample_rate as usize,
        );
        let flac_stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
            .map_err(|e| anyhow::anyhow!("FLAC encoding failed: {:?}", e))?;
        let mut sink = flacenc::bitsink::ByteSink::new();
        flac_stream
            .write(&mut sink)
            .map_err(|e| anyhow::anyhow!("FLAC write failed: {:?}", e))?;

        write_file_atomically(&out_path, sink.as_slice()).context("Failed to write FLAC file")?;
    } else {
        write_file_atomically(&out_path, data).context("Failed to write audio file")?;
    }

    Ok(out_path)
}

/// 确认指定目录所在文件系统至少还有给定字节数的可用空间。
pub fn ensure_available_space(dir: &Path, required_bytes: u64) -> Result<()> {
    let available = fs2::available_space(dir)
        .with_context(|| format!("Failed to query available disk space for {}", dir.display()))?;
    anyhow::ensure!(
        available >= required_bytes,
        "Insufficient disk space: required {required_bytes} bytes, available {available} bytes"
    );
    Ok(())
}

/// 将字节先写入同目录临时文件，成功同步后再替换最终路径。
pub fn write_file_atomically(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    ensure_available_space(parent, data.len() as u64)?;

    let temp_path = unique_temp_path(path);
    let write_result = (|| -> Result<()> {
        let mut temp = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .with_context(|| format!("Failed to create temp file {}", temp_path.display()))?;
        temp.write_all(data)
            .with_context(|| format!("Failed to write temp file {}", temp_path.display()))?;
        temp.sync_all()
            .with_context(|| format!("Failed to sync temp file {}", temp_path.display()))?;
        drop(temp);
        replace_with_temp(&temp_path, path)
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

pub(super) fn replace_with_temp(temp_path: &Path, final_path: &Path) -> Result<()> {
    match fs::rename(temp_path, final_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            fs::remove_file(final_path).with_context(|| {
                format!("Failed to replace existing file {}", final_path.display())
            })?;
            fs::rename(temp_path, final_path).with_context(|| {
                format!(
                    "Failed to move temp file {} to {}",
                    temp_path.display(),
                    final_path.display()
                )
            })
        }
        Err(error) => Err(error).with_context(|| {
            format!(
                "Failed to move temp file {} to {}",
                temp_path.display(),
                final_path.display()
            )
        }),
    }
}

pub(super) fn unique_temp_path(path: &Path) -> PathBuf {
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("harubble-audio");

    path.with_file_name(format!(
        ".{file_name}.{}.{}.part",
        std::process::id(),
        timestamp.saturating_add(counter as u128)
    ))
}
