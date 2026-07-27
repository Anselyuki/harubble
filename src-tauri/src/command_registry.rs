//! # 命令注册表（Command Registry）
//!
//! 本模块是所有 Tauri command 注册的**唯一真相来源**。
//!
//! ## 设计目标
//!
//! 通过"回调宏"（higher-order macro）模式，将命令列表定义一次，
//! 在多个调用点复用，消除 `main.rs` 的 `generate_handler![]` 与
//! 测试模块中的 `REGISTERED_TAURI_COMMANDS` 之间的手动同步负担。
//!
//! ## 使用方式
//!
//! ```rust,ignore
//! // 在 main.rs 中生成 handler 列表
//! macro_rules! expand_handler {
//!     ($(($path:path, $name:literal, $domain:ident, $priority:ident, $cancel:ident),)*) => {
//!         tauri::generate_handler![$($path,)*]
//!     };
//! }
//! let handler = for_each_tauri_command!(expand_handler);
//!
//! // 在测试中生成命令名称列表
//! macro_rules! collect_names {
//!     ($(($path:path, $name:literal, $domain:ident, $priority:ident, $cancel:ident),)*) => {
//!         &[$($name,)*]
//!     };
//! }
//! const NAMES: &[&str] = for_each_tauri_command!(collect_names);
//! ```
//!
//! ## 条目格式
//!
//! 每条记录形如：
//! ```text
//! (handler_path, "name", Domain, Priority, CancelPolicy)
//! ```
//!
//! - `handler_path`：Rust 函数路径，用于 `generate_handler!`
//! - `"name"`：Tauri command 名称字符串（与前端 `invoke` 调用保持一致）
//! - `Domain`：命令所属调度域（见 `command_scheduling` 模块）
//! - `Priority`：调度优先级
//! - `CancelPolicy`：取消策略
//!
//! ## 注意事项
//!
//! - 新增或删除 command 时，**只需修改本文件**，`main.rs` 与测试侧均会自动同步。
//! - `record_song_heat` 此前曾在 `COMMAND_SPECS` 中遗漏，已在此处补全。
//! - 条目总数：68 条。

