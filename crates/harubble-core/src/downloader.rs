//! 单曲下载、封面下载与写盘编排工具。
//!
//! 该模块负责围绕单首歌曲或专辑封面执行网络下载、元数据构建、歌词侧车准备与最终
//! 落盘，是下载执行阶段连接上游 API、音频处理与本地文件系统的高层入口。

use crate::api::{AlbumDetail, ApiClient, SongDetail};
use crate::audio::{
    detect_image_mime, encode_cover_as_jpeg, sanitize_filename, save_audio, tag_flac, AudioFormat,
    FlacMetadata, OutputFormat,
};
use crate::download::model::DownloadTaskStatus;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// `FlacMetadata` 的拥有型版本，可安全跨异步边界传递。
///
/// 与借用型的 `FlacMetadata<'a>` 不同，这个结构体持有全部字段所有权，适合
/// 通过 channel 在线程或任务间传递。
#[derive(Debug, Clone)]
pub struct OwnedFlacMetadata {
    pub title: String,
    pub artists: Vec<String>,
    pub album: String,
    pub album_artists: Vec<String>,
    pub track_number: Option<u32>,
    pub total_tracks: Option<u32>,
    pub disc_number: Option<u32>,
    pub total_discs: Option<u32>,
    /// 已编码为 JPEG 的封面字节；没有可嵌入封面时为 `None`。
    pub cover_jpeg: Option<Vec<u8>>,
}

impl OwnedFlacMetadata {
    /// 构造一个借用当前结构体字段的 [`FlacMetadata`] 视图。
    pub fn as_borrowed(&self) -> FlacMetadata<'_> {
        FlacMetadata {
            title: &self.title,
            artists: &self.artists,
            album: &self.album,
            album_artists: &self.album_artists,
            track_number: self.track_number,
            total_tracks: self.total_tracks,
            disc_number: self.disc_number,
            total_discs: self.total_discs,
            cover: self
                .cover_jpeg
                .as_deref()
                .map(|bytes| ("image/jpeg" as &'static str, bytes)),
        }
    }
}

/// 记录一次下载产物来源链路的种子数据。
///
/// 适用于后续构造本地库存来源证明、校验下载是否来自可信上游，或在写盘后为
/// 音频文件补充可追溯元信息。
#[derive(Debug, Clone)]
pub struct DownloadProvenanceSeed {
    /// 本次下载所使用的上游音频源地址。
    pub source_url: String,
    /// 原始音频字节在任何转码或写标签前的 MD5 校验值。
    pub source_audio_checksum: String,
    /// 描述本次写入流水线的稳定指纹，用于区分不同处理路径。
    pub processing_fingerprint: String,
}

/// 下载阶段产出的写入载荷，包含后续落盘所需的全部数据。
///
/// 适用于把网络下载阶段与本地写盘阶段解耦：调用方可以先生成该载荷，再交给
/// 阻塞任务或独立写入线程执行 [`write_payload_to_disk`]。
#[derive(Debug)]
pub struct WritePayload {
    /// 完整的音频字节数据。
    pub audio_bytes: Vec<u8>,
    /// 音频文件写入目录。
    pub output_dir: PathBuf,
    /// 输出文件基础名（不含扩展名）。
    pub base_name: String,
    /// 目标输出格式。
    pub format: OutputFormat,
    /// 仅在输出为 FLAC 时使用的标签元数据。
    pub flac_metadata: Option<OwnedFlacMetadata>,
    /// 已下载的歌词文本。
    pub lyric_text: Option<String>,
    /// 记录可信下载链路来源信息的种子数据。
    pub provenance_seed: DownloadProvenanceSeed,
    /// 与下载阶段共享的取消标志。
    pub cancellation_flag: Option<Arc<AtomicBool>>,
}

/// 下载 FLAC 时可选的标签元数据覆盖项。
/// 空字符串或空数组表示“沿用接口返回值”。
pub struct MetaOverride {
    /// 覆盖写入 FLAC 标签的专辑名。
    pub album_name: String,
    /// 覆盖写入 FLAC 标签的艺术家列表。
    pub artists: Vec<String>,
    /// 覆盖写入 FLAC 标签的专辑艺术家列表。
    pub album_artists: Vec<String>,
}

/// 下载过程中产生的进度信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadStage {
    Downloading,
    Writing,
}

