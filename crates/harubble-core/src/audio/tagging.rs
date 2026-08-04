//! FLAC 文件标签与封面元数据写入工具。
//!
//! 提供 [`tag_flac`]：以原子写方式向已落盘的 FLAC 文件写入 Vorbis Comment 标签与封面图片块，
//! 写入过程先在临时文件上完成，再替换最终文件，避免标签写入中断时破坏已存在的音频内容。

use anyhow::{Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use super::format::FlacMetadata;
use super::fs::{ensure_available_space, replace_with_temp, unique_temp_path};

/// 为已写出的 FLAC 文件写入标签与封面元数据。
pub fn tag_flac(path: &Path, metadata: &FlacMetadata<'_>) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let original = fs::read(path)
        .with_context(|| format!("Failed to read FLAC before tagging: {}", path.display()))?;
    ensure_available_space(parent, original.len() as u64)?;

    let temp_path = unique_temp_path(path);
    let tag_result = (|| -> Result<()> {
        let mut temp = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
            .with_context(|| format!("Failed to create temp FLAC {}", temp_path.display()))?;
        temp.write_all(&original)
            .with_context(|| format!("Failed to copy FLAC to temp file {}", temp_path.display()))?;
        temp.sync_all()
            .with_context(|| format!("Failed to sync temp FLAC {}", temp_path.display()))?;
        drop(temp);

        apply_flac_tags(&temp_path, metadata)?;
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(&temp_path)
            .with_context(|| format!("Failed to reopen tagged FLAC {}", temp_path.display()))?
            .sync_all()
            .with_context(|| format!("Failed to sync tagged FLAC {}", temp_path.display()))?;
        replace_with_temp(&temp_path, path)
    })();

    if tag_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    tag_result
}

fn apply_flac_tags(path: &Path, metadata: &FlacMetadata<'_>) -> Result<()> {
    let mut tag = metaflac::Tag::read_from_path(path)
        .with_context(|| format!("Failed to open FLAC for tagging: {}", path.display()))?;

    {
        let vc = tag.vorbis_comments_mut();
        vc.set_title(vec![metadata.title.to_string()]);
        vc.set_album(vec![metadata.album.to_string()]);

        if metadata.artists.is_empty() {
            vc.remove_artist();
        } else {
            vc.set_artist(metadata.artists.to_vec());
        }

        if metadata.album_artists.is_empty() {
            vc.remove_album_artist();
        } else {
            vc.set_album_artist(metadata.album_artists.to_vec());
        }

        if let Some(track_number) = metadata.track_number {
            vc.set_track(track_number);
        } else {
            vc.remove_track();
        }

        if let Some(total_tracks) = metadata.total_tracks {
            vc.set_total_tracks(total_tracks);
            vc.set("TRACKTOTAL", vec![total_tracks.to_string()]);
        } else {
            vc.remove_total_tracks();
            vc.remove("TRACKTOTAL");
        }

        if let Some(disc_number) = metadata.disc_number {
            vc.set("DISCNUMBER", vec![disc_number.to_string()]);
        } else {
            vc.remove("DISCNUMBER");
        }

        if let Some(total_discs) = metadata.total_discs {
            vc.set("TOTALDISCS", vec![total_discs.to_string()]);
            vc.set("DISCTOTAL", vec![total_discs.to_string()]);
        } else {
            vc.remove("TOTALDISCS");
            vc.remove("DISCTOTAL");
        }
    }

    tag.remove_picture_type(metaflac::block::PictureType::CoverFront);

    if let Some((mime_type, cover)) = metadata.cover {
        tag.add_picture(
            mime_type.to_string(),
            metaflac::block::PictureType::CoverFront,
            cover.to_vec(),
        );
    }

    tag.save()
        .with_context(|| format!("Failed to save FLAC tags: {}", path.display()))?;
    Ok(())
}
