# Playback Command Scheduling

> 播放资源隔离、command 调度与退让规则。状态机和音频安全不变量见 [playback-state-machine.md](./playback-state-machine.md)。

## Decision

Harubble 使用 **Command Domain + 独立资源域 + 降级策略**：

1. CPAL 输出回调由音频后端调度，不依赖 Tauri async runtime。
2. 播放启动、切歌、seek、音频流下载、probe 与初始缓冲使用专用 `playback_api`、`PlaybackActor` 和 `harubble-playback` runtime。
3. command 注册以 `src-tauri/src/command_registry.rs` 为准；domain、priority 与 cancel policy 以 `src-tauri/src/command_scheduling.rs` 的 `COMMAND_SPECS` 为准。
4. priority 只负责局部排序；实时安全依赖资源隔离、可取消、可降级以及回调不阻塞。

## Command Domains

| Domain               | Priority labels                         | Examples                                                        | Executor / resource                                 | Loading policy                              |
| -------------------- | --------------------------------------- | --------------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------- |
| `PlaybackControl`    | `Playback`                              | pause, resume, volume, get state                                | `AudioPlayer` 短路径                                | 允许执行，但不得做网络或磁盘重活            |
| `PlaybackTransition` | `Playback`                              | play, next, previous, seek commit                               | `PlaybackActor` on `harubble-playback`              | 新 transition supersede 旧启动              |
| `PlaybackSideEffect` | `CriticalSideEffect`                    | record / clear listening history                                | app runtime side-effect helper                      | 不阻塞播放成功返回                          |
| `InteractiveUi`      | `Interactive`                           | albums, search, preferences, logs, tags, collections            | app runtime + ordinary `api`                        | 不被 gate 阻塞，但不得使用播放资源域        |
| `VisualAux`          | `Visual`                                | cached artwork path, theme extraction, lyrics                   | visual helper + backend `visual_aux_lock`           | `Loading` 时等待 gate，并串行执行           |
| `BackgroundIo`       | `Background` 或交互取消用 `Interactive` | downloads, inventory, search rebuild, warmup, tag registry sync | background tasks + ordinary `api` 或 `download_api` | 领取新后台工作前等待 gate；已启动任务不硬停 |
| `Maintenance`        | `CriticalSideEffect`                    | cache clear, HTTP client reset                                  | app/playback/image/download 资源域 fan-out          | 按命令语义执行；清理音频缓存前停止播放器    |

CPAL 输出回调不进入 `COMMAND_SPECS`；指标中使用 `command.domain = "RealtimeAudio"` 和 `command.priority = "Realtime"`。

## Architecture

```text
Frontend / media keys / lifecycle
  -> Tauri command shim
       -> RealtimeAudio: CPAL callback path, no async wait
       -> PlaybackControl: direct short AudioPlayer operation
       -> PlaybackActor: play/seek/next/previous on harubble-playback
       -> VisualAux: cached artwork/theme/lyrics, playback gate + serial lock
       -> BackgroundIo: downloads/inventory/search/tag work, background gate
       -> InteractiveUi: normal UI/data commands
```

### Registries

- `command_registry.rs` 是 Tauri command 注册单一事实源。
- `COMMAND_SPECS` 是 Tauri command 与内部后台入口的调度元数据来源。
- 覆盖测试保证每个 command 都有 spec，并限制 `playback_api`、`playback_runtime`、`PlaybackActor`、`image_api` 和 `download_api` 的使用范围。
- 不在文档中维护逐 command 清单；新增或删除 command 时更新两份 registry 和契约测试。

### PlaybackActor

- play/next/previous/seek 与系统媒体控制统一提交到 actor inbox。
- actor 领取任务时推进 request id；旧会话仍在 `Loading` 时立即让旧 stop flag 失效。
- 每个 job 持有 `PlaybackLoadGate` ticket，旧 ticket 不能释放更新的 gate。
- 启动流程继续使用 `request_id`、`session_id`、`stop_flag` 和 gate ticket 保护 supersede 与取消。
- actor 记录 queue wait、queue depth、run time、request id 和 ticket superseded 指标。

### Resource Layout

