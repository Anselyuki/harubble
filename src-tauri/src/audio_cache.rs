use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;
use walkdir::WalkDir;

const APP_CACHE_DIR: &str = "harubble";
const AUDIO_CACHE_DIR: &str = "audio";
const AUDIO_CACHE_SOFT_LIMIT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const AUDIO_CACHE_TARGET_BYTES: u64 = AUDIO_CACHE_SOFT_LIMIT_BYTES * 8 / 10;

static AUDIO_CACHE_CLEANUP_RUNNING: AtomicBool = AtomicBool::new(false);
static AUDIO_CACHE_LEASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static AUDIO_CACHE_LEASES: OnceLock<Mutex<HashMap<PathBuf, LeaseRecord>>> = OnceLock::new();
static AUDIO_CACHE_IO_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 统一串行化播放缓存的准备、写入、晋升和手动清理。
pub fn io_lock() -> &'static tokio::sync::Mutex<()> {
    &AUDIO_CACHE_IO_LOCK
}

/// 一次播放准备对单首音频缓存路径的所有权。
///
/// 同一歌曲快速重建播放会话时，新租约会立即取代旧租约。异步下载或清理任务必须
/// 在修改文件前验证租约，避免旧会话删除或覆盖新会话刚创建的缓存。
#[derive(Clone, Debug)]
pub struct AudioCacheLease {
    inner: Arc<AudioCacheLeaseInner>,
}

#[derive(Debug)]
struct AudioCacheLeaseInner {
    cache_path: PathBuf,
    staging_path: PathBuf,
    generation: u64,
    validity: Arc<AtomicBool>,
}

struct LeaseRecord {
    generation: u64,
    validity: Arc<AtomicBool>,
}

impl AudioCacheLease {
    pub fn cache_path(&self) -> &Path {
        &self.inner.cache_path
    }

    /// 返回该播放代次专用的下载暂存文件路径。
    ///
    /// 暂存文件始终与 canonical cache 分离，旧会话即使延迟写入或清理，也不会
    /// 触碰新会话正在使用的文件。
    pub fn staging_path(&self) -> &Path {
        &self.inner.staging_path
    }

    pub fn generation(&self) -> u64 {
        self.inner.generation
    }

    pub fn is_current(&self) -> bool {
        cache_leases()
            .lock()
            .unwrap()
            .get(&self.inner.cache_path)
            .is_some_and(|record| {
                record.generation == self.inner.generation && record.validity.load(Ordering::SeqCst)
            })
    }
}

impl Drop for AudioCacheLeaseInner {
    fn drop(&mut self) {
        let mut leases = cache_leases().lock().unwrap();
        if leases
            .get(&self.cache_path)
            .is_some_and(|record| record.generation == self.generation)
        {
            leases.remove(&self.cache_path);
        }
        self.validity.store(false, Ordering::SeqCst);
    }
}

fn cache_leases() -> &'static Mutex<HashMap<PathBuf, LeaseRecord>> {
    AUDIO_CACHE_LEASES.get_or_init(|| Mutex::new(HashMap::new()))
}

struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        AUDIO_CACHE_CLEANUP_RUNNING.store(false, Ordering::SeqCst);
    }
}

struct CacheFileEntry {
    path: PathBuf,
    size: u64,
    modified_at: SystemTime,
}

/// 返回音频缓存目录路径。
pub fn audio_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| std::env::temp_dir().join("cache"))
        .join(APP_CACHE_DIR)
        .join(AUDIO_CACHE_DIR)
}

/// 确保音频缓存目录存在，并返回其路径。
pub fn ensure_audio_cache_dir() -> Result<PathBuf> {
    let dir = audio_cache_dir();
    fs::create_dir_all(&dir).context("Failed to create audio cache directory")?;
    Ok(dir)
}

const ALLOWED_EXTENSIONS: &[&str] = &["flac", "wav", "mp3", "ogg", "bin"];

