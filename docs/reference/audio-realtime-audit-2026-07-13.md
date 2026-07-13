# 音频实时路径静态审计 · 2026-07-13

对应优化路线图条目 **P1-6**（“以基准数据决定是否改造 CPAL 回调”）。本文是**静态审计交付物**，只做代码走查与基准指标规划；实际是否触发重构由后续硬件基准数据决定。

## 1. 概览

CPAL 输出回调是本项目唯一跑在系统音频实时线程上的代码：

- 关键闭包在 `src-tauri/src/player/backend/cpal.rs:369`（`build_stream` 内 `move |data: &mut [T], _| { ... }`）。
- 闭包体一步转发到 `write_output_data_with_metrics`（`cpal.rs:570-634`）。
- 该函数进一步调用 `SampleBuffer::try_pop_realtime_frames_into`（`src-tauri/src/player/stream.rs:496-502`），核心弹样本逻辑在 `pop_frames_into`（`stream.rs:504-559`）。
- 采样队列由 `crossbeam_queue::ArrayQueue<f32>`（`stream.rs:386`）承载，本身 lock-free。

任何在这条路径上出现的**锁等待、堆分配、日志、文件 IO、系统调用**都会挤占 CoreAudio / WASAPI / ALSA 分配给回调的 buffer 周期。回调超时的直接后果是底层驱动读到静音（underrun），用户听感为爆音或断音；连续 underrun 还会导致设备把流标记为不稳定并触发重建。

上层节奏参考：

- 假设 48 kHz、双通道、buffer 大小 512 帧 → 单个回调周期约 **10.6 ms**。
- 假设 48 kHz、buffer 128 帧 → 单个回调周期约 **2.7 ms**（macOS AirPods 一类的低延迟路径）。
- 96 kHz、buffer 256 帧（Hi-Res 输出路径）→ 单个回调周期约 **2.7 ms**。
- 回调运行时间需要显著低于周期，通常留 30–50% 余量。

关键辅助线程与其非实时性质：

- **`player-output-monitor`**（`cpal.rs:658-689`）：由 `spawn_stream_monitor` 拉起的普通 std 线程，每 20 ms 轮询一次 stop / underrun / metrics / finish；跑在普通调度优先级上，只做无锁读、原子交换与前端事件派发。不是实时线程，允许阻塞。
- **进度发射线程**（`controller.rs:428-442`）：每 100 ms 快照一次 `state` 并转发到 WebView，也不是实时线程。
- **CPAL 内部输出线程**：由 `cpal::traits::DeviceTrait::build_output_stream` 内部创建（本项目不持有句柄），驱动调度、无法直接观察线程 ID，本审计只谈其上运行的闭包。

## 2. 静态清单（回调闭包内的潜在阻塞点）

以下逐项对应“若在实时线程执行会有阻塞风险”的构造，按 `write_output_data_with_metrics` 展开走查的顺序列出。

