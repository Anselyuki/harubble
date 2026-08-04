//! 播放输入探测、解码与样本缓冲处理。
//!
//! 该模块负责描述播放器输入来源、探测音频格式、启动后台解码线程，并通过样本缓冲
//! 在解码侧与播放后端之间传递 PCM 数据。

use crate::player::stream_helpers::*;
// 将从 stream_helpers 移出的纯函数与常量重新导出，确保同模块测试的 `super::` 引用保持不变。
pub(crate) use crate::player::stream_helpers::{
    ensure_decoded_format_matches_source, sanitize_pcm_sample, timestamp_delta_to_frames,
    SINC_RESAMPLE_CHUNK_FRAMES,
};
use anyhow::{Context, Result};
use crossbeam_queue::ArrayQueue;
use rubato::{Resampler, SincFixedIn};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// 播放解码线程上报致命错误时使用的回调类型。
pub type PlaybackErrorHandler = Arc<dyn Fn(String) + Send + Sync>;
use symphonia::core::audio::SampleBuffer as SymphoniaSampleBuffer;
use symphonia::core::codecs::{Decoder as SymphoniaDecoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, Track};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

/// 描述解码后或输出端期望的音频格式。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioFormat {
    /// 音频通道数，至少为 1。
    pub channels: u16,
    /// 采样率，单位为 Hz。
    pub sample_rate: u32,
    /// 音频总时长，单位为秒。
    pub duration_secs: f64,
    /// 源或输出 PCM 位深；未知时为 `None`。
    pub bits_per_sample: Option<u16>,
}

impl AudioFormat {
    /// 创建音频格式描述，并带上已知 PCM 位深。
    pub fn with_bits_per_sample(
        channels: u16,
        sample_rate: u32,
        duration_secs: f64,
        bits_per_sample: Option<u16>,
    ) -> Self {
        Self {
            channels,
            sample_rate,
            duration_secs,
            bits_per_sample,
        }
        .normalized()
    }

    /// 返回经过最小值归一化后的音频格式。
    ///
    /// 该方法会将通道数和采样率修正到至少为 `1`，并把时长裁剪到不小于 `0.0`。
    pub fn normalized(self) -> Self {
        Self {
            channels: self.channels.max(1),
            sample_rate: self.sample_rate.max(1),
            duration_secs: self.duration_secs.max(0.0),
            bits_per_sample: self.bits_per_sample,
        }
    }
}

/// 可被播放器读取的同步音频流抽象。
pub trait AudioReadStream: Read + Seek + Send + Sync {}
impl<T> AudioReadStream for T where T: Read + Seek + Send + Sync {}

type BoxedAudioReader = Box<dyn AudioReadStream>;

/// 播放器可消费的输入来源。
#[derive(Clone)]
pub enum PlaybackInput {
    /// 已完整缓存到本地磁盘的音频文件。
    CachedFile(PathBuf),
    /// 正在增长中的缓存文件句柄，适用于边下载边播放。
    GrowingFile(GrowingFileHandle),
}

impl PlaybackInput {
    /// 使用完整缓存文件构造播放输入。
    pub fn cached_file(path: PathBuf) -> Self {
        Self::CachedFile(path)
    }

    /// 使用增长中的缓存文件构造播放输入。
    pub fn growing_file(handle: GrowingFileHandle) -> Self {
        Self::GrowingFile(handle)
    }

    /// 探测当前输入的音频格式，并在读取器初始化或探测失败时自动重试。
    pub fn inspect_format_with_retry(
        &self,
        stop_flag: Option<Arc<AtomicBool>>,
    ) -> Result<AudioFormat> {
        inspect_audio_reader_with_retry(self, stop_flag)
    }

    /// 启动后台解码线程，将输入流转换为目标采样缓冲。
    ///
    /// `source_format` 为探测得到的源格式，`target_format` 为输出后端协商后的目标格式。
    /// 返回的线程句柄会在内部持续写入 `sample_buffer`，直到解码结束、出错或收到停止信号。
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_decode_worker(
        &self,
        source_format: AudioFormat,
        target_format: AudioFormat,
        sample_buffer: SampleBuffer,
        stop_flag: Arc<AtomicBool>,
        pause_flag: Arc<AtomicBool>,
        start_position_secs: f64,
        error_handler: PlaybackErrorHandler,
    ) -> Result<JoinHandle<()>> {
        let hint = self.build_hint();
        Ok(spawn_decode_worker(
            self.clone(),
            hint,
            source_format,
            target_format,
            sample_buffer,
            stop_flag,
            pause_flag,
            start_position_secs,
            error_handler,
        ))
    }

    fn open_reader(&self, stop_flag: Option<Arc<AtomicBool>>) -> Result<BoxedAudioReader> {
        match self {
            Self::CachedFile(path) => {
                let file = File::open(path).with_context(|| {
                    format!("Failed to open cached audio file {}", path.display())
                })?;
                Ok(Box::new(file))
            }
            Self::GrowingFile(handle) => Ok(Box::new(handle.open_reader(stop_flag)?)),
        }
    }

    fn build_hint(&self) -> Hint {
        let mut hint = Hint::new();
        let extension = match self {
            Self::CachedFile(path) => path.extension().and_then(|value| value.to_str()),
            Self::GrowingFile(handle) => handle.path().extension().and_then(|value| value.to_str()),
        };
        if let Some(extension) = extension {
            hint.with_extension(extension);
        }
        hint
    }
}

/// 供边下载边播放场景共享的增长文件句柄。
#[derive(Clone)]
pub struct GrowingFileHandle {
    path: PathBuf,
    state: Arc<(Mutex<GrowingFileState>, Condvar)>,
}

#[derive(Default)]
struct GrowingFileState {
    available_len: u64,
    expected_total_len: Option<u64>,
    complete: bool,
    error: Option<String>,
}

