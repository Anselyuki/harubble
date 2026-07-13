# IPC 契约现状清单（2026-07-13）

> **用途**：P0-3 契约快照，反映本轮架构优化后的实际状态。
> **单一事实源**：`src-tauri/src/command_registry.rs` 的 `for_each_tauri_command!` 宏。
> **对比**：与 `ipc-inventory-2026-07-10.md` 的主要差异见底部"关键改进"章节。

---

## 一、Command 汇总

共 **68 个 Tauri command**，全部通过 `for_each_tauri_command!` 宏统一注册于 `main.rs`。

### 调度域分布

| 域                                   | 命令数 | 代表命令                                                        |
| ------------------------------------ | ------ | --------------------------------------------------------------- |
| InteractiveUi                        | 43     | list_collections, get_albums, get_preferences …                 |
| PlaybackControl / PlaybackTransition | 8      | play_song, pause_playback, seek_current_playback …              |
| BackgroundIo                         | 9      | rescan_local_inventory, create_download_job, schedule_rebuild … |
| VisualAux                            | 3      | get_song_lyrics, extract_image_theme, get_image_data_url        |
| PlaybackSideEffect / Maintenance     | 5      | record_song_heat, clear_listening_history, clear_audio_cache …  |

### 错误类型覆盖（P0-4 全域完成）

全部 13 个命令域均已从 `Result<T, String>` 迁移为结构化域错误枚举：

| 域                             | 错误类型              | 序列化格式                                                                                  |
| ------------------------------ | --------------------- | ------------------------------------------------------------------------------------------- |
| collection                     | `CollectionError`     | `{code:"notFound"\|"readOnly"\|"database"\|"serialization"\|"unsupportedVersion", detail?}` |
| library                        | `LibraryError`        | `{code:"network"\|"notFound"\|"internal", detail?}`                                         |
| search                         | `SearchError`         | `{code:"notReady"\|"internal", detail?}`                                                    |
| downloads                      | `DownloadError`       | `{code:"notFound"\|"network"\|"io"\|"invalidState"\|"internal", detail?}`                   |
| tag_editor                     | `TagEditorError`      | `{code:"io"\|"serialization"\|"unsupportedVersion"\|"internal", detail?}`                   |
| playback                       | `PlaybackError`       | `{code:"superseded"\|"noActiveTrack"\|…\|"audio"\|"io"\|"internal", detail?}`               |
| preferences                    | `PreferencesError`    | `{code:"notFound"\|"io"\|"internal", detail?}`                                              |
| logging                        | `LoggingError`        | `{code:"io"\|"internal", detail?}`                                                          |
| local_inventory                | `LocalInventoryError` | `{code:"io"\|"internal", detail?}`                                                          |
| homepage                       | `HomepageError`       | `{code:"network"\|"internal", detail?}`                                                     |
| tag_registry                   | `TagRegistryError`    | `{code:"network"\|"internal", detail?}`                                                     |
| window                         | `WindowError`         | `{code:"internal", detail?}`                                                                |
| collection（同上，已是试点域） | —                     | —                                                                                           |

所有枚举使用 `#[serde(tag = "code", content = "detail", rename_all = "camelCase")]`，
前端 `src/lib/types.ts` 有对应联合类型，`domainErrors.ts` 提供按域格式化函数。

---

## 二、事件 Map（AppEventMap，无变化）

共 **10 个事件**，定义于 `src/lib/appEvents.ts`：

| 事件名                           | 载荷类型                          |
| -------------------------------- | --------------------------------- |
| `player-state-changed`           | `PlayerState`                     |
| `player-progress`                | `PlayerState`                     |
| `player-ended`                   | `PlaybackEndedEvent`              |
| `download-manager-state-changed` | `DownloadManagerSnapshot`         |
| `download-job-updated`           | `DownloadJobSnapshot`             |
| `download-task-progress`         | `DownloadTaskProgressEvent`       |
| `app-error-recorded`             | `AppErrorEvent`                   |
| `local-inventory-state-changed`  | `LocalInventorySnapshot`          |
| `local-inventory-scan-progress`  | `LocalInventoryScanProgressEvent` |
| `homepage-belong-ready`          | `void`                            |

---

## 三、前端 wrapper 覆盖

| 文件                       | 覆盖域                            | 模式                              |
| -------------------------- | --------------------------------- | --------------------------------- |
| `src/lib/api.ts`           | 除 collection / settings 外全部域 | `invoke<T>` + `invokePlayback<T>` |
| `src/lib/collectionApi.ts` | collection（10 个）               | `invoke<T>`                       |
| `src/lib/settingsApi.ts`   | preferences / logging             | `invoke<T>`                       |

---

## 四、COMMAND_SPECS 与调度元数据

调度元数据（`CommandDomain` / `CommandPriority` / `CancelPolicy`）集中维护于
`src-tauri/src/command_scheduling.rs`，由 `COMMAND_SPECS` 常量保存。

`command_registry_covers_all_tauri_commands` 测试确保
`COMMAND_SPECS` ↔ `ALL_TAURI_COMMAND_NAMES`（宏生成）无漂移。

---

## 五、关键改进（对比 2026-07-10 快照）

| 项目               | 2026-07-10                                                 | 2026-07-13                                                 |
| ------------------ | ---------------------------------------------------------- | ---------------------------------------------------------- |
| 错误协议           | 所有域返回 `Result<T, String>`                             | 全部13域结构化错误枚举，code 稳定                          |
| 单一事实源         | 3份手工清单（generate_handler / COMMAND_SPECS / 测试常量） | `for_each_tauri_command!` 宏统一生成                       |
| `record_song_heat` | 在 `generate_handler![]` 注册但在 COMMAND_SPECS 缺失       | 已修复漂移，全部对齐                                       |
| `collectionApi.ts` | 10 个 `invoke` 无显式泛型                                  | 全部补齐 `invoke<T>`                                       |
| 事件 map           | 缺少 `local-inventory-scan-progress`                       | 已补加                                                     |
| 前端错误格式化     | 直接拼接 `error.message`（暴露 Rust 内部字符串）           | domainErrors.ts 按 code 匹配本地化文案，兜底 generic_error |
| IPC 契约测试       | 仅覆盖 Collection + Playback 2 个域                        | ipc-contract.test.ts 覆盖全部 13 个域（244 测试）          |

---

> **生成时间**：2026-07-13  
> **下次更新触发条件**：新增命令域、更改错误枚举变体、新增事件类型。
