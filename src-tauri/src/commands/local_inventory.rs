//! 本地库存扫描、快照管理与音频元数据读取相关的 Tauri command。
//!
//! 当前暴露的接口覆盖库存快照读取、重新扫描、扫描取消与本地音频文件技术元数据提取，
//! 主要用于前端同步本地文件存在性、校验状态、扫描进度与音频详情展示。

use crate::app_state::AppState;
use crate::local_inventory::{emit_local_inventory_state_changed, spawn_inventory_scan};
use harubble_core::{
    candidate_relative_paths, AudioFormat, LocalInventorySnapshot, VerificationMode,
};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, State};

/// 获取当前本地库存扫描快照。
///
/// 适用于本地库存面板初始化、页面恢复后同步状态，或在事件流缺失时兜底拉取当前扫描结果。
/// 返回值为当前最新的本地库存快照。
/// 该接口不会主动启动扫描；若需要刷新结果，应显式调用重新扫描接口。
#[tauri::command]
pub async fn get_local_inventory_snapshot(
    state: State<'_, AppState>,
) -> Result<LocalInventorySnapshot, String> {
    Ok(state.local_inventory_service.snapshot().await)
}

/// 以当前输出目录重新触发本地库存扫描。
///
/// 适用于用户手动刷新本地库存、切换校验模式，或在下载目录内容发生变化后重建库存快照。
/// 入参 `verification_mode` 为可选校验模式覆盖；返回值为触发扫描当下的最新快照。
/// 该接口会异步启动扫描流程，返回时不代表扫描已经完成；调用方应结合后续事件或再次读取快照观察最终结果。
#[tauri::command]
pub async fn rescan_local_inventory(
    app: AppHandle,
    state: State<'_, AppState>,
    verification_mode: Option<VerificationMode>,
) -> Result<LocalInventorySnapshot, String> {
    let root_output_dir = state.preferences().output_dir;
    spawn_inventory_scan(
        app,
        state.inner().clone(),
        root_output_dir,
        verification_mode,
    );
    Ok(state.local_inventory_service.snapshot().await)
}

/// 取消当前进行中的本地库存扫描，并返回最新快照。
///
/// 适用于用户主动中止耗时扫描，或在即将切换目录/退出页面前停止当前扫描任务。
/// 返回值为取消动作之后的最新快照。
/// 该接口只会影响当前进行中的扫描；若此时没有活动扫描，返回的快照可能仅表示当前状态未发生变化。
#[tauri::command]
pub async fn cancel_local_inventory_scan(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LocalInventorySnapshot, String> {
    let snapshot = state.local_inventory_service.cancel_scan().await;
    emit_local_inventory_state_changed(&app, &snapshot);
    Ok(snapshot)
}

/// 本地音频文件的技术元数据。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFileMetadata {
    /// 音频格式名称（FLAC / MP3 / WAV）。
    pub format: String,
    /// 采样率，单位 Hz。
    pub sample_rate: u32,
    /// 声道数。
    pub channels: u16,
    /// 位深（FLAC/WAV 可获取，MP3 为 None）。
    pub bits_per_sample: Option<u16>,
    /// 音频时长，单位秒。
    pub duration_secs: f64,
    /// 码率，单位 kbps（由文件大小与时长计算）。
    pub bitrate_kbps: Option<u32>,
    /// 文件大小，单位字节。
    pub file_size: u64,
}

/// 读取本地已下载歌曲的音频技术元数据。
///
/// 适用于歌曲详情面板展示采样率、码率、位深等音频技术信息。
/// 入参 `album_name` 与 `song_name` 用于定位本地文件（与库存扫描使用相同的路径规则）。
/// 若文件不存在或无法解析，返回 `None`。
#[tauri::command]
pub async fn get_audio_metadata(
    state: State<'_, AppState>,
    album_name: String,
    song_name: String,
) -> Result<Option<AudioFileMetadata>, String> {
    let output_dir = state.output_dir();
    tokio::task::spawn_blocking(move || read_audio_metadata(&output_dir, &album_name, &song_name))
        .await
        .map_err(|e| e.to_string())?
}