| #   | 位置                                                                                           | 类型                                                                                        | 当前是否可能阻塞                                                                                                                                                                                                      | 建议                                                                    |
| --- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- |
| 1   | `cpal.rs:588` `stop_flag.load(SeqCst)`                                                         | 原子读                                                                                      | 否                                                                                                                                                                                                                    | 保留                                                                    |
| 2   | `cpal.rs:593-595` `if scratch.len() < data.len() { scratch.resize(data.len(), 0.0); }`         | 堆分配                                                                                      | 首帧或 buffer 尺寸增长时分配；稳定后为 no-op                                                                                                                                                                          | 已足够；如指标恶化，改为在 `build_stream` 预分配（见 §5）               |
| 3   | `cpal.rs:596-597` `output.fill(0.0)`                                                           | memset                                                                                      | 否                                                                                                                                                                                                                    | 保留                                                                    |
| 4   | `cpal.rs:599` `output_gain(volume)` → `volume.load(Relaxed)`                                   | 原子读                                                                                      | 否                                                                                                                                                                                                                    | 保留                                                                    |
| 5   | `cpal.rs:601` `samples.try_pop_realtime_frames_into(...)`                                      | 进入 SampleBuffer                                                                           | 见下方细项 5a–5d                                                                                                                                                                                                      | —                                                                       |
| 5a  | `stream.rs:510` `current_error()` → `stream.rs:609-615` `error.lock().unwrap()`                | `std::sync::Mutex::lock`                                                                    | **有条件阻塞**：`error_flag.load(Acquire)` 为真时才锁，即生产端已调 `fail()`。稳态下永远不进入这条分支。                                                                                                              | 保留（见 §5）；如需彻底摘除，改为 `ArcSwapOption<String>` 或 `OnceLock` |
| 5b  | `stream.rs:520/537/544` `queue.len()`/`queue.pop()`                                            | crossbeam ArrayQueue（lock-free MPMC）                                                      | 否                                                                                                                                                                                                                    | 保留                                                                    |
| 5c  | `stream.rs:551` `condvar.notify_all()`（成功弹样本时）                                         | 唤醒生产端                                                                                  | `std::sync::Condvar::notify_all` 只是通知内核 futex/信号量，不获取任何用户态锁；不 park 当前线程。开销为一次条件性 syscall。                                                                                          | 保留；若 P99 抖动严重，可改为原子水位 + `park_timeout`                  |
| 5d  | `stream.rs:519-548` 尾部弹清空、writable_samples 计算                                          | 纯算术                                                                                      | 否                                                                                                                                                                                                                    | 保留                                                                    |
| 6   | `cpal.rs:603/626-628` `callback_metrics.*` / `underrun_requested.store` / `finish_fired.store` | 原子写                                                                                      | 否                                                                                                                                                                                                                    | 保留                                                                    |
| 7   | `cpal.rs:606-611` 错误分支中的 `error_handler(error)`                                          | 用户回调；`Arc<dyn Fn(String)+Send+Sync>`                                                   | **一次性**：由 `buffer_error_reported.swap(true, SeqCst)` 门控，整个会话最多触发一次。其内部会 `String::clone` 并把日志投递到 runtime；触发时刻单次开销可能达毫秒级，但整个会话不会重复发生，此后该会话进入静音收敛。 | 保留；如要求“0 分配”，可改为原子 code + monitor 线程读值上报            |
| 8   | `cpal.rs:614` `smoother.smooth_audio(...)`                                                     | 纯算术，`last_frame: Vec<f32>` 在 `OutputSmoother::new` 分配、`ensure_channels` 才有 resize | 否（通道数不变时无 resize）                                                                                                                                                                                           | 保留                                                                    |
| 9   | `cpal.rs:616-619` 采样写回 + `dither.next()`                                                   | 纯算术                                                                                      | 否                                                                                                                                                                                                                    | 保留                                                                    |
| 10  | `cpal.rs:622` `frames_rendered.fetch_add(..., Relaxed)`                                        | 原子写                                                                                      | 否                                                                                                                                                                                                                    | 保留                                                                    |
| 11  | `cpal.rs:369-390` 闭包捕获的 `Arc` 引用（clone 均在 `build_stream` 前完成）                    | 无                                                                                          | 否                                                                                                                                                                                                                    | 保留                                                                    |

**用 grep 显式确认过的负面结论**（回调闭包内没有下列构造）：

- `grep -n "log::\|tracing::\|eprintln!\|println!\|LogPayload" src-tauri/src/player/backend/cpal.rs` → 0 命中。
- `grep -n "fs::\|File::\|read_to_string\|std::fs" src-tauri/src/player/backend/cpal.rs` → 0 命中。
- `Vec::push` 在回调路径上不存在；只有 `scratch.resize` 一处（第 2 项）与 `OutputSmoother::last_frame.resize`（仅在 `ensure_channels` 内、通道数变化时）。
- `RwLock` 在整个 `player/` 目录内 0 命中。

结论：回调路径实际存在的“非无锁”结构就是 §2 中 #2、#5a、#5c、#7 四项，其中 #5a 与 #7 都被显式的原子布尔门控，#5c 是内核 futex 通知、#2 稳态无分配。当前实现相当保守，属于“可用但值得实测确认”的状态。

## 3. `player/controller.rs` 的跨 await 锁扫描

`controller.rs` 中所有对 `state`/`backend`/`queue`/`stop_flag`/`pause_flag`/`media_session` 的锁都走 `std::sync::Mutex`（非 tokio 的 async Mutex）。

- 用 `grep -nE "async fn"` 扫描：文件中**没有直接的 `async fn`**，只有一处 `playback_runtime.spawn(async move { ... })`（`controller.rs:587`）和其中嵌套的 `playback_runtime.spawn(async move { ... })`（`controller.rs:682`）。
- 全文档只有一个 `.await`，出现在 `controller.rs:595` 的 `spawn_blocking(...).await`。

对 `controller.rs:587-661` 这段 rebuffer 编排逐行看 await 前后锁的生命周期：

1. `let wait_result = playback_runtime.handle().spawn_blocking(move || { ... }).await` — `wait_for_samples_or_end` 在阻塞线程执行；`.await` 前后没有任何 mutex guard。
2. `let should_resume = { let state = player.state.lock().unwrap(); ...; };`（`:628`）— guard 只活在裸大括号块内，在下一个 `.await` 之前已经 drop。
3. `if let Err(error) = player.backend.lock().unwrap().resume() { ... }`（`:636`）— 表达式作用域内 guard 立即 drop。
4. `{ let mut state = player.state.lock().unwrap(); ... }`（`:652`）— 同样在裸块内 drop。
5. 后续 `emit_state_and_sync(...)` 与 `player.rebuffering.store(...)` 均在锁作用域外。

