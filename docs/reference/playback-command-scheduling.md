# Playback Command Scheduling

> 播放资源隔离与 command 调度当前实现。本文记录播放资源域、command 作用域、优先级、退让策略与可观测性；状态机和音频安全不变量见 [playback-state-machine.md](./playback-state-machine.md)。

## Decision

Harubble 应采用 **Command Domain + 独立资源域 + 降级策略**，而不是把所有任务放在同一个 runtime 后只增加 priority 字段。

当前结论：

1. 播放仍然留在同一应用进程内；CPAL 输出回调由音频后端调度，不依赖 Tauri async runtime。
2. 播放启动、切歌、seek、音频流下载、probe、初始缓冲走专用 `playback_api`、`PlaybackActor` 与 `harubble-playback` runtime。
3. command 作用域、优先级与取消策略由 `src-tauri/src/command_scheduling.rs` 的静态 registry 声明，并由测试覆盖 Tauri command 与内部后台入口。
4. priority 只用于同一作用域内的局部排序；真正保护音频的是资源域隔离、可取消、可降级和实时回调不阻塞。
5. 单独进程暂不作为首选。它能隔离崩溃，但不能天然解决网络、磁盘和输出设备争抢，还会引入 IPC、状态同步和媒体控制复杂度。

## External References

这些项目只作为架构模式参考，不复制实现。