/// 下载过程中产生的进度信息。
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// 当前这条进度对应的歌曲名。
    pub song_name: String,
    /// 当前任务阶段。
    pub status: DownloadTaskStatus,
    /// 底层下载流程阶段。
    pub stage: DownloadStage,
    /// 当前文件已接收的字节数。
    pub bytes_done: u64,
    /// 当前文件的总字节数，未知时为 `None`。
    pub bytes_total: Option<u64>,
    /// 当前歌曲在本批次中的下标，从 0 开始。
    pub song_index: usize,
    /// 本批次的歌曲总数。
    pub song_count: usize,
}

fn lyric_sidecar_path(audio_path: &Path) -> PathBuf {
    audio_path.with_extension("lrc")
}

#[derive(Serialize)]
struct ProcessingFingerprint<'a> {
    output_format: OutputFormat,
    source_format: AudioFormat,
    writes_flac_tags: bool,
    embeds_cover: bool,
    writes_lyric_sidecar: bool,
    base_name: &'a str,
}

fn source_audio_checksum(audio_bytes: &[u8]) -> String {
    format!("{:x}", md5::compute(audio_bytes))
}

fn build_processing_fingerprint(
    source_format: AudioFormat,
    format: OutputFormat,
    base_name: &str,
    flac_metadata: Option<&OwnedFlacMetadata>,
    lyric_text: Option<&str>,
) -> String {
    let writes_flac_tags = flac_metadata.is_some()
        && (source_format == AudioFormat::Flac
            || (source_format == AudioFormat::Wav && format == OutputFormat::Flac));
    let fingerprint = ProcessingFingerprint {
        output_format: format,
        source_format,
        writes_flac_tags,
        embeds_cover: flac_metadata
            .and_then(|metadata| metadata.cover_jpeg.as_ref())
            .is_some(),
        writes_lyric_sidecar: lyric_text.is_some(),
        base_name,
    };
    serde_json::to_string(&fingerprint).unwrap_or_else(|_| {
        format!(
            "{:?}:{:?}:{writes_flac_tags}:{}:{}:{}",
            format,
            source_format,
            fingerprint.embeds_cover,
            fingerprint.writes_lyric_sidecar,
            base_name
        )
    })
}

fn write_lyric_sidecar(audio_path: &Path, lyric_text: &str) -> Result<PathBuf> {
    let lyric_path = lyric_sidecar_path(audio_path);
    std::fs::write(&lyric_path, lyric_text.as_bytes())
        .with_context(|| format!("Failed to write lyric sidecar {}", lyric_path.display()))?;
    Ok(lyric_path)
}

async fn fetch_lyric_text(client: &ApiClient, song: &SongDetail) -> Result<Option<String>> {
    let Some(lyric_url) = song.lyric_url.as_deref().filter(|url| !url.is_empty()) else {
        return Ok(None);
    };

    let lyric_text = client
        .download_text(lyric_url)
        .await
        .with_context(|| format!("Failed to download lyric text for {}", song.name))?;
    if lyric_text.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(lyric_text))
}

/// 下载被取消时返回的类型化错误。
#[derive(Debug)]
pub struct DownloadCancelledError;

impl std::fmt::Display for DownloadCancelledError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "download cancelled")
    }
}

impl std::error::Error for DownloadCancelledError {}

fn ensure_not_cancelled(cancellation_flag: Option<&Arc<AtomicBool>>) -> Result<()> {
    if cancellation_flag
        .map(|flag| flag.load(Ordering::SeqCst))
        .unwrap_or(false)
    {
        return Err(DownloadCancelledError.into());
    }

    Ok(())
}

fn emit_progress(
    on_progress: &(impl Fn(DownloadProgress) + ?Sized),
    song_name: &str,
    stage: DownloadStage,
    bytes_done: u64,
    bytes_total: Option<u64>,
    song_index: usize,
    song_count: usize,
) {
    let status = match stage {
        DownloadStage::Downloading => DownloadTaskStatus::Downloading,
        DownloadStage::Writing => DownloadTaskStatus::Writing,
    };

    on_progress(DownloadProgress {
        song_name: song_name.to_string(),
        status,
        stage,
        bytes_done,
        bytes_total,
        song_index,
        song_count,
    });
}

/// 根据专辑名生成专辑下载目录路径。
pub fn album_output_dir(base_out_dir: &Path, album_name: &str) -> PathBuf {
    base_out_dir.join(sanitize_filename(album_name))
}