**结论：`controller.rs` 内没有跨 await 持锁的情况。**

需要额外注意的次级风险（不属于跨 await，但审计时值得记录）：

- `backend.lock().unwrap().resume()`（`controller.rs:636`）在同步上下文中持锁调用底层 CPAL `Stream::play`。如果驱动内部的 play 阻塞（观察到 Windows 上偶发几十毫秒延迟），持锁时间会一起延长。这只影响命令响应延迟，不影响音频实时路径。
- 若后续把控制器改为 `tokio::sync::Mutex` 或引入直接 `async fn`，需要重新审计本节。

## 4. 建议的基准指标（供 benchmark 团队实施）

现有 `AudioCallbackMetrics { silence_due_to_lock, underrun_frames }` 由 `spawn_stream_monitor`（`cpal.rs:645-690`）每 1 秒聚合上报一次。以下是需要**新增**采集才能形成完整基线的指标。

| 指标                                                              | 采样方式                                                     | 目标位置                                   |
| ----------------------------------------------------------------- | ------------------------------------------------------------ | ------------------------------------------ |
| **buffer underrun 次数 / 小时**                                   | 已有 `underrun_frames`，需要按小时窗口累计并按输出采样率归一 | 新增 monitor 上报字段                      |
| **callback 运行时间分布**（P50/P95/P99/P999，单位 μs）            | 在闭包开头/结尾各取 `Instant::now()`，写入 lock-free 直方图  | `write_output_data_with_metrics` 入口/出口 |
| **回调超时次数**（`elapsed > buffer_period_ms`）                  | 直方图基础上加计数                                           | 同上                                       |
| **回调超时率**（超时次数 / 回调总数）                             | 派生                                                         | 同上                                       |
| **SampleBuffer 队列水位**（P5/P50/P95，单位 samples）             | 每次回调进入时读一次 `queue.len()`，蓄水位直方图             | `stream.rs` 或 `cpal.rs`                   |
| **silence_due_to_lock 事件/小时**                                 | 已存在，扩展到小时窗口                                       | monitor                                    |
| **buffer_error_reported 触发次数**（本应等于 0 或 1，跨会话累计） | 累加计数器                                                   | controller                                 |
| **设备重建次数**（同设备内的 stream rebuild）                     | 观察 `build_stream` 调用点频次                               | 后端启动埋点                               |
| **端到端 fail_session 触发次数**（分类：设备错、解码错、缓冲错）  | 已在 log_center                                              | 现有日志聚合                               |

采样格式分桶：**f32 / f64 / i32 / i24 / i16 / u32 / u24 / u16 / i8 / u8** 分别单独跑一遍代表曲目（30 分钟起播时长）。真实设备优先覆盖：

- macOS 内置扬声器（默认 f32/48000）
- macOS AirPods（低 buffer f32）
- Windows WASAPI 独占（i16 或 i32/48000）
- Windows WASAPI 共享（f32 mixer 输出）
- Linux ALSA 直连（i16/44100 与 f32/48000）

**采集实现建议**：

- 直方图使用 `hdrhistogram` 或自研 log-scale bucket（如 32 桶覆盖 1 μs–100 ms）；数据结构必须是单写单读或原子桶，避免在回调里加锁。
- 采集点写入的原子操作用 `Ordering::Relaxed`；只有跨会话切换或 monitor 上报点才用 `Release/Acquire`。
- 每次 monitor 采样上报后 `swap` 归零，与现有 `AudioCallbackMetrics::drain`（`cpal.rs:188-193`）保持同一模式。
- 直方图内存必须在 `build_stream` 之前预分配，禁止在回调内出现 `Vec::push` 或 `Box::new`。

**归一化口径**：所有“每小时”指标都以 `frames_rendered` 而不是 wall clock 归一，避免暂停 / rebuffer 期把静音时间算进来。

## 5. 不改的理由

对 §2 中被标记“可能阻塞但当前保留”的四处，简述保留原因：

- **#2 `scratch.resize`**：CPAL 单次流内 `data.len()` 通常保持稳定（同 buffer size 同格式），首帧后基本进入 no-op 分支；改为提前预分配收益极小且需要多引一层 handle 尺寸变更的分支，暂不重构。
- **#5a `error.lock()`**：`error_flag` 是 `AtomicBool::Acquire`，在 `fail()` 未被调用时永远为 false，锁根本不会被访问。真进入这条路径时会话已经处于故障态、正在走终结路径，短暂争用即使阻塞也不影响连续回放（此后所有回调都落入静音分支直到 stop）。移除需要引入 `arc-swap`/`OnceLock` 或改用 `Atomic<*mut String>`，对当前故障率不划算。
- **#5c `condvar.notify_all()`**：这是让生产端从 push 阶段的 `wait_timeout` 中及时醒来的关键唤醒点；如果去掉，生产端最多要等一个 50 ms 的 timeout 才能续接下一批解码，反而拉低填充速度。std::sync::Condvar 的 notify 不 park 当前线程，实测代价一般在微秒级。
- **#7 `error_handler(...)`**：单会话至多触发一次；触发时会话已经处于错误状态，性能损失被 fail 路径吞掉。改造为纯原子上报 + monitor 读值需要重排 error 语义（当前 error 附带字符串消息），性价比低。

