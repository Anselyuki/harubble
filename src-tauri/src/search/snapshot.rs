use anyhow::{Context, Result};
use harubble_core::ApiClient;
use pinyin::ToPinyin;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

const SNAPSHOT_FILE_NAME: &str = "library_search_snapshot.json";
const INDEX_ROOT_DIR_NAME: &str = "indexes";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibrarySearchSnapshot {
    pub root_output_dir: String,
    pub inventory_version: String,
    pub built_at: String,
    pub albums: Vec<LibrarySearchAlbumRecord>,
    pub songs: Vec<LibrarySearchSongRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibrarySearchAlbumRecord {
    pub album_cid: String,
    pub album_title: String,
    pub artist_line: Option<String>,
    pub intro: Option<String>,
    pub belong: Option<String>,
    pub album_title_pinyin_full: Option<String>,
    pub album_title_pinyin_initials: Option<String>,
    pub artist_line_pinyin_full: Option<String>,
    pub artist_line_pinyin_initials: Option<String>,
    pub belong_pinyin_full: Option<String>,
    pub belong_pinyin_initials: Option<String>,
    pub tag_values: Option<String>,
    pub tag_values_pinyin_full: Option<String>,
    pub tag_values_pinyin_initials: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibrarySearchSongRecord {
    pub album_cid: String,
    pub song_cid: String,
    pub album_title: String,
    pub song_title: String,
    pub artist_line: Option<String>,
    pub song_title_pinyin_full: Option<String>,
    pub song_title_pinyin_initials: Option<String>,
    pub artist_line_pinyin_full: Option<String>,
    pub artist_line_pinyin_initials: Option<String>,
    pub tag_values: Option<String>,
    pub tag_values_pinyin_full: Option<String>,
    pub tag_values_pinyin_initials: Option<String>,
}

pub(crate) async fn build_library_search_snapshot(
    api: Arc<ApiClient>,
    albums: Vec<harubble_core::Album>,
    tag_registry: crate::tag_registry::TagRegistryService,
    root_output_dir: String,
    inventory_version: String,
) -> Result<LibrarySearchSnapshot> {
    let mut album_records = Vec::with_capacity(albums.len());
    let mut song_records = Vec::new();

    for album in albums {
        let album_artist_line = join_artists(&album.artists);
        let detail = api
            .get_album_detail(&album.cid)
            .await
            .with_context(|| format!("failed to fetch album detail {}", album.cid))?;
        let fallback_artist_line = detail
            .artists
            .as_ref()
            .and_then(|artists| join_artists(artists))
            .or(album_artist_line.clone());

        let album_tag_text = tag_registry.get_all_locale_tag_values_for_album(&album.cid);
        let album_tag_text_opt = normalize_optional_text(Some(album_tag_text));
        album_records.push(LibrarySearchAlbumRecord {
            album_cid: album.cid.clone(),
            album_title: album.name.clone(),
            artist_line: fallback_artist_line.clone(),
            intro: normalize_optional_text(detail.intro.clone()),
            belong: normalize_optional_text(Some(detail.belong.clone())),
            album_title_pinyin_full: to_full_pinyin(&album.name),
            album_title_pinyin_initials: to_pinyin_initials(&album.name),
            artist_line_pinyin_full: album_artist_line.as_deref().and_then(to_full_pinyin),
            artist_line_pinyin_initials: album_artist_line.as_deref().and_then(to_pinyin_initials),
            belong_pinyin_full: to_full_pinyin(&detail.belong),
            belong_pinyin_initials: to_pinyin_initials(&detail.belong),
            tag_values_pinyin_full: album_tag_text_opt.as_deref().and_then(to_full_pinyin),
            tag_values_pinyin_initials: album_tag_text_opt.as_deref().and_then(to_pinyin_initials),
            tag_values: album_tag_text_opt,
        });

        song_records.extend(detail.songs.into_iter().map(|song| {
            let artist_line = join_artists(&song.artists).or_else(|| fallback_artist_line.clone());
            let song_tag_text =
                tag_registry.get_all_locale_tag_values_for_song(&song.cid, &album.cid);
            let song_tag_text_opt = normalize_optional_text(Some(song_tag_text));
            LibrarySearchSongRecord {
                album_cid: album.cid.clone(),
                song_cid: song.cid,
                album_title: album.name.clone(),
                song_title: song.name.clone(),
                artist_line_pinyin_full: artist_line.as_deref().and_then(to_full_pinyin),
                artist_line_pinyin_initials: artist_line.as_deref().and_then(to_pinyin_initials),
                song_title_pinyin_full: to_full_pinyin(&song.name),
                song_title_pinyin_initials: to_pinyin_initials(&song.name),
                artist_line,
                tag_values_pinyin_full: song_tag_text_opt.as_deref().and_then(to_full_pinyin),
                tag_values_pinyin_initials: song_tag_text_opt
                    .as_deref()
                    .and_then(to_pinyin_initials),
                tag_values: song_tag_text_opt,
            }
        }));
    }

    Ok(LibrarySearchSnapshot {
        root_output_dir,
        inventory_version,
        built_at: OffsetDateTime::now_utc()
            .format(&Iso8601::DEFAULT)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string()),
        albums: album_records,
        songs: song_records,
    })
}

/// 为单个专辑构建搜索快照记录（供增量更新使用）。
///
/// 与 [`build_library_search_snapshot`] 的整批构建逻辑保持一致，
/// 但只处理一个专辑，允许调用方按需局部刷新。适用于远端 tag registry
/// 同步或本地 tag 编辑后仅刷新受影响专辑的场景。
///
/// # 参数
/// - `api`：用于拉取专辑详情的上游客户端。
/// - `tag_registry`：用于汇总 album / song 全语种 tag 值的注册表服务。
/// - `album_cid`：待刷新的专辑 CID。
/// - `_locale`：保留位以对齐调用方语义；当前实现按全语种聚合，参数不参与派生。
///
/// # 返回值
/// - `Ok((album_record, song_records))` 该专辑最新的 album 记录及其所属歌曲记录。
/// - `Err(...)` 当拉取专辑详情失败时返回错误，调用方应回退到全量重建。
///
/// # 注意
/// - artist_line 从 `AlbumDetail.artists` 派生；歌曲优先使用自身 artists，缺失时
///   回退到专辑级 artist_line。
/// - tag_values 汇总全部语种，与 [`build_library_search_snapshot`] 语义一致。
pub(crate) async fn build_snapshot_records_for_album(
    api: Arc<ApiClient>,
    tag_registry: crate::tag_registry::TagRegistryService,
    album_cid: &str,
    _locale: crate::preferences::Locale,
) -> Result<(LibrarySearchAlbumRecord, Vec<LibrarySearchSongRecord>)> {
    let detail = api
        .get_album_detail(album_cid)
        .await
        .with_context(|| format!("failed to fetch album detail {album_cid}"))?;

    let album_artist_line = detail
        .artists
        .as_ref()
        .and_then(|artists| join_artists(artists));
    let fallback_artist_line = album_artist_line.clone();

    let album_tag_text = tag_registry.get_all_locale_tag_values_for_album(album_cid);
    let album_tag_text_opt = normalize_optional_text(Some(album_tag_text));
    let album_record = LibrarySearchAlbumRecord {
        album_cid: album_cid.to_string(),
        album_title: detail.name.clone(),
        artist_line: album_artist_line.clone(),
        intro: normalize_optional_text(detail.intro.clone()),
        belong: normalize_optional_text(Some(detail.belong.clone())),
        album_title_pinyin_full: to_full_pinyin(&detail.name),
        album_title_pinyin_initials: to_pinyin_initials(&detail.name),
        artist_line_pinyin_full: album_artist_line.as_deref().and_then(to_full_pinyin),
        artist_line_pinyin_initials: album_artist_line.as_deref().and_then(to_pinyin_initials),
        belong_pinyin_full: to_full_pinyin(&detail.belong),
        belong_pinyin_initials: to_pinyin_initials(&detail.belong),
        tag_values_pinyin_full: album_tag_text_opt.as_deref().and_then(to_full_pinyin),
        tag_values_pinyin_initials: album_tag_text_opt.as_deref().and_then(to_pinyin_initials),
        tag_values: album_tag_text_opt,
    };

    let album_title = detail.name.clone();
    let song_records = detail
        .songs
        .into_iter()
        .map(|song| {
            let artist_line = join_artists(&song.artists).or_else(|| fallback_artist_line.clone());
            let song_tag_text =
                tag_registry.get_all_locale_tag_values_for_song(&song.cid, album_cid);
            let song_tag_text_opt = normalize_optional_text(Some(song_tag_text));
            LibrarySearchSongRecord {
                album_cid: album_cid.to_string(),
                song_cid: song.cid,
                album_title: album_title.clone(),
                song_title: song.name.clone(),
                artist_line_pinyin_full: artist_line.as_deref().and_then(to_full_pinyin),
                artist_line_pinyin_initials: artist_line.as_deref().and_then(to_pinyin_initials),
                song_title_pinyin_full: to_full_pinyin(&song.name),
                song_title_pinyin_initials: to_pinyin_initials(&song.name),
                artist_line,
                tag_values_pinyin_full: song_tag_text_opt.as_deref().and_then(to_full_pinyin),
                tag_values_pinyin_initials: song_tag_text_opt
                    .as_deref()
                    .and_then(to_pinyin_initials),
                tag_values: song_tag_text_opt,
            }
        })
        .collect();

    Ok((album_record, song_records))
}

pub(crate) fn load_library_search_snapshot(
    base_dir: &Path,
) -> Result<Option<LibrarySearchSnapshot>> {
    let path = snapshot_file_path(base_dir);
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    if content.trim().is_empty() {
        return Ok(None);
    }

    serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))
        .map(Some)
}

