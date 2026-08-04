use anyhow::{anyhow, Context, Result};
use filetime::FileTime;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const IMAGE_CACHE_SOFT_LIMIT_BYTES: u64 = 512 * 1024 * 1024;
const IMAGE_CACHE_TARGET_BYTES: u64 = 384 * 1024 * 1024;
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "png", "gif", "webp"];

static IMAGE_CACHE_CLEANUP_RUNNING: AtomicBool = AtomicBool::new(false);
static IMAGE_DOWNLOAD_PERMITS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);

struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        IMAGE_CACHE_CLEANUP_RUNNING.store(false, Ordering::SeqCst);
    }
}

struct CacheFileEntry {
    path: PathBuf,
    size: u64,
    modified_at: std::time::SystemTime,
}

#[derive(Debug)]
pub(crate) enum ImageCacheError {
    Network(anyhow::Error),
    Storage(anyhow::Error),
}

impl std::fmt::Display for ImageCacheError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Network(error) | Self::Storage(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ImageCacheError {}

fn cache_key(image_url: &str) -> String {
    format!("{:x}", md5::compute(image_url.as_bytes()))
}

fn cache_path(cache_dir: &Path, image_url: &str, extension: &str) -> PathBuf {
    cache_dir.join(format!("{}.{}", cache_key(image_url), extension))
}

fn extension_for_bytes(bytes: &[u8]) -> Result<&'static str> {
    match harubble_core::audio::detect_image_mime(bytes) {
        Some("image/jpeg") => Ok("jpg"),
        Some("image/png") => Ok("png"),
        Some("image/gif") => Ok("gif"),
        Some("image/webp") => Ok("webp"),
        _ => Err(anyhow!("unsupported cached image format")),
    }
}

fn touch(path: &Path) {
    let _ = filetime::set_file_mtime(path, FileTime::now());
}

/// 查找已落盘的封面，并用修改时间记录最近一次访问。
pub(crate) fn find_cached_image(cache_dir: &Path, image_url: &str) -> Option<PathBuf> {
    for extension in IMAGE_EXTENSIONS {
        let path = cache_path(cache_dir, image_url, extension);
        let Ok(metadata) = path.metadata() else {
            continue;
        };
        if metadata.is_file() && metadata.len() > 0 {
            touch(&path);
            return Some(path);
        }
    }
    None
}

/// 将下载完成的图片原子写入缓存，并返回可供 asset protocol 使用的绝对路径。
pub(crate) fn store_cached_image(
    cache_dir: &Path,
    image_url: &str,
    bytes: &[u8],
) -> Result<PathBuf> {
    fs::create_dir_all(cache_dir)
        .with_context(|| format!("failed to create image cache {}", cache_dir.display()))?;
    let extension = extension_for_bytes(bytes)?;
    let destination = cache_path(cache_dir, image_url, extension);
    if destination.is_file() {
        touch(&destination);
        return Ok(destination);
    }

    let temp_path = cache_dir.join(format!(
        ".{}-{}.tmp",
        cache_key(image_url),
        uuid::Uuid::new_v4()
    ));
    fs::write(&temp_path, bytes)
        .with_context(|| format!("failed to write cached image {}", temp_path.display()))?;
    if let Err(error) = fs::rename(&temp_path, &destination) {
        let _ = fs::remove_file(&temp_path);
        if destination.is_file() {
            touch(&destination);
            return Ok(destination);
        }
        return Err(error)
            .with_context(|| format!("failed to publish cached image {}", destination.display()));
    }
    Ok(destination)
}

/// 返回可用的本地封面路径；仅在磁盘未命中时访问远端。
pub(crate) async fn get_or_download(
    cache_dir: PathBuf,
    image_url: String,
    api: Arc<harubble_core::ApiClient>,
) -> std::result::Result<PathBuf, ImageCacheError> {
    let lookup_dir = cache_dir.clone();
    let lookup_url = image_url.clone();
    let cached = tokio::task::spawn_blocking(move || find_cached_image(&lookup_dir, &lookup_url))
        .await
        .map_err(|error| {
            ImageCacheError::Storage(anyhow!("image cache lookup worker failed: {error}"))
        })?;
    if let Some(path) = cached {
        return Ok(path);
    }

    let _download_permit = IMAGE_DOWNLOAD_PERMITS.acquire().await.map_err(|error| {
        ImageCacheError::Storage(anyhow!("image download limiter closed: {error}"))
    })?;
    let retry_dir = cache_dir.clone();
    let retry_url = image_url.clone();
    let cached_after_wait =
        tokio::task::spawn_blocking(move || find_cached_image(&retry_dir, &retry_url))
            .await
            .map_err(|error| {
                ImageCacheError::Storage(anyhow!("image cache recheck worker failed: {error}"))
            })?;
    if let Some(path) = cached_after_wait {
        return Ok(path);
    }

    let bytes = api
        .download_bytes_coalesced(&image_url)
        .await
        .map_err(ImageCacheError::Network)?;
    let store_dir = cache_dir.clone();
    let store_url = image_url.clone();
    let path =
        tokio::task::spawn_blocking(move || store_cached_image(&store_dir, &store_url, &bytes))
            .await
            .map_err(|error| {
                ImageCacheError::Storage(anyhow!("image cache store worker failed: {error}"))
            })?
            .map_err(ImageCacheError::Storage)?;
    spawn_cleanup_if_needed(cache_dir);
    Ok(path)
}