| Project                                                                        | Relevant pattern                                                                                                                                                                               | Harubble takeaway                                                             |
| ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| [MPD / Music Player Daemon](https://github.com/MusicPlayerDaemon/MPD)          | `src/player/Thread.cxx` 将 player thread 定义为 decoder thread 与 output thread(s) 之间的桥接，且 player thread 自身不做 I/O；`src/player/Control.hxx` 用明确的 `PlayerCommand` 驱动播放线程。 | 播放核心应有独立控制入口；音频数据面和输出面不应被普通 UI/库操作直接占用。    |
| [Strawberry Music Player](https://github.com/strawberrymusicplayer/strawberry) | `src/core/player.h` 让 `Player` 编排 playlist/url handler/engine；`src/core/player.cpp` 在曲目即将结束时预加载下一首，实际播放交给 engine。                                                    | command 层应只做编排；播放 engine、预加载、URL 解析和 UI 状态同步要保持边界。 |
| [Feishin](https://github.com/jeffvli/feishin)                                  | README 明确桌面端支持 MPV player backend 与 Web player backend。                                                                                                                               | 播放后端应是清晰的可替换资源域，UI/API 调用不应假设只有一种播放执行路径。     |

## Current State

当前实现：

- `AppState` 内有普通 `api`、专用 `playback_api`、视觉辅助 `image_api` 和下载 `download_api`。
- `AppState` 内有专用 `harubble-playback` runtime，当前配置为 2 个 worker threads 和 4 个 blocking threads。
- `src-tauri/src/command_scheduling.rs` 已声明 command domain、priority 和 cancel policy，并用测试覆盖 Tauri command 注册表。
- `play_song`、`seek_current_playback`、`play_next`、`play_previous` 已通过 `PlaybackActor` inbox 调度。
- 系统媒体控制的 next/previous/seek 已通过同一个 `PlaybackActor` 调度。
- 新 playback transition 被 actor 领取时会先 supersede 旧播放启动 request；若旧会话仍处于 `Loading`，会让其 stop flag 立即失效，避免旧下载/probe/初始缓冲继续占用播放资源。
- 播放启动期间的歌曲详情、音频下载、缓存准备、格式探测、初始缓冲等待已进入播放资源域。
- `PlaybackLoadGate` 已在 playback transition 提交时激活，并用 ticket 防止旧启动任务释放新启动窗口。
- 封面 data URL、主题色提取已在前端串行/合并，后端 image/theme/lyrics command 也已通过 VisualAux 锁串行，并在播放启动 gate 活跃时退让。
- 下载执行循环、本地库存扫描、搜索索引重建、belong 预热和 tag registry 同步在领取新后台工作前会等待播放启动 gate；已运行的下载/扫描不被强行中断。
- 收听历史记录已从播放启动主路径移到 `PlaybackSideEffect` 后台入口，失败只写日志，不阻塞 `play_song` 成功返回。
- playback transition、playback startup、VisualAux 已记录 `command.domain`、`command.priority`、`command.queue_wait_ms`、`command.run_ms`、`playback.session_id`、`playback.request_id`、`playback.loading_ms` 和 supersede 状态；playback side effect 目前没有独立排队层，只记录 `command.domain`、`command.priority`、`command.run_ms`；VisualAux 与 BackgroundIo 退让路径已记录 `resource_gate.wait_ms`。
- CPAL 回调已经避免等待 `SampleBuffer` 或 `PlayerState` 锁；拿不到锁时只静音或跳过进度写入，并通过 monitor 线程聚合记录 `audio.callback_silence_due_to_lock` 与 `audio.callback_underrun_frames`。

仍需要继续收敛的风险：

- 普通 UI 和偏好保存仍不被 gate 阻塞；它们只能使用普通资源域，后续需要继续避免长时间同步写入影响交互。
- `image_api` / `download_api` 已隔离普通 UI client；`image_api` 使用更短超时与小资源上限，`download_api` 使用独立连接池配置。

## Command Domains

所有 Tauri command 和内部后台入口都在 `COMMAND_SPECS` 中声明 domain、priority 和 cancel policy。当前 registry 使用以下 domain。

| Domain               | Priority labels                         | Examples                                                               | Executor / resource                                 | Loading policy                                     |
| -------------------- | --------------------------------------- | ---------------------------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------- |
| `PlaybackControl`    | `Playback`                              | pause, resume, volume, get state                                       | 直接调用 `AudioPlayer` 的短路径                     | 允许执行，但不得做网络/磁盘重活                    |
| `PlaybackTransition` | `Playback`                              | play song, next, previous, seek commit                                 | `PlaybackActor` on `harubble-playback`              | 新 transition 会 supersede 旧启动                  |
| `PlaybackSideEffect` | `CriticalSideEffect`                    | record / clear listening history                                       | app runtime side-effect helper                      | 不阻塞播放成功返回                                 |
| `InteractiveUi`      | `Interactive`                           | albums, search, preferences, logs, tags, collections, download list    | app runtime + ordinary `api`                        | 不被 gate 阻塞，但不得使用播放资源域               |
| `VisualAux`          | `Visual`                                | image data URL, theme extraction, lyrics                               | `image_api` + backend `visual_aux_lock`             | `Loading` 时等待 gate，并串行执行                  |
| `BackgroundIo`       | `Background` 或交互取消用 `Interactive` | download create/cancel/retry, download loop, inventory, search, warmup | background tasks + ordinary `api` 或 `download_api` | 领取新后台工作前等待 gate；已启动任务不硬停        |
| `Maintenance`        | `CriticalSideEffect`                    | clear audio cache, clear response cache, reset HTTP clients            | app/playback/image/download 四个资源域的 fan-out    | 按命令语义执行；`clear_audio_cache` 会先停止播放器 |

优先级标签含义：

- `Playback`：播放控制和播放 transition。
- `CriticalSideEffect`：维护入口或播放相关副作用，必须短、可解释、可观测。
- `Interactive`：用户交互入口，不能反向等待播放域。
- `Visual`：可退让的视觉辅助任务。
- `Background`：下载、扫描、搜索索引和预热等后台任务。

CPAL 输出回调不进入 `COMMAND_SPECS`；它通过音频后端线程运行，并在指标中记录 `command.domain = "RealtimeAudio"`、`command.priority = "Realtime"`。

## Architecture

```text
Frontend / media keys / lifecycle
  -> Tauri command shim
       -> RealtimeAudio: CPAL callback path, no async wait
       -> PlaybackControl: direct short AudioPlayer operation
       -> PlaybackActor: play/seek/next/previous, unbounded inbox on harubble-playback
       -> VisualAux: image/theme/lyrics through image_api, playback gate + backend serial lock
       -> BackgroundIo: downloads/inventory/search/tag warmup through background gate
       -> AppRuntime: normal UI/data commands
```

### Command Registry

`src-tauri/src/command_scheduling.rs` 是当前 command 调度真相来源。它维护 `COMMAND_SPECS` 静态表，把每个 Tauri command 和内部后台入口映射到 `CommandDomain`、`CommandPriority` 与 `CancelPolicy`。

当前 registry 还通过测试约束三类资源域：

- `playback_api`、`playback_runtime` 和 `PlaybackActor` 只能出现在播放路径、媒体控制、网络重置、播放 gate 与 actor 内部。
- `image_api` 只能用于 library 视觉辅助、通知封面、网络重置和 `AppState` 初始化。
- `download_api` 只能用于下载 command、下载桥接、网络重置和 `AppState` 初始化。

### PlaybackActor

`PlaybackActor` 负责把 play/next/previous/seek 串成一个明确的播放控制流。

当前规则：

- `play_song`、`seek_current_playback`、`play_next`、`play_previous` 和系统媒体 next/previous/seek 统一提交到 actor inbox。
- actor 在领取消息时调用 `begin_playback_transition`，推进 request id，并在旧会话仍处于 `Loading` 时让旧 stop flag 立即失效。
- 每个 actor job 持有一个 `PlaybackLoadGate` ticket；job 完成后释放 ticket，避免旧启动窗口误释放新窗口。
- 具体播放启动流程仍由 `AppState` helper 执行，并用 `request_id`、`session_id`、`stop_flag` 和 gate ticket 完成 supersede 与取消保护。
- actor 记录 `command.queue_wait_ms`、`command.queue_depth`、`command.run_ms`、`playback.request_id` 与 `playback.ticket_superseded`。

```text
PlaybackActor inbox
  -> begin_playback_transition
  -> enter PlaybackLoadGate
  -> run AppState playback helper
  -> drop gate ticket
  -> record playback.transition_completed metrics
```

### Resource Layout

当前没有单独的 `ResourceRegistry` 类型；资源注册表由 `AppState` 字段和 `command_scheduling.rs` 的静态测试共同承担。四个 HTTP client 已在 `AppState` 中归拢到 `api_clients: ApiClients` 子结构，访问路径为 `state.api_clients.api` 等。

```text
AppState
  api_clients
    api              -> library/search/preferences/logging/tag/collection
    playback_api     -> song detail + audio stream for active playback
    image_api        -> cover/theme/lyrics/notification artwork, low priority
    download_api     -> download job preparation + large transfers
  playback_runtime   -> PlaybackActor + playback startup async/blocking work
  playback_actor     -> playback transition inbox
  playback_load_gate -> cross-domain startup backpressure signal
  visual_aux_lock    -> backend serialization for image/theme/lyrics
```

当前已拆出四个 API client，并用静态测试限制资源域误用：

- `api`：普通 UI、首页、tag、搜索索引和轻量数据读取。
- `playback_api`：播放启动、音频流下载、probe 前置数据。
- `image_api`：封面 data URL、主题色、歌词、通知封面临时缓存；短超时、小资源上限。
- `download_api`：下载任务准备、专辑封面落盘、歌词侧车和音频大文件下载；独立连接池配置。

### PlaybackLoadGate

`PlaybackLoadGate` 是跨 domain 的退让信号，不是互斥锁。

当前落地：

- `PlaybackActor` 领取 playback transition 后发布 gate active；actor job drop ticket 时释放，旧 ticket 不能释放更新的 gate。
- `VisualAux` 进入 gate 后延迟约 350ms，再通过后端 `visual_aux_lock` 串行执行。
- album/song 过期结果丢弃仍由前端请求序号与缓存 key 负责；后端目前只保证不并发抢资源。
- `BackgroundIo` 在领取新 job/task 前等待 gate inactive；已在运行的下载不硬停，但进度事件继续节流。
- `PlaybackSideEffect` 在后台执行，不阻塞播放成功返回。
- `InteractiveUi` 不被 gate 阻塞，但只能使用普通资源域。

这比“全局暂停其他任务”更稳，因为不会让 UI 卡死，也不会强杀已经打开的文件/网络连接。

## Command Mapping

当前 command 归属如下。

| Command / entry                                                      | Domain                            | Notes                                                         |
| -------------------------------------------------------------------- | --------------------------------- | ------------------------------------------------------------- |
| `play_song`                                                          | `PlaybackTransition`              | 进入 `PlaybackActor`，supersede 旧启动                        |
| `seek_current_playback`                                              | `PlaybackTransition`              | 高频输入前端节流；后端 latest-wins                            |
| `play_next`, `play_previous`                                         | `PlaybackTransition`              | media key 与 UI 共用同一路径                                  |
| system media next/previous/seek                                      | `PlaybackTransition`              | 不直接 spawn 任意 async task，统一 dispatch                   |
| `pause_playback`, `resume_playback`                                  | `PlaybackControl`                 | 短路径，不做网络/磁盘                                         |
| `get_player_state`                                                   | `PlaybackControl`                 | 只读快照                                                      |
| `set_playback_volume`                                                | `PlaybackControl`                 | 音量立即生效；偏好持久化仍走普通偏好写入锁                    |
| `get_albums`, `get_album_detail`, `get_song_detail`, search/homepage | `InteractiveUi`                   | 普通 `api`，不得进入 playback runtime                         |
| `get_song_lyrics`                                                    | `VisualAux`                       | 播放 Loading 时延后；通过 `image_api` 执行                    |
| `get_image_data_url`, `extract_image_theme`                          | `VisualAux`                       | 后端 `visual_aux_lock` 串行；过期结果由前端请求序号丢弃       |
| download create/list/cancel/retry                                    | `BackgroundIo` 或 `InteractiveUi` | 创建/取消/重试/清理为 BackgroundIo；list/get 为 InteractiveUi |
| download execution loop                                              | `BackgroundIo`                    | Loading 时不领取新 job/task                                   |
| inventory scan / audio metadata scan                                 | `BackgroundIo`                    | 新扫描在 gate active 时等待；取消为交互优先级                 |
| search rebuild / belong warmup / tag registry sync                   | `BackgroundIo`                    | 启动新后台工作前等待 gate inactive                            |
| cache clear / HTTP reset                                             | `Maintenance`                     | fan-out 到 app/playback/image/download 四个资源域             |
| listening history record                                             | `PlaybackSideEffect`              | 不应阻塞 `play_song` 成功返回                                 |

## Degradation Rules

播放启动或切换专辑时，低优先级 domain 的行为必须可预测。

| Situation                        | Required behavior                                                             |
| -------------------------------- | ----------------------------------------------------------------------------- |
| 用户快速切换 album               | 旧 album 的 image/theme/lyrics 结果由前端请求序号丢弃；新请求经后端锁串行执行 |
| 用户快速 next/previous           | 新 transition supersede 旧 request；旧下载/probe/buffer 等待尽快停止          |
| seek 拖拽中                      | 前端本地 preview；后端只接收最终 seek commit                                  |
| 下载队列正准备开始下一首         | 若播放器 `Loading`，等待 gate inactive                                        |
| 下载任务已经在传输               | 不硬杀；继续常规取消/背压，进度事件节流                                       |
| inventory/search/tag 预热启动    | 启动前等待 gate inactive                                                      |
| 日志/偏好/历史写入               | 不占用 playback runtime；历史记录进入 `PlaybackSideEffect`                    |
| HTTP reset / response cache 清理 | 立即 fan-out 到 app/playback/image/download 四个资源域                        |
| 输出回调拿不到锁                 | 输出静音或跳过状态写入；不得等待                                              |

## Implementation Status

### Command Registry

- 已新增 `CommandSpec` 静态表。
- 已为现有 Tauri command 和内部后台入口标注 domain/priority/cancel policy。
- 已增加测试：每个 command 必须有 spec；`playback_api`、`playback_runtime`、`PlaybackActor`、`image_api` 和 `download_api` 只能在允许的模块中出现。
- 文档 gate：修改 command 时必须同步本文件或 registry。

### PlaybackActor

- 已用 `PlaybackActor` inbox 替代 play/seek/next/previous 的直接 spawn；command shim 继续通过 `dispatch_playback_transition` 保持对外签名稳定。
- media controls 的 next/previous/seek 已 dispatch 到同一个 actor。
- 已用 `request_id`、`session_id`、`stop_flag` 和 `PlaybackLoadGate` ticket 约束 song detail、download stream、probe 和 initial buffer；后续如需要更强观测，可再收敛为显式 `PlaybackTicket` 结构。
- listening history 已从播放启动主路径移到 `PlaybackSideEffect`，并记录 side-effect run 指标。

### Resource Gate And Aux Queues

- 已引入 `PlaybackLoadGate`，并接入 `PlaybackActor`。
- 已为后端 image/theme/lyrics command 增加 gate 检查与后端串行锁，并记录 VisualAux queue/run/gate 指标；过期结果丢弃仍由前端请求序号负责。
- 已保留前端 image 串行/合并策略作为第一道保护，后端 gate/锁作为第二道保护。
- 已让下载 worker、inventory scan、搜索索引重建、belong/tag 预热在启动新工作前等待 gate inactive，并在实际等待 gate 时记录 BackgroundIo gate 指标。

### Split Clients

- 已在 `api` / `playback_api` 之外增加 `image_api` 和 `download_api`。
- 已将 image/theme/lyrics/通知封面切到 `image_api`。
- 已将下载任务准备、下载执行循环、专辑封面落盘、歌词侧车和音频大文件下载切到 `download_api`。
- 已让 `reset_http_client` 和 `clear_response_cache` fan-out 到 app/playback/image/download 四个资源域。
- 已为 `image_api` 配置短超时和小资源大小限制；已为 `download_api` 配置独立连接池参数。
- 已为 playback transition、playback side effect、VisualAux 和 BackgroundIo gate 等待补结构化日志字段；待继续：必要时再为 download worker 增加显式并发令牌。

### Optional OS/Process Isolation

只有在当前资源隔离、gate 和 actor 方案仍无法保护音频 callback deadline 时，才考虑更重的隔离：

- macOS/Windows/Linux 的线程优先级或音频工作组能力需要平台分别实现，不能作为跨平台默认方案。
- 单独播放进程适合隔离崩溃或第三方播放后端，但会增加 IPC、媒体键、状态同步、缓存句柄和错误恢复复杂度。
- 如果未来切到 MPV/GStreamer 一类外部 backend，进程隔离可以作为 backend adapter，而不是让所有 Harubble 播放逻辑拆出去。

## Verification

### Static Checks

- command registry 覆盖全部 Tauri command 和内部调度入口。
- 非播放模块不能引用 `playback_api`。
- 非播放模块不能引用 `playback_runtime` 或 `PlaybackActor` 内部 handle。
- 非视觉辅助模块不能引用 `image_api`。
- 非下载模块不能引用 `download_api`。
- 播放启动路径不能引用普通 `api` 拉取歌曲详情或音频流。
- `RealtimeAudio` 路径不得出现 blocking lock、await 或 channel send wait。

### Concurrency Tests

- 连续 `play_song(A)` -> `play_song(B)` -> `play_song(C)`，只有 C 可进入 `Playing`。
- 连续 seek 只执行最后一次 commit。
- 切换 album 时旧 image/theme 请求不会覆盖新 album 状态。
- 播放 `Loading` 期间下载 worker 不领取新 task。
- `set_playback_volume` 在偏好保存卡住时仍能立即影响输出增益。
- media key next/previous 与前端 next/previous 共享 actor 排序和 supersede 行为。

### Runtime Metrics

已为 playback transition、playback startup、playback side effect、VisualAux、BackgroundIo gate 等待以及 RealtimeAudio 指标按适用域落地以下结构化日志字段：

- `command.domain`
- `command.priority`
- `command.queue_wait_ms`（playback side effect 目前没有独立排队层，不记录该字段；未来接入排队器时再补上）
- `command.run_ms`
- `resource_gate.wait_ms`
- `playback.request_id`
- `playback.session_id`
- `playback.ticket_superseded`
- `playback.loading_ms`
- `command.queue_depth`
- `audio.callback_silence_due_to_lock`
- `audio.callback_underrun_frames`

本文件规划的调度与运行时观测项已完成；后续新增 command 或资源域时，继续先更新 registry、资源域约束和本节指标清单。

验收目标：

- 快速切歌/切专辑时旧 session 不再有输出或 ended/progress 事件影响当前 session。
- `audio.callback_silence_due_to_lock` 只能偶发，不能随 album 切换持续升高。
- `playback.loading_ms` p95 不随图片/主题/下载任务并发显著恶化。
- 下载和视觉任务的 queue wait 增加是可接受降级；音频噪声和错格式输出不可接受。

## Change Checklist

涉及播放调度的改动至少验证：

1. `cargo test --manifest-path src-tauri/Cargo.toml player::backend::cpal`
2. `cargo test --manifest-path src-tauri/Cargo.toml player::stream`
3. `cargo test --manifest-path src-tauri/Cargo.toml app_state::playback`
4. `cargo test --manifest-path src-tauri/Cargo.toml player::controller`
5. `bunx vitest run src/lib/api.test.ts`
6. `cargo check --manifest-path src-tauri/Cargo.toml`
7. `git diff --check`

跨前后端状态、事件或 command registry 的改动，再跑 `bun run check`。
