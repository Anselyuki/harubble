# Playback State Machine

> 播放控制面、音频数据面与前端同步面的架构设计。目标是任何异常路径都只能进入静音、暂停、失败或 superseded，不允许把未校验 PCM、错格式缓冲或过期会话继续送到音频设备。

Command 作用域、资源域和调度优先级的完整改造建议见 [playback-command-scheduling.md](./playback-command-scheduling.md)。

## Scope

播放链路分三层：

| Layer            | Owner                                                                         | Responsibility                                                  |
| ---------------- | ----------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Control plane    | `src-tauri/src/player/controller.rs`, `src-tauri/src/app_state/playback.rs`   | 会话 ID、请求 ID、队列、加载/播放/暂停/失败状态、前端事件       |
| Audio data plane | `src-tauri/src/player/stream.rs`                                              | 输入探测、缓存/流式读取、解码、seek、重采样、声道映射、样本缓冲 |
| Output plane     | `src-tauri/src/player/backend/mod.rs`, `src-tauri/src/player/backend/cpal.rs` | 输出设备格式协商、CPAL stream、回调填充、音量、完成/错误回调    |

前端只消费后端广播的 `PlayerState` 与 `PlaybackEndedEvent`，不直接推断底层音频线程状态。

播放不是独立进程；它在同一应用进程中使用专用资源域：

- 播放 command、系统媒体 next/previous/seek、歌曲详情、音频流下载、播放缓存准备、格式探测和初始缓冲等待走 `playback_api` 与 `harubble-playback` runtime。
- 库页、搜索、下载管理、歌词、封面转换、主题色提取和后台预热走普通 `api` 与应用 runtime。
- 两个 API client 有独立 HTTP 连接池、响应缓存和 in-flight 请求表，避免切换专辑时封面/主题/库请求占住播放启动资源。
- 手动清理响应缓存或网络配置变更时必须同时作用于两个 client，避免播放链路留在旧代理或旧连接池中。
- 下载执行循环在播放器 `Loading` 时不会主动启动新的批次或下一首下载任务，避免播放启动窗口与下载系统争抢物理网络/磁盘带宽。已经进行中的下载任务不强制中断。
- CPAL 输出回调仍由音频后端调度；它不能等待业务锁，也不能依赖任一 async runtime 才能填充音频设备缓冲。

## State Model

`PlayerState` 是对外快照，不是内部线程状态的全部表达。内部状态由 `session_id`、`active_request_id`、`stop_flag`、`pause_flag`、`SampleBuffer` 与 `OutputFormat` 共同约束。

| State        | Snapshot                                                                           | Invariants                                                                                                 |
| ------------ | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `Idle`       | `song_cid = None`, `is_loading = false`, `is_playing = false`, `is_paused = false` | 无有效输出流；旧 session 已失效                                                                            |
| `Loading`    | `song_cid = Some`, `is_loading = true`, `is_playing = false`, `is_paused = false`  | 已安装新 `stop_flag` / `pause_flag`；旧输出已 stop；允许下载/探测/解码预热                                 |
| `Playing`    | `is_loading = false`, `is_playing = true`, `is_paused = false`                     | CPAL stream 已启动；进度由 output monitor 节流广播                                                         |
| `Paused`     | `is_loading = false`, `is_playing = false`, `is_paused = true`                     | 后端 stream pause；解码线程受 `pause_flag` 阻塞；保留歌曲上下文                                            |
| `Failed`     | 对外表现为 `Idle` 或保留暂停态，取决于错误类型                                     | 必须设置 `stop_flag` 或暂停输出；不得继续消耗旧 buffer                                                     |
| `Superseded` | 新 session/request 的状态覆盖旧状态                                                | 旧回调必须被 `session_id` / `stop_flag` 拦截                                                               |
| `Ended`      | `is_playing = false`, `progress = duration`                                        | 对外保留结束 session 的快照与 `player-ended` 事件；内部立即推进活跃 session、设置 `stop_flag` 并释放输出流 |

