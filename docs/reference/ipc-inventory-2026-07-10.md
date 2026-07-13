# IPC 契约现状清单（2026-07-10）

> **用途**：P0-3 契约试点基线快照。列出 Tauri command、事件、`COMMAND_SPECS` 调度元数据、前端 wrapper 覆盖情况与已观察到的漂移。仅记录事实，不给改造建议。
> **来源**：`docs/plans/architecture-optimization-2026-07-10.md` 首批落地清单第 4 项。
> **快照生成时间**：2026-07-10。行号均对齐当次快照，若源码演进请重新生成。

## 一、Command 完整清单

`src-tauri/src/main.rs:270-338` 的 `invoke_handler` 共注册 **68 个 command**。列头「Spec」列出 `CommandDomain` / `CommandPriority` / `CancelPolicy`；列头「Wrapper」中 `i` = `invoke`、`i<T>` = 带显式泛型 `invoke<T>`、`iP` = `invokePlayback`、「无」= 无前端 wrapper。

### 1.1 collection（10）

| Command                        | Rust 源                      | 签名要点                                                                                                      | Spec                                                                     | Wrapper                                       |
| ------------------------------ | ---------------------------- | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | --------------------------------------------- |
| `list_collections`             | `commands/collection.rs:24`  | sync, `-> Result<Vec<CollectionSummary>, String>`                                                             | InteractiveUi / Interactive / Cooperative（`command_scheduling.rs:166`） | `collectionApi.ts:5` `i<CollectionSummary[]>` |
| `get_collection`               | `commands/collection.rs:35`  | sync, `id:String -> Result<Collection,String>`                                                                | InteractiveUi / Interactive / Cooperative（`:172`）                      | `collectionApi.ts:9` `i<Collection>`          |
| `create_collection`            | `commands/collection.rs:45`  | sync, `name,description:String, cover_path:Option<String> -> Result<Collection,String>`                       | InteractiveUi / Interactive / Cooperative（`:178`）                      | `collectionApi.ts:17` `i<Collection>`         |
| `update_collection`            | `commands/collection.rs:63`  | sync, `id, name?:String, description?:String, cover_path:Option<Option<String>> -> Result<Collection,String>` | InteractiveUi / Interactive / Cooperative（`:184`）                      | `collectionApi.ts:30` `i<Collection>`         |
| `delete_collection`            | `commands/collection.rs:87`  | sync, `id -> Result<(),String>`                                                                               | InteractiveUi / Interactive / Cooperative（`:190`）                      | `collectionApi.ts:39` `i<void>`               |
| `add_songs_to_collection`      | `commands/collection.rs:96`  | sync, `id, song_ids:Vec<String> -> Result<(),String>`                                                         | InteractiveUi / Interactive / Cooperative（`:196`）                      | `collectionApi.ts:46` `i<void>`               |
| `remove_songs_from_collection` | `commands/collection.rs:109` | sync, `id, song_ids:Vec<String> -> Result<(),String>`                                                         | InteractiveUi / Interactive / Cooperative（`:202`）                      | `collectionApi.ts:53` `i<void>`               |
| `reorder_collection_songs`     | `commands/collection.rs:122` | sync, `id, song_ids:Vec<String> -> Result<(),String>`                                                         | InteractiveUi / Interactive / Cooperative（`:208`）                      | `collectionApi.ts:60` `i<void>`               |
| `export_collection`            | `commands/collection.rs:135` | sync, `id -> Result<String,String>`                                                                           | InteractiveUi / Interactive / Cooperative（`:214`）                      | `collectionApi.ts:64` `i<string>`             |
| `import_collection`            | `commands/collection.rs:146` | sync, `json:String -> Result<Collection,String>`                                                              | InteractiveUi / Interactive / Cooperative（`:220`）                      | `collectionApi.ts:68` `i<Collection>`         |

### 1.2 library（7）

