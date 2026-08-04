# Playback State Machine

> 播放控制面、音频数据面与前端同步面的架构设计。目标是任何异常路径都只能进入静音、暂停、失败或 superseded，不允许把未校验 PCM、错格式缓冲或过期会话继续送到音频设备。

Command 作用域、资源域和调度优先级的当前实现见 [playback-command-scheduling.md](./playback-command-scheduling.md)。

## Scope

播放链路分三层：

| Layer            | Owner                                                                         | Responsibility                                                  |
| ---------------- | ----------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Control plane    | `src-tauri/src/player/controller.rs`, `src-tauri/src/app_state/playback.rs`   | 会话 ID、请求 ID、队列、加载/播放/暂停/失败状态、前端事件       |
| Audio data plane | `src-tauri/src/player/stream.rs`                                              | 输入探测、缓存/流式读取、解码、seek、重采样、声道映射、样本缓冲 |
| Output plane     | `src-tauri/src/player/backend/mod.rs`, `src-tauri/src/player/backend/cpal.rs` | 输出设备格式协商、CPAL stream、回调填充、音量、完成/错误回调    |

前端只消费后端提供的 `PlayerState` 与 `PlaybackEndedEvent`：启动时可通过 `get_player_state` 取得快照，运行中通过事件更新；不会直接推断底层音频线程状态。

播放不是独立进程；它在同一应用进程中使用专用资源域：

- `play_song`、`seek_current_playback`、`play_next`、`play_previous` 和系统媒体 next/previous/seek 先进入 `PlaybackActor` inbox，再在 `harubble-playback` runtime 上执行。
- 歌曲详情、音频流下载、播放缓存准备、格式探测和初始缓冲等待走 `playback_api` 与 `harubble-playback` runtime。
- 库页、搜索、偏好、日志、tag 和合集等交互入口走普通 `api` 与应用 runtime；歌词详情查询使用 `image_api`，歌词文本下载仍使用普通 `api`；主题色提取和封面缓存使用 `image_api`，但只有歌词与主题色提取进入 VisualAux gate/串行 dispatcher；下载任务准备和执行走 `download_api`。
- 四个 API client 各自维护 HTTP 客户端、响应缓存和 in-flight 请求表，避免切换专辑时封面/主题/下载/库请求占住播放启动资源。
- 手动清理响应缓存或网络配置变更时必须同时作用于 `api`、`playback_api`、`image_api` 和 `download_api`，避免任一资源域留在旧代理或旧连接池中。
- 下载执行循环、本地库存扫描、搜索索引重建、belong 预热和 tag registry 同步在领取新后台工作前会等待 `PlaybackLoadGate` 解除；已经进行中的下载或扫描不强制中断。
- CPAL 输出回调仍由音频后端调度；它不能等待业务锁，也不能依赖任一 async runtime 才能填充音频设备缓冲。

## State Model

`PlayerState` 是对外快照，不是内部线程状态的全部表达。内部状态由 `session_id`、`active_request_id`、`stop_flag`、`pause_flag`、`SampleBuffer` 与 `OutputFormat` 共同约束。

| State        | Snapshot                                                                           | Invariants                                                                                                                            |
| ------------ | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `Idle`       | `song_cid = None`, `is_loading = false`, `is_playing = false`, `is_paused = false` | 无有效输出流；旧 session 已失效                                                                                                       |
| `Loading`    | `song_cid = Some`, `is_loading = true`, `is_playing = false`, `is_paused = false`  | 新选择已安装 fresh `stop_flag` / `pause_flag`；旧会话失效，但 CPAL 可保留旧设备流输出 equilibrium。重缓冲时则暂停当前流，等待样本恢复 |
| `Playing`    | `is_loading = false`, `is_playing = true`, `is_paused = false`                     | CPAL stream 已启动；新流从静音淡入，进度由 output monitor 节流广播                                                                    |
| `Paused`     | `is_loading = false`, `is_playing = false`, `is_paused = true`                     | 后端 stream pause；解码线程受 `pause_flag` 阻塞；保留歌曲上下文。用户也可在重缓冲等待期间切入此状态                                   |
| `Failed`     | 对外表现为 `Idle` 或保留暂停态，取决于错误类型                                     | 必须设置 `stop_flag` 或暂停输出；不得继续消耗旧 buffer                                                                                |
| `Superseded` | 新 session/request 的状态覆盖旧状态                                                | 旧回调必须被 `session_id` / `stop_flag` 拦截；已启动的新流可能已被更新请求接管为 transition keepalive，过期启动路径不得再无条件 stop  |
| `Ended`      | `is_playing = false`, `progress = duration`                                        | 对外保留结束 session 的快照与 `player-ended` 事件；内部立即推进活跃 session、设置 `stop_flag` 并释放输出流                            |

