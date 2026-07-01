# Playback Command Scheduling

> 播放资源隔离与 command 调度改造建议。本文回答“是否需要给播放单独分配资源、是否要细分 command 作用域、是否只靠优先级就够”的架构问题；状态机和音频安全不变量见 [playback-state-machine.md](./playback-state-machine.md)。

## Decision

Harubble 应采用 **Command Domain + 独立资源域 + 降级策略**，而不是把所有任务放在同一个 runtime 后只增加 priority 字段。

推荐结论：

1. 播放仍然留在同一应用进程内；CPAL 输出回调由音频后端调度，不依赖 Tauri async runtime。
2. 播放启动、切歌、seek、音频流下载、probe、初始缓冲必须走专用 `playback_api` 与 `harubble-playback` runtime。
3. 下一步应引入 `CommandRouter` 和 `PlaybackActor`，让 command 先声明作用域、优先级、取消策略和资源域，再进入对应 executor。
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

当前已经具备第一层隔离，并已开始落地调度域：

- `AppState` 内有普通 `api` 和专用 `playback_api`。
- `AppState` 内有专用 `harubble-playback` runtime，当前配置为 2 个 worker threads 和 4 个 blocking threads。
- `src-tauri/src/command_scheduling.rs` 已声明 command domain、priority 和 cancel policy，并用测试覆盖 Tauri command 注册表。
- `play_song`、`seek_current_playback`、`play_next`、`play_previous` 已通过 `dispatch_playback_transition` 调度。
- 系统媒体控制的 next/previous/seek 已通过同一个 playback transition dispatcher 调度。
- 新 playback transition 提交时会先 supersede 旧播放启动 request；若旧会话仍处于 `Loading`，会让其 stop flag 立即失效，避免旧下载/probe/初始缓冲继续占用播放资源。
- 播放启动期间的歌曲详情、音频下载、缓存准备、格式探测、初始缓冲等待已进入播放资源域。
- `PlaybackLoadGate` 已在 playback transition 提交时激活，并用 ticket 防止旧启动任务释放新启动窗口。
- 封面 data URL、主题色提取已在前端串行/合并，后端 image/theme/lyrics command 也已通过 VisualAux 锁串行，并在播放启动 gate 活跃时退让。
- 下载执行循环、本地库存扫描、搜索索引重建、belong 预热和 tag registry 同步在领取新后台工作前会等待播放启动 gate；已运行的下载/扫描不被强行中断。
- 收听历史记录已从播放启动主路径移到后台 side effect，失败只写日志，不阻塞 `play_song` 成功返回。
- CPAL 回调已经避免等待 `SampleBuffer` 或 `PlayerState` 锁；拿不到锁时只静音或跳过进度写入。

仍需要继续收敛的风险：

- `dispatch_playback_transition` 仍是轻量 dispatcher，不是完整 actor；它已经统一 request 创建和 supersede，但还没有独立 inbox、队列观测和 side-effect 调度。
- 普通 UI 和偏好保存仍不被 gate 阻塞；它们只能使用普通资源域，后续需要继续避免长时间同步写入影响交互。
- `playback_api` 已隔离音频下载，但图片/下载任务仍共享普通 `api`；切换专辑时普通资源争抢已通过 gate/串行化缓解，Phase 4 仍应拆出 `image_api` 与 `download_api`。

## Command Domains

所有 Tauri command 和后台入口都应归入以下 domain。每个 domain 声明 executor、优先级、可取消性和播放 Loading 时的策略。