| Command                  | Rust 源                   | 签名要点                                                                      | Spec                                                                     | Wrapper                          |
| ------------------------ | ------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------------------------------------ | -------------------------------- |
| `get_albums`             | `commands/library.rs:17`  | async, `-> Result<Vec<Album>,String>`                                         | InteractiveUi / Interactive / Cooperative（`command_scheduling.rs:118`） | `api.ts:116` `i`                 |
| `get_album_detail`       | `commands/library.rs:41`  | async, `album_cid -> Result<AlbumDetail,String>`                              | InteractiveUi / Interactive / Cooperative（`:124`）                      | `api.ts:130` `i<AlbumDetail>`    |
| `get_song_detail`        | `commands/library.rs:78`  | async, `cid -> Result<SongDetail,String>`                                     | InteractiveUi / Interactive / Cooperative（`:130`）                      | `api.ts:149` `i<SongDetail>`     |
| `get_song_lyrics`        | `commands/library.rs:111` | async, `cid -> Result<Option<String>,String>`（走 `dispatch_visual_aux`）     | VisualAux / Visual / LatestWins（`:382`）                                | `api.ts:166` `i<string \| null>` |
| `extract_image_theme`    | `commands/library.rs:145` | async, `image_url -> Result<ThemePalette,String>`（走 `dispatch_visual_aux`） | VisualAux / Visual / LatestWins（`:388`）                                | `api.ts:272` `i<ThemePalette>`   |
| `get_image_data_url`     | `commands/library.rs:181` | async, `image_url -> Result<String,String>`（走 `dispatch_visual_aux`）       | VisualAux / Visual / LatestWins（`:394`）                                | `api.ts:309` `i<string>`         |
| `get_default_output_dir` | `commands/library.rs:208` | sync, `-> String`（**无 `Result`**）                                          | InteractiveUi / Interactive / NeverCancel（`:352`）                      | `api.ts:227` `i`                 |

### 1.3 search（1）

| Command          | Rust 源                 | 签名要点                                                                      | Spec                                                | Wrapper                                 |
| ---------------- | ----------------------- | ----------------------------------------------------------------------------- | --------------------------------------------------- | --------------------------------------- |
| `search_library` | `commands/search.rs:16` | async, `request:SearchLibraryRequest -> Result<SearchLibraryResponse,String>` | InteractiveUi / Interactive / Cooperative（`:136`） | `api.ts:177` `i<SearchLibraryResponse>` |

### 1.4 playback（8）

| Command                 | Rust 源                    | 签名要点                                                                                                                                | Spec                                                              | Wrapper                 |
| ----------------------- | -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- | ----------------------- |
| `play_song`             | `commands/playback.rs:16`  | async, `song_cid, cover_url:Opt, playback_context:Opt<PlaybackContext> -> Result<PlaybackStartResult, PlaybackError>`（**结构化错误**） | PlaybackTransition / Playback / SupersedePlaybackSession（`:70`） | `api.ts:185` `iP`       |
| `pause_playback`        | `commands/playback.rs:37`  | sync, `-> Result<(),String>`                                                                                                            | PlaybackControl / Playback / NeverCancel（`:94`）                 | `api.ts:193` `iP<void>` |
| `resume_playback`       | `commands/playback.rs:47`  | sync, `-> Result<(),String>`                                                                                                            | PlaybackControl / Playback / NeverCancel（`:100`）                | `api.ts:197` `iP<void>` |
| `seek_current_playback` | `commands/playback.rs:57`  | async, `position_secs:f64 -> Result<PlaybackStartResult, PlaybackError>`（**结构化错误**）                                              | PlaybackTransition / Playback / LatestWins（`:76`）               | `api.ts:203` `iP`       |
| `play_next`             | `commands/playback.rs:79`  | async, `-> Result<PlaybackStartResult, PlaybackError>`                                                                                  | PlaybackTransition / Playback / SupersedePlaybackSession（`:82`） | `api.ts:207` `iP`       |
| `play_previous`         | `commands/playback.rs:93`  | async, `-> Result<PlaybackStartResult, PlaybackError>`                                                                                  | PlaybackTransition / Playback / SupersedePlaybackSession（`:88`） | `api.ts:211` `iP`       |
| `get_player_state`      | `commands/playback.rs:109` | sync, `-> Result<PlayerState, String>`                                                                                                  | PlaybackControl / Playback / NeverCancel（`:106`）                | `api.ts:215` `i`        |
| `set_playback_volume`   | `commands/playback.rs:120` | async, `volume:f64 -> Result<f64,String>`                                                                                               | PlaybackControl / Playback / NeverCancel（`:112`）                | `api.ts:219` `i`        |