## Transitions

| Event                             | From                          | To        | Guard                                                                           |
| --------------------------------- | ----------------------------- | --------- | ------------------------------------------------------------------------------- |
| `PlaySelection(cid)`              | any                           | `Loading` | `begin_playback_request()` 后仍是最新 request                                   |
| `PrepareInputOk`                  | `Loading`                     | `Loading` | cache/pending marker 与 streaming handle 一致                                   |
| `ProbeOk(source_format)`          | `Loading`                     | `Loading` | `session_id` 仍 active                                                          |
| `OutputNegotiated(output_format)` | `Loading`                     | `Loading` | `output_format` 来自当前默认设备或安全 fallback，并保留设备 ID 与 sample format |
| `InitialBufferReady`              | `Loading`                     | `Playing` | sample buffer 有可播放数据，且 session 仍 active                                |
| `Pause`                           | `Playing`                     | `Paused`  | 后端 pause 成功                                                                 |
| `Resume`                          | `Paused`                      | `Playing` | 后端 play 成功                                                                  |
| `Seek(position)`                  | `Playing` / `Paused`          | `Loading` | 当前非 loading；新 session 继承当前歌曲                                         |
| `Next` / `Previous`               | `Playing` / `Paused` / `Idle` | `Loading` | 队列存在目标                                                                    |
| `DecodeError`                     | `Loading` / `Playing`         | unchanged | 单个坏包跳过；其他解码/读取失败才会 `SampleBuffer::fail` 并停止当前 session     |
| `OutputError`                     | `Playing`                     | `Failed`  | 输出错误立即失效当前 session 并静音，不允许继续播放脏数据                       |
| `Stop`                            | any                           | `Idle`    | 推进 `session_id`，设置 `stop_flag`                                             |
| `Finish`                          | `Playing`                     | `Ended`   | 当前 session 通过原子认领；发出结束事件前先让旧输出和迟到回调失效               |

## Audio Safety Invariants

这些规则优先级高于无缝播放体验。违反时必须静音、失败或 supersede。