/// 根据歌曲 CID 与源地址推导缓存文件路径。
pub fn cached_song_path(song_cid: &str, source_url: &str) -> Result<PathBuf> {
    let raw_extension = Path::new(source_url.split('?').next().unwrap_or(source_url))
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("bin");

    let extension = if ALLOWED_EXTENSIONS.contains(&raw_extension) {
        raw_extension
    } else {
        "bin"
    };

    Ok(ensure_audio_cache_dir()?.join(format!("{song_cid}.{extension}")))
}

/// 返回给定缓存文件对应的 `.pending` 标记文件路径。
pub fn pending_marker_path(cache_path: &Path) -> PathBuf {
    let mut marker_name = cache_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("audio")
        .to_string();
    marker_name.push_str(".pending");
    cache_path.with_file_name(marker_name)
}

/// 判断目标歌曲缓存文件是否已经完整可用。
pub fn is_song_cached(cache_path: &Path) -> bool {
    cache_path.is_file() && !pending_marker_path(cache_path).exists()
}

/// 将给定路径登记为最新播放缓存使用者。
pub fn acquire_song_cache_lease(cache_path: &Path) -> AudioCacheLease {
    let generation = AUDIO_CACHE_LEASE_SEQUENCE.fetch_add(1, Ordering::SeqCst) + 1;
    let staging_path = staging_cache_path(cache_path, generation);
    let validity = Arc::new(AtomicBool::new(true));
    if let Some(previous) = cache_leases().lock().unwrap().insert(
        cache_path.to_path_buf(),
        LeaseRecord {
            generation,
            validity: Arc::clone(&validity),
        },
    ) {
        previous.validity.store(false, Ordering::SeqCst);
    }
    AudioCacheLease {
        inner: Arc::new(AudioCacheLeaseInner {
            cache_path: cache_path.to_path_buf(),
            staging_path,
            generation,
            validity,
        }),
    }
}

/// 根据 canonical 路径和播放代次生成独立的下载暂存路径。
///
/// 保留原始扩展名，确保 Symphonia 在暂存文件尚未提升为 canonical 文件时仍能
/// 通过路径提示识别 WAV/FLAC 等容器格式。
pub fn staging_cache_path(cache_path: &Path, generation: u64) -> PathBuf {
    let stem = cache_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("audio");
    let extension = cache_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bin");
    cache_path.with_file_name(format!("{stem}.generation-{generation}.{extension}"))
}

/// 删除单首歌曲的本地音频缓存及其下载中标记。
///
/// 播放器在确认当前歌曲的缓存读写失败后使用该操作，让下一次加载重新从远端获取
/// 数据；不存在的文件视为已清理，避免并发失败路径把恢复流程变成错误。
pub fn remove_song_cache(cache_path: &Path) -> Result<()> {
    let pending_marker = pending_marker_path(cache_path);
    // 先删除数据文件，成功后再移除 marker。若数据文件暂时仍被占用，保留 marker
    // 可以阻止下次加载把不完整文件误判为可用缓存。
    for path in [cache_path, &pending_marker] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("Failed to remove audio cache file {}", path.display())
                })
            }
        }
    }
    Ok(())
}

/// 仅当租约仍是该路径的最新所有者时删除缓存。
pub fn remove_song_cache_if_current(lease: &AudioCacheLease) -> Result<bool> {
    let leases = cache_leases().lock().unwrap();
    let is_current = leases.get(lease.cache_path()).is_some_and(|record| {
        record.generation == lease.generation() && record.validity.load(Ordering::SeqCst)
    });

    remove_staging_cache(lease)?;

    if !is_current {
        return Ok(false);
    }

    remove_song_cache(lease.cache_path())?;
    Ok(true)
}