### 1.5 window（1）

| Command            | Rust 源                | 签名要点                                   | Spec                                                | Wrapper          |
| ------------------ | ---------------------- | ------------------------------------------ | --------------------------------------------------- | ---------------- |
| `show_main_window` | `commands/window.rs:7` | sync, `app:AppHandle -> Result<(),String>` | InteractiveUi / Interactive / NeverCancel（`:334`） | `api.ts:223` `i` |

### 1.6 preferences（6）

| Command                             | Rust 源                       | 签名要点                                                                            | Spec                                                | Wrapper                                        |
| ----------------------------------- | ----------------------------- | ----------------------------------------------------------------------------------- | --------------------------------------------------- | ---------------------------------------------- |
| `get_preferences`                   | `commands/preferences.rs:19`  | async, `-> Result<AppPreferences,String>`                                           | InteractiveUi / Interactive / Cooperative（`:298`） | `api.ts:384` `i<AppPreferences>`               |
| `set_preferences`                   | `commands/preferences.rs:36`  | async, `app:AppHandle, preferences:AppPreferences -> Result<AppPreferences,String>` | InteractiveUi / Interactive / Cooperative（`:304`） | `api.ts:390` `i<AppPreferences>`               |
| `export_preferences`                | `commands/preferences.rs:70`  | async, `output_path:String -> Result<AppPreferences,String>`                        | InteractiveUi / Interactive / Cooperative（`:310`） | **无前端 wrapper**                             |
| `import_preferences`                | `commands/preferences.rs:93`  | async, `app:AppHandle, input_path:String -> Result<AppPreferences,String>`          | InteractiveUi / Interactive / Cooperative（`:316`） | **无前端 wrapper**                             |
| `get_notification_permission_state` | `commands/preferences.rs:127` | sync, `-> Result<String,String>`                                                    | InteractiveUi / Interactive / Cooperative（`:322`） | **无前端 wrapper**                             |
| `send_test_notification`            | `commands/preferences.rs:148` | sync, `-> Result<(),String>`                                                        | InteractiveUi / Interactive / Cooperative（`:328`） | `api.ts:366` `i`，`settingsApi.ts:4` re-export |

### 1.7 local_inventory（4）

| Command                        | Rust 源                          | 签名要点                                                                                          | Spec                                                   | Wrapper                                     |
| ------------------------------ | -------------------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------ | ------------------------------------------- |
| `get_local_inventory_snapshot` | `commands/local_inventory.rs:21` | async, `-> Result<LocalInventorySnapshot,String>`                                                 | InteractiveUi / Interactive / Cooperative（`:358`）    | `api.ts:370` `i<LocalInventorySnapshot>`    |
| `rescan_local_inventory`       | `commands/local_inventory.rs:33` | async, `app, verification_mode:Option<VerificationMode> -> Result<LocalInventorySnapshot,String>` | BackgroundIo / Background / Cooperative（`:364`）      | **无前端 wrapper**                          |
| `cancel_local_inventory_scan`  | `commands/local_inventory.rs:54` | async, `app -> Result<LocalInventorySnapshot,String>`                                             | BackgroundIo / **Interactive** / NeverCancel（`:370`） | **无前端 wrapper**                          |
| `get_audio_metadata`           | `commands/local_inventory.rs:89` | async, `album_name, song_name -> Result<Option<AudioFileMetadata>,String>`                        | BackgroundIo / Background / Cooperative（`:376`）      | `api.ts:377` `i<AudioFileMetadata \| null>` |

### 1.8 logging（2）

| Command               | Rust 源                  | 签名要点                                                     | Spec                                                | Wrapper                                                                  |
| --------------------- | ------------------------ | ------------------------------------------------------------ | --------------------------------------------------- | ------------------------------------------------------------------------ |
| `list_log_records`    | `commands/logging.rs:16` | sync, `query:LogViewerQuery -> Result<LogViewerPage,String>` | InteractiveUi / Interactive / Cooperative（`:340`） | `api.ts:396` `i<LogViewerPage>` + `settingsApi.ts:13` `i<LogViewerPage>` |
| `get_log_file_status` | `commands/logging.rs:29` | sync, `-> Result<LogFileStatus,String>`                      | InteractiveUi / Interactive / Cooperative（`:346`） | `api.ts:400` `i<LogFileStatus>` + `settingsApi.ts:17` `i<LogFileStatus>` |