fn cover_extension_from_mime(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "jpg",
    }
}

/// 将专辑封面字节写为稳定命名的 `cover.*` 文件。
pub fn write_album_cover_bytes(album_dir: &Path, cover_bytes: &[u8]) -> Result<PathBuf> {
    std::fs::create_dir_all(album_dir)?;

    let mime = detect_image_mime(cover_bytes).unwrap_or("image/jpeg");
    let extension = cover_extension_from_mime(mime);
    let cover_path = album_dir.join(format!("cover.{extension}"));

    std::fs::write(&cover_path, cover_bytes)
        .with_context(|| format!("Failed to write album cover {}", cover_path.display()))?;

    Ok(cover_path)
}

/// 判断专辑目录下是否已经存在封面文件。
pub fn album_cover_exists(album_dir: &Path) -> bool {
    ["jpg", "png", "gif", "webp"]
        .iter()
        .map(|extension| album_dir.join(format!("cover.{extension}")))
        .any(|path| path.exists())
}

/// 下载专辑封面并写入专辑目录。
///
/// 当下载封面失败时返回 `Ok(None)`，以避免把封面失败升级为整首歌下载失败。
pub async fn download_album_cover(
    client: &ApiClient,
    album: &AlbumDetail,
    album_dir: &Path,
    cancellation_flag: Option<&Arc<AtomicBool>>,
) -> Result<Option<PathBuf>> {
    ensure_not_cancelled(cancellation_flag)?;

    let cover_bytes = match client.download_bytes(&album.cover_url, |_, _| {}).await {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };

    ensure_not_cancelled(cancellation_flag)?;

    write_album_cover_bytes(album_dir, &cover_bytes).map(Some)
}

/// 下载单首歌曲到磁盘，并在可能时为 FLAC 文件写入标签。
///
/// `format` 用于指定目标输出格式。除非源文件是 WAV 且目标格式为
/// [`OutputFormat::Flac`]，否则 WAV 和 MP3 会按原始格式直接写出；
/// 如果满足上述条件，则会先用纯 Rust 方案转码为 FLAC，再按需写入
/// FLAC 元数据。
///
/// 下载分块期间持续回调，并在写盘开始前额外上报一次 `Writing` 阶段进度；
/// 回调始终把当前任务视为单文件批次，因此 `song_index = 0`、`song_count = 1`。
///
/// 返回最终写入的文件路径。
///
/// 这是一个便捷封装：它会顺序执行下载阶段 helper 与 [`write_payload_to_disk`]。
/// 如果需要流水线式执行，请分别调用这两个阶段。
#[allow(clippy::too_many_arguments)]
pub async fn download_song(
    client: &ApiClient,
    song: &SongDetail,
    album: &AlbumDetail,
    out_dir: &Path,
    format: OutputFormat,
    download_lyrics: bool,
    meta: &MetaOverride,
    cancellation_flag: Option<Arc<AtomicBool>>,
    on_progress: impl Fn(DownloadProgress) + Send + Sync + 'static,
) -> Result<PathBuf> {
    let progress_fn = Arc::new(on_progress);
    let payload_progress_fn = Arc::clone(&progress_fn);

    let payload = download_song_payload(
        client,
        song,
        album,
        out_dir,
        format,
        download_lyrics,
        meta,
        cancellation_flag,
        move |p| payload_progress_fn(p),
    )
    .await?;

    write_payload_to_disk(&payload, Some(progress_fn.as_ref()))
}