## Transitions

| Event                             | From                          | To        | Guard                                                                           |
| --------------------------------- | ----------------------------- | --------- | ------------------------------------------------------------------------------- |
| `PlaySelection(cid)`              | any                           | `Loading` | `begin_playback_transition()` 后仍是最新 request                                |
| `PrepareInputOk`                  | `Loading`                     | `Loading` | cache/pending marker 与 streaming handle 一致                                   |
| `ProbeOk(source_format)`          | `Loading`                     | `Loading` | `session_id` 仍 active                                                          |
| `OutputNegotiated(output_format)` | `Loading`                     | `Loading` | `output_format` 来自当前默认设备或安全 fallback，并保留设备 ID 与 sample format |
| `InitialBufferReady`              | `Loading`                     | `Playing` | sample buffer 有可播放数据，且 session 仍 active                                |
| `Pause`                           | `Playing`                     | `Paused`  | 后端 pause 成功                                                                 |
| `Resume`                          | `Paused`                      | `Playing` | 后端 play 成功                                                                  |
| `ActiveProducerUnderfill`         | `Playing`                     | `Loading` | 当前 callback 填不满且生产端未结束；不消费尾部样本，整段平滑静音并请求 rebuffer |
| `RebufferReady`                   | `Loading`                     | `Playing` | 等到约 3 秒样本（不超过容量一半），session 仍 active 且用户未暂停               |
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
7. **No partial active underrun:** 生产端尚未结束且不足以填满当前 callback 时，不消费已经排队的尾部样本；整段输出平滑静音并通知控制面进入 rebuffer，避免“半段音频 + 半段静音”的硬切。生产端已经结束时才允许消费最后的完整帧并静音剩余部分。
8. **No cache poison loop:** 探测、解码、初始缓冲失败会清理当前 cache 文件和 pending marker，避免坏缓存反复复用。
9. **No errored-buffer playback:** `SampleBuffer::fail` 会丢弃已排队样本；CPAL 输出回调整块静音并上报一次错误，不播放失败前残留样本。
10. **No packet poison from recoverable decode errors:** 单个 packet 返回 `DecodeError` 时跳过该包继续解码；读取错误、`ResetRequired`、格式漂移等不可恢复错误才会失败当前 worker。
11. **No unsafe volume fallback:** 非有限音量必须视为静音，而不是默认最大音量。
12. **No overfilled startup buffer:** 初始缓冲上限应保留容量头部空间，极端高采样率设备也不能把队列塞到满后再靠侥幸运行。
13. **No gain amplification on invalid state:** 输出增益读取到非有限值时必须静音，不能把污染后的 volume 解释成满音量。
14. **No decoded format drift:** 每个 decoded buffer 的实际声道数和采样率必须仍匹配探测得到的 `source_format`；否则失败当前 worker，不能用旧格式解释新 PCM。
15. **No post-open stale state commit:** `play_stream` 成功打开 CPAL stream 后、广播 `Playing` 前必须再次校验 session；若已 superseded/stop，返回过期启动错误且不得提交旧状态。不能在这里无条件 stop，因为更新请求可能已经 quiesce 并接管该流作为 transition keepalive。
16. **No timestamp/frame confusion:** seek 返回的 `required_ts - actual_ts` 是 timebase timestamp，必须先通过 `TimeBase` 转秒再乘采样率得到待丢弃帧数，不能直接当帧数跳过。
17. **No lingering ended stream:** 自然结束不是一个仍持有输出流的内部状态。`finish_session(session_id)` 必须推进 `active_session_id`、设置 `stop_flag`、停止后端 stream，再广播 ended 快照和 `player-ended`。
18. **No sample-queue lock wait on the normal path:** `SampleBuffer` 使用 lock-free 的 `crossbeam_queue::ArrayQueue<f32>`，正常采样弹出不与解码线程争用采样队列互斥锁。错误标志置位后，`current_error()` 会短暂读取错误字符串 mutex，因此不能把整条 callback 错误路径描述为绝对无锁；若 `PlayerState` 状态锁被 UI/媒体同步暂时占用，本次回调继续输出音频，只跳过进度快照写入。
19. **Dedicated playback resource domain:** 播放 command、媒体控制切歌/seek、播放启动和音频流下载必须使用 `PlaybackActor`、`playback_api` 与 `harubble-playback` runtime；普通 UI、视觉辅助、下载、搜索和后台预热任务不得复用播放客户端或播放 runtime。
20. **Background work yields during playback startup:** 播放器处于 `Loading` 时，本地库存扫描、搜索索引重建、belong 预热和 tag registry 同步等待 gate inactive 后再领取新后台工作。下载执行循环也先无超时等待 gate；gate 返回后若 `PlayerState.is_loading` 仍为 true，则按 helper 进入时起 30 秒的总阈值继续轮询，触顶记录警告并继续。已启动的下载或扫描只通过常规取消语义结束，不因播放启动被硬停。
21. **No realtime side effects:** 音频实时回调的正常热路径不得新增锁等待、IO、日志写入、channel send wait 或无界分配；观测数据只能写入原子计数器或预分配结构，由非实时线程聚合。
22. **No hard new-stream edge:** 每个新输出流的 `OutputSmoother` 从零增益开始，在 64 帧内线性淡入；活跃流进入临时静音时同样按最后一帧平滑衰减，避免设备切换、启动和 underrun 边界产生爆音。
23. **No dithered silence:** 浮点输出不加 dither；整数输出只对本次真正消费且增益大于零的音频样本应用 1 LSB TPDF dither。初始空缓冲、已停止的 transition keepalive 与静音音量必须保持数值 equilibrium。