pub(crate) fn save_library_search_snapshot(
    base_dir: &Path,
    snapshot: &LibrarySearchSnapshot,
) -> Result<()> {
    std::fs::create_dir_all(base_dir)
        .with_context(|| format!("failed to create {}", base_dir.display()))?;
    let path = snapshot_file_path(base_dir);
    let content = serde_json::to_string_pretty(snapshot)?;
    std::fs::write(&path, content).with_context(|| format!("failed to write {}", path.display()))
}

pub(crate) fn snapshot_file_path(base_dir: &Path) -> PathBuf {
    base_dir.join(SNAPSHOT_FILE_NAME)
}

pub(crate) fn indexes_root_dir(base_dir: &Path) -> PathBuf {
    base_dir.join(INDEX_ROOT_DIR_NAME)
}

pub(crate) fn inventory_index_dir(base_dir: &Path, inventory_version: &str) -> PathBuf {
    indexes_root_dir(base_dir).join(index_directory_name(inventory_version))
}

/// 删除除当前版本外的历史搜索索引目录。
///
/// 搜索索引属于可重建缓存，每次库存扫描都会生成新的版本目录。调用方在确认
/// `keep_inventory_version` 已可正常打开后可调用本函数回收旧版本，避免缓存目录
/// 随应用启动次数持续增长。
pub(crate) fn cleanup_obsolete_search_indexes(
    base_dir: &Path,
    keep_inventory_version: &str,
) -> Result<usize> {
    let indexes_dir = indexes_root_dir(base_dir);
    if !indexes_dir.exists() {
        return Ok(0);
    }

    let keep_dir_name = index_directory_name(keep_inventory_version);
    let mut removed = 0;
    let mut first_error = None;
    for entry in std::fs::read_dir(&indexes_dir)
        .with_context(|| format!("failed to read {}", indexes_dir.display()))?
    {
        let entry = match entry
            .with_context(|| format!("failed to read entry in {}", indexes_dir.display()))
        {
            Ok(entry) => entry,
            Err(error) => {
                first_error.get_or_insert(error);
                continue;
            }
        };
        let file_type = match entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", entry.path().display()))
        {
            Ok(file_type) => file_type,
            Err(error) => {
                first_error.get_or_insert(error);
                continue;
            }
        };
        if !file_type.is_dir() || entry.file_name() == keep_dir_name.as_str() {
            continue;
        }
        match std::fs::remove_dir_all(entry.path())
            .with_context(|| format!("failed to remove {}", entry.path().display()))
        {
            Ok(()) => removed += 1,
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }
    Ok(removed)
}

fn join_artists(artists: &[String]) -> Option<String> {
    let line = artists
        .iter()
        .map(|artist| artist.trim())
        .filter(|artist| !artist.is_empty())
        .collect::<Vec<_>>()
        .join(", ");

    if line.is_empty() {
        None
    } else {
        Some(line)
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn to_full_pinyin(input: &str) -> Option<String> {
    let syllables = input
        .to_pinyin()
        .flatten()
        .map(|item| item.plain().to_string())
        .collect::<Vec<_>>();

    if syllables.is_empty() {
        None
    } else {
        Some(syllables.join(" "))
    }
}

fn to_pinyin_initials(input: &str) -> Option<String> {
    let initials = input
        .to_pinyin()
        .flatten()
        .filter_map(|item| item.plain().chars().next())
        .collect::<String>();

    if initials.is_empty() {
        None
    } else {
        Some(initials)
    }
}

fn index_directory_name(inventory_version: &str) -> String {
    inventory_version
        .chars()
        .map(|character| match character {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => character,
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{cleanup_obsolete_search_indexes, indexes_root_dir, inventory_index_dir};
    use tempfile::tempdir;

    #[test]
    fn cleanup_keeps_only_the_active_index_directory() {
        let temp_dir = tempdir().expect("temp dir");
        let indexes_dir = indexes_root_dir(temp_dir.path());
        std::fs::create_dir_all(inventory_index_dir(temp_dir.path(), "inv-current"))
            .expect("current index");
        std::fs::create_dir_all(inventory_index_dir(temp_dir.path(), "inv-old-1"))
            .expect("old index 1");
        std::fs::create_dir_all(inventory_index_dir(temp_dir.path(), "inv-old-2"))
            .expect("old index 2");

        let removed =
            cleanup_obsolete_search_indexes(temp_dir.path(), "inv-current").expect("cleanup");

        assert_eq!(removed, 2);
        assert!(inventory_index_dir(temp_dir.path(), "inv-current").is_dir());
        assert_eq!(
            std::fs::read_dir(indexes_dir)
                .expect("read indexes")
                .count(),
            1
        );
    }
}