## 6. 改造门槛

由 benchmark 数据触发的决策矩阵（阈值仅作起点，实测后按平台校准）：

| 观测数据（30 分钟连续播放，同设备与采样格式）                                                                  | 决策                                                                                                                                                           |
| -------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `underrun_frames_per_hour == 0` 且 `silence_due_to_lock_per_hour == 0` 且 `callback_p99 < 0.4 × buffer_period` | **不改**。当前实现足够。                                                                                                                                       |
| `underrun_frames_per_hour ≤ 10`（相当于每小时丢 <0.2 ms）                                                      | **不改**，但记录基线，回归时对比。                                                                                                                             |
| `underrun_frames_per_hour ∈ (10, 100]` 或 `callback_p99 ∈ [0.4, 0.7] × buffer_period`                          | **软改造**：把 `scratch` 预分配、`current_error` 的 mutex 改为 `arc-swap`，其余不动。                                                                          |
| `underrun_frames_per_hour > 100` 或 `callback_p999 > buffer_period` 或 `silence_due_to_lock_per_hour > 5`      | **硬改造**：`SampleBuffer` 换为专用 SPSC ring buffer（例如 `rtrb`），生产端改用原子水位 + `Thread::park_timeout` 替代 Condvar，`error_handler` 改为原子 code。 |
| 特定平台/格式（例如 Windows WASAPI 共享 f32）单独超阈值                                                        | **局部改造**：只在受影响后端切换实现，其余保留。                                                                                                               |
| 出现设备重建 > 2 次/小时                                                                                       | 与 P1-6 无关，交给设备热插拔/失败恢复专项处理。                                                                                                                |

**软 / 硬改造成本估计**（供 P1-6 决策阶段权衡）：

- **软改造**：`scratch` 预分配 + `current_error` 换 `arc_swap::ArcSwapOption<String>`。代码改动 <50 行；不影响外部 API；测试覆盖延用 `cpal.rs` 现有 15 项单元测试即可，只需要新增一个“error path 无 mutex”回归。工时约 0.5–1 人日。
- **硬改造**：`SampleBuffer` 用 SPSC 实现替换（推荐评估 `rtrb` / `ringbuf::HeapRb`）。会牵动 `stream.rs:417-616` 的 `push` / `fail` / `finish` / `wait_for_samples` 全套语义，以及 `stream.rs` 现存的 30+ 个契约测试。同时 push 端从 Condvar 转为 `Thread::park_timeout` 后需要重构解码线程的中断路径。工时估 3–5 人日；风险点在多平台一致性验证。
- **局部改造**：仅在受影响后端切换 SampleBuffer 类型（例如 Windows 单独用 SPSC），通过 feature flag 隔离；工时 2–3 人日，长期维护成本高，最好只作过渡形态。

benchmark 数据落库前，本文视为待验证假设；若首轮实测证明当前实现全绿，本文档转为“P1-6 免改造依据”存档，后续每次动到 §2 中任一项时需要重新跑一次基准并追加数据到本文末。

## 7. 附录：与主流 SPSC ring buffer 的对比要点

只作为硬改造启动时的先验参考，不代表当前需要迁移。

| 维度         | 当前 `crossbeam_queue::ArrayQueue<f32>` | 典型 SPSC (`rtrb` / `ringbuf::HeapRb`)                      |
| ------------ | --------------------------------------- | ----------------------------------------------------------- |
| 并发模型     | MPMC                                    | SPSC（单生产者单消费者）                                    |
| pop 单次开销 | 一次 CAS + 一次原子递增                 | 无 CAS，纯原子读写                                          |
| 批量弹样本   | 需要循环 `pop()` 单个                   | 直接 `read_chunk` 拿连续切片                                |
| Wait 唤醒    | 用户自持 `Condvar`                      | 常见做法是自己配 `parking_lot::Condvar` 或 `event_listener` |
| 峰值抖动     | 未测；理论上 MPMC 通用实现比 SPSC 略高  | 通常更低                                                    |
| 迁移代价     | 现状                                    | 需要重排 push/fail/finish 与生产端 wait 语义                |

判定标准：只有当基准数据显示 `queue.pop()` 单次开销 P99 > 500 ns，或队列 push/pop 冲突主导 callback 抖动时，才启动此项迁移；否则维持 crossbeam 版即可。
