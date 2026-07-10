//! 音频格式识别、音频写盘与 FLAC 标签处理工具。
//!
//! 该模块提供音频格式探测、输出格式定义、文件名清洗、音频保存、封面编码与 FLAC
//! 标签写入等能力，是下载落盘流水线中的音频处理基础模块。

pub mod format;
pub mod fs;
pub mod image;
pub mod tagging;

pub use format::{sanitize_filename, AudioFormat, FlacMetadata, OutputFormat};
pub use fs::{ensure_available_space, save_audio, write_file_atomically};
pub use image::{detect_image_mime, encode_cover_as_jpeg};
pub use tagging::tag_flac;

#[cfg(test)]
mod tests {
    use super::{tag_flac, write_file_atomically, FlacMetadata};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn atomic_write_replaces_existing_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("song.wav");
        fs::write(&path, b"old").expect("old file");
        write_file_atomically(&path, b"new audio").expect("atomic write");
        assert_eq!(fs::read(&path).expect("final file"), b"new audio");
        let temp_entries = fs::read_dir(dir.path())
            .expect("read tempdir")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".part"))
            .count();
        assert_eq!(temp_entries, 0);
    }

    #[test]
    fn atomic_write_removes_temp_file_when_replace_fails() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("not-a-file");
        fs::create_dir(&path).expect("directory target");
        let error = write_file_atomically(&path, b"new audio").expect_err("replace should fail");
        assert!(
            format!("{error:#}").contains("Failed to move temp file")
                || format!("{error:#}").contains("Failed to replace existing file"),
            "{error:#}"
        );
        assert_no_part_files(dir.path());
    }

    #[test]
    fn tag_flac_failure_keeps_original_file_and_removes_temp_file() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("song.flac");
        fs::write(&path, b"not a flac").expect("invalid flac fixture");
        let metadata = FlacMetadata {
            title: "title",
            artists: &[],
            album: "album",
            album_artists: &[],
            track_number: None,
            total_tracks: None,
            disc_number: None,
            total_discs: None,
            cover: None,
        };
        tag_flac(&path, &metadata).expect_err("invalid FLAC should fail tagging");
        assert_eq!(fs::read(&path).expect("final file"), b"not a flac");
        assert_no_part_files(dir.path());
    }

    fn assert_no_part_files(dir: &std::path::Path) {
        let temp_entries = fs::read_dir(dir)
            .expect("read tempdir")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".part"))
            .count();
        assert_eq!(temp_entries, 0);
    }
}
