# Playback Command Scheduling

> 播放资源隔离、command 调度与退让规则。状态机和音频安全不变量见 [playback-state-machine.md](./playback-state-machine.md)。

## Decision

Harubble 使用 **Command Domain + 独立资源域 + 降级策略**：

1. CPAL 输出回调由音频后端调度，不依赖 Tauri async runtime。
2. 播放启动、切歌、seek、音频流下载、probe 与初始缓冲使用专用 `playback_api`、`PlaybackActor` 和 `harubble-playback` runtime。
3. Tauri command 的 handler path 与调度标签在 `src-tauri/src/command_registry.rs` 的同一条目声明；`command_scheduling.rs` 从该宏派生 Tauri specs，并单独声明内部后台入口。
4. `priority` 与 `cancel policy` 当前是日志、查询和架构断言使用的声明式元数据，不是通用优先级队列或取消执行器。实际排序、supersede 与退让由 `PlaybackActor`、`PlaybackLoadGate` 和各命令自己的控制流实现。

## Command Domains

| Domain               | Declared priority labels                | Examples                                                                   | Actual execution path                                                   | Loading policy                                               |
| -------------------- | --------------------------------------- | -------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ------------------------------------------------------------ |
| `PlaybackControl`    | `Playback`                              | pause, resume, volume, get state                                           | `AudioPlayer` 短路径                                                    | 允许执行，但不得做网络或磁盘重活                             |
| `PlaybackTransition` | `Playback`                              | play, next, previous, seek commit                                          | `PlaybackActor` on `harubble-playback`                                  | actor 通过 request/session 语义让新 transition 取代旧启动    |
| `PlaybackSideEffect` | `CriticalSideEffect`                    | `record_song_heat`、`clear_listening_history`                              | record 走普通 API + `spawn_blocking`；clear 走 side-effect helper       | 与播放启动解耦；domain 标签本身不负责分发                    |
| `InteractiveUi`      | `Interactive`                           | albums, search, preferences, logs, tags, collections                       | app runtime + ordinary `api`                                            | 不被 gate 阻塞，但不得使用播放资源域                         |
| `VisualAux`          | `Visual`                                | lyrics、theme extraction、cached artwork、菜单同步和主题包 inspect/preview | 仅 lyrics/theme extraction 进入 `dispatch_visual_aux`；其余各走自身路径 | dispatcher 内任务等待 gate 并串行；cached artwork 刻意不串行 |
| `BackgroundIo`       | `Background` 或交互取消用 `Interactive` | downloads, inventory, search rebuild, warmup, tag registry sync            | background tasks + ordinary `api` 或 `download_api`                     | 已接入入口先无超时等 gate；下载另有残留 loading 阈值         |
| `Maintenance`        | `CriticalSideEffect`                    | cache clear, HTTP client reset                                             | app/playback/image/download 资源域 fan-out                              | 按命令语义执行；清理音频缓存前停止播放器                     |

CPAL 输出回调不进入 command spec；指标中使用 `command.domain = "RealtimeAudio"` 和 `command.priority = "Realtime"`。

## Architecture

```text
Frontend / media keys / lifecycle
  -> Tauri command shim
       -> RealtimeAudio: CPAL callback path, no async wait
       -> PlaybackControl: direct short AudioPlayer operation
       -> PlaybackActor: play/seek/next/previous on harubble-playback
       -> VisualAux: metadata classification
            -> lyrics/theme extraction: playback gate + serial lock
            -> cached artwork/menu/theme-package helpers: command-owned path
       -> BackgroundIo: downloads/inventory/search/tag work, background gate
       -> InteractiveUi: normal UI/data commands
```

### Registries

- `command_registry.rs` 是 Tauri handler 注册项和调度标签列表的单一事实源。handler path 交给 `generate_handler!`；相邻名称字面量供 spec 查询与指标使用，并由测试约束为 path 的末段函数名。
- `command_scheduling.rs` 的 `TAURI_COMMAND_SPECS` 由 `for_each_tauri_command!` 自动生成；下载执行循环、库存扫描、搜索重建、belong 预热和 tag registry 同步等非 Tauri 入口维护在 `INTERNAL_COMMAND_SPECS`。
- 覆盖测试检查 registry 派生的 Tauri specs、显式声明的内部 specs、名称唯一性和资源标识符使用范围；它不能自动发现未加入 `INTERNAL_COMMAND_SPECS` 的新内部入口。
- 不在文档中维护逐 command 清单；新增或删除 Tauri command 时修改 registry 条目，并同步前端 bridge/type 与契约测试，不再手工维护第二份 Tauri spec 表。

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
  visual_aux_lock    -> get_song_lyrics / extract_image_theme serialization