| Domain               | Priority | Examples                                                        | Executor                                    | Loading policy                                              |
| -------------------- | -------- | --------------------------------------------------------------- | ------------------------------------------- | ----------------------------------------------------------- |
| `RealtimeAudio`      | P0       | CPAL output callback                                            | OS/audio backend thread                     | 永不等待业务锁；失败只静音、跳过进度或停止当前 session      |
| `PlaybackControl`    | P1       | pause, resume, stop, toggle, volume, get state                  | 直接调用 `AudioPlayer` 的短路径             | 允许执行，但不得做网络/磁盘重活                             |
| `PlaybackTransition` | P1       | play song, next, previous, seek commit                          | `PlaybackActor` on `harubble-playback`      | 可抢占旧启动；同类请求 latest-wins                          |
| `PlaybackStartupIo`  | P1       | song detail, audio stream, cache prepare, probe, initial buffer | `playback_api` + playback blocking pool     | 只服务当前 playback ticket；旧 ticket 立即 supersede/cancel |
| `PlaybackSideEffect` | P2       | listening history, scrobble, media metadata artwork refresh     | side-effect/background executor             | 延后到 `Playing` 后，不阻塞播放成功返回                     |
| `InteractiveUi`      | P3       | albums, album detail, search, preferences, logs, tags           | app runtime + ordinary `api`                | 允许执行，但不得使用播放资源域                              |
| `VisualAux`          | P4       | image data URL, theme extraction, lyrics                        | visual queue, concurrency 1                 | `Loading` 时延迟、合并、取消过期 album 请求                 |
| `BackgroundIo`       | P5       | download worker, inventory scan, metadata scan, cache warmup    | background executor + ordinary/download API | `Loading` 时不启动新任务；已启动任务只走常规取消/背压       |
| `Maintenance`        | P2/P3    | clear cache, reset HTTP client, recovery                        | router fan-out                              | 必须显式声明是否影响 playback client；默认避开播放启动窗口  |

优先级含义：

- P0 不进入 async 调度队列。
- P1 只给播放控制和播放启动使用。
- P2 可以影响播放环境，但必须短、可解释、可观测。
- P3-P5 都是可退让任务，不能反向等待播放域。

## Proposed Architecture

```text
Frontend / media keys / lifecycle
  -> Tauri command shim
  -> CommandRouter
       -> RealtimeAudio: CPAL callback path, no async wait
       -> PlaybackControl: direct short AudioPlayer operation
       -> PlaybackActor: play/seek/next/previous, latest-wins
       -> VisualQueue: artwork/theme/lyrics, serialized and cancellable
       -> BackgroundQueue: downloads/inventory/cache warmup
       -> AppRuntime: normal UI/data commands
```

### CommandRouter

`CommandRouter` 是后端统一入口，不需要改变对外 Tauri command 名称。每个 command shim 只负责解析入参并调用 router。

建议结构：

```rust
enum CommandDomain {
    PlaybackControl,
    PlaybackTransition,
    PlaybackStartupIo,
    PlaybackSideEffect,
    InteractiveUi,
    VisualAux,
    BackgroundIo,
    Maintenance,
}

enum CommandPriority {
    Realtime,
    Playback,
    CriticalSideEffect,
    Interactive,
    Visual,
    Background,
}

enum CancelPolicy {
    NeverCancel,
    LatestWins { key: &'static str },
    SupersedePlaybackSession,
    Cooperative,
}

struct CommandSpec {
    name: &'static str,
    domain: CommandDomain,
    priority: CommandPriority,
    cancel_policy: CancelPolicy,
}
```

第一阶段可以不用做复杂宏，只要把 command registry 做成静态表，并加测试保证每个 `#[tauri::command]` 都有归属即可。

### PlaybackActor

`PlaybackActor` 负责把 play/next/previous/seek 串成一个明确的播放控制流。

建议规则：

- `PlaySelection`、`Next`、`Previous` 抢占旧的 `PlaySelection` / `Seek`。
- 高频 seek 只保留最后一次提交；拖拽过程中的 preview 不进后端。
- actor 为每次启动创建 `PlaybackTicket`，包含 `request_id`、预期 `session_id`、取消 token、开始时间和 intent。
- 所有 metadata/audio/probe/buffer 子任务都必须携带 ticket；ticket 失效后立即停止写缓存、停止等待初始缓冲，并返回 `Superseded`。
- actor 只编排播放；CPAL callback、decode worker 内部实时路径仍遵守状态机文档中的锁规则。

示意：

```text
PlaybackActor inbox
  high: Stop, PlaySelection, Next, Previous
  normal: SeekCommit
  low: PreloadCandidate

on command:
  cancel previous startup ticket if superseded
  begin_playback_request
  begin_loading_session
  prepare input/probe/buffer with ticket
  open output stream
  emit Playing or fail/superseded
```

### ResourceRegistry

把当前散落在 `AppState` 上的 runtime/client 收敛为一个资源注册表，减少误用。

目标形态：

```text
ResourceRegistry
  api.app          -> library/search/preferences/logging
  api.playback     -> song detail + audio stream for active playback
  api.image        -> cover/theme/lyrics, coalesced and low priority
  api.download     -> download worker, large transfers
  runtime.playback -> playback actor + startup async work
  runtime.blocking_playback -> probe/open/cache cleanup with small cap
  runtime.background -> download write, inventory scan, history side effect
```