impl GrowingFileHandle {
    /// 创建新的增长文件句柄及其对应的写入文件。
    ///
    /// 调用方可持续向返回的 `File` 写入音频数据，并通过当前句柄向读取侧广播可读长度变化。
    pub fn new(path: PathBuf) -> Result<(Self, File)> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create parent directory {}", parent.display())
            })?;
        }

        let writer = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)
            .with_context(|| format!("Failed to create cache file {}", path.display()))?;

        Ok((
            Self {
                path,
                state: Arc::new((Mutex::new(GrowingFileState::default()), Condvar::new())),
            },
            writer,
        ))
    }

    /// 以读取端模式打开当前增长文件。
    ///
    /// 可选的 `stop_flag` 使读取端能在外部播放会话终止时中断阻塞等待；若传入
    /// `None`，则读取端仅依赖写入端的 `mark_complete` / `mark_error` 唤醒。
    pub fn open_reader(&self, stop_flag: Option<Arc<AtomicBool>>) -> Result<GrowingFileReader> {
        let file = OpenOptions::new()
            .read(true)
            .open(&self.path)
            .with_context(|| {
                format!(
                    "Failed to open streaming cache file {}",
                    self.path.display()
                )
            })?;
        Ok(GrowingFileReader {
            file,
            position: 0,
            state: Arc::clone(&self.state),
            stop_flag,
        })
    }

    /// 返回底层缓存文件路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 通知读取侧当前增长文件已经可读取到指定字节位置。
    pub fn publish_available_len(&self, len: u64) {
        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.available_len = len;
        condvar.notify_all();
    }

    /// 标记增长文件已完成写入。
    ///
    /// 调用后读取侧将不再等待新的可读长度。
    pub fn mark_complete(&self) {
        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.complete = true;
        condvar.notify_all();
    }

    /// 标记增长文件写入失败，并附带错误消息。
    ///
    /// 调用后读取侧将在下次读取或等待时收到错误。
    pub fn mark_error(&self, message: impl Into<String>) {
        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.error = Some(message.into());
        state.complete = true;
        condvar.notify_all();
    }

    /// 设置预期的文件总长度（来自 Content-Length）。
    ///
    /// 设置后 `SeekFrom::End` 将基于此值计算偏移，使解码器能正确推断流时长。
    pub fn set_expected_total_len(&self, len: u64) {
        let (lock, _) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.expected_total_len = Some(len);
    }
}

/// 增长文件的读取端。
///
/// 读取操作会在可读长度不足时阻塞等待，直到写入端追加数据或标记完成/失败。
/// 可选的 `stop_flag` 允许外部播放会话终止时中断阻塞的 condvar 等待，避免网络
/// 挂起场景下解码线程无限驻留。
pub struct GrowingFileReader {
    file: File,
    position: u64,
    state: Arc<(Mutex<GrowingFileState>, Condvar)>,
    stop_flag: Option<Arc<AtomicBool>>,
}

/// condvar 等待超时时间。
///
/// 即使写入端既未追加数据也未标记完成/失败，读取端也会每隔该间隔醒来检查
/// `stop_flag`，从而保证在播放会话已终止时能及时释放解码线程。
const GROWING_FILE_WAIT_TIMEOUT: Duration = Duration::from_millis(200);
/// 音频探测失败后的重试次数。
const AUDIO_PROBE_RETRY_ATTEMPTS: usize = 6;
/// 音频探测失败后每次重试前的等待间隔。
const AUDIO_PROBE_RETRY_DELAY: Duration = Duration::from_millis(120);

impl Read for GrowingFileReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();

        while state.error.is_none()
            && !state.complete
            && state.available_len.saturating_sub(self.position) < buf.len() as u64
        {
            if self
                .stop_flag
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::SeqCst))
            {
                return Err(io::Error::other("Playback stopped"));
            }
            let (next_state, _) = condvar
                .wait_timeout(state, GROWING_FILE_WAIT_TIMEOUT)
                .unwrap();
            state = next_state;
        }

        if let Some(error) = &state.error {
            return Err(io::Error::other(error.clone()));
        }

        if self
            .stop_flag
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::SeqCst))
        {
            return Err(io::Error::other("Playback stopped"));
        }

        if self.position >= state.available_len {
            return Ok(0);
        }

        let available = (state.available_len - self.position) as usize;
        drop(state);

        let read_len = available.min(buf.len());
        self.file.seek(SeekFrom::Start(self.position))?;
        let written = self.file.read(&mut buf[..read_len])?;
        self.position += written as u64;
        Ok(written)
    }
}

impl Seek for GrowingFileReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let next = match position {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::Current(offset) => self.position as i128 + offset as i128,
            SeekFrom::End(offset) => {
                let (lock, _) = &*self.state;
                let state = lock.lock().unwrap();
                if let Some(error) = &state.error {
                    return Err(io::Error::other(error.clone()));
                }
                let end = state.expected_total_len.unwrap_or(state.available_len);
                end as i128 + offset as i128
            }
        };

        if next < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek before start of stream",
            ));
        }

        self.position = next as u64;
        Ok(self.position)
    }
}

/// 解码线程与输出线程之间共享的采样缓冲区。
#[derive(Clone)]
pub struct SampleBuffer {
    inner: Arc<SampleBufferInner>,
}

const MAX_BUFFER_SAMPLES: usize = 192_000 * 2 * 15;
const SAMPLE_BUFFER_PUSH_CHUNK_SAMPLES: usize = 1024;

struct SampleBufferInner {
    queue: ArrayQueue<f32>,
    finished: AtomicBool,
    error_flag: AtomicBool,
    error: Mutex<Option<String>>,
    wait_lock: Mutex<()>,
    condvar: Condvar,
}