/// 删除该播放代次独占的暂存文件，但保留已经完整可用的 canonical 缓存。
///
/// 正常停止、切歌以及输出设备失败都使用该操作，避免把输入缓存误判为故障来源。
pub fn remove_staging_cache(lease: &AudioCacheLease) -> Result<()> {
    remove_file_if_present(lease.staging_path())?;
    remove_file_if_present(&pending_marker_path(lease.staging_path()))
}

/// 仅当租约仍有效时移除 pending marker，将下载结果标记为完整缓存。
pub fn complete_song_cache_if_current(lease: &AudioCacheLease) -> Result<bool> {
    let leases = cache_leases().lock().unwrap();
    let is_current = leases.get(lease.cache_path()).is_some_and(|record| {
        record.generation == lease.generation() && record.validity.load(Ordering::SeqCst)
    });
    if !is_current {
        remove_file_if_present(lease.staging_path())?;
        remove_file_if_present(&pending_marker_path(lease.staging_path()))?;
        return Ok(false);
    }

    let staging_marker = pending_marker_path(lease.staging_path());
    let canonical_marker = pending_marker_path(lease.cache_path());
    fs::write(
        &canonical_marker,
        format!("generation:{}", lease.generation()),
    )
    .with_context(|| {
        format!(
            "Failed to create cache marker {}",
            canonical_marker.display()
        )
    })?;

    // The preparation path owns the canonical file while this lease is current. Remove an
    // unexpected old destination before promotion so the operation works on Windows too.
    remove_file_if_present(lease.cache_path())?;
    fs::rename(lease.staging_path(), lease.cache_path()).with_context(|| {
        format!(
            "Failed to promote streaming cache file {} to {}",
            lease.staging_path().display(),
            lease.cache_path().display()
        )
    })?;
    remove_file_if_present(&staging_marker)?;
    remove_file_if_present(&canonical_marker)?;
    Ok(true)
}

fn remove_file_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("Failed to remove audio cache file {}", path.display())),
    }
}

/// 当缓存体积超过软上限时，在后台触发一次清理任务。
///
/// 该方法具备去重保护：若已有清理线程在运行，则本次调用会直接返回。
pub fn spawn_cleanup_if_needed() {
    let Ok(dir) = ensure_audio_cache_dir() else {
        return;
    };

    let Ok(total_size) = calculate_cache_size(&dir) else {
        return;
    };

    if total_size <= AUDIO_CACHE_SOFT_LIMIT_BYTES {
        return;
    }

    if AUDIO_CACHE_CLEANUP_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(move || {
        let _guard = CleanupGuard;
        let _ = evict_cache_dir_until_target(
            &dir,
            AUDIO_CACHE_SOFT_LIMIT_BYTES,
            AUDIO_CACHE_TARGET_BYTES,
        );
    });
}

/// 清空音频缓存目录，并返回本次移除的条目数量。
pub fn clear_audio_cache() -> Result<u64> {
    let dir = ensure_audio_cache_dir()?;
    // Invalidate all generations before deleting. Existing writers will observe a stale lease
    // and stop publishing; a new playback generation cannot acquire ownership until deletion is
    // complete because the registry lock is held for the whole operation.
    let mut leases = cache_leases().lock().unwrap();
    for record in leases.values() {
        record.validity.store(false, Ordering::SeqCst);
    }
    leases.clear();
    let mut removed = 0_u64;

    for entry in fs::read_dir(&dir).context("Failed to read audio cache directory")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .with_context(|| format!("Failed to remove cache directory {}", path.display()))?;
        } else {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to remove cache file {}", path.display()))?;
        }
        removed += 1;
    }

    Ok(removed)
}

fn is_pending_marker(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.ends_with(".pending"))
        .unwrap_or(false)
}

fn active_cache_artifacts() -> HashSet<PathBuf> {
    let leases = cache_leases().lock().unwrap();
    active_cache_artifacts_from(&leases)
}