/// 回调宏：将完整的 Tauri command 列表传递给调用方宏 `$mac`。
///
/// 调用方宏须接受如下形式的 token 流：
/// ```text
/// $mac!( (path, "name", Domain, Priority, CancelPolicy), ... )
/// ```
///
/// 使用 `#[macro_export]` 导出至 crate 根，确保 `main.rs` 及测试模块均可直接引用。
#[macro_export]
macro_rules! for_each_tauri_command {
    ($mac:ident) => {
        $mac!(
            // ── Collection (10) ──────────────────────────────────────────────
            (commands::collection::list_collections, "list_collections", InteractiveUi, Interactive, Cooperative),
            (commands::collection::get_collection, "get_collection", InteractiveUi, Interactive, Cooperative),
            (commands::collection::create_collection, "create_collection", InteractiveUi, Interactive, Cooperative),
            (commands::collection::update_collection, "update_collection", InteractiveUi, Interactive, Cooperative),
            (commands::collection::delete_collection, "delete_collection", InteractiveUi, Interactive, Cooperative),
            (commands::collection::add_songs_to_collection, "add_songs_to_collection", InteractiveUi, Interactive, Cooperative),
            (commands::collection::remove_songs_from_collection, "remove_songs_from_collection", InteractiveUi, Interactive, Cooperative),
            (commands::collection::reorder_collection_songs, "reorder_collection_songs", InteractiveUi, Interactive, Cooperative),
            (commands::collection::export_collection, "export_collection", InteractiveUi, Interactive, Cooperative),
            (commands::collection::import_collection, "import_collection", InteractiveUi, Interactive, Cooperative),

            // ── Library (8) ──────────────────────────────────────────────────
            (commands::library::get_albums, "get_albums", InteractiveUi, Interactive, Cooperative),
            (commands::library::get_album_detail, "get_album_detail", InteractiveUi, Interactive, Cooperative),
            (commands::library::refresh_album_detail, "refresh_album_detail", InteractiveUi, Interactive, Cooperative),
            (commands::library::get_song_detail, "get_song_detail", InteractiveUi, Interactive, Cooperative),
            (commands::library::get_song_lyrics, "get_song_lyrics", VisualAux, Visual, LatestWins),
            (commands::library::extract_image_theme, "extract_image_theme", VisualAux, Visual, LatestWins),
            (commands::library::get_cached_image_path, "get_cached_image_path", VisualAux, Visual, Cooperative),
            (commands::library::get_default_output_dir, "get_default_output_dir", InteractiveUi, Interactive, NeverCancel),

            // ── AlbumCatalog (2) ─────────────────────────────────────────────
            (commands::album_catalog::get_album_catalog, "get_album_catalog", InteractiveUi, Interactive, Cooperative),
            (commands::album_catalog::refresh_album_catalog, "refresh_album_catalog", InteractiveUi, Interactive, Cooperative),

            // ── Search (1) ───────────────────────────────────────────────────
            (commands::search::search_library, "search_library", InteractiveUi, Interactive, Cooperative),

            // ── Playback (8) ─────────────────────────────────────────────────
            (commands::playback::play_song, "play_song", PlaybackTransition, Playback, SupersedePlaybackSession),
            (commands::playback::pause_playback, "pause_playback", PlaybackControl, Playback, NeverCancel),
            (commands::playback::resume_playback, "resume_playback", PlaybackControl, Playback, NeverCancel),
            (commands::playback::seek_current_playback, "seek_current_playback", PlaybackTransition, Playback, LatestWins),
            (commands::playback::play_next, "play_next", PlaybackTransition, Playback, SupersedePlaybackSession),
            (commands::playback::play_previous, "play_previous", PlaybackTransition, Playback, SupersedePlaybackSession),
            (commands::playback::get_player_state, "get_player_state", PlaybackControl, Playback, NeverCancel),
            (commands::playback::set_playback_volume, "set_playback_volume", PlaybackControl, Playback, NeverCancel),

            // ── Window (1) ───────────────────────────────────────────────────
            (commands::window::show_main_window, "show_main_window", InteractiveUi, Interactive, NeverCancel),

            // ── Menu (1) ─────────────────────────────────────────────────────
            (commands::menu::sync_playback_menu_state, "sync_playback_menu_state", VisualAux, Visual, LatestWins),

            // ── Preferences (6) ──────────────────────────────────────────────
            (commands::preferences::get_preferences, "get_preferences", InteractiveUi, Interactive, Cooperative),
            (commands::preferences::set_preferences, "set_preferences", InteractiveUi, Interactive, Cooperative),
            (commands::preferences::export_preferences, "export_preferences", InteractiveUi, Interactive, Cooperative),
            (commands::preferences::import_preferences, "import_preferences", InteractiveUi, Interactive, Cooperative),
            (commands::preferences::get_notification_permission_state, "get_notification_permission_state", InteractiveUi, Interactive, Cooperative),
            (commands::preferences::send_test_notification, "send_test_notification", InteractiveUi, Interactive, Cooperative),

            // ── LocalInventory (4) ───────────────────────────────────────────
            (commands::local_inventory::get_local_inventory_snapshot, "get_local_inventory_snapshot", InteractiveUi, Interactive, Cooperative),
            (commands::local_inventory::rescan_local_inventory, "rescan_local_inventory", BackgroundIo, Background, Cooperative),
            (commands::local_inventory::cancel_local_inventory_scan, "cancel_local_inventory_scan", BackgroundIo, Interactive, NeverCancel),
            (commands::local_inventory::get_audio_metadata, "get_audio_metadata", BackgroundIo, Background, Cooperative),

            // ── Logging (2) ──────────────────────────────────────────────────
            (commands::logging::list_log_records, "list_log_records", InteractiveUi, Interactive, Cooperative),
            (commands::logging::get_log_file_status, "get_log_file_status", InteractiveUi, Interactive, Cooperative),

            // ── Downloads (11) ───────────────────────────────────────────────
            (commands::downloads::clear_audio_cache, "clear_audio_cache", Maintenance, CriticalSideEffect, Cooperative),
            (commands::downloads::clear_response_cache, "clear_response_cache", Maintenance, CriticalSideEffect, Cooperative),
            (commands::downloads::reset_http_client, "reset_http_client", Maintenance, CriticalSideEffect, Cooperative),
            (commands::downloads::create_download_job, "create_download_job", BackgroundIo, Background, Cooperative),
            (commands::downloads::list_download_jobs, "list_download_jobs", InteractiveUi, Interactive, Cooperative),
            (commands::downloads::get_download_job, "get_download_job", InteractiveUi, Interactive, Cooperative),
            (commands::downloads::cancel_download_job, "cancel_download_job", BackgroundIo, Interactive, NeverCancel),
            (commands::downloads::cancel_download_task, "cancel_download_task", BackgroundIo, Interactive, NeverCancel),
            (commands::downloads::retry_download_job, "retry_download_job", BackgroundIo, Background, Cooperative),
            (commands::downloads::retry_download_task, "retry_download_task", BackgroundIo, Background, Cooperative),
            (commands::downloads::clear_download_history, "clear_download_history", BackgroundIo, Background, Cooperative),

            // ── Homepage (6) ─────────────────────────────────────────────────
            // 注意：record_song_heat 此前在 COMMAND_SPECS 中遗漏，此处已补全。
            (commands::homepage::get_latest_albums, "get_latest_albums", InteractiveUi, Interactive, Cooperative),
            (commands::homepage::get_albums_by_series, "get_albums_by_series", InteractiveUi, Interactive, Cooperative),
            (commands::homepage::get_recent_history, "get_recent_history", InteractiveUi, Interactive, Cooperative),
            (commands::homepage::record_song_heat, "record_song_heat", PlaybackSideEffect, CriticalSideEffect, Cooperative),
            (commands::homepage::clear_listening_history, "clear_listening_history", PlaybackSideEffect, CriticalSideEffect, Cooperative),
            (commands::homepage::get_homepage_status, "get_homepage_status", InteractiveUi, Interactive, Cooperative),

            // ── TagRegistry (2) ──────────────────────────────────────────────
            (commands::tag_registry::get_tag_dimensions, "get_tag_dimensions", InteractiveUi, Interactive, Cooperative),
            (commands::tag_registry::get_albums_by_tag_dimension, "get_albums_by_tag_dimension", InteractiveUi, Interactive, Cooperative),

            // ── TagEditor (10) ───────────────────────────────────────────────
            (commands::tag_editor::get_tag_editor_merged, "get_tag_editor_merged", InteractiveUi, Interactive, Cooperative),
            (commands::tag_editor::get_tag_editor_local_overlay, "get_tag_editor_local_overlay", InteractiveUi, Interactive, Cooperative),
            (commands::tag_editor::set_tag_editor_entity_tag, "set_tag_editor_entity_tag", InteractiveUi, Interactive, Cooperative),
            (commands::tag_editor::remove_tag_editor_entity_tag, "remove_tag_editor_entity_tag", InteractiveUi, Interactive, Cooperative),
            (commands::tag_editor::add_tag_editor_dimension, "add_tag_editor_dimension", InteractiveUi, Interactive, Cooperative),
            (commands::tag_editor::remove_tag_editor_dimension, "remove_tag_editor_dimension", InteractiveUi, Interactive, Cooperative),
            (commands::tag_editor::apply_tag_editor_remote_update, "apply_tag_editor_remote_update", InteractiveUi, Interactive, Cooperative),
            (commands::tag_editor::resolve_tag_editor_conflict, "resolve_tag_editor_conflict", InteractiveUi, Interactive, Cooperative),
            (commands::tag_editor::export_tag_editor_registry, "export_tag_editor_registry", InteractiveUi, Interactive, Cooperative),
            (commands::tag_editor::import_tag_editor_registry, "import_tag_editor_registry", InteractiveUi, Interactive, Cooperative),

            // ── ThemePackages (9 · Phase 1 MVP) ──────────────────────────────
            (commands::theme_packages::list_theme_packages, "list_theme_packages", BackgroundIo, Interactive, Cooperative),
            (commands::theme_packages::inspect_theme_package, "inspect_theme_package", VisualAux, Visual, LatestWins),
            (commands::theme_packages::install_theme_package_from_file, "install_theme_package_from_file", BackgroundIo, Background, Cooperative),
            (commands::theme_packages::install_theme_package_from_url, "install_theme_package_from_url", BackgroundIo, Background, Cooperative),
            (commands::theme_packages::uninstall_theme_package, "uninstall_theme_package", BackgroundIo, Background, Cooperative),
            (commands::theme_packages::set_active_theme_package, "set_active_theme_package", InteractiveUi, Interactive, LatestWins),
            (commands::theme_packages::preview_theme_package, "preview_theme_package", VisualAux, Visual, LatestWins),
            (commands::theme_packages::dismiss_theme_preview, "dismiss_theme_preview", InteractiveUi, Interactive, LatestWins),
            (commands::theme_packages::export_theme_package, "export_theme_package", BackgroundIo, Background, Cooperative),
        )
    };
}