```text
AppState
  api_clients
    api              -> ordinary UI/data requests
    playback_api     -> active playback metadata + audio stream
    image_api        -> artwork cache/theme/notification artwork
    download_api     -> download preparation + large transfers
  playback_runtime   -> actor + playback startup async/blocking work
  playback_actor     -> transition inbox
  playback_load_gate -> cross-domain startup backpressure
  visual_aux_lock    -> artwork/theme/lyrics serialization
```

封面通过 `get_cached_image_path` 写入受大小约束的磁盘缓存，前端再用 Tauri asset protocol 加载；同 URL 的并发请求会合并。主题提取复用同一磁盘缓存，避免重复访问 CDN。

### PlaybackLoadGate

`PlaybackLoadGate` 是跨 domain 的退让信号，不是互斥锁：

- transition 被 actor 领取后发布 active，job 结束时由 ticket 释放。
- `VisualAux` 等待 gate 后进入后端串行锁；过期结果由前端请求序号丢弃。
- `BackgroundIo` 在领取新工作前等待 inactive；已经运行的下载或扫描不强制中断。
- `PlaybackSideEffect` 在后台执行，不阻塞播放成功返回。
- `InteractiveUi` 不被 gate 阻塞，但只能使用普通资源域。

## Degradation Rules

| Situation                        | Required behavior                                                |
| -------------------------------- | ---------------------------------------------------------------- |
| 快速切换 album                   | 旧 artwork/theme/lyrics 结果由前端请求序号丢弃                   |
| 快速 next/previous               | 新 transition supersede 旧 request；旧下载/probe/buffer 尽快停止 |
| seek 拖拽                        | 前端本地 preview；后端只接收最终 commit                          |
| 下载队列准备开始下一首           | 播放器 `Loading` 时等待 gate inactive                            |
| 下载任务已经传输                 | 不硬杀；继续常规取消与背压，进度事件保持节流                     |
| inventory/search/tag 工作待启动  | 启动前等待 gate inactive                                         |
| 日志、偏好与历史写入             | 不占用 playback runtime                                          |
| HTTP reset / response cache 清理 | fan-out 到 app/playback/image/download 四个资源域                |
| 输出回调拿不到状态锁             | 继续输出音频并跳过状态写入，不得等待                             |

## Verification

### Static Checks

- command registry 覆盖全部 Tauri command 和内部调度入口。
- 非允许模块不能引用专用 API client、playback runtime 或 actor handle。
- 播放启动路径不能引用普通 `api` 拉取歌曲详情或音频流。
- `RealtimeAudio` 路径不得出现 lock wait、await、I/O、日志写入或 channel send wait。

### Concurrency Tests

- 连续 `play_song(A)` -> `play_song(B)` -> `play_song(C)`，只有 C 可进入 `Playing`。
- 连续 seek 只执行最后一次 commit。
- 切换 album 时旧 artwork/theme 请求不能覆盖新状态。
- 播放 `Loading` 期间下载 worker 不领取新 task。
- media key 与前端 next/previous 共享 actor 排序和 supersede 行为。

### Runtime Metrics

按适用域记录：

- `command.domain` / `command.priority`
- `command.queue_wait_ms` / `command.queue_depth` / `command.run_ms`
- `resource_gate.wait_ms`
- `playback.request_id` / `playback.session_id` / `playback.ticket_superseded`
- `playback.loading_ms`
- `audio.callback_silence_due_to_lock` / `audio.callback_underrun_frames`

快速切歌后旧 session 不得继续输出或影响当前事件；`playback.loading_ms` p95 不应随图片、主题或下载并发显著恶化。只有 callback P99/P999、underrun 或 lock silence 指标持续恶化并可稳定复现时，才考虑 scratch 预分配、错误状态无锁化或专用 SPSC ring buffer。

## Change Checklist

涉及播放调度的改动至少运行：

1. `cargo test --manifest-path src-tauri/Cargo.toml player::backend::cpal`
2. `cargo test --manifest-path src-tauri/Cargo.toml player::stream`
3. `cargo test --manifest-path src-tauri/Cargo.toml app_state::playback`
4. `cargo test --manifest-path src-tauri/Cargo.toml player::controller`
5. `bunx vitest run src/lib/api.test.ts`
6. `cargo check --manifest-path src-tauri/Cargo.toml`
7. `git diff --check`

跨前后端状态、事件或 registry 的改动再运行 `bun run check`。