1. **No stale output:** 所有 progress、finish、error callback 都检查 `session_id` 和 `stop_flag`。
2. **No unbounded samples:** `SampleBuffer` 受 `MAX_BUFFER_SAMPLES` 限制，生产端在满缓冲时等待。
3. **No dirty PCM:** 进入 `SampleBuffer` 与 `SampleConverter` 的样本必须经过 `sanitize_pcm_sample`，非有限值变为 `0.0`，越界值裁剪到 `[-1.0, 1.0]`。
4. **No incomplete frames:** CPAL 输出端只消费并统计完整声道帧；尾部不足一帧的样本必须保留在 `SampleBuffer` 中等待补齐，本次输出静音。若生产端已经结束且不可能补齐，则丢弃最终半帧并以静音结束，避免声道错位或输出线程卡住。
5. **No format drift:** `negotiate_output_format` 返回完整 `OutputFormat`，包含 `AudioFormat`、`OutputSampleFormat` 和可用时的设备 ID；`play_stream` 必须按设备 ID、声道数、采样率、sample format 重新校验，任何漂移都会让启动失败。
6. **No forced device rate by default:** 优先使用支持范围内与源音频声道/采样率匹配的 exact config；没有 exact match 时才使用可用的设备 `default_output_config()` 或 nearest fallback。
7. **No all-or-nothing underrun:** 回调缓冲不足时，已到达的完整帧先播放，剩余部分静音。
8. **No cache poison loop:** 探测、解码、初始缓冲失败会清理当前 cache 文件和 pending marker，避免坏缓存反复复用。
9. **No errored-buffer playback:** `SampleBuffer::fail` 会丢弃已排队样本；CPAL 输出回调整块静音并上报一次错误，不播放失败前残留样本。
10. **No packet poison from recoverable decode errors:** 单个 packet 返回 `DecodeError` 时跳过该包继续解码；读取错误、`ResetRequired`、格式漂移等不可恢复错误才会失败当前 worker。
11. **No unsafe volume fallback:** 非有限音量必须视为静音，而不是默认最大音量。
12. **No overfilled startup buffer:** 初始缓冲上限应保留容量头部空间，极端高采样率设备也不能把队列塞到满后再靠侥幸运行。
13. **No gain amplification on invalid state:** 输出增益读取到非有限值时必须静音，不能把污染后的 volume 解释成满音量。
14. **No decoded format drift:** 每个 decoded buffer 的实际声道数和采样率必须仍匹配探测得到的 `source_format`；否则失败当前 worker，不能用旧格式解释新 PCM。
15. **No post-open stale stream:** `play_stream` 成功打开 CPAL stream 后、广播 `Playing` 前必须再次校验 session；若已 superseded/stop，立即 stop 刚打开的 stream 并返回失败。
16. **No timestamp/frame confusion:** seek 返回的 `required_ts - actual_ts` 是 timebase timestamp，必须先通过 `TimeBase` 转秒再乘采样率得到待丢弃帧数，不能直接当帧数跳过。
17. **No lingering ended stream:** 自然结束不是一个仍持有输出流的内部状态。`finish_session(session_id)` 必须推进 `active_session_id`、设置 `stop_flag`、停止后端 stream，再广播 ended 快照和 `player-ended`。
18. **No realtime lock wait:** CPAL 输出回调不能阻塞等待 `SampleBuffer` 或 `PlayerState` 互斥锁；若缓冲锁暂时被解码线程占用，本次回调输出静音且不推进播放进度；若状态锁暂时被 UI/媒体同步占用，本次回调继续输出音频但跳过进度快照写入。
19. **Dedicated playback resource domain:** 播放 command、媒体控制切歌/seek、播放启动和音频流下载必须使用 `playback_api` 与 `harubble-playback` runtime；普通 UI、图片、下载、搜索和后台预热任务不得复用播放客户端或播放 runtime。
20. **No new download work during playback startup:** 播放器处于 `Loading` 时，下载执行循环不能启动新批次或领取下一首任务；已启动的下载任务只通过常规取消语义结束，不因播放启动被硬停。

## Startup Flow

```text
play_song_internal
  -> begin_playback_request
  -> get_song_detail via playback_api / harubble-playback
  -> prepare_playback_context
  -> begin_loading_session
       stop old backend
       advance session_id
       install fresh stop/pause flags
       emit Loading
  -> prepare_playback_input
       cached file OR growing file + download task
       streaming download uses playback_api / harubble-playback
  -> inspect_format_with_retry on harubble-playback blocking pool
  -> negotiate_output_format
       prefer exact source/output match, then device default or nearest fallback
       keep OutputFormat(audio_format, sample_format, device_identity)
  -> spawn_decode_worker
       decode -> verify decoded format -> sanitize -> resample/remix -> SampleBuffer
  -> wait_for_initial_buffer
       NewSelection: larger streaming prebuffer
       InteractiveRestart: short prebuffer
  -> start_stream_playback
       verify negotiated device id and exact output format still supported
       build CPAL stream
       emit Playing
```

## OutputFormat Contract

`AudioFormat` 只描述解码/重采样目标：声道数、采样率和时长。它不能单独作为输出设备契约使用。

`OutputFormat` 是后端协商结果，必须跨越 `negotiate_output_format -> spawn_decode_worker -> start_stream_playback -> play_stream` 整条链路：

- `audio_format`：解码线程转换到这个声道数和采样率。
- `sample_format`：CPAL stream 必须以同一个样本格式打开，不能在开流时重新猜一个“兼容”的格式。
- `device_identity`：使用 CPAL device ID 作为唯一身份；协商阶段如果无法识别设备则启动失败，开流前必须确认当前默认设备仍是同一个身份。

这条契约的目的是让设备切换、sample format 变化、默认采样率变化都走失败/重试路径，而不是把旧 buffer 送入不确定的输出流。