impl SampleBuffer {
    /// 创建一个空的采样缓冲区。
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SampleBufferInner {
                queue: ArrayQueue::new(MAX_BUFFER_SAMPLES),
                finished: AtomicBool::new(false),
                error_flag: AtomicBool::new(false),
                error: Mutex::new(None),
                wait_lock: Mutex::new(()),
                condvar: Condvar::new(),
            }),
        }
    }

    /// 返回采样缓冲区允许持有的最大样本数。
    pub fn max_capacity_samples() -> usize {
        MAX_BUFFER_SAMPLES
    }

    /// 追加一批已解码的浮点采样。
    ///
    /// 当缓冲区已满时会阻塞等待消费者消费后再写入。
    pub fn push(&self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        let mut offset = 0_usize;

        while offset < samples.len() {
            while self.inner.queue.is_full()
                && !self.inner.finished.load(Ordering::Acquire)
                && !self.inner.error_flag.load(Ordering::Acquire)
            {
                let guard = self.inner.wait_lock.lock().unwrap();
                let _ = self
                    .inner
                    .condvar
                    .wait_timeout(guard, Duration::from_millis(50))
                    .unwrap();
            }

            if self.inner.finished.load(Ordering::Acquire)
                || self.inner.error_flag.load(Ordering::Acquire)
            {
                self.inner.condvar.notify_all();
                return;
            }

            let end = (offset + SAMPLE_BUFFER_PUSH_CHUNK_SAMPLES).min(samples.len());
            while offset < end {
                match self.inner.queue.push(sanitize_pcm_sample(samples[offset])) {
                    Ok(()) => offset += 1,
                    Err(_) => break,
                }
            }
            self.inner.condvar.notify_all();
        }
    }

    /// 标记采样缓冲区不会再写入新的数据。
    pub fn finish(&self) {
        self.inner.finished.store(true, Ordering::Release);
        self.inner.condvar.notify_all();
    }

    /// 标记采样缓冲区失败并唤醒等待中的消费者。
    pub fn fail(&self, message: impl Into<String>) {
        *self.inner.error.lock().unwrap() = Some(message.into());
        self.inner.error_flag.store(true, Ordering::Release);
        self.inner.finished.store(true, Ordering::Release);
        self.inner.condvar.notify_all();
    }

    /// 从缓冲区中弹出尽可能多的完整声道帧写入 `output`。
    ///
    /// `frame_channels` 必须是当前输出格式的声道数。方法只会消费完整帧，避免把
    /// 不足一帧的样本弹出后静音丢弃，导致下一次回调从错误声道边界继续播放。
    /// 返回值会说明本次写入了多少采样，以及缓冲区是否已经结束或失败。
    #[cfg(test)]
    pub fn pop_complete_frames_into(&self, output: &mut [f32], frame_channels: usize) -> PopStatus {
        self.pop_frames_into(output, frame_channels, false)
    }

    /// 尝试从缓冲区弹出完整声道帧。
    ///
    /// 该方法供实时输出回调使用；内部使用 lock-free 有界队列，正常路径不会因为
    /// 解码线程写入而阻塞音频 callback。
    #[cfg(test)]
    pub fn try_pop_complete_frames_into(
        &self,
        output: &mut [f32],
        frame_channels: usize,
    ) -> Option<PopStatus> {
        Some(self.pop_frames_into(output, frame_channels, false))
    }

    /// 尝试弹出一整个实时输出 callback 所需的完整声道帧。
    ///
    /// 如果缓冲区当前不足以填满本次 callback，且生产端尚未结束，该方法不会消费已有尾部样本，
    /// 而是返回 `written = 0`。调用方可据此输出整段静音并触发重新缓冲，避免“半段音频 + 半段静音”
    /// 的硬切边界造成爆音。
    pub fn try_pop_realtime_frames_into(
        &self,
        output: &mut [f32],
        frame_channels: usize,
    ) -> Option<PopStatus> {
        Some(self.pop_frames_into(output, frame_channels, true))
    }

    fn pop_frames_into(
        &self,
        output: &mut [f32],
        frame_channels: usize,
        require_full_callback: bool,
    ) -> PopStatus {
        if let Some(error) = self.current_error() {
            return PopStatus {
                written: 0,
                finished: true,
                error: Some(error),
            };
        }

        let frame_channels = frame_channels.max(1);
        let writable_samples = output.len() - (output.len() % frame_channels);
        let available_samples = self.inner.queue.len();
        let available_complete_samples = available_samples - (available_samples % frame_channels);

        if require_full_callback
            && !self.inner.finished.load(Ordering::Acquire)
            && available_complete_samples < writable_samples
        {
            return PopStatus {
                written: 0,
                finished: false,
                error: self.current_error(),
            };
        }

        let target_samples = writable_samples.min(available_complete_samples);
        let mut written = 0_usize;
        while written < target_samples {
            match self.inner.queue.pop() {
                Some(sample) => {
                    output[written] = sample;
                    written += 1;
                }
                None => break,
            }
        }

        if written == 0 && self.inner.finished.load(Ordering::Acquire) && available_samples > 0 {
            while self.inner.queue.pop().is_some() {}
        }

        if written > 0 {
            self.inner.condvar.notify_all();
        }

        PopStatus {
            written,
            finished: self.inner.finished.load(Ordering::Acquire) && self.inner.queue.is_empty(),
            error: self.current_error(),
        }
    }

    /// 等待缓冲区中至少出现指定数量的采样。
    ///
    /// 当缓冲区报错、播放被停止，或流结束时仍没有任何可播放采样时返回错误。
    pub fn wait_for_samples(&self, minimum_samples: usize, stop_flag: &AtomicBool) -> Result<()> {
        match self.wait_for_samples_or_end(minimum_samples, stop_flag)? {
            SampleWaitOutcome::Ready => Ok(()),
            SampleWaitOutcome::Ended => {
                anyhow::bail!("Audio stream ended before playback could start")
            }
        }
    }

    /// 等待缓冲区中至少出现指定数量的采样，或等待生产端自然结束。
    pub fn wait_for_samples_or_end(
        &self,
        minimum_samples: usize,
        stop_flag: &AtomicBool,
    ) -> Result<SampleWaitOutcome> {
        while self.inner.queue.len() < minimum_samples
            && !self.inner.finished.load(Ordering::Acquire)
            && !self.inner.error_flag.load(Ordering::Acquire)
            && !stop_flag.load(Ordering::SeqCst)
        {
            let guard = self.inner.wait_lock.lock().unwrap();
            let _ = self
                .inner
                .condvar
                .wait_timeout(guard, Duration::from_millis(50))
                .unwrap();
        }

        if let Some(error) = self.current_error() {
            anyhow::bail!(error);
        }
        if stop_flag.load(Ordering::SeqCst) {
            anyhow::bail!("Playback stopped");
        }
        if self.inner.finished.load(Ordering::Acquire) && self.inner.queue.is_empty() {
            return Ok(SampleWaitOutcome::Ended);
        }
        Ok(SampleWaitOutcome::Ready)
    }

    #[cfg(test)]
    pub(crate) fn hold_lock_for_test(&self, duration: Duration) {
        thread::sleep(duration);
    }

    fn current_error(&self) -> Option<String> {
        if self.inner.error_flag.load(Ordering::Acquire) {
            self.inner.error.lock().unwrap().clone()
        } else {
            None
        }
    }
}

/// 等待采样缓冲区填充时的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleWaitOutcome {
    /// 已经达到调用方要求的最小采样数，或仍有可播放尾部采样。
    Ready,
    /// 生产端已自然结束且缓冲区已空。
    Ended,
}