## Startup Flow

```text
PlaybackActor job
  -> begin_playback_transition
  -> play_song_for_request
  -> get_song_detail via playback_api / harubble-playback
  -> prepare_playback_context
  -> begin_loading_session
       set old session stop flag
       quiesce old backend; keep device stream at numeric equilibrium
       fallback to stop only if quiesce fails
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
       macOS: build and play new CPAL stream, then replace/release previous keepalive
       non-macOS: stop previous stream before opening the new stream
       fade new audio in from silence
       re-check session without unconditionally stopping a superseded stream
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
- `playback_api` is the only API client allowed in playback metadata/audio startup paths; ordinary app commands use `api`, image fetch/cache work uses `image_api`, and download work uses `download_api`. `get_song_lyrics` is intentionally split: metadata through `image_api`, lyric text through ordinary `api`.
- `harubble-playback` is reserved for `PlaybackActor`, media-control seek/next/previous, playback startup async tasks and playback blocking work; ordinary UI/background work must not schedule onto it.

## Frontend Synchronization

Frontend controller state is a projection of backend state:

- `player-state-changed` updates song metadata, loading/playing/paused, queue flags, duration and volume.
- `player-progress` updates time only when the same session remains active.
- `player-ended` must carry `sessionId`; stale ended events are ignored.
- `isPlayTogglePending` is UI-only pending state and must clear once backend snapshot reaches the target state.
- Lyrics and album theme extraction call `dispatch_visual_aux`: they wait for the playback load gate, serialize behind the backend `visual_aux_lock`, and still rely on frontend request sequencing to discard stale results. Cached artwork path resolution uses the same bounded disk cache and `image_api`, but intentionally bypasses that dispatcher so it can complete while theme extraction is pending; the frontend exposes the path through the Tauri asset protocol.

Frontend must not synthesize backend states that contradict `PlayerState`. Buttons may show pending/loading, but playback truth comes from backend events or `get_player_state`.

## Recovery Policy

| Failure                                                                                   | Policy                                                                                                                                                                                                                                             |
| ----------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Network/download failure before playback                                                  | fail command, remove pending cache                                                                                                                                                                                                                 |
| Probe/fatal decode failure                                                                | fail session, remove bad cache, report retryable audio/io error                                                                                                                                                                                    |
| Decoder `DecodeError` for one packet during playback                                      | skip packet and continue decoding                                                                                                                                                                                                                  |
| Active producer underfills an output callback                                             | preserve queued samples, smoothly silence the whole callback, report underrun once and request rebuffer                                                                                                                                            |
| Rebuffer requested                                                                        | set visible state to `Loading`, pause backend, wait for about 3 seconds of samples capped at half capacity, then resume if the session is still active and not user-paused                                                                         |
| Non-finite/out-of-range sample                                                            | sanitize to finite bounded PCM                                                                                                                                                                                                                     |
| Decode worker failure after output startup                                                | fail current session; output callback silences errored buffer                                                                                                                                                                                      |
| Output stream error                                                                       | fail current session, set stop flag, reset visible state and log error                                                                                                                                                                             |
| Decoded buffer channel count or sample rate drifts from probed source                     | fail decode worker; mark sample buffer failed; fail current session                                                                                                                                                                                |
| Device ID, sample format, channel count or sample rate changed between negotiate and play | fail startup; caller can retry with the new default output format                                                                                                                                                                                  |
| Session becomes inactive immediately after opening stream                                 | return expired/superseded without committing `Playing`; do not unconditionally stop a stream that a newer transition may already use as keepalive                                                                                                  |
| Output device identity unavailable                                                        | fail startup; do not open a stream against an unidentified device                                                                                                                                                                                  |
| Natural playback finish                                                                   | claim the session once, stop output, keep the ended snapshot for frontend auto-next, emit `player-ended`                                                                                                                                           |
| Non-finite volume                                                                         | clamp to silence (`0.0`)                                                                                                                                                                                                                           |
| Seek result timestamp delta                                                               | convert timebase timestamp to audio frames before trimming decoded buffers                                                                                                                                                                         |
| Initial buffer would exceed capacity headroom                                             | cap to `SampleBuffer::max_capacity_samples() / 2`                                                                                                                                                                                                  |
| Non-finite gain state inside output path                                                  | output silence, never amplify                                                                                                                                                                                                                      |
| Superseded command                                                                        | old command returns `PlaybackErrorCode::Superseded`; no user-facing error toast                                                                                                                                                                    |
| Transition quiesce cannot keep the device stream alive                                    | log a warning and fall back to ordinary backend stop; if fallback stop also fails, fail the transition                                                                                                                                             |
| Integer output conversion                                                                 | apply TPDF dither only to real non-muted audio; keep empty, stopped and muted output at equilibrium                                                                                                                                                |
| Player state lock is busy during output progress callback                                 | keep rendering audio; skip only the current progress state write                                                                                                                                                                                   |
| Album switch starts visual enhancement while playback is starting                         | delay/serialize lyrics and theme extraction; let cached artwork resolve independently; coalesce identical image requests                                                                                                                           |
| Ordinary app API/HTTP work overlaps playback startup                                      | playback metadata and audio download use the dedicated playback API client and playback runtime                                                                                                                                                    |
| Download queue wants to start new work while playback is loading                          | wait for the gate without a timeout; if `player.is_loading` remains true afterwards, poll against the 30-second threshold measured from helper entry; when gate waiting has already exhausted that threshold, warn on the first check and continue |
| Inventory/search/tag warmup wants to start new background work while playback is loading  | wait on `PlaybackLoadGate` before beginning the next background unit                                                                                                                                                                               |
| Network/proxy configuration changes                                                       | reset `api`、`playback_api`、`image_api` and `download_api`                                                                                                                                                                                        |

## Test Coverage Map

| Invariant                                                                       | Tests                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Output format coverage, exact/default fallback priority and sample-format drift | `player::backend::cpal`, `output_config_`, `exact_output_config_accepts_previously_negotiated_device_default`, `exact_output_config_requires_previously_negotiated_sample_format`                                                                              |
| Active underfill preserves queued samples and silences the whole callback       | `f32_output_silences_underfilled_callback_without_consuming_samples`, `f32_output_counts_underrun_frames_when_buffer_underfills_callback`, `rebuffer_target_samples_uses_three_seconds_with_capacity_cap`                                                      |
| Incomplete trailing frame kept or safely dropped                                | `sample_buffer_keeps_incomplete_frames_for_later_completion`, `sample_buffer_drops_final_incomplete_frame_as_silence`, `f32_output_silences_incomplete_trailing_frame_without_consuming_it`                                                                    |
| Output sample/volume sanitization                                               | `f32_output_sanitizes_non_finite_samples_and_volume`                                                                                                                                                                                                           |
| Errored buffer never reaches output                                             | `sample_buffer_fail_discards_queued_samples`, `sample_buffer_wait_reports_buffer_error_before_stop_flag`, `f32_output_silences_buffer_errors_without_advancing_progress`                                                                                       |
| Output callback uses the lock-free sample queue on its normal path              | `sample_buffer_try_pop_is_lock_free_for_realtime_reader`, `f32_output_reads_ring_buffer_without_lock_silence`; error-string retrieval remains a separate mutex-backed path                                                                                     |
| Output progress callback never waits on player state lock                       | `progress_callback_skips_update_when_state_lock_is_busy`                                                                                                                                                                                                       |
| Transition keeps the device stream alive and preserves newer ownership          | `transition_keeps_backend_alive_when_quiesce_succeeds`, `transition_falls_back_to_stop_when_quiesce_fails`, `transition_reports_stop_failure_after_quiesce_failure`, `superseded_stream_start_preserves_transition_keepalive`                                  |
| New-stream fade and integer dither boundaries                                   | `f32_output_fades_in_new_stream_from_silence`, `integer_output_formats_enable_tpdf_dither`, `integer_output_does_not_dither_empty_initial_buffer`, `integer_output_keeps_stopped_transition_at_equilibrium`, `integer_output_keeps_muted_audio_at_equilibrium` |
| Buffer and converter sanitization                                               | `sample_buffer_sanitizes_samples_before_queueing`, `sample_converter_sanitizes_non_finite_and_out_of_range_samples`                                                                                                                                            |
| Decoded format drift fails before conversion                                    | `decoded_format_must_match_probed_source_format`                                                                                                                                                                                                               |
| Probe retry                                                                     | `audio_probe_retries_after_transient_failure`                                                                                                                                                                                                                  |
| Seek timestamp/frame conversion                                                 | `seek_timestamp_delta_is_converted_to_audio_frames`                                                                                                                                                                                                            |
| 24-bit WAV scale                                                                | `decode_24bit_wav_preserves_low_amplitude_samples`, `decode_manual_24bit_pcm_wav_preserves_sample_scale`                                                                                                                                                       |
| Intent-aware prebuffer                                                          | `app_state::playback`                                                                                                                                                                                                                                          |
| Queue and volume state helpers                                                  | `player::controller`                                                                                                                                                                                                                                           |
| Frontend image in-flight dedupe and cached lookup independence                  | `src/lib/api.test.ts`                                                                                                                                                                                                                                          |
| Playback resource identifier boundaries                                         | `command_scheduling::tests::playback_resources_are_only_used_from_allowed_modules`                                                                                                                                                                             |
| Playback gate ticket ownership and waiter release                               | `playback_load_gate::tests::old_ticket_cannot_release_newer_loading_window`, `playback_load_gate::tests::wait_until_inactive_unblocks_when_current_ticket_drops`                                                                                               |

后台下载、库存、搜索和 tag 工作在启动前调用 gate 的事实目前由实现路径与架构守卫共同复核；`cargo check` 只能证明这些路径可编译，不能单独证明运行时一定按预期退让。

## Change Checklist

Any playback change must verify:

1. `cargo test --manifest-path src-tauri/Cargo.toml player::backend::cpal`
2. `cargo test --manifest-path src-tauri/Cargo.toml player::stream`
3. `cargo test --manifest-path src-tauri/Cargo.toml app_state::playback`
4. `cargo test --manifest-path src-tauri/Cargo.toml player::controller`
5. `cargo check --manifest-path src-tauri/Cargo.toml`
6. `git diff --check`

For frontend state changes, also run the player-related Vitest suites and `bun run check`.