/// 在后台按总字节数回收最久未访问的封面。
pub(crate) fn spawn_cleanup_if_needed(cache_dir: PathBuf) {
    if IMAGE_CACHE_CLEANUP_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(move || {
        let _guard = CleanupGuard;
        let _ = evict_cache_dir_until_target(
            &cache_dir,
            IMAGE_CACHE_SOFT_LIMIT_BYTES,
            IMAGE_CACHE_TARGET_BYTES,
        );
    });
}

fn collect_cache_files(cache_dir: &Path) -> Result<Vec<CacheFileEntry>> {
    let mut files = Vec::new();
    for entry in fs::read_dir(cache_dir)
        .with_context(|| format!("failed to read image cache {}", cache_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let is_image = path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| IMAGE_EXTENSIONS.contains(&value));
        if !is_image {
            continue;
        }
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }
        files.push(CacheFileEntry {
            path,
            size: metadata.len(),
            modified_at: metadata
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
        });
    }
    files.sort_by_key(|entry| entry.modified_at);
    Ok(files)
}

fn evict_cache_dir_until_target(
    cache_dir: &Path,
    soft_limit: u64,
    target_size: u64,
) -> Result<u64> {
    if !cache_dir.is_dir() {
        return Ok(0);
    }
    let files = collect_cache_files(cache_dir)?;
    let mut total_size = files.iter().map(|entry| entry.size).sum::<u64>();
    if total_size <= soft_limit {
        return Ok(total_size);
    }

    for file in files {
        if total_size <= target_size {
            break;
        }
        match fs::remove_file(&file.path) {
            Ok(()) => total_size = total_size.saturating_sub(file.size),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to remove cached image {}", file.path.display())
                });
            }
        }
    }
    Ok(total_size)
}

#[cfg(test)]
mod tests {
    use super::{cache_path, evict_cache_dir_until_target, find_cached_image, store_cached_image};
    use filetime::FileTime;
    use tempfile::tempdir;

    #[test]
    fn stores_and_finds_image_by_url_hash() {
        let temp = tempdir().expect("tempdir");
        let url = "https://web.hycdn.cn/siren/test.png";
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];

        let stored = store_cached_image(temp.path(), url, &png).expect("store image");

        assert_eq!(stored, cache_path(temp.path(), url, "png"));
        assert_eq!(find_cached_image(temp.path(), url), Some(stored));
    }

    #[test]
    fn rejects_unknown_image_bytes() {
        let temp = tempdir().expect("tempdir");
        let error = store_cached_image(temp.path(), "https://example.com/image", b"not-image")
            .expect_err("unsupported image");

        assert!(error
            .to_string()
            .contains("unsupported cached image format"));
    }

    #[test]
    fn evicts_oldest_images_until_target_size() {
        let temp = tempdir().expect("tempdir");
        let oldest = temp.path().join("oldest.jpg");
        let middle = temp.path().join("middle.png");
        let newest = temp.path().join("newest.webp");
        std::fs::write(&oldest, vec![1_u8; 40]).expect("oldest");
        std::fs::write(&middle, vec![2_u8; 40]).expect("middle");
        std::fs::write(&newest, vec![3_u8; 40]).expect("newest");
        filetime::set_file_mtime(&oldest, FileTime::from_unix_time(10, 0)).expect("old time");
        filetime::set_file_mtime(&middle, FileTime::from_unix_time(20, 0)).expect("middle time");
        filetime::set_file_mtime(&newest, FileTime::from_unix_time(30, 0)).expect("new time");

        let remaining = evict_cache_dir_until_target(temp.path(), 80, 40).expect("evict");

        assert_eq!(remaining, 40);
        assert!(!oldest.exists());
        assert!(!middle.exists());
        assert!(newest.exists());
    }
}