## Control Plane Rules

- `active_request_id` protects async command ordering before a session is fully active.
- `session_id` protects callbacks and frontend events after a session is active.
- `stop_flag` is the shared cancellation primitive for download, probe, decode, output monitor, and progress emitter.
- `pause_flag` only pauses decode work; it must never be used as cancellation.
- `fail_session(session_id)` may clear state only if the failed session is still active.
- `finish_session(session_id)` uses an atomic compare-and-swap to let exactly one natural-finish path claim the session; the visible snapshot keeps the ended `session_id`, while the internal active session has already advanced.
- `playback_api` is the only API client allowed in playback metadata/audio startup paths; ordinary app commands must use `api`.
- `harubble-playback` is reserved for playback commands, media-control seek/next/previous, playback startup async tasks and playback blocking work; ordinary UI/background work must not schedule onto it.

## Frontend Synchronization

Frontend controller state is a projection of backend state:

- `player-state-changed` updates song metadata, loading/playing/paused, queue flags, duration and volume.
- `player-progress` updates time only when the same session remains active.
- `player-ended` must carry `sessionId`; stale ended events are ignored.
- `isPlayTogglePending` is UI-only pending state and must clear once backend snapshot reaches the target state.
- Lyrics, album artwork data URL conversion and album theme extraction are auxiliary tasks. They must not compete with playback startup: lyrics wait until the backend leaves `Loading`, and album visual requests are delayed, serialized and in-flight deduplicated.

Frontend must not synthesize backend states that contradict `PlayerState`. Buttons may show pending/loading, but playback truth comes from backend events or `get_player_state`.

## Recovery Policy

| Failure                                                                                   | Policy                                                                                                   |
| ----------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| Network/download failure before playback                                                  | fail command, remove pending cache                                                                       |
| Probe/fatal decode failure                                                                | fail session, remove bad cache, report retryable audio/io error                                          |
| Decoder `DecodeError` for one packet during playback                                      | skip packet and continue decoding                                                                        |
| Buffer underrun                                                                           | play complete frames already available, pad silence                                                      |
| Non-finite/out-of-range sample                                                            | sanitize to finite bounded PCM                                                                           |
| Decode worker failure after output startup                                                | fail current session; output callback silences errored buffer                                            |
| Output stream error                                                                       | fail current session, set stop flag, reset visible state and log error                                   |
| Decoded buffer channel count or sample rate drifts from probed source                     | fail decode worker; mark sample buffer failed; fail current session                                      |
| Device ID, sample format, channel count or sample rate changed between negotiate and play | fail startup; caller can retry with the new default output format                                        |
| Session becomes inactive immediately after opening stream                                 | stop the newly opened stream and return superseded/startup failure                                       |
| Output device identity unavailable                                                        | fail startup; do not open a stream against an unidentified device                                        |
| Natural playback finish                                                                   | claim the session once, stop output, keep the ended snapshot for frontend auto-next, emit `player-ended` |
| Non-finite volume                                                                         | clamp to silence (`0.0`)                                                                                 |
| Seek result timestamp delta                                                               | convert timebase timestamp to audio frames before trimming decoded buffers                               |
| Initial buffer would exceed capacity headroom                                             | cap to `SampleBuffer::max_capacity_samples() / 2`                                                        |
| Non-finite gain state inside output path                                                  | output silence, never amplify                                                                            |
| Superseded command                                                                        | old command returns `PlaybackErrorCode::Superseded`; no user-facing error toast                          |
| Sample buffer lock is busy during output callback                                         | output silence for the callback; keep queued samples and do not advance progress                         |
| Player state lock is busy during output progress callback                                 | keep rendering audio; skip only the current progress state write                                         |
| Album switch starts visual enhancement while playback is starting                         | delay/serialize album artwork and theme requests; coalesce identical image downloads                     |
| Ordinary app API/HTTP work overlaps playback startup                                      | playback metadata and audio download use the dedicated playback API client and playback runtime          |
| Download queue wants to start new work while playback is loading                          | defer starting the new job/task until backend leaves `Loading`                                           |
| Network/proxy configuration changes                                                       | reset both ordinary and playback HTTP clients                                                            |