fn active_cache_artifacts_from(leases: &HashMap<PathBuf, LeaseRecord>) -> HashSet<PathBuf> {
    leases
        .iter()
        .flat_map(|(cache_path, record)| {
            let staging_path = staging_cache_path(cache_path, record.generation);
            [
                cache_path.clone(),
                pending_marker_path(cache_path),
                staging_path.clone(),
                pending_marker_path(&staging_path),
            ]
        })
        .collect()
}

fn calculate_cache_size(dir: &Path) -> Result<u64> {
    let mut total_size = 0_u64;

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if !entry.file_type().is_file() || is_pending_marker(entry.path()) {
            continue;
        }
        total_size += entry.metadata()?.len();
    }

    Ok(total_size)
}

fn collect_cache_files(dir: &Path) -> Result<Vec<CacheFileEntry>> {
    let active_artifacts = active_cache_artifacts();
    let mut files = Vec::new();

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if !entry.file_type().is_file()
            || is_pending_marker(entry.path())
            || active_artifacts.contains(entry.path())
        {
            continue;
        }

        let metadata = entry.metadata()?;
        files.push(CacheFileEntry {
            path: entry.path().to_path_buf(),
            size: metadata.len(),
            modified_at: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        });
    }

    files.sort_by_key(|entry| entry.modified_at);
    Ok(files)
}

fn evict_cache_dir_until_target(dir: &Path, soft_limit: u64, target_size: u64) -> Result<u64> {
    let mut total_size = calculate_cache_size(dir)?;
    if total_size <= soft_limit {
        return Ok(total_size);
    }

    let files = collect_cache_files(dir)?;
    for file in files {
        if total_size <= target_size {
            break;
        }

        // Re-check while holding the same registry lock used by lease acquisition. This closes
        // the window between collection and deletion for a cached file that just became active.
        let leases = cache_leases().lock().unwrap();
        if active_cache_artifacts_from(&leases).contains(&file.path) {
            continue;
        }
        match fs::remove_file(&file.path) {
            Ok(()) => {
                total_size = total_size.saturating_sub(file.size);
                remove_file_if_present(&pending_marker_path(&file.path))?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("Failed to remove cache file {}", file.path.display())
                })
            }
        }
    }

    Ok(total_size)
}

#[cfg(test)]
mod tests {
    use super::{
        acquire_song_cache_lease, calculate_cache_size, complete_song_cache_if_current,
        evict_cache_dir_until_target, pending_marker_path, remove_song_cache,
        remove_song_cache_if_current, remove_staging_cache,
    };
    use std::fs;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn counts_incomplete_audio_but_excludes_pending_marker_size() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_file = temp_dir.path().join("song.flac");
        fs::write(&audio_file, vec![1_u8; 32]).expect("audio file");
        fs::write(pending_marker_path(&audio_file), vec![1_u8; 64]).expect("pending marker");