fn read_audio_metadata(
    output_dir: &str,
    album_name: &str,
    song_name: &str,
) -> Result<Option<AudioFileMetadata>, String> {
    let root = Path::new(output_dir);

    if !root.exists() {
        return Ok(None);
    }

    let candidates = candidate_relative_paths(album_name, song_name);
    let file_path = candidates
        .iter()
        .map(|rel| root.join(rel))
        .find(|p| p.is_file());

    let Some(path) = file_path else {
        return Ok(None);
    };

    let file_size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();

    let detected = {
        let mut buf = [0u8; 12];
        let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
        std::io::Read::read(&mut f, &mut buf).map_err(|e| e.to_string())?;
        AudioFormat::detect(&buf)
    };

    let meta = match detected {
        AudioFormat::Flac => read_flac_metadata(&path, file_size)?,
        AudioFormat::Wav => read_wav_metadata(&path, file_size)?,
        AudioFormat::Mp3 => read_mp3_metadata(&path, file_size)?,
        AudioFormat::Unknown => return Ok(None),
    };

    Ok(Some(meta))
}

fn read_flac_metadata(path: &Path, file_size: u64) -> Result<AudioFileMetadata, String> {
    let tag = metaflac::Tag::read_from_path(path).map_err(|e| e.to_string())?;
    let stream_info = tag
        .get_streaminfo()
        .ok_or_else(|| "FLAC missing StreamInfo".to_string())?;

    let sample_rate = stream_info.sample_rate;
    let channels = stream_info.num_channels as u16;
    let bits_per_sample = stream_info.bits_per_sample as u16;
    let duration_secs = if sample_rate > 0 {
        stream_info.total_samples as f64 / sample_rate as f64
    } else {
        0.0
    };
    let bitrate_kbps = if duration_secs > 0.0 {
        Some((file_size as f64 * 8.0 / duration_secs / 1000.0) as u32)
    } else {
        None
    };

    Ok(AudioFileMetadata {
        format: "FLAC".to_string(),
        sample_rate,
        channels,
        bits_per_sample: Some(bits_per_sample),
        duration_secs,
        bitrate_kbps,
        file_size,
    })
}

fn read_wav_metadata(path: &Path, file_size: u64) -> Result<AudioFileMetadata, String> {
    let reader = hound::WavReader::open(path).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let duration_secs = if spec.sample_rate > 0 {
        reader.duration() as f64 / spec.sample_rate as f64
    } else {
        0.0
    };
    let bitrate_kbps = if duration_secs > 0.0 {
        Some((file_size as f64 * 8.0 / duration_secs / 1000.0) as u32)
    } else {
        None
    };

    Ok(AudioFileMetadata {
        format: "WAV".to_string(),
        sample_rate: spec.sample_rate,
        channels: spec.channels,
        bits_per_sample: Some(spec.bits_per_sample),
        duration_secs,
        bitrate_kbps,
        file_size,
    })
}

fn read_mp3_metadata(path: &Path, file_size: u64) -> Result<AudioFileMetadata, String> {
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    hint.with_extension("mp3");

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| e.to_string())?;

    let track = probed
        .format
        .default_track()
        .ok_or_else(|| "No audio track found".to_string())?;

    let codec_params = &track.codec_params;
    let sample_rate = codec_params.sample_rate.unwrap_or(0);
    let channels = codec_params
        .channels
        .map(|ch| ch.count() as u16)
        .unwrap_or(0);
    let duration_secs = match (codec_params.n_frames, codec_params.time_base) {
        (Some(n_frames), Some(time_base)) => {
            let d = time_base.calc_time(n_frames);
            d.seconds as f64 + d.frac
        }
        _ => {
            if sample_rate > 0 {
                file_size as f64 * 8.0 / (sample_rate as f64 * channels.max(1) as f64 * 16.0)
            } else {
                0.0
            }
        }
    };
    let bitrate_kbps = if duration_secs > 0.0 {
        Some((file_size as f64 * 8.0 / duration_secs / 1000.0) as u32)
    } else {
        None
    };

    Ok(AudioFileMetadata {
        format: "MP3".to_string(),
        sample_rate,
        channels,
        bits_per_sample: None,
        duration_secs,
        bitrate_kbps,
        file_size,
    })
}