/// 流水线下载的下载阶段：执行封面、歌词与音频的网络拉取，并返回后续写盘
/// 所需的 [`WritePayload`]。
///
/// 该函数只执行网络相关 I/O，不触碰本地文件系统；返回的载荷可以交给独立的
/// 写入线程或阻塞任务去执行 [`write_payload_to_disk`]。
#[allow(clippy::too_many_arguments)]
pub(crate) async fn download_song_payload(
    client: &ApiClient,
    song: &SongDetail,
    album: &AlbumDetail,
    out_dir: &Path,
    format: OutputFormat,
    download_lyrics: bool,
    meta: &MetaOverride,
    cancellation_flag: Option<Arc<AtomicBool>>,
    on_progress: impl Fn(DownloadProgress) + Send + 'static,
) -> Result<WritePayload> {
    ensure_not_cancelled(cancellation_flag.as_ref())?;

    // Fetch cover image (failure is non-fatal)
    let cover_bytes: Option<Vec<u8>> = client
        .download_bytes(&album.cover_url, |_, _| {})
        .await
        .ok();
    let embedded_cover = cover_bytes
        .as_deref()
        .and_then(|bytes| encode_cover_as_jpeg(bytes).ok());

    ensure_not_cancelled(cancellation_flag.as_ref())?;

    // Fetch lyrics
    let lyric_text = if download_lyrics {
        fetch_lyric_text(client, song).await?
    } else {
        None
    };

    let name_for_progress = song.name.clone();
    let progress_fn = Arc::new(on_progress);
    let download_progress_fn = Arc::clone(&progress_fn);
    let cancellation_flag_for_download = cancellation_flag.clone();

    // Stream audio into memory
    let mut audio_bytes = Vec::new();
    client
        .download_stream(&song.source_url, |chunk, done, total| {
            if cancellation_flag_for_download
                .as_ref()
                .map(|flag| flag.load(Ordering::SeqCst))
                .unwrap_or(false)
            {
                return Ok(false);
            }

            audio_bytes.extend_from_slice(chunk);
            emit_progress(
                download_progress_fn.as_ref(),
                &name_for_progress,
                DownloadStage::Downloading,
                done,
                total,
                0,
                1,
            );

            Ok(true)
        })
        .await?;

    ensure_not_cancelled(cancellation_flag.as_ref())?;

    // Build FLAC metadata if applicable
    let flac_metadata = build_owned_flac_metadata(song, album, meta, embedded_cover.as_deref());

    let source_format = AudioFormat::detect(&audio_bytes);
    let provenance_seed = DownloadProvenanceSeed {
        source_url: song.source_url.clone(),
        source_audio_checksum: source_audio_checksum(&audio_bytes),
        processing_fingerprint: build_processing_fingerprint(
            source_format,
            format,
            &song.name,
            flac_metadata.as_ref(),
            lyric_text.as_deref(),
        ),
    };

    Ok(WritePayload {
        audio_bytes,
        output_dir: out_dir.to_path_buf(),
        base_name: song.name.clone(),
        format,
        flac_metadata,
        lyric_text,
        provenance_seed,
        cancellation_flag,
    })
}

/// 流水线下载的第二阶段：将音频写入磁盘、写入 FLAC 标签，并按需写入歌词侧车
/// 文件。
///
/// 该函数只执行本地文件系统 I/O，适合运行在阻塞任务或独立写入线程中。
///
/// 如果提供 `on_progress`，会在写入开始前以 `DownloadStage::Writing` 上报一次。
pub fn write_payload_to_disk(
    payload: &WritePayload,
    on_progress: Option<&dyn Fn(DownloadProgress)>,
) -> Result<PathBuf> {
    ensure_not_cancelled(
        payload
            .cancellation_flag
            .as_ref()
            .map(|f| f as &Arc<AtomicBool>),
    )?;

    if let Some(progress_fn) = on_progress {
        emit_progress(
            progress_fn,
            &payload.base_name,
            DownloadStage::Writing,
            payload.audio_bytes.len() as u64,
            Some(payload.audio_bytes.len() as u64),
            0,
            1,
        );
    }

    let out_path = save_audio(
        &payload.audio_bytes,
        &payload.output_dir,
        &payload.base_name,
        payload.format,
    )
    .with_context(|| format!("Failed to save audio file for {}", payload.base_name))?;

    // Tag FLAC if applicable
    if let Some(ref flac_meta) = payload.flac_metadata {
        let detected = AudioFormat::detect(&payload.audio_bytes);
        let is_flac_output = detected == AudioFormat::Flac
            || (detected == AudioFormat::Wav && payload.format == OutputFormat::Flac);

        if is_flac_output && out_path.extension().and_then(|e| e.to_str()) == Some("flac") {
            ensure_not_cancelled(
                payload
                    .cancellation_flag
                    .as_ref()
                    .map(|f| f as &Arc<AtomicBool>),
            )?;
            tag_flac(&out_path, &flac_meta.as_borrowed()).with_context(|| {
                format!("Failed to write FLAC metadata for {}", payload.base_name)
            })?;
        }
    }

    // Write lyric sidecar
    if let Some(ref lyric_text) = payload.lyric_text {
        ensure_not_cancelled(
            payload
                .cancellation_flag
                .as_ref()
                .map(|f| f as &Arc<AtomicBool>),
        )?;
        write_lyric_sidecar(&out_path, lyric_text)
            .with_context(|| format!("Failed to save lyric sidecar for {}", payload.base_name))?;
    }

    Ok(out_path)
}