短期可以先保留 `api` / `playback_api`，但文档和测试要禁止非播放代码拿 `playback_api`，也禁止播放启动拿普通 `api`。

### PlaybackLoadGate

`PlaybackLoadGate` 是跨 domain 的退让信号，不是互斥锁。

当前落地：

- playback transition 提交时发布 gate active；ticket drop 时释放，旧 ticket 不能释放更新的 gate。
- `VisualAux` 进入 gate 后延迟约 350ms，再通过后端 `visual_aux_lock` 串行执行。
- album/song 过期结果丢弃仍由前端请求序号与缓存 key 负责；后端目前只保证不并发抢资源。
- `BackgroundIo` 在领取新 job/task 前等待 gate inactive；已在运行的下载不硬停，但进度事件继续节流。
- `PlaybackSideEffect` 默认在后台执行，不阻塞播放成功返回。
- `InteractiveUi` 不被 gate 阻塞，但只能使用普通资源域。

这比“全局暂停其他任务”更稳，因为不会让 UI 卡死，也不会强杀已经打开的文件/网络连接。

## Command Mapping

当前 command 建议归属如下。

| Command / entry                                                      | Domain                                          | Notes                                                        |
| -------------------------------------------------------------------- | ----------------------------------------------- | ------------------------------------------------------------ |
| `play_song`                                                          | `PlaybackTransition`                            | 进入 `PlaybackActor`，supersede 旧启动                       |
| `seek_current_playback`                                              | `PlaybackTransition`                            | 高频输入前端节流；后端 latest-wins                           |
| `play_next`, `play_previous`                                         | `PlaybackTransition`                            | media key 与 UI 共用同一路径                                 |
| system media next/previous/seek                                      | `PlaybackTransition`                            | 不直接 spawn 任意 async task，统一 dispatch                  |
| `pause_playback`, `resume_playback`                                  | `PlaybackControl`                               | 短路径，不做网络/磁盘                                        |
| `get_player_state`                                                   | `PlaybackControl`                               | 只读快照                                                     |
| `set_playback_volume`                                                | `PlaybackControl` + `PlaybackSideEffect`        | 音量立即生效；偏好持久化可 side-effect 化                    |
| `get_albums`, `get_album_detail`, `get_song_detail`, search/homepage | `InteractiveUi`                                 | 普通 `api`，不得进入 playback runtime                        |
| `get_song_lyrics`                                                    | `VisualAux`                                     | 播放 Loading 时延后，结果必须按 song/session 校验            |
| `get_image_data_url`, `extract_image_theme`                          | `VisualAux`                                     | concurrency 1、in-flight dedupe、album switch cancel         |
| download create/list/cancel/retry                                    | `InteractiveUi` command + `BackgroundIo` worker | command 只改队列；worker 受 gate 约束                        |
| download execution loop                                              | `BackgroundIo`                                  | Loading 时不领取新 job/task                                  |
| inventory scan / audio metadata scan                                 | `BackgroundIo`                                  | 可暂停或降低并发                                             |
| cache clear / HTTP reset                                             | `Maintenance`                                   | 必须声明是否影响 playback client，并避开非必要的播放启动窗口 |
| listening history record                                             | `PlaybackSideEffect`                            | 不应阻塞 `play_song` 成功返回                                |

## Degradation Rules

播放启动或切换专辑时，低优先级 domain 的行为必须可预测。

| Situation                | Required behavior                                                                                      |
| ------------------------ | ------------------------------------------------------------------------------------------------------ |
| 用户快速切换 album       | 旧 album 的 image/theme/lyrics 请求取消或结果丢弃；新 album 请求串行开始                               |
| 用户快速 next/previous   | 旧 playback ticket superseded；旧下载/probe/buffer 等待尽快停止                                        |
| seek 拖拽中              | 前端本地 preview；后端只接收最终 seek commit                                                           |
| 下载队列正准备开始下一首 | 若播放器 `Loading`，等待 gate inactive                                                                 |
| 下载任务已经在传输       | 不硬杀；继续常规取消/背压，进度事件节流                                                                |
| 日志/偏好/历史写入       | 不占用 playback runtime；必要时延后                                                                    |
| HTTP reset               | 普通 client 可立即 reset；playback client 若正在 Loading，除非用户明确修复网络，否则延后到当前启动结束 |
| 输出回调拿不到锁         | 输出静音或跳过状态写入；不得等待                                                                       |