### 1.9 downloads（11）

| Command                  | Rust 源                     | 签名要点                                                                             | Spec                                                     | Wrapper                                           |
| ------------------------ | --------------------------- | ------------------------------------------------------------------------------------ | -------------------------------------------------------- | ------------------------------------------------- |
| `clear_audio_cache`      | `commands/downloads.rs:28`  | sync, `-> Result<u64,String>`                                                        | Maintenance / CriticalSideEffect / Cooperative（`:448`） | `api.ts:240` `i` + `settingsApi.ts:7` `i<number>` |
| `clear_response_cache`   | `commands/downloads.rs:40`  | sync, `-> Result<(),String>`                                                         | Maintenance / CriticalSideEffect / Cooperative（`:454`） | `api.ts:244` `i`                                  |
| `reset_http_client`      | `commands/downloads.rs:52`  | sync, `-> Result<(),String>`                                                         | Maintenance / CriticalSideEffect / Cooperative（`:460`） | `api.ts:248` `i`                                  |
| `create_download_job`    | `commands/downloads.rs:62`  | async, `app, request:CreateDownloadJobRequest -> Result<DownloadJobSnapshot,String>` | BackgroundIo / Background / Cooperative（`:400`）        | `api.ts:328` `i`                                  |
| `list_download_jobs`     | `commands/downloads.rs:106` | async, `-> Result<DownloadManagerSnapshot,String>`                                   | InteractiveUi / Interactive / Cooperative（`:406`）      | `api.ts:332` `i`                                  |
| `get_download_job`       | `commands/downloads.rs:119` | async, `job_id -> Result<Option<DownloadJobSnapshot>,String>`                        | InteractiveUi / Interactive / Cooperative（`:412`）      | **无前端 wrapper**                                |
| `cancel_download_job`    | `commands/downloads.rs:133` | async, `app, job_id -> Result<Option<DownloadJobSnapshot>,String>`                   | BackgroundIo / **Interactive** / NeverCancel（`:418`）   | `api.ts:338` `i`                                  |
| `cancel_download_task`   | `commands/downloads.rs:162` | async, `app, job_id, task_id -> Result<Option<DownloadJobSnapshot>,String>`          | BackgroundIo / **Interactive** / NeverCancel（`:424`）   | `api.ts:345` `i`                                  |
| `retry_download_job`     | `commands/downloads.rs:192` | async, `app, job_id -> Result<Option<DownloadJobSnapshot>,String>`                   | BackgroundIo / Background / Cooperative（`:430`）        | `api.ts:351` `i`                                  |
| `retry_download_task`    | `commands/downloads.rs:221` | async, `app, job_id, task_id -> Result<Option<DownloadJobSnapshot>,String>`          | BackgroundIo / Background / Cooperative（`:436`）        | `api.ts:358` `i`                                  |
| `clear_download_history` | `commands/downloads.rs:251` | async, `app -> Result<usize,String>`                                                 | BackgroundIo / Background / Cooperative（`:442`）        | `api.ts:362` `i`                                  |

### 1.10 homepage（6）

| Command                   | Rust 源                    | 签名要点                                                             | Spec                                                            | Wrapper                          |
| ------------------------- | -------------------------- | -------------------------------------------------------------------- | --------------------------------------------------------------- | -------------------------------- |
| `get_latest_albums`       | `commands/homepage.rs:15`  | async, `limit:u32 -> Result<Vec<Album>,String>`                      | InteractiveUi / Interactive / Cooperative（`:142`）             | `api.ts:404` `i<Album[]>`        |
| `get_albums_by_series`    | `commands/homepage.rs:42`  | async, `-> Result<Vec<SeriesGroup>,String>`                          | InteractiveUi / Interactive / Cooperative（`:148`）             | `api.ts:408` `i<SeriesGroup[]>`  |
| `get_recent_history`      | `commands/homepage.rs:103` | async, `limit:u32 -> Result<Vec<HistoryEntry>,String>`               | InteractiveUi / Interactive / Cooperative（`:154`）             | `api.ts:419` `i<HistoryEntry[]>` |
| `record_song_heat`        | `commands/homepage.rs:120` | async, `song_cid, cover_url:Opt -> Result<(),String>`                | **未注册**                                                      | `api.ts:415` `i`                 |
| `clear_listening_history` | `commands/homepage.rs:151` | async, `-> Result<u32,String>`（走 `dispatch_playback_side_effect`） | PlaybackSideEffect / CriticalSideEffect / Cooperative（`:466`） | `api.ts:423` `i<number>`         |
| `get_homepage_status`     | `commands/homepage.rs:168` | async, `-> Result<HomepageStatus,String>`                            | InteractiveUi / Interactive / Cooperative（`:160`）             | `api.ts:427` `i<HomepageStatus>` |