```

封面通过 `get_cached_image_path` 写入受大小约束的磁盘缓存，前端再用 Tauri asset protocol 加载；同 URL 的并发请求会合并。主题提取复用同一磁盘缓存，避免重复访问 CDN。

### PlaybackLoadGate

`PlaybackLoadGate` 是跨 domain 的退让信号，不是互斥锁：

- transition 被 actor 领取后发布 active，job 结束时由 ticket 释放。
- `get_song_lyrics` 与 `extract_image_theme` 通过 `dispatch_visual_aux` 等待 gate，再进入后端串行锁；过期结果由前端请求序号丢弃。
- `get_cached_image_path` 虽标记为 `VisualAux`，但直接使用图片缓存，不等待该 gate，也不与主题提取串行；同 URL 请求由缓存层/前端 in-flight 合并。菜单同步和主题包 inspect/preview 也不会仅因 domain 标签自动进入 dispatcher。
- `BackgroundIo` 在领取新工作前等待 gate inactive；该 gate 等待本身没有 30 秒超时，已经运行的下载或扫描也不强制中断。下载执行循环在 gate 返回后还会检查 `PlayerState.is_loading`：若仍为 true，则以 helper 进入时起 30 秒为总阈值继续轮询；触顶记录警告并继续领取工作。若 gate 等待本身已超过阈值，返回后的首次检查即可触顶。
- `PlaybackSideEffect` 只是调度分类：`record_song_heat` 在自身 command 中等待普通 API 查询与 blocking 历史写入，`clear_listening_history` 才进入 `dispatch_playback_side_effect`；两者都不占用 playback runtime，也不阻塞先前已经完成的播放启动。
- `InteractiveUi` 不被 gate 阻塞，但只能使用普通资源域。

## Degradation Rules

| Situation                        | Required behavior                                                                          |
| -------------------------------- | ------------------------------------------------------------------------------------------ |
| 快速切换 album                   | 旧 artwork/theme/lyrics 结果由前端请求序号丢弃；cached artwork 查询不等待 theme dispatcher |
| 快速 next/previous               | 新 transition supersede 旧 request；旧下载/probe/buffer 尽快停止                           |
| seek 拖拽                        | 前端本地 preview；后端只接收最终 commit                                                    |
| 下载队列准备开始下一首           | 先无超时等待 gate inactive；返回后若 `is_loading` 残留，则按 30 秒总阈值轮询，触顶后继续   |
| 下载任务已经传输                 | 不硬杀；继续常规取消与背压，进度事件保持节流                                               |
| inventory/search/tag 工作待启动  | 启动前等待 gate inactive                                                                   |
| 日志、偏好与历史写入             | 不占用 playback runtime                                                                    |
| HTTP reset / response cache 清理 | fan-out 到 app/playback/image/download 四个资源域                                          |
| 输出回调拿不到状态锁             | 继续输出音频并跳过状态写入，不得等待                                                       |

## Verification

### Static Checks And Boundaries

- 宏生成的 Tauri specs 与 handler 共用 registry；测试检查 handler path/name 一致、已声明 spec 的名称不重复。
- 显式内部 specs 只覆盖 `INTERNAL_COMMAND_SPECS` 中已声明的入口；新增内部后台入口仍需人工登记和复核。
- 非允许模块不能引用专用 API client、playback runtime 或 actor handle。
- 播放启动路径不能引用普通 `api` 拉取歌曲详情或音频流。
- `RealtimeAudio` 正常热路径不得新增 lock wait、await、I/O、日志写入或 channel send wait；当前测试覆盖部分具体行为，但没有一项静态分析能完整证明这条约束。

### Automated Coverage

- `command_scheduling::tests` 覆盖 registry/spec 对应、声明元数据和专用资源标识符边界。
- `playback_load_gate::tests` 覆盖旧 ticket 不能释放新 loading window，以及当前 ticket 释放后 waiter 解阻塞。
- `src/lib/api.test.ts` 覆盖同图片请求合并、主题请求合并，以及 cached artwork 不被主题提取串行阻塞。

### Required Concurrency Scenarios

以下是调度改动的验收场景；除上面点名的用例外，不表示每一项当前都有独立自动化测试：

- 连续 `play_song(A)` -> `play_song(B)` -> `play_song(C)`，只有 C 可进入 `Playing`。
- 连续 seek 只执行最后一次 commit。
- 切换 album 时旧 artwork/theme 请求不能覆盖新状态。
- 下载 worker 先等待 gate inactive（无 30 秒超时）；返回后若 `is_loading` 仍未收敛，则按 helper 进入时起 30 秒的总阈值记录警告并继续领取新 task。
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
5. `bunx vitest run src/lib/features/contract/ipc-contract.test.ts src/lib/api.test.ts`
6. `cargo check --manifest-path src-tauri/Cargo.toml`
7. `git diff --check`

跨前后端状态、事件或 registry 的改动再运行 `bun run check`。