        let total_size = calculate_cache_size(temp_dir.path()).expect("cache size");
        assert_eq!(total_size, 32);
    }

    #[test]
    fn evicts_stale_incomplete_audio_and_its_pending_marker() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_file = temp_dir.path().join("song.generation-3.flac");
        let marker = pending_marker_path(&audio_file);
        fs::write(&audio_file, vec![1_u8; 40]).expect("stale audio file");
        fs::write(&marker, b"pending").expect("stale pending marker");

        let remaining = evict_cache_dir_until_target(temp_dir.path(), 20, 0).expect("eviction");

        assert_eq!(remaining, 0);
        assert!(!audio_file.exists());
        assert!(!marker.exists());
    }

    #[test]
    fn evicts_oldest_files_until_target_size_is_reached() {
        let temp_dir = tempdir().expect("tempdir");
        let oldest = temp_dir.path().join("oldest.flac");
        fs::write(&oldest, vec![1_u8; 40]).expect("oldest");
        thread::sleep(Duration::from_millis(20));

        let middle = temp_dir.path().join("middle.flac");
        fs::write(&middle, vec![1_u8; 40]).expect("middle");
        thread::sleep(Duration::from_millis(20));

        let newest = temp_dir.path().join("newest.flac");
        fs::write(&newest, vec![1_u8; 40]).expect("newest");

        let remaining_size =
            evict_cache_dir_until_target(temp_dir.path(), 80, 40).expect("eviction");

        assert_eq!(remaining_size, 40);
        assert!(!oldest.exists());
        assert!(!middle.exists());
        assert!(newest.exists());
    }

    #[test]
    fn removes_one_song_cache_and_its_pending_marker() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_file = temp_dir.path().join("song.flac");
        let marker = pending_marker_path(&audio_file);
        fs::write(&audio_file, b"audio").expect("audio cache");
        fs::write(&marker, b"pending").expect("pending marker");

        remove_song_cache(&audio_file).expect("cache removal");
        remove_song_cache(&audio_file).expect("idempotent cache removal");

        assert!(!audio_file.exists());
        assert!(!marker.exists());
    }

    #[test]
    fn staging_cleanup_preserves_healthy_canonical_cache() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_file = temp_dir.path().join("song.wav");
        fs::write(&audio_file, b"healthy cache").expect("canonical cache");
        let lease = acquire_song_cache_lease(&audio_file);
        let staging_marker = pending_marker_path(lease.staging_path());
        fs::write(lease.staging_path(), b"partial stream").expect("staging cache");
        fs::write(&staging_marker, b"pending").expect("staging marker");

        remove_staging_cache(&lease).expect("staging cleanup");

        assert_eq!(
            fs::read(&audio_file).expect("canonical remains"),
            b"healthy cache"
        );
        assert!(!lease.staging_path().exists());
        assert!(!staging_marker.exists());
    }

    #[test]
    fn stale_lease_cannot_delete_newer_streaming_cache() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_file = temp_dir.path().join("song.wav");
        let stale_lease = acquire_song_cache_lease(&audio_file);
        let stale_marker = pending_marker_path(stale_lease.staging_path());
        fs::write(stale_lease.staging_path(), b"old audio").expect("old staging");
        fs::write(&stale_marker, b"pending").expect("old marker");

        let current_lease = acquire_song_cache_lease(&audio_file);
        let current_marker = pending_marker_path(current_lease.staging_path());
        fs::write(current_lease.staging_path(), b"new audio").expect("new staging");
        fs::write(&current_marker, b"pending").expect("new marker");

        assert!(!remove_song_cache_if_current(&stale_lease).expect("stale cleanup"));
        assert!(!stale_lease.staging_path().exists());
        assert_eq!(
            fs::read(current_lease.staging_path()).expect("new cache remains"),
            b"new audio"
        );
        assert!(current_marker.exists());

        assert!(remove_song_cache_if_current(&current_lease).expect("current cleanup"));
        assert!(!current_lease.staging_path().exists());
        assert!(!current_marker.exists());
    }

    #[test]
    fn stale_lease_cannot_mark_newer_download_complete() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_file = temp_dir.path().join("song.wav");
        let stale_lease = acquire_song_cache_lease(&audio_file);
        let current_lease = acquire_song_cache_lease(&audio_file);
        let stale_marker = pending_marker_path(stale_lease.staging_path());
        let current_marker = pending_marker_path(current_lease.staging_path());
        fs::write(stale_lease.staging_path(), b"stale audio").expect("stale audio");
        fs::write(&stale_marker, b"pending").expect("stale marker");
        fs::write(current_lease.staging_path(), b"current audio").expect("current audio");
        fs::write(&current_marker, b"pending").expect("current marker");

        assert!(!complete_song_cache_if_current(&stale_lease).expect("stale completion"));
        assert!(!stale_lease.staging_path().exists());
        assert!(current_marker.exists());
        assert!(complete_song_cache_if_current(&current_lease).expect("current completion"));
        assert_eq!(
            fs::read(&audio_file).expect("promoted cache"),
            b"current audio"
        );
        assert!(!current_marker.exists());
    }

    #[test]
    fn delayed_old_cleanup_cannot_remove_new_generation_staging() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_file = temp_dir.path().join("song.wav");
        let old_lease = acquire_song_cache_lease(&audio_file);
        let old_marker = pending_marker_path(old_lease.staging_path());
        fs::write(old_lease.staging_path(), b"old").expect("old staging");
        fs::write(&old_marker, b"pending").expect("old marker");

        let barrier = Arc::new(Barrier::new(2));
        let cleanup_lease = old_lease.clone();
        let cleanup_barrier = Arc::clone(&barrier);
        let cleanup = thread::spawn(move || {
            cleanup_barrier.wait();
            remove_song_cache_if_current(&cleanup_lease).expect("old cleanup");
        });

        let new_lease = acquire_song_cache_lease(&audio_file);
        let new_marker = pending_marker_path(new_lease.staging_path());
        fs::write(new_lease.staging_path(), b"new").expect("new staging");
        fs::write(&new_marker, b"pending").expect("new marker");
        barrier.wait();
        cleanup.join().expect("cleanup thread");

        assert_eq!(
            fs::read(new_lease.staging_path()).expect("new survives"),
            b"new"
        );
        assert!(new_marker.exists());
    }

    #[test]
    fn delayed_old_writer_cannot_open_new_generation_file() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_file = temp_dir.path().join("song.wav");
        let old_lease = acquire_song_cache_lease(&audio_file);
        let barrier = Arc::new(Barrier::new(2));
        let old_path = old_lease.staging_path().to_path_buf();
        let writer_barrier = Arc::clone(&barrier);
        let writer = thread::spawn(move || {
            writer_barrier.wait();
            fs::write(&old_path, b"old bytes").expect("old writer");
        });

        let new_lease = acquire_song_cache_lease(&audio_file);
        fs::write(new_lease.staging_path(), b"new bytes").expect("new writer");
        barrier.wait();
        writer.join().expect("writer thread");

        assert_eq!(
            fs::read(new_lease.staging_path()).expect("new file remains isolated"),
            b"new bytes"
        );
        assert_eq!(
            fs::read(old_lease.staging_path()).expect("old file"),
            b"old bytes"
        );
    }

    #[test]
    fn old_completion_after_new_completion_cannot_replace_canonical_cache() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_file = temp_dir.path().join("song.wav");
        let old_lease = acquire_song_cache_lease(&audio_file);
        fs::write(old_lease.staging_path(), b"old complete").expect("old staging");
        let barrier = Arc::new(Barrier::new(2));
        let old_completion_lease = old_lease.clone();
        let completion_barrier = Arc::clone(&barrier);
        let old_completion = thread::spawn(move || {
            completion_barrier.wait();
            complete_song_cache_if_current(&old_completion_lease).expect("old completion")
        });

        let new_lease = acquire_song_cache_lease(&audio_file);
        fs::write(new_lease.staging_path(), b"new complete").expect("new staging");
        assert!(complete_song_cache_if_current(&new_lease).expect("new completion"));
        barrier.wait();
        assert!(!old_completion.join().expect("old completion thread"));
        assert_eq!(
            fs::read(&audio_file).expect("canonical cache"),
            b"new complete"
        );
    }

    #[test]
    fn active_cached_file_is_not_evicted_until_lease_is_released() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_file = temp_dir.path().join("song.wav");
        fs::write(&audio_file, vec![1_u8; 40]).expect("cache");
        let lease = acquire_song_cache_lease(&audio_file);

        let remaining = evict_cache_dir_until_target(temp_dir.path(), 20, 0).expect("eviction");
        assert_eq!(remaining, 40);
        assert!(audio_file.exists());

        drop(lease);
        let remaining = evict_cache_dir_until_target(temp_dir.path(), 20, 0).expect("eviction");
        assert_eq!(remaining, 0);
        assert!(!audio_file.exists());
    }
}