### 1.11 tag_registry（2）

| Command                       | Rust 源                       | 签名要点                                               | Spec                                                | Wrapper                          |
| ----------------------------- | ----------------------------- | ------------------------------------------------------ | --------------------------------------------------- | -------------------------------- |
| `get_tag_dimensions`          | `commands/tag_registry.rs:13` | sync, `-> Result<Vec<TagDimensionResolved>,String>`    | InteractiveUi / Interactive / Cooperative（`:226`） | `api.ts:431` `i<TagDimension[]>` |
| `get_albums_by_tag_dimension` | `commands/tag_registry.rs:25` | async, `dimension_key -> Result<Vec<TagGroup>,String>` | InteractiveUi / Interactive / Cooperative（`:232`） | `api.ts:437` `i<TagGroup[]>`     |

### 1.12 tag_editor（10）

| Command                          | Rust 源                      | 签名要点                                                                                  | Spec                                                | Wrapper                                |
| -------------------------------- | ---------------------------- | ----------------------------------------------------------------------------------------- | --------------------------------------------------- | -------------------------------------- |
| `get_tag_editor_merged`          | `commands/tag_editor.rs:14`  | sync, `-> Result<TagRegistry,String>`                                                     | InteractiveUi / Interactive / Cooperative（`:238`） | `api.ts:443` `i<TagEditorRegistry>`    |
| `get_tag_editor_local_overlay`   | `commands/tag_editor.rs:20`  | sync, `-> Result<TagRegistry,String>`                                                     | InteractiveUi / Interactive / Cooperative（`:244`） | `api.ts:447` `i<TagEditorRegistry>`    |
| `set_tag_editor_entity_tag`      | `commands/tag_editor.rs:26`  | async, `entity_type, cid, dimension_key, values:Vec<LocalizedValue> -> Result<(),String>` | InteractiveUi / Interactive / Cooperative（`:250`） | `api.ts:456` `i`                       |
| `remove_tag_editor_entity_tag`   | `commands/tag_editor.rs:44`  | async, `entity_type, cid, dimension_key -> Result<(),String>`                             | InteractiveUi / Interactive / Cooperative（`:256`） | `api.ts:469` `i`                       |
| `add_tag_editor_dimension`       | `commands/tag_editor.rs:59`  | async, `key, label_zh, label_en -> Result<(),String>`                                     | InteractiveUi / Interactive / Cooperative（`:262`） | `api.ts:481` `i`                       |
| `remove_tag_editor_dimension`    | `commands/tag_editor.rs:74`  | async, `key -> Result<(),String>`                                                         | InteractiveUi / Interactive / Cooperative（`:268`） | `api.ts:485` `i`                       |
| `apply_tag_editor_remote_update` | `commands/tag_editor.rs:87`  | async, `new_remote:TagRegistry -> Result<MergeResult,String>`                             | InteractiveUi / Interactive / Cooperative（`:274`） | `api.ts:491` `i<TagEditorMergeResult>` |
| `resolve_tag_editor_conflict`    | `commands/tag_editor.rs:100` | async, `entity_type, cid, dimension_key, keep:ConflictResolution -> Result<(),String>`    | InteractiveUi / Interactive / Cooperative（`:280`） | `api.ts:502` `i`                       |
| `export_tag_editor_registry`     | `commands/tag_editor.rs:120` | async, `path -> Result<(),String>`                                                        | InteractiveUi / Interactive / Cooperative（`:286`） | `api.ts:511` `i`                       |
| `import_tag_editor_registry`     | `commands/tag_editor.rs:136` | async, `path -> Result<MergeResult,String>`                                               | InteractiveUi / Interactive / Cooperative（`:292`） | `api.ts:517` `i<TagEditorMergeResult>` |