## Test Coverage Map

| Invariant                                                                       | Tests                                                                                                                                                                                                                                                        |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Output format coverage, exact/default fallback priority and sample-format drift | `player::backend::cpal`, `output_config_`, `exact_output_config_accepts_previously_negotiated_device_default`, `exact_output_config_requires_previously_negotiated_sample_format`                                                                            |
| Partial underrun preserves complete frames                                      | `f32_output_preserves_partial_buffered_samples`                                                                                                                                                                                                              |
| Incomplete trailing frame kept or safely dropped                                | `sample_buffer_keeps_incomplete_frames_for_later_completion`, `sample_buffer_drops_final_incomplete_frame_as_silence`, `f32_output_silences_incomplete_trailing_frame_without_consuming_it`                                                                  |
| Output sample/volume sanitization                                               | `f32_output_sanitizes_non_finite_samples_and_volume`                                                                                                                                                                                                         |
| Errored buffer never reaches output                                             | `sample_buffer_fail_discards_queued_samples`, `sample_buffer_wait_reports_buffer_error_before_stop_flag`, `f32_output_silences_buffer_errors_without_advancing_progress`                                                                                     |
| Output callback never waits on buffer lock                                      | `sample_buffer_try_pop_returns_none_when_locked`, `f32_output_silences_when_sample_buffer_is_locked`                                                                                                                                                         |
| Output progress callback never waits on player state lock                       | `progress_callback_skips_update_when_state_lock_is_busy`                                                                                                                                                                                                     |
| Buffer and converter sanitization                                               | `sample_buffer_sanitizes_samples_before_queueing`, `sample_converter_sanitizes_non_finite_and_out_of_range_samples`                                                                                                                                          |
| Decoded format drift fails before conversion                                    | `decoded_format_must_match_probed_source_format`                                                                                                                                                                                                             |
| Probe retry                                                                     | `audio_probe_retries_after_transient_failure`                                                                                                                                                                                                                |
| Seek timestamp/frame conversion                                                 | `seek_timestamp_delta_is_converted_to_audio_frames`                                                                                                                                                                                                          |
| 24-bit WAV scale                                                                | `decode_24bit_wav_preserves_low_amplitude_samples`, `decode_manual_24bit_pcm_wav_preserves_sample_scale`                                                                                                                                                     |
| Intent-aware prebuffer                                                          | `app_state::playback`                                                                                                                                                                                                                                        |
| Queue and volume state helpers                                                  | `player::controller`                                                                                                                                                                                                                                         |
| Album visual request serialization and image in-flight dedupe                   | `src/lib/api.test.ts`                                                                                                                                                                                                                                        |
| Playback resource-domain isolation                                              | `src-tauri/src/app_state/mod.rs`, `src-tauri/src/app_state/playback.rs`, `src-tauri/src/app_state/media_controls.rs`, `src-tauri/src/commands/playback.rs`, `src-tauri/src/commands/downloads.rs`, `src-tauri/src/network_monitor.rs` compile/check coverage |
| Download startup yields to playback startup                                     | `src-tauri/src/downloads/bridge.rs` compile/check coverage                                                                                                                                                                                                   |

## Change Checklist

Any playback change must verify:

1. `rtk cargo test --manifest-path src-tauri/Cargo.toml player::backend::cpal`
2. `rtk cargo test --manifest-path src-tauri/Cargo.toml player::stream`
3. `rtk cargo test --manifest-path src-tauri/Cargo.toml app_state::playback`
4. `rtk cargo test --manifest-path src-tauri/Cargo.toml player::controller`
5. `rtk cargo check --manifest-path src-tauri/Cargo.toml`
6. `rtk git diff --check`

For frontend state changes, also run the player-related Vitest suites and `rtk bun run check`.