## Migration Plan

### Phase 1: Command Registry

- 新增 `CommandSpec` 静态表。
- 为现有 Tauri command 标注 domain/priority/cancel policy。
- 增加测试：每个 command 必须有 spec；`playback_api` 只允许播放启动路径使用；`playback_runtime` 只允许播放路径使用。
- 文档 gate：修改 command 时必须同步本文件或 registry。

### Phase 2: PlaybackActor

- 用 actor 替代 `run_on_playback_runtime` 直接 spawn 的 play/seek/next/previous。
- media controls 也 dispatch 到 actor。
- 引入 `PlaybackTicket`，把 request/session/cancel/intent 传入 song detail、download stream、probe 和 initial buffer。
- 将 listening history 从播放启动主路径移到 `PlaybackSideEffect`。

### Phase 3: Resource Gate And Aux Queues

- 已引入 `PlaybackLoadGate`，并接入 playback transition dispatcher。
- 已为后端 image/theme/lyrics command 增加 gate 检查与后端串行锁；过期结果丢弃仍由前端请求序号负责。
- 已保留前端 image 串行/合并策略作为第一道保护，后端 gate/锁作为第二道保护。
- 已让下载 worker、inventory scan、搜索索引重建、belong/tag 预热在启动新工作前等待 gate inactive。

### Phase 4: Split More Clients

- 在 `api` / `playback_api` 之外增加 `image_api` 和 `download_api`。
- `download_api` 可配置更低并发或独立连接池，避免大文件下载影响普通 UI。
- `image_api` 支持 in-flight dedupe 和短超时，失败不影响播放。
- `reset_http_client` 和 `clear_response_cache` 通过 registry fan-out，并按 domain 记录日志。

### Phase 5: Optional OS/Process Isolation

只有在 Phase 1-4 后仍有明确证据显示 WebView 或后台任务会让音频 callback 错过 deadline，才考虑更重的隔离：

- macOS/Windows/Linux 的线程优先级或音频工作组能力需要平台分别实现，不能作为跨平台默认方案。
- 单独播放进程适合隔离崩溃或第三方播放后端，但会增加 IPC、媒体键、状态同步、缓存句柄和错误恢复复杂度。
- 如果未来切到 MPV/GStreamer 一类外部 backend，进程隔离可以作为 backend adapter，而不是让所有 Harubble 播放逻辑拆出去。

## Verification

### Static Checks

- command registry 覆盖全部 Tauri command。
- 非播放模块不能引用 `playback_api`。
- 非播放模块不能引用 `playback_runtime` 或 `PlaybackActor` 内部 handle。
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

建议新增结构化日志字段：

- `command.domain`
- `command.priority`
- `command.queue_wait_ms`
- `command.run_ms`
- `playback.session_id`
- `playback.request_id`
- `playback.ticket_superseded`
- `playback.loading_ms`
- `audio.callback_silence_due_to_lock`
- `audio.callback_underrun_frames`
- `resource_gate.wait_ms`

验收目标：

- 快速切歌/切专辑时旧 session 不再有输出或 ended/progress 事件影响当前 session。
- `audio.callback_silence_due_to_lock` 只能偶发，不能随 album 切换持续升高。
- `playback.loading_ms` p95 不随图片/主题/下载任务并发显著恶化。
- 下载和视觉任务的 queue wait 增加是可接受降级；音频噪声和错格式输出不可接受。

## Change Checklist

涉及播放调度的改动至少验证：

1. `rtk cargo test --manifest-path src-tauri/Cargo.toml player::backend::cpal`
2. `rtk cargo test --manifest-path src-tauri/Cargo.toml player::stream`
3. `rtk cargo test --manifest-path src-tauri/Cargo.toml app_state::playback`
4. `rtk cargo test --manifest-path src-tauri/Cargo.toml player::controller`
5. `rtk bunx vitest run src/lib/api.test.ts`
6. `rtk cargo check --manifest-path src-tauri/Cargo.toml`
7. `rtk git diff --check`

跨前后端状态、事件或 command registry 的改动，再跑 `rtk bun run check`。
