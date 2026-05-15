//! 播放输入探测、解码与样本缓冲处理。
//!
//! 该模块负责描述播放器输入来源、探测音频格式、启动后台解码线程，并通过样本缓冲
//! 在解码侧与播放后端之间传递 PCM 数据。

use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// 播放解码线程上报致命错误时使用的回调类型。
pub type PlaybackErrorHandler = Arc<dyn Fn(String) + Send + Sync>;
use symphonia::core::audio::SampleBuffer as SymphoniaSampleBuffer;
use symphonia::core::codecs::{
    CodecParameters, Decoder as SymphoniaDecoder, DecoderOptions, CODEC_TYPE_NULL,
};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, Track};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

/// 描述解码后或输出端期望的音频格式。
#[derive(Debug, Clone, Copy)]
pub struct AudioFormat {
    /// 音频通道数，至少为 1。
    pub channels: u16,
    /// 采样率，单位为 Hz。
    pub sample_rate: u32,
    /// 音频总时长，单位为秒。
    pub duration_secs: f64,
}

impl AudioFormat {
    /// 返回经过最小值归一化后的音频格式。
    ///
    /// 该方法会将通道数和采样率修正到至少为 `1`，并把时长裁剪到不小于 `0.0`。
    pub fn normalized(self) -> Self {
        Self {
            channels: self.channels.max(1),
            sample_rate: self.sample_rate.max(1),
            duration_secs: self.duration_secs.max(0.0),
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

    /// 探测当前输入的音频格式。
    ///
    /// 返回值包含通道数、采样率和可推断出的总时长。
    pub fn inspect_format(&self) -> Result<AudioFormat> {
        inspect_audio_reader(self.open_reader()?, self.build_hint())
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
        let reader = self.open_reader()?;
        let hint = self.build_hint();
        Ok(spawn_decode_worker(
            reader,
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

    fn open_reader(&self) -> Result<BoxedAudioReader> {
        match self {
            Self::CachedFile(path) => {
                let file = File::open(path).with_context(|| {
                    format!("Failed to open cached audio file {}", path.display())
                })?;
                Ok(Box::new(file))
            }
            Self::GrowingFile(handle) => Ok(Box::new(handle.open_reader()?)),
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
    pub fn open_reader(&self) -> Result<GrowingFileReader> {
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
        })
    }

    /// 返回底层缓存文件路径。
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 向增长文件追加一个字节块，并更新读取侧可见长度。
    pub fn append_chunk(&self, writer: &mut File, chunk: &[u8]) -> Result<()> {
        writer
            .write_all(chunk)
            .context("Failed to append audio chunk to cache file")?;
        let position = writer
            .stream_position()
            .context("Failed to inspect cache file position")?;

        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        state.available_len = position;
        condvar.notify_all();
        Ok(())
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
pub struct GrowingFileReader {
    file: File,
    position: u64,
    state: Arc<(Mutex<GrowingFileState>, Condvar)>,
}

impl Read for GrowingFileReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let (lock, condvar) = &*self.state;
        let mut state = lock.lock().unwrap();

        while self.position >= state.available_len && !state.complete && state.error.is_none() {
            state = condvar.wait(state).unwrap();
        }

        if let Some(error) = &state.error {
            return Err(io::Error::other(error.clone()));
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
    inner: Arc<(Mutex<SampleBufferState>, Condvar)>,
}

const MAX_BUFFER_SAMPLES: usize = 44100 * 2 * 10;

struct SampleBufferState {
    queue: VecDeque<f32>,
    finished: bool,
    error: Option<String>,
}

impl SampleBuffer {
    /// 创建一个空的采样缓冲区。
    pub fn new() -> Self {
        Self {
            inner: Arc::new((
                Mutex::new(SampleBufferState {
                    queue: VecDeque::new(),
                    finished: false,
                    error: None,
                }),
                Condvar::new(),
            )),
        }
    }

    /// 追加一批已解码的浮点采样。
    ///
    /// 当缓冲区已满时会阻塞等待消费者消费后再写入。
    pub fn push(&self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        let (lock, condvar) = &*self.inner;
        let mut state = lock.lock().unwrap();
        while state.queue.len() >= MAX_BUFFER_SAMPLES && !state.finished && state.error.is_none() {
            let (next_state, _) = condvar
                .wait_timeout(state, Duration::from_millis(50))
                .unwrap();
            state = next_state;
        }
        state.queue.extend(samples.iter().copied());
        condvar.notify_all();
    }

    /// 标记采样缓冲区不会再写入新的数据。
    pub fn finish(&self) {
        let (lock, condvar) = &*self.inner;
        let mut state = lock.lock().unwrap();
        state.finished = true;
        condvar.notify_all();
    }

    /// 标记采样缓冲区失败并唤醒等待中的消费者。
    pub fn fail(&self, message: impl Into<String>) {
        let (lock, condvar) = &*self.inner;
        let mut state = lock.lock().unwrap();
        state.error = Some(message.into());
        state.finished = true;
        condvar.notify_all();
    }

    /// 从缓冲区中弹出尽可能多的采样写入 `output`。
    ///
    /// 返回值会说明本次写入了多少采样，以及缓冲区是否已经结束或失败。
    pub fn pop_into(&self, output: &mut [f32]) -> PopStatus {
        let (lock, condvar) = &*self.inner;
        let mut state = lock.lock().unwrap();
        let mut written = 0_usize;

        while written < output.len() {
            match state.queue.pop_front() {
                Some(sample) => {
                    output[written] = sample;
                    written += 1;
                }
                None => break,
            }
        }

        let status = PopStatus {
            written,
            finished: state.finished && state.queue.is_empty(),
            error: state.error.clone(),
        };

        if written > 0 {
            condvar.notify_all();
        }

        status
    }

    /// 等待缓冲区中至少出现指定数量的采样。
    ///
    /// 当缓冲区报错、播放被停止，或流结束时仍没有任何可播放采样时返回错误。
    pub fn wait_for_samples(&self, minimum_samples: usize, stop_flag: &AtomicBool) -> Result<()> {
        let (lock, condvar) = &*self.inner;
        let mut state = lock.lock().unwrap();

        while state.queue.len() < minimum_samples
            && !state.finished
            && state.error.is_none()
            && !stop_flag.load(Ordering::SeqCst)
        {
            let (next_state, _) = condvar
                .wait_timeout(state, Duration::from_millis(50))
                .unwrap();
            state = next_state;
        }

        if let Some(error) = &state.error {
            anyhow::bail!(error.clone());
        }
        if stop_flag.load(Ordering::SeqCst) {
            anyhow::bail!("Playback stopped");
        }
        if state.finished && state.queue.is_empty() {
            anyhow::bail!("Audio stream ended before playback could start");
        }
        Ok(())
    }
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

/// 探测一个已打开音频读取器的格式信息。
///
/// 该函数会选取默认或首个可解码音轨，并返回播放器后续协商所需的基础参数。
pub fn inspect_audio_reader(reader: BoxedAudioReader, hint: Hint) -> Result<AudioFormat> {
    Ok(open_audio_reader(reader, hint)?.audio_format)
}

#[allow(clippy::too_many_arguments)]
fn spawn_decode_worker(
    reader: BoxedAudioReader,
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
                } = open_audio_reader(reader, hint)?;
                let mut converter = SampleConverter::new(source_format, target_format);
                let mut decoded_samples: Option<SymphoniaSampleBuffer<f32>> = None;
                let mut remaining_seek_frames = if start_position_secs > 0.0 {
                    let frames = seek_to_time(format.as_mut(), track_id, start_position_secs)?;
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
                error_handler(message.clone());
                sample_buffer.fail(message);
            }
        })
        .expect("Failed to spawn audio decode worker")
}

fn seek_to_time(format: &mut dyn FormatReader, track_id: u32, seconds: f64) -> Result<u64> {
    let seek_result = format
        .seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: Time::from(seconds.max(0.0)),
                track_id: Some(track_id),
            },
        )
        .context("Failed to seek audio stream")?;

    Ok(seek_result
        .required_ts
        .saturating_sub(seek_result.actual_ts))
}

fn open_audio_reader(reader: BoxedAudioReader, hint: Hint) -> Result<OpenedAudioReader> {
    let media_source = Box::new(SymphoniaSource { inner: reader });
    let media_source_stream = MediaSourceStream::new(media_source, Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
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

fn audio_format_from_codec_params(codec_params: &CodecParameters) -> Result<AudioFormat> {
    let channels = codec_params
        .channels
        .context("Missing audio channel layout")?
        .count() as u16;
    let sample_rate = codec_params
        .sample_rate
        .context("Missing audio sample rate")?;

    Ok(AudioFormat {
        channels,
        sample_rate,
        duration_secs: codec_duration_secs(codec_params),
    }
    .normalized())
}

fn codec_duration_secs(codec_params: &CodecParameters) -> f64 {
    match (codec_params.n_frames, codec_params.time_base) {
        (Some(n_frames), Some(time_base)) => {
            let duration = time_base.calc_time(n_frames);
            duration.seconds as f64 + duration.frac
        }
        _ => 0.0,
    }
}

struct SampleConverter {
    source_channels: usize,
    target_channels: usize,
    source_rate: u32,
    target_rate: u32,
    pending: Vec<f32>,
    pending_base_frame: u64,
    next_source_frame: f64,
}

impl SampleConverter {
    fn new(source_format: AudioFormat, target_format: AudioFormat) -> Self {
        Self {
            source_channels: source_format.channels.max(1) as usize,
            target_channels: target_format.channels.max(1) as usize,
            source_rate: source_format.sample_rate.max(1),
            target_rate: target_format.sample_rate.max(1),
            pending: Vec::new(),
            pending_base_frame: 0,
            next_source_frame: 0.0,
        }
    }

    fn push_chunk(&mut self, samples: &[f32]) -> Vec<f32> {
        self.pending.extend_from_slice(samples);
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

        let mut output = Vec::new();
        let ratio = self.source_rate as f64 / self.target_rate as f64;
        let available_abs_frames = self.pending_base_frame + available_frames as u64;

        while (self.next_source_frame.floor() as u64) < available_abs_frames {
            let source_frame_abs = self.next_source_frame.floor() as u64;
            let local_frame = (source_frame_abs - self.pending_base_frame) as usize;
            if local_frame >= available_frames {
                break;
            }
            self.push_remixed_frame(local_frame, &mut output);
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
}