## 二、Event 完整清单

| 事件名                           | 常量定义                                                | 载荷类型                                                                           | 发射位置                                                                       | AppEventMap 注册          | 前端订阅                                                       |
| -------------------------------- | ------------------------------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ------------------------- | -------------------------------------------------------------- |
| `player-state-changed`           | `player/events.rs:14`                                   | `PlayerState` (`player/state.rs`)                                                  | `player/events.rs:37` `emit_state`                                             | `src/lib/appEvents.ts:18` | `appRuntimeBootstrap.svelte.ts:264` + `miniPlayerBridge.ts:35` |
| `player-progress`                | `player/events.rs:19`                                   | `PlayerState`                                                                      | `player/events.rs:42` `emit_progress` / `:49` `emit_progress_snapshot`         | `appEvents.ts:19`         | `appRuntimeBootstrap.svelte.ts:273` + `miniPlayerBridge.ts:43` |
| `player-ended`                   | `player/events.rs:23`                                   | `PlaybackEndedEvent` (`player/events.rs:28`)                                       | `player/events.rs:54` `emit_ended`                                             | `appEvents.ts:20`         | `appRuntimeBootstrap.svelte.ts:280`                            |
| `download-manager-state-changed` | `downloads/events.rs:13`                                | `DownloadManagerSnapshot` (`harubble-core/src/download/model.rs`)                  | `downloads/events.rs:27` `emit_download_manager_state_changed`（多处调用）     | `appEvents.ts:21`         | `appRuntimeBootstrap.svelte.ts:287`                            |
| `download-job-updated`           | `downloads/events.rs:17`                                | `DownloadJobSnapshot` (`harubble-core/src/download/model.rs`)                      | `downloads/events.rs:31` `emit_download_job_updated`                           | `appEvents.ts:22`         | `appRuntimeBootstrap.svelte.ts:295`                            |
| `download-task-progress`         | `downloads/events.rs:21`                                | `DownloadTaskProgressEvent` (`harubble-core/src/download/model.rs:551`)            | `downloads/bridge.rs:331` / `:445` `app.emit(DOWNLOAD_TASK_PROGRESS, ...)`     | `appEvents.ts:23`         | `appRuntimeBootstrap.svelte.ts:303`                            |
| `app-error-recorded`             | `logging.rs:14`（`APP_ERROR_RECORDED`，**私有 const**） | `AppErrorEvent` (`logging.rs:95`)                                                  | `logging.rs:320`（`LogCenter::record` 内联发射）                               | `appEvents.ts:24`         | `appRuntimeBootstrap.svelte.ts:311`                            |
| `local-inventory-state-changed`  | `local_inventory.rs:23`                                 | `LocalInventorySnapshot` (`harubble-core/src/local_inventory/mod.rs`)              | `local_inventory.rs:202` `emit_local_inventory_state_changed`                  | `appEvents.ts:25`         | `appRuntimeBootstrap.svelte.ts:319`                            |
| `local-inventory-scan-progress`  | `local_inventory.rs:24`                                 | `LocalInventoryScanProgressEvent` (`harubble-core/src/local_inventory/mod.rs:200`) | `local_inventory.rs:209` `emit_local_inventory_scan_progress`                  | `appEvents.ts:26`         | **前端无 `listen`**（仅在 `AppEventMap` 中注册）               |
| `homepage-belong-ready`          | **无常量**，字符串字面量                                | `void`（`()`）                                                                     | `app_state/mod.rs:548` / `:603` `app_handle.emit("homepage-belong-ready", ())` | `appEvents.ts:27`         | `appRuntimeBootstrap.svelte.ts:336`                            |

## 三、Scheduling 覆盖统计

`COMMAND_SPECS` 位于 `src-tauri/src/command_scheduling.rs:68-507`，共 **73 条**（`spec()` 出现 73 次）。

按 `CommandDomain` 分布：

| Domain             | 数量   |
| ------------------ | ------ |
| InteractiveUi      | 43     |
| BackgroundIo       | 14     |
| PlaybackControl    | 4      |
| PlaybackTransition | 4      |
| VisualAux          | 3      |
| Maintenance        | 3      |
| PlaybackSideEffect | 2      |
| **合计**           | **73** |