/// 根据歌曲、专辑信息与可选封面字节构造拥有型 FLAC 元数据。
fn build_owned_flac_metadata(
    song: &SongDetail,
    album: &AlbumDetail,
    meta: &MetaOverride,
    embedded_cover: Option<&[u8]>,
) -> Option<OwnedFlacMetadata> {
    let eff_album = if meta.album_name.is_empty() {
        album.name.clone()
    } else {
        meta.album_name.clone()
    };
    let eff_artists = if meta.artists.is_empty() {
        song.artists.clone()
    } else {
        meta.artists.clone()
    };
    let eff_album_artists = if meta.album_artists.is_empty() {
        album
            .artists
            .as_deref()
            .filter(|artists| !artists.is_empty())
            .unwrap_or(&eff_artists)
            .to_vec()
    } else {
        meta.album_artists.clone()
    };
    let track_number = album
        .songs
        .iter()
        .position(|entry| entry.cid == song.cid)
        .map(|index| (index + 1) as u32);
    let total_tracks = (!album.songs.is_empty()).then_some(album.songs.len() as u32);

    Some(OwnedFlacMetadata {
        title: song.name.clone(),
        artists: eff_artists,
        album: eff_album,
        album_artists: eff_album_artists,
        track_number,
        total_tracks,
        disc_number: Some(1),
        total_discs: Some(1),
        cover_jpeg: embedded_cover.map(|b| b.to_vec()),
    })
}