/// 一次从采样缓冲区弹出后的结果摘要。
pub struct PopStatus {
    /// 本次实际写入输出缓冲区的采样数。
    pub written: usize,
    /// 当前缓冲区是否已经完全结束且没有剩余采样。
    pub finished: bool,
    /// 若生产端失败，这里携带对应错误消息。
    pub error: Option<String>,
}

struct SymphoniaSource {
    inner: BoxedAudioReader,
}

impl Read for SymphoniaSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl Seek for SymphoniaSource {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek(position)
    }
}

impl MediaSource for SymphoniaSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

struct OpenedAudioReader {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn SymphoniaDecoder>,
    track_id: u32,
    audio_format: AudioFormat,
}

fn inspect_audio_reader_with_retry(
    input: &PlaybackInput,
    stop_flag: Option<Arc<AtomicBool>>,
) -> Result<AudioFormat> {
    let hint = input.build_hint();
    open_audio_reader_with_retry(
        || input.open_reader(stop_flag.clone()),
        &hint,
        stop_flag.as_deref(),
    )
    .map(|reader| reader.audio_format)
}

#[allow(clippy::too_many_arguments)]
fn spawn_decode_worker(
    input: PlaybackInput,
    hint: Hint,
    source_format: AudioFormat,
    target_format: AudioFormat,
    sample_buffer: SampleBuffer,
    stop_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    start_position_secs: f64,
    error_handler: PlaybackErrorHandler,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("audio-decode-worker".into())
        .spawn(move || {
            let result = (|| -> Result<()> {
                let OpenedAudioReader {
                    mut format,
                    mut decoder,
                    track_id,
                    ..
                } = open_audio_reader_with_retry(
                    || input.open_reader(Some(Arc::clone(&stop_flag))),
                    &hint,
                    Some(&stop_flag),
                )?;
                let mut converter = SampleConverter::new(source_format, target_format);
                let mut decoded_samples: Option<SymphoniaSampleBuffer<f32>> = None;
                let mut remaining_seek_frames = if start_position_secs > 0.0 {
                    let frames = seek_to_time(
                        format.as_mut(),
                        track_id,
                        start_position_secs,
                        source_format.sample_rate,
                    )?;
                    decoder.reset();
                    frames
                } else {
                    0
                };

                loop {
                    while pause_flag.load(Ordering::SeqCst) && !stop_flag.load(Ordering::SeqCst) {
                        thread::sleep(Duration::from_millis(50));
                    }

                    if stop_flag.load(Ordering::SeqCst) {
                        break;
                    }

                    let packet = match format.next_packet() {
                        Ok(packet) => packet,
                        Err(SymphoniaError::IoError(error))
                            if error.kind() == io::ErrorKind::UnexpectedEof =>
                        {
                            break;
                        }
                        Err(SymphoniaError::ResetRequired) => {
                            anyhow::bail!("Audio decoder reset required");
                        }
                        Err(error) => return Err(error).context("Failed to read audio packet"),
                    };

                    if packet.track_id() != track_id {
                        continue;
                    }

                    match decoder.decode(&packet) {
                        Ok(audio_buf) => {
                            ensure_decoded_format_matches_source(audio_buf.spec(), source_format)?;
                            let required_samples =
                                audio_buf.capacity() * audio_buf.spec().channels.count();
                            let channels = audio_buf.spec().channels.count();
                            if decoded_samples
                                .as_ref()
                                .is_none_or(|buffer| buffer.capacity() < required_samples)
                            {
                                decoded_samples = Some(SymphoniaSampleBuffer::<f32>::new(
                                    audio_buf.capacity() as u64,
                                    *audio_buf.spec(),
                                ));
                            }

                            let buffer = decoded_samples
                                .as_mut()
                                .expect("decoded sample buffer must exist");
                            buffer.clear();
                            buffer.copy_interleaved_ref(audio_buf);

                            let available_frames = (buffer.samples().len() / channels) as u64;
                            let skip_frames = remaining_seek_frames.min(available_frames);
                            remaining_seek_frames -= skip_frames;
                            let skip_samples = skip_frames as usize * channels;
                            let samples = &buffer.samples()[skip_samples..];
                            if samples.is_empty() {
                                continue;
                            }

                            let converted = converter.push_chunk(samples);
                            sample_buffer.push(&converted);
                        }
                        Err(SymphoniaError::DecodeError(_)) => continue,
                        Err(SymphoniaError::IoError(error))
                            if error.kind() == io::ErrorKind::UnexpectedEof =>
                        {
                            break;
                        }
                        Err(SymphoniaError::ResetRequired) => {
                            anyhow::bail!("Audio decoder reset required");
                        }
                        Err(error) => return Err(error).context("Failed to decode audio packet"),
                    }
                }

                let converted = converter.finish();
                sample_buffer.push(&converted);
                sample_buffer.finish();
                Ok(())
            })();

            if let Err(error) = result {
                let message = format!("{error:#}");
                sample_buffer.fail(message.clone());
                error_handler(message);
            }
        })
        .expect("Failed to spawn audio decode worker")
}

fn seek_to_time(
    format: &mut dyn FormatReader,
    track_id: u32,
    seconds: f64,
    sample_rate: u32,
) -> Result<u64> {
    let seek_result = format
        .seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: Time::from(seconds.max(0.0)),
                track_id: Some(track_id),
            },
        )
        .context("Failed to seek audio stream")?;

    let timestamp_delta = seek_result
        .required_ts
        .saturating_sub(seek_result.actual_ts);
    let track = format
        .tracks()
        .iter()
        .find(|track| track.id == seek_result.track_id)
        .context("Seeked audio track is no longer available")?;
    let time_base = track
        .codec_params
        .time_base
        .context("Missing audio time base after seek")?;
    Ok(timestamp_delta_to_frames(
        timestamp_delta,
        time_base,
        sample_rate,
    ))
}

fn open_audio_reader_with_retry<F>(
    mut open_reader: F,
    hint: &Hint,
    stop_flag: Option<&AtomicBool>,
) -> Result<OpenedAudioReader>
where
    F: FnMut() -> Result<BoxedAudioReader>,
{
    let mut last_error: Option<anyhow::Error> = None;

    for attempt in 0..AUDIO_PROBE_RETRY_ATTEMPTS {
        if stop_flag.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
            anyhow::bail!("Playback stopped");
        }

        match open_reader().and_then(|reader| open_audio_reader(reader, hint)) {
            Ok(reader) => return Ok(reader),
            Err(error) => {
                last_error = Some(error);
                if attempt + 1 == AUDIO_PROBE_RETRY_ATTEMPTS {
                    break;
                }
            }
        }

        if stop_flag.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
            anyhow::bail!("Playback stopped");
        }

        thread::sleep(AUDIO_PROBE_RETRY_DELAY);
    }

    Err(last_error.expect("audio probe retry should record an error"))
        .context("Failed to probe audio stream after retries")
}