**已注册但不在 `invoke_handler` 中的条目**（6 个内部后台入口，非 Tauri command，见 `command_scheduling.rs:610-617` `INTERNAL_SCHEDULED_ENTRIES`）：

- `record_listening_history`（`:472`，PlaybackSideEffect / CriticalSideEffect / Cooperative）
- `download_execution_loop`（`:478`，BackgroundIo / Background / Cooperative）
- `local_inventory_scan`（`:484`）
- `library_search_rebuild`（`:490`）
- `belong_warmup`（`:496`）
- `tag_registry_sync`（`:502`）

**已在 `invoke_handler` 中但未注册 spec 的漏网条目**：

- `record_song_heat`（`commands/homepage.rs:120`，`main.rs:324`）—— 未列入 `COMMAND_SPECS`，也未出现在测试白名单 `REGISTERED_TAURI_COMMANDS`（`command_scheduling.rs:540-608`）中，因此 `command_registry_covers_all_tauri_commands` 测试无法察觉 spec 缺失。

数值核对：73 spec = 68 handler + 6 internal - 1（`record_song_heat` 漏注册）。

## 四、前端 IPC 出口分裂现状

### 4.1 `src/lib/api.ts`

- 总 command 调用：48 处（`invoke` / `invokePlayback`）。
- `invokePlayback` 定义于 `api.ts:77`，7 处调用覆盖 6 个 command：`play_song`（`:185`）、`pause_playback`（`:193`）、`resume_playback`（`:197`）、`seek_current_playback`（`:203`）、`play_next`（`:207`）、`play_previous`（`:211`）。
- `invoke<T>` 显式泛型 wrapper：18 处。
- 裸 `invoke('...')` 无泛型：23 处。示例：`api.ts:116` `get_albums`、`:215` `get_player_state`、`:219` `set_playback_volume`、`:223` `show_main_window`、`:227` `get_default_output_dir`、`:240` `clear_audio_cache`、`:244` `clear_response_cache`、`:248` `reset_http_client`、`:328` `create_download_job`、`:332` `list_download_jobs`、`:338` `cancel_download_job`、`:345` `cancel_download_task`、`:351` `retry_download_job`、`:358` `retry_download_task`、`:362` `clear_download_history`、`:366` `send_test_notification`、`:415` `record_song_heat`、`:456` `set_tag_editor_entity_tag`、`:469` `remove_tag_editor_entity_tag`、`:481` `add_tag_editor_dimension`、`:485` `remove_tag_editor_dimension`、`:502` `resolve_tag_editor_conflict`、`:511` `export_tag_editor_registry`。
- 走 `invokePlayback` 的 command 未包含 `get_player_state` / `set_playback_volume`（Spec 中同属 `PlaybackControl`），也未包含 `clear_listening_history`（`PlaybackSideEffect`）。这些 command 返回错误不会被 `PlaybackCommandError` 判别。

### 4.2 `src/lib/collectionApi.ts`

10 个 command wrapper（`:5, 9, 17, 30, 39, 46, 53, 60, 64, 68`），全部使用 `invoke<T>` 显式泛型。泛型覆盖率 10 / 10 = 100%。

### 4.3 `src/lib/settingsApi.ts`

3 个 wrapper 全部使用 `invoke<T>` 显式泛型：`:7` `clear_audio_cache`（泛型 `number`）、`:13` `list_log_records`（泛型 `LogViewerPage`）、`:17` `get_log_file_status`（泛型 `LogFileStatus`）。此外从 `api.ts` re-export `selectDirectory` / `sendTestNotification`（`:4`）。泛型覆盖率 3 / 3 = 100%。

### 4.4 其他直接 import `invoke` 的前端文件

`src/` 下直接 `import { invoke } from '@tauri-apps/api/core'` 只有三个文件：`api.ts:1`、`collectionApi.ts:1`、`settingsApi.ts:1`。无 Svelte 组件、store、controller 绕过 wrapper 直接调用 `invoke`。事件 `listen` 直接 import 仅出现在 `miniPlayerBridge.ts:8`（受控例外，位于独立播放器窗口 bridge）。

## 五、已观察到的漂移或缺口

**Command / Spec 层**

