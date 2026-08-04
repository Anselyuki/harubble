//! 音频格式与输出格式类型定义，以及文件名清洗工具。
//!
//! 提供 [`AudioFormat`]（源文件格式检测）、[`OutputFormat`]（用户期望的落盘格式）
//! 与 [`FlacMetadata`]（FLAC 标签写入时所需的元数据结构），以及 [`sanitize_filename`] 工具函数。

use serde::{Deserialize, Serialize};

/// 根据原始音频字节识别音频格式。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioFormat {
    Wav,
    Mp3,
    Flac,
    Unknown,
}

impl AudioFormat {
    /// 根据文件头字节判断音频格式。
    ///
    /// # 示例
    ///
    /// ```
    /// use harubble_core::AudioFormat;
    ///
    /// assert_eq!(AudioFormat::detect(b"RIFF\0\0\0\0WAVE"), AudioFormat::Wav);
    /// assert_eq!(AudioFormat::detect(b"fLaC\0\0\0\0"), AudioFormat::Flac);
    /// ```
    pub fn detect(data: &[u8]) -> Self {
        if data.starts_with(b"RIFF") && data.get(8..12) == Some(b"WAVE") {
            AudioFormat::Wav
        } else if data.starts_with(b"ID3")
            || data.starts_with(&[0xFF, 0xFB])
            || data.starts_with(&[0xFF, 0xF3])
            || data.starts_with(&[0xFF, 0xF2])
        {
            AudioFormat::Mp3
        } else if data.starts_with(b"fLaC") {
            AudioFormat::Flac
        } else {
            AudioFormat::Unknown
        }
    }

    /// 返回当前音频格式对应的默认文件扩展名。
    ///
    /// # 示例
    ///
    /// ```
    /// use harubble_core::AudioFormat;
    ///
    /// assert_eq!(AudioFormat::Flac.extension(), "flac");
    /// assert_eq!(AudioFormat::Unknown.extension(), "bin");
    /// ```
    pub fn extension(self) -> &'static str {
        match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Flac => "flac",
            AudioFormat::Unknown => "bin",
        }
    }
}

/// 用户选择的输出格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OutputFormat {
    /// 保持为 WAV 原始格式，不做转码。
    #[default]
    Wav,
    /// 将 WAV 转码为 FLAC，以便获得更小体积和更完整的标签支持。
    Flac,
    /// 保持为 MP3 原始格式，不做转码。
    Mp3,
}

impl OutputFormat {
    /// 返回用于界面展示的格式名称。
    ///
    /// # 示例
    ///
    /// ```
    /// use harubble_core::OutputFormat;
    ///
    /// assert_eq!(OutputFormat::Wav.label(), "WAV (Lossless)");
    /// assert_eq!(OutputFormat::Mp3.label(), "MP3");
    /// ```
    pub fn label(self) -> &'static str {
        match self {
            OutputFormat::Wav => "WAV (Lossless)",
            OutputFormat::Flac => "FLAC (Lossless)",
            OutputFormat::Mp3 => "MP3",
        }
    }
}

/// 写入 FLAC Vorbis Comment 与封面块时使用的元数据。
///
/// 适用于在音频文件已经落盘后补齐标题、艺术家、曲序与封面等信息；调用方应保证
/// 目标文件确实是可写的 FLAC 文件。
pub struct FlacMetadata<'a> {
    /// 曲目标题。
    pub title: &'a str,
    /// 曲目艺术家列表。
    pub artists: &'a [String],
    /// 专辑名称。
    pub album: &'a str,
    /// 专辑艺术家列表。
    pub album_artists: &'a [String],
    /// 曲目序号。
    pub track_number: Option<u32>,
    /// 专辑总曲数。
    pub total_tracks: Option<u32>,
    /// 光盘序号。
    pub disc_number: Option<u32>,
    /// 总光盘数。
    pub total_discs: Option<u32>,
    /// 封面数据，格式为 `(mime_type, bytes)`。
    pub cover: Option<(&'static str, &'a [u8])>,
}

/// 清洗文件名片段中不适合落盘的字符。
///
/// # 示例
///
/// ```
/// use harubble_core::audio::sanitize_filename;
///
/// assert_eq!(sanitize_filename("A/B:C"), "A_B_C");
/// assert_eq!(sanitize_filename("  hello  "), "hello");
/// ```
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}