fn open_audio_reader(reader: BoxedAudioReader, hint: &Hint) -> Result<OpenedAudioReader> {
    let media_source = Box::new(SymphoniaSource { inner: reader });
    let media_source_stream = MediaSourceStream::new(media_source, Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            hint,
            media_source_stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .context("Failed to probe audio stream")?;

    let format = probed.format;
    let track = select_track(&*format)?;
    let codec_params = track.codec_params.clone();
    let track_id = track.id;
    let audio_format = audio_format_from_codec_params(&codec_params)?;
    let decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .context("Failed to create audio decoder")?;

    Ok(OpenedAudioReader {
        format,
        decoder,
        track_id,
        audio_format,
    })
}

fn select_track(format: &dyn FormatReader) -> Result<&Track> {
    format
        .default_track()
        .or_else(|| {
            format
                .tracks()
                .iter()
                .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
        })
        .context("No supported audio track found")
}

struct SampleConverter {
    source_channels: usize,
    target_channels: usize,
    source_rate: u32,
    target_rate: u32,
    pending: Vec<f32>,
    pending_base_frame: u64,
    next_source_frame: f64,
    sinc_resampler: Option<SincFixedIn<f32>>,
}

impl SampleConverter {
    fn new(source_format: AudioFormat, target_format: AudioFormat) -> Self {
        let source_channels = source_format.channels.max(1) as usize;
        let target_channels = target_format.channels.max(1) as usize;
        let source_rate = source_format.sample_rate.max(1);
        let target_rate = target_format.sample_rate.max(1);
        let sinc_resampler = if source_rate == target_rate {
            None
        } else {
            create_sinc_resampler(source_rate, target_rate, target_channels)
        };
        Self {
            source_channels,
            target_channels,
            source_rate,
            target_rate,
            pending: Vec::new(),
            pending_base_frame: 0,
            next_source_frame: 0.0,
            sinc_resampler,
        }
    }

    fn push_chunk(&mut self, samples: &[f32]) -> Vec<f32> {
        self.pending
            .extend(samples.iter().copied().map(sanitize_pcm_sample));
        self.drain_available(false)
    }

    fn finish(&mut self) -> Vec<f32> {
        self.drain_available(true)
    }

    fn drain_available(&mut self, finalizing: bool) -> Vec<f32> {
        let available_frames = self.pending.len() / self.source_channels;
        if available_frames == 0 {
            return Vec::new();
        }

        if self.source_rate == self.target_rate {
            let mut output = Vec::with_capacity(available_frames * self.target_channels);
            for frame in 0..available_frames {
                self.push_remixed_frame(frame, &mut output);
            }
            let consumed_samples = available_frames * self.source_channels;
            self.pending.drain(..consumed_samples);
            self.pending_base_frame += available_frames as u64;
            return output;
        }

        if self.sinc_resampler.is_some() {
            return self.drain_available_with_sinc(finalizing, available_frames);
        }

        self.drain_available_with_linear_interpolation(finalizing, available_frames)
    }

    fn drain_available_with_sinc(&mut self, finalizing: bool, available_frames: usize) -> Vec<f32> {
        let frames_to_process = if finalizing {
            available_frames
        } else {
            available_frames - (available_frames % SINC_RESAMPLE_CHUNK_FRAMES)
        };
        if frames_to_process == 0 {
            return Vec::new();
        }

        let mut output = Vec::new();
        let mut processed_frames = 0_usize;

        while processed_frames + SINC_RESAMPLE_CHUNK_FRAMES <= frames_to_process {
            let planar = self.build_remixed_planar(processed_frames, SINC_RESAMPLE_CHUNK_FRAMES);
            let output_planar = match self.sinc_resampler.as_mut() {
                Some(resampler) => resampler.process(&planar, None),
                None => unreachable!("sinc resampler checked by caller"),
            };
            match output_planar {
                Ok(planar) => output.extend(interleave_planar(&planar)),
                Err(_) => {
                    return self
                        .drain_available_with_linear_interpolation(finalizing, available_frames);
                }
            }
            processed_frames += SINC_RESAMPLE_CHUNK_FRAMES;
        }

        if finalizing && processed_frames < frames_to_process {
            let remaining_frames = frames_to_process - processed_frames;
            let planar = self.build_remixed_planar(processed_frames, remaining_frames);
            let output_planar = match self.sinc_resampler.as_mut() {
                Some(resampler) => resampler.process_partial(Some(&planar), None),
                None => unreachable!("sinc resampler checked by caller"),
            };
            match output_planar {
                Ok(planar) => output.extend(interleave_planar(&planar)),
                Err(_) => {
                    return self
                        .drain_available_with_linear_interpolation(finalizing, available_frames);
                }
            }
            processed_frames = frames_to_process;
        }

        let samples_to_drop = processed_frames * self.source_channels;
        self.pending.drain(..samples_to_drop);
        self.pending_base_frame += processed_frames as u64;

        output
    }

    fn drain_available_with_linear_interpolation(
        &mut self,
        finalizing: bool,
        available_frames: usize,
    ) -> Vec<f32> {
        let mut output = Vec::new();
        let ratio = self.source_rate as f64 / self.target_rate as f64;
        let available_abs_frames = self.pending_base_frame + available_frames as u64;

        loop {
            let source_frame_abs = self.next_source_frame.floor() as u64;
            if source_frame_abs >= available_abs_frames {
                break;
            }
            if !finalizing && source_frame_abs + 1 >= available_abs_frames {
                break;
            }
            let local_frame = (source_frame_abs - self.pending_base_frame) as usize;
            if local_frame >= available_frames {
                break;
            }
            let next_local_frame = (local_frame + 1).min(available_frames - 1);
            let fraction = self.next_source_frame - source_frame_abs as f64;
            self.push_interpolated_frame(
                local_frame,
                next_local_frame,
                fraction as f32,
                &mut output,
            );
            self.next_source_frame += ratio;
        }

        let frames_to_drop = if finalizing {
            available_frames
        } else {
            let keep_from = self.next_source_frame.floor() as u64;
            keep_from.saturating_sub(self.pending_base_frame) as usize
        };

        let frames_to_drop = frames_to_drop.min(available_frames);
        let samples_to_drop = frames_to_drop * self.source_channels;
        self.pending.drain(..samples_to_drop);
        self.pending_base_frame += frames_to_drop as u64;

        output
    }

    fn push_remixed_frame(&self, source_frame: usize, output: &mut Vec<f32>) {
        let base = source_frame * self.source_channels;

        if self.source_channels == self.target_channels {
            output.extend_from_slice(&self.pending[base..base + self.source_channels]);
            return;
        }

        if self.source_channels == 1 {
            let sample = self.pending[base];
            output.extend(std::iter::repeat_n(sample, self.target_channels));
            return;
        }

        if self.target_channels == 1 {
            let sum = self.pending[base..base + self.source_channels]
                .iter()
                .copied()
                .sum::<f32>();
            output.push(sum / self.source_channels as f32);
            return;
        }

        for target_channel in 0..self.target_channels {
            let mapped = target_channel.min(self.source_channels - 1);
            output.push(self.pending[base + mapped]);
        }
    }

    fn build_remixed_planar(&self, start_frame: usize, frame_count: usize) -> Vec<Vec<f32>> {
        let mut planar = vec![Vec::with_capacity(frame_count); self.target_channels];
        for source_frame in start_frame..start_frame + frame_count {
            self.push_remixed_frame_to_planar(source_frame, &mut planar);
        }
        planar
    }

    fn push_remixed_frame_to_planar(&self, source_frame: usize, output: &mut [Vec<f32>]) {
        let base = source_frame * self.source_channels;

        if self.source_channels == self.target_channels {
            for (channel, target) in output.iter_mut().enumerate().take(self.target_channels) {
                target.push(self.pending[base + channel]);
            }
            return;
        }

        if self.source_channels == 1 {
            let sample = self.pending[base];
            for target in output.iter_mut().take(self.target_channels) {
                target.push(sample);
            }
            return;
        }

        if self.target_channels == 1 {
            let sum = self.pending[base..base + self.source_channels]
                .iter()
                .copied()
                .sum::<f32>();
            output[0].push(sum / self.source_channels as f32);
            return;
        }

        for (target_channel, target) in output.iter_mut().enumerate().take(self.target_channels) {
            let mapped = target_channel.min(self.source_channels - 1);
            target.push(self.pending[base + mapped]);
        }
    }

    fn push_interpolated_frame(
        &self,
        source_frame: usize,
        next_source_frame: usize,
        fraction: f32,
        output: &mut Vec<f32>,
    ) {
        if source_frame == next_source_frame {
            self.push_remixed_frame(source_frame, output);
            return;
        }

        if self.source_channels == self.target_channels {
            for channel in 0..self.source_channels {
                output.push(self.interpolated_sample(
                    source_frame,
                    next_source_frame,
                    channel,
                    fraction,
                ));
            }
            return;
        }

        if self.source_channels == 1 {
            let sample = self.interpolated_sample(source_frame, next_source_frame, 0, fraction);
            output.extend(std::iter::repeat_n(sample, self.target_channels));
            return;
        }

        if self.target_channels == 1 {
            let sum = (0..self.source_channels)
                .map(|channel| {
                    self.interpolated_sample(source_frame, next_source_frame, channel, fraction)
                })
                .sum::<f32>();
            output.push(sum / self.source_channels as f32);
            return;
        }

        for target_channel in 0..self.target_channels {
            let mapped = target_channel.min(self.source_channels - 1);
            output.push(self.interpolated_sample(
                source_frame,
                next_source_frame,
                mapped,
                fraction,
            ));
        }
    }

    fn interpolated_sample(
        &self,
        source_frame: usize,
        next_source_frame: usize,
        channel: usize,
        fraction: f32,
    ) -> f32 {
        let current = self.pending[source_frame * self.source_channels + channel];
        let next = self.pending[next_source_frame * self.source_channels + channel];
        sanitize_pcm_sample(current + (next - current) * fraction)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_decoded_format_matches_source, open_audio_reader, open_audio_reader_with_retry,
        timestamp_delta_to_frames, AudioFormat, BoxedAudioReader, GrowingFileHandle, Hint,
        SampleBuffer, SampleConverter, SampleWaitOutcome, SymphoniaSampleBuffer,
        SINC_RESAMPLE_CHUNK_FRAMES,
    };
    use std::fs::File;
    use std::io::{self, Read, Seek, SeekFrom, Write};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;
    use symphonia::core::audio::{Channels, SignalSpec};
    use symphonia::core::units::TimeBase;
    use tempfile::tempdir;

    struct FlakyReader {
        inner: File,
        fail_once: AtomicBool,
    }

    impl FlakyReader {
        fn new(inner: File) -> Self {
            Self {
                inner,
                fail_once: AtomicBool::new(true),
            }
        }

        fn maybe_fail(&self) -> io::Result<()> {
            if self.fail_once.swap(false, Ordering::SeqCst) {
                Err(io::Error::other("transient probe failure"))
            } else {
                Ok(())
            }
        }
    }

    impl Read for FlakyReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.maybe_fail()?;
            self.inner.read(buf)
        }
    }

    impl Seek for FlakyReader {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            self.maybe_fail()?;
            self.inner.seek(position)
        }
    }

    #[test]
    fn growing_file_reader_waits_for_requested_read_size_before_completion() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_path = temp_dir.path().join("growing-read.bin");
        let (handle, mut writer) = GrowingFileHandle::new(audio_path).expect("growing file");
        writer.write_all(b"abcde").expect("first chunk");
        writer.flush().expect("flush first chunk");
        handle.publish_available_len(5);

        let mut reader = handle.open_reader(None).expect("reader");
        let (tx, rx) = std::sync::mpsc::channel();
        let reader_thread = thread::spawn(move || {
            let mut buf = [0_u8; 10];
            let read = reader.read(&mut buf).expect("read should complete");
            tx.send((read, buf)).expect("send read result");
        });

        assert!(rx.recv_timeout(Duration::from_millis(80)).is_err());

        writer.write_all(b"fghij").expect("second chunk");
        writer.flush().expect("flush second chunk");
        handle.publish_available_len(10);

        let (read, buf) = rx
            .recv_timeout(Duration::from_secs(1))
            .expect("reader should finish once requested bytes are available");
        assert_eq!(read, 10);
        assert_eq!(&buf, b"abcdefghij");
        reader_thread.join().expect("reader thread");
    }

    #[test]
    fn audio_probe_retries_after_transient_failure() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_path = temp_dir.path().join("probe-test.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&audio_path, spec).expect("wav writer");
        writer.write_sample(0i16).expect("sample");
        writer.write_sample(1024i16).expect("sample");
        writer.finalize().expect("finalize");

        let attempts = AtomicUsize::new(0);
        let mut hint = Hint::new();
        hint.with_extension("wav");

        let opened = open_audio_reader_with_retry(
            || {
                let attempt = attempts.fetch_add(1, Ordering::SeqCst);
                let file = File::open(&audio_path).expect("open audio file");
                if attempt == 0 {
                    Ok(Box::new(FlakyReader::new(file)) as BoxedAudioReader)
                } else {
                    Ok(Box::new(file) as BoxedAudioReader)
                }
            },
            &hint,
            None,
        )
        .expect("probe should recover");

        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        assert_eq!(opened.audio_format.channels, 1);
        assert_eq!(opened.audio_format.sample_rate, 44_100);
    }

    #[test]
    fn sample_converter_uses_sinc_resampler_for_rate_conversion() {
        let mut converter = SampleConverter::new(
            AudioFormat {
                channels: 1,
                sample_rate: 48_000,
                duration_secs: 0.0,
                bits_per_sample: None,
            },
            AudioFormat {
                channels: 1,
                sample_rate: 44_100,
                duration_secs: 0.0,
                bits_per_sample: None,
            },
        );

        let input = (0..SINC_RESAMPLE_CHUNK_FRAMES)
            .map(|frame| ((frame as f32 / 32.0).sin() * 0.5).clamp(-1.0, 1.0))
            .collect::<Vec<_>>();
        let output = converter.push_chunk(&input);

        assert!(!output.is_empty());
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.iter().all(|sample| (-1.0..=1.0).contains(sample)));
        assert_ne!(output, input);
    }

    #[test]
    fn sample_converter_sanitizes_non_finite_and_out_of_range_samples() {
        let mut converter = SampleConverter::new(
            AudioFormat {
                channels: 1,
                sample_rate: 1,
                duration_secs: 0.0,
                bits_per_sample: None,
            },
            AudioFormat {
                channels: 1,
                sample_rate: 1,
                duration_secs: 0.0,
                bits_per_sample: None,
            },
        );

        let output = converter.push_chunk(&[f32::NAN, f32::INFINITY, -2.0, 0.5]);

        assert_eq!(output, vec![0.0, 0.0, -1.0, 0.5]);
    }

    #[test]
    fn decoded_format_must_match_probed_source_format() {
        let source = AudioFormat {
            channels: 2,
            sample_rate: 48_000,
            duration_secs: 10.0,
            bits_per_sample: None,
        };
        let stereo = SignalSpec::new(48_000, Channels::FRONT_LEFT | Channels::FRONT_RIGHT);
        let mono = SignalSpec::new(48_000, Channels::FRONT_LEFT);
        let wrong_rate = SignalSpec::new(44_100, Channels::FRONT_LEFT | Channels::FRONT_RIGHT);

        ensure_decoded_format_matches_source(&stereo, source).expect("matching format");

        let channel_error =
            ensure_decoded_format_matches_source(&mono, source).expect_err("channel drift");
        assert!(
            format!("{channel_error:#}").contains("Decoded audio channel count changed"),
            "{channel_error:#}"
        );

        let rate_error =
            ensure_decoded_format_matches_source(&wrong_rate, source).expect_err("rate drift");
        assert!(
            format!("{rate_error:#}").contains("Decoded audio sample rate changed"),
            "{rate_error:#}"
        );
    }

    #[test]
    fn seek_timestamp_delta_is_converted_to_audio_frames() {
        let time_base = TimeBase::new(1, 1_000);

        assert_eq!(timestamp_delta_to_frames(250, time_base, 48_000), 12_000);
    }

    #[test]
    fn sample_buffer_rejects_push_after_finish() {
        let buffer = SampleBuffer::new();
        buffer.finish();
        buffer.push(&[1.0, 0.5]);

        let mut output = [0.0_f32; 2];
        let status = buffer.pop_complete_frames_into(&mut output, 2);

        assert_eq!(status.written, 0);
        assert!(status.finished);
        assert_eq!(output, [0.0, 0.0]);
    }

    #[test]
    fn sample_buffer_sanitizes_samples_before_queueing() {
        let buffer = SampleBuffer::new();
        buffer.push(&[f32::NAN, f32::NEG_INFINITY, 1.25, -0.25]);

        let mut output = [1.0_f32; 4];
        let status = buffer.pop_complete_frames_into(&mut output, 2);

        assert_eq!(status.written, 4);
        assert_eq!(output, [0.0, 0.0, 1.0, -0.25]);
    }

    #[test]
    fn sample_buffer_keeps_incomplete_frames_for_later_completion() {
        let buffer = SampleBuffer::new();
        buffer.push(&[0.25]);

        let mut output = [1.0_f32; 2];
        let status = buffer.pop_complete_frames_into(&mut output, 2);

        assert_eq!(status.written, 0);
        assert_eq!(output, [1.0, 1.0]);

        buffer.push(&[-0.25]);
        let status = buffer.pop_complete_frames_into(&mut output, 2);

        assert_eq!(status.written, 2);
        assert_eq!(output, [0.25, -0.25]);
    }

    #[test]
    fn sample_buffer_drops_final_incomplete_frame_as_silence() {
        let buffer = SampleBuffer::new();
        buffer.push(&[0.25]);
        buffer.finish();

        let mut output = [1.0_f32; 2];
        let status = buffer.pop_complete_frames_into(&mut output, 2);

        assert_eq!(status.written, 0);
        assert!(status.finished);
        assert_eq!(output, [1.0, 1.0]);
    }

    #[test]
    fn sample_buffer_try_pop_is_lock_free_for_realtime_reader() {
        let buffer = SampleBuffer::new();
        buffer.push(&[0.25, -0.25]);
        let locked_buffer = buffer.clone();

        let handle = thread::spawn(move || {
            locked_buffer.hold_lock_for_test(Duration::from_millis(100));
        });
        thread::sleep(Duration::from_millis(10));

        let mut output = [1.0_f32; 2];
        let status = buffer
            .try_pop_complete_frames_into(&mut output, 2)
            .expect("ring buffer reader should not wait on a sample mutex");
        assert_eq!(status.written, 2);
        assert_eq!(output, [0.25, -0.25]);

        handle.join().expect("lock holder should finish");
    }

    #[test]
    fn sample_buffer_fail_discards_queued_samples() {
        let buffer = SampleBuffer::new();
        buffer.push(&[0.25, -0.25]);
        buffer.fail("decode failed");

        let mut output = [1.0_f32; 2];
        let status = buffer.pop_complete_frames_into(&mut output, 2);

        assert_eq!(status.written, 0);
        assert!(status.finished);
        assert_eq!(status.error.as_deref(), Some("decode failed"));
        assert_eq!(output, [1.0, 1.0]);
    }

    #[test]
    fn sample_buffer_wait_can_report_natural_end() {
        let buffer = SampleBuffer::new();
        let stop_flag = AtomicBool::new(false);
        buffer.finish();

        let outcome = buffer
            .wait_for_samples_or_end(1, &stop_flag)
            .expect("natural end should be reported");

        assert_eq!(outcome, SampleWaitOutcome::Ended);
    }

    #[test]
    fn sample_buffer_wait_reports_buffer_error_before_stop_flag() {
        let buffer = SampleBuffer::new();
        let stop_flag = AtomicBool::new(true);
        buffer.fail("decode failed");

        let error = buffer
            .wait_for_samples(1, &stop_flag)
            .expect_err("buffer error");

        assert!(format!("{error:#}").contains("decode failed"), "{error:#}");
    }

    #[test]
    fn sample_buffer_push_respects_capacity_with_large_chunks() {
        let buffer = SampleBuffer::new();
        let producer = buffer.clone();
        let total_samples = SampleBuffer::max_capacity_samples() + 512;
        let samples = vec![0.25_f32; total_samples];

        let handle = thread::spawn(move || {
            producer.push(&samples);
        });

        let mut drained = 0_usize;
        let mut output = vec![0.0_f32; 1024];
        while drained < total_samples {
            let status = buffer.pop_complete_frames_into(&mut output, 2);
            drained += status.written;
            if status.written == 0 {
                thread::sleep(Duration::from_millis(5));
            }
        }

        handle.join().expect("producer should finish");
        assert_eq!(drained, total_samples);
    }

    #[test]
    fn decode_24bit_wav_preserves_low_amplitude_samples() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_path = temp_dir.path().join("decode-24bit.wav");
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&audio_path, spec).expect("wav writer");
        writer.write_sample(0_i32).expect("sample");
        writer.write_sample(256_i32).expect("sample");
        writer.write_sample(-256_i32).expect("sample");
        writer.write_sample(512_i32).expect("sample");
        writer.finalize().expect("finalize");

        let mut hint = Hint::new();
        hint.with_extension("wav");

        let reader = Box::new(File::open(&audio_path).expect("open audio file"));
        let mut opened = open_audio_reader(reader, &hint).expect("open reader");
        let packet = opened.format.next_packet().expect("packet");
        let audio_buf = opened.decoder.decode(&packet).expect("decode");

        let mut decoded =
            SymphoniaSampleBuffer::<f32>::new(audio_buf.capacity() as u64, *audio_buf.spec());
        decoded.copy_interleaved_ref(audio_buf);

        let samples = decoded.samples();
        assert_eq!(samples.len(), 4);
        assert!(samples[0].abs() < 0.000_001);
        assert!((samples[1] - 0.000_030_517_578).abs() < 0.000_01);
        assert!((samples[2] + 0.000_030_517_578).abs() < 0.000_01);
        assert!((samples[3] - 0.000_061_035_156).abs() < 0.000_01);
    }

    #[test]
    fn decode_manual_24bit_pcm_wav_preserves_sample_scale() {
        let temp_dir = tempdir().expect("tempdir");
        let audio_path = temp_dir.path().join("manual-24bit.wav");
        std::fs::write(&audio_path, pcm24_wav_bytes(&[0, 256, -256, 512])).expect("write wav");

        let mut hint = Hint::new();
        hint.with_extension("wav");

        let reader = Box::new(File::open(&audio_path).expect("open audio file"));
        let mut opened = open_audio_reader(reader, &hint).expect("open reader");
        let packet = opened.format.next_packet().expect("packet");
        let audio_buf = opened.decoder.decode(&packet).expect("decode");

        let mut decoded =
            SymphoniaSampleBuffer::<f32>::new(audio_buf.capacity() as u64, *audio_buf.spec());
        decoded.copy_interleaved_ref(audio_buf);

        let samples = decoded.samples();
        assert_eq!(samples.len(), 4);
        assert!(samples[0].abs() < 0.000_001);
        assert!((samples[1] - 0.000_030_517_578).abs() < 0.000_01);
        assert!((samples[2] + 0.000_030_517_578).abs() < 0.000_01);
        assert!((samples[3] - 0.000_061_035_156).abs() < 0.000_01);
    }

    fn pcm24_wav_bytes(samples: &[i32]) -> Vec<u8> {
        let mut data = Vec::with_capacity(samples.len() * 3);
        for sample in samples {
            let raw = (*sample as u32) & 0x00ff_ffff;
            data.extend_from_slice(&raw.to_le_bytes()[..3]);
        }

        let mut fmt = Vec::new();
        fmt.extend_from_slice(&1_u16.to_le_bytes());
        fmt.extend_from_slice(&2_u16.to_le_bytes());
        fmt.extend_from_slice(&48_000_u32.to_le_bytes());
        fmt.extend_from_slice(&288_000_u32.to_le_bytes());
        fmt.extend_from_slice(&6_u16.to_le_bytes());
        fmt.extend_from_slice(&24_u16.to_le_bytes());

        let mut body = b"WAVE".to_vec();
        append_wav_chunk(&mut body, b"fmt ", &fmt);
        append_wav_chunk(&mut body, b"data", &data);

        let mut wav = b"RIFF".to_vec();
        wav.extend_from_slice(&(body.len() as u32).to_le_bytes());
        wav.extend_from_slice(&body);
        wav
    }

    fn append_wav_chunk(target: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
        target.extend_from_slice(tag);
        target.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        target.extend_from_slice(payload);
        if payload.len() % 2 == 1 {
            target.push(0);
        }
    }
}