/// 下载整张专辑到 `out_dir/<album_name>/` 目录下。
///
/// 会在每个文件的每个下载分块后调用 `on_progress`，并按专辑曲序返回
/// 最终写入的文件路径列表。
#[allow(dead_code)]
pub(crate) async fn download_album(
    client: &ApiClient,
    album_cid: &str,
    base_out_dir: &Path,
    format: OutputFormat,
    download_lyrics: bool,
    on_progress: impl Fn(DownloadProgress) + Send + Clone + 'static,
) -> Result<Vec<PathBuf>> {
    let album = client.get_album_detail(album_cid).await?;
    let song_count = album.songs.len();

    let album_dir = album_output_dir(base_out_dir, &album.name);
    std::fs::create_dir_all(&album_dir)?;

    let cover_bytes: Option<Vec<u8>> = client
        .download_bytes(&album.cover_url, |_, _| {})
        .await
        .ok();
    if let Some(cover_bytes) = cover_bytes.as_deref() {
        let _ = write_album_cover_bytes(&album_dir, cover_bytes);
    }
    let embedded_cover = cover_bytes
        .as_deref()
        .and_then(|bytes| encode_cover_as_jpeg(bytes).ok());

    let mut paths = Vec::new();

    for (idx, song_entry) in album.songs.iter().enumerate() {
        let song_detail = client.get_song_detail(&song_entry.cid).await?;
        let lyric_text = if download_lyrics {
            fetch_lyric_text(client, &song_detail).await?
        } else {
            None
        };
        let progress_fn = on_progress.clone();
        let song_name = song_detail.name.clone();

        let audio_bytes = client
            .download_bytes(&song_detail.source_url, move |done, total| {
                emit_progress(
                    &progress_fn,
                    &song_name,
                    DownloadStage::Downloading,
                    done,
                    total,
                    idx,
                    song_count,
                );
            })
            .await?;

        emit_progress(
            &on_progress,
            &song_detail.name,
            DownloadStage::Writing,
            audio_bytes.len() as u64,
            Some(audio_bytes.len() as u64),
            idx,
            song_count,
        );

        let out_path = save_audio(&audio_bytes, &album_dir, &song_detail.name, format)
            .with_context(|| format!("Failed to save audio file for {}", song_detail.name))?;

        if out_path.extension().and_then(|e| e.to_str()) == Some("flac") {
            let album_artists = album
                .artists
                .as_deref()
                .filter(|artists| !artists.is_empty())
                .unwrap_or(&song_detail.artists);
            let cover = embedded_cover.as_deref().map(|bytes| ("image/jpeg", bytes));

            tag_flac(
                &out_path,
                &FlacMetadata {
                    title: &song_detail.name,
                    artists: &song_detail.artists,
                    album: &album.name,
                    album_artists,
                    track_number: Some((idx + 1) as u32),
                    total_tracks: Some(song_count as u32),
                    disc_number: Some(1),
                    total_discs: Some(1),
                    cover,
                },
            )
            .with_context(|| format!("Failed to write FLAC metadata for {}", song_detail.name))?;
        }

        if let Some(lyric_text) = lyric_text.as_deref() {
            write_lyric_sidecar(&out_path, lyric_text).with_context(|| {
                format!("Failed to save lyric sidecar for {}", song_detail.name)
            })?;
        }

        paths.push(out_path);
    }

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::{
        album_output_dir, download_song, lyric_sidecar_path, write_album_cover_bytes,
        write_lyric_sidecar, MetaOverride,
    };
    use crate::api::ApiClient;
    use crate::audio::OutputFormat;
    use anyhow::Result;
    use std::path::Path;

    #[test]
    fn builds_album_output_dir_from_sanitized_album_name() {
        let base_dir = Path::new("/tmp/downloads");
        let album_dir = album_output_dir(base_dir, "A/B:C?D");

        assert_eq!(album_dir, Path::new("/tmp/downloads").join("A_B_C_D"));
    }

    #[test]
    fn writes_album_cover_with_stable_name_and_detected_extension() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let album_dir = temp_dir.path().join("album");
        let png_bytes = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00];

        let cover_path = write_album_cover_bytes(&album_dir, &png_bytes)?;

        assert_eq!(cover_path, album_dir.join("cover.png"));
        assert!(cover_path.exists(), "cover file should exist");
        assert_eq!(std::fs::read(&cover_path)?, png_bytes);

        Ok(())
    }

    #[test]
    fn writes_lrc_sidecar_next_to_audio_file() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let audio_path = temp_dir.path().join("In Due Time.flac");

        std::fs::write(&audio_path, b"fLaC")?;
        let lyric_path = write_lyric_sidecar(&audio_path, "[00:01.000]In Due Time")?;

        assert_eq!(lyric_path, lyric_sidecar_path(&audio_path));
        assert_eq!(
            std::fs::read_to_string(&lyric_path)?,
            "[00:01.000]In Due Time"
        );

        Ok(())
    }

    #[tokio::test]
    #[ignore = "hits the live upstream API"]
    async fn downloads_real_flac_with_metadata() -> Result<()> {
        let client = ApiClient::new()?;
        let song = client.get_song_detail("048760").await?;
        let album = client.get_album_detail(&song.album_cid).await?;
        let temp_dir = tempfile::tempdir()?;

        let output_path = download_song(
            &client,
            &song,
            &album,
            temp_dir.path(),
            OutputFormat::Flac,
            true,
            &MetaOverride {
                album_name: String::new(),
                artists: Vec::new(),
                album_artists: Vec::new(),
            },
            None,
            |_| {},
        )
        .await?;

        let tag = metaflac::Tag::read_from_path(&output_path)?;
        let comments = tag
            .vorbis_comments()
            .ok_or_else(|| anyhow::anyhow!("missing vorbis comments"))?;

        assert_eq!(
            comments.title().map(|items| items.as_slice()),
            Some([song.name.clone()].as_slice())
        );
        assert_eq!(
            comments.artist().map(|items| items.as_slice()),
            Some(song.artists.as_slice())
        );
        assert_eq!(
            comments.album().map(|items| items.as_slice()),
            Some([album.name.clone()].as_slice())
        );
        assert_eq!(
            comments.album_artist().map(|items| items.as_slice()),
            Some(
                album
                    .artists
                    .as_deref()
                    .filter(|artists| !artists.is_empty())
                    .unwrap_or(&song.artists)
            )
        );
        assert_eq!(comments.track(), Some(1));
        assert_eq!(comments.total_tracks(), Some(album.songs.len() as u32));
        let picture = tag
            .pictures()
            .next()
            .ok_or_else(|| anyhow::anyhow!("expected embedded cover art"))?;
        assert_eq!(picture.mime_type, "image/jpeg");

        let lyric_path = output_path.with_extension("lrc");
        assert!(lyric_path.exists(), "expected lyric sidecar file");
        let lyric_text = std::fs::read_to_string(&lyric_path)?;
        assert!(
            lyric_text.contains("[00:"),
            "expected synchronized LRC lyric content"
        );

        Ok(())
    }
}