- `record_song_heat`（`commands/homepage.rs:120`）注册进 `invoke_handler`（`main.rs:324`）但 **未加入 `COMMAND_SPECS`**，也 **未加入 `REGISTERED_TAURI_COMMANDS` 白名单**（`command_scheduling.rs:540-608`），`command_registry_covers_all_tauri_commands` 测试无法发现 spec 缺失。
- `get_default_output_dir`（`commands/library.rs:208`）签名为 `-> String`，**唯一**不返回 `Result` 的 command。
- `play_song` / `seek_current_playback` / `play_next` / `play_previous`（`commands/playback.rs:16, 57, 79, 93`）返回 `Result<_, PlaybackError>`（结构化错误），其余 67 个 command 均返回 `Result<_, String>`。
- Domain 与 Priority 不一致条目：`cancel_local_inventory_scan`（`command_scheduling.rs:370`，BackgroundIo + **Interactive**）、`cancel_download_job`（`:418`）、`cancel_download_task`（`:424`）。仅此三处。

**前端 wrapper 层**

- `api.ts` 6 个 playback wrapper 走 `invokePlayback`，但 Spec 中同属 playback 域的 `get_player_state`（`:106` PlaybackControl）、`set_playback_volume`（`:112` PlaybackControl）走裸 `invoke`；`clear_listening_history`（`:466` PlaybackSideEffect）也走裸 `invoke`。判别通道未覆盖全部 playback command。
- `list_log_records`、`get_log_file_status`、`clear_audio_cache` **在两处都有 wrapper**（`api.ts:396 / 400 / 240` 与 `settingsApi.ts:13 / 17 / 7`）；同一 command `clear_audio_cache` 在 `api.ts` 无泛型、在 `settingsApi.ts` 有泛型 `number`，两处泛型不一致。
- 6 个 command **完全没有前端 wrapper**：`export_preferences`（`commands/preferences.rs:70`）、`import_preferences`（`:93`）、`get_notification_permission_state`（`:127`）、`rescan_local_inventory`（`commands/local_inventory.rs:33`）、`cancel_local_inventory_scan`（`:54`）、`get_download_job`（`commands/downloads.rs:119`）。
- `api.ts` 内 23 处裸 `invoke('...')` 与 18 处 `invoke<T>('...')` 并存，而 `collectionApi.ts` / `settingsApi.ts` 全走泛型。文件内的泛型覆盖差异未受强制约束。

**Event 层**

- `local-inventory-scan-progress`（`local_inventory.rs:24`）在后端每次扫描进度都发射（`:209`）、在 `AppEventMap`（`appEvents.ts:26`）中类型化注册，但 **前端无 `listen` 订阅**（`appRuntimeBootstrap.svelte.ts` 只订阅了 `local-inventory-state-changed`，`:319`）。载荷被丢弃。
- `homepage-belong-ready`（`app_state/mod.rs:548, 603`）**没有常量定义**，两处都是字符串字面量硬编码，与其余事件（如 `PLAYER_STATE_CHANGED`）风格不一致。
- `APP_ERROR_RECORDED`（`logging.rs:14`）声明为私有 `const`（非 `pub(crate)`），也不在专门的 `events` 模块中，与其余 `pub(crate)` 常量导出的事件风格不一致。
- 事件常量分布于 4 个模块：`player/events.rs`（3 个）、`downloads/events.rs`（3 个）、`local_inventory.rs`（2 个）、`logging.rs`（1 个）；`homepage-belong-ready` 无源头模块。前端 `AppEventMap` 集中在 `appEvents.ts`。

## 六、复现方式

- Command 清单：`rg -n "#\[tauri::command\]" src-tauri/src/commands/ | wc -l`（应为 68）与 `rg -n "^\s*[a-z_]+::[a-z_]+," src-tauri/src/main.rs | head -80`。
- Spec 清单：`rg -n "spec\(" src-tauri/src/command_scheduling.rs | wc -l`（应为 73）。
- 事件常量：`rg -n "^pub\(crate\) const [A-Z_]+: &str = " src-tauri/src/`。
- 前端 wrapper：`rg -n "invoke(?:<[^>]+>)?\(" src/lib/api.ts src/lib/settingsApi.ts src/lib/collectionApi.ts`。
