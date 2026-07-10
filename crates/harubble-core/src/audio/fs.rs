//! 音频文件写盘、原子替换与磁盘空间检查工具。
//!
//! 核心能力：[`save_audio`]（将音频字节写盘并按需执行 WAV→FLAC 转码）、
//! [`write_file_atomically`]（通过临时文件原子替换避免写盘中断留下残缺文件）、
//! [`ensure_available_space`]（写盘前磁盘空间预检）。

use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::format::{AudioFormat, OutputFormat};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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
        let cursor = std::io::Cursor::new(data);
        let mut reader = hound::WavReader::new(cursor).context("Failed to read WAV data")?;
        let spec = reader.spec();

        let samples: Vec<i32> = reader
            .samples::<i32>()
            .collect::<Result<_, _>>()
            .context("Failed to read WAV samples")?;

        let config = flacenc::config::Encoder::default()
            .into_verified()
            .map_err(|e| anyhow::anyhow!("FLAC encoder config error: {:?}", e))?;
        let source = flacenc::source::MemSource::from_samples(
            &samples,
            spec.channels as usize,
            spec.bits_per_sample as usize,
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
