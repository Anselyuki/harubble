#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Harubble Tauri 桌面后端。
//!
//! 这个二进制 crate 通过 Tauri 命令和播放器事件向前端暴露后端能力。
//!
//! # 命令面
//!
//! Svelte 前端通过 `@tauri-apps/api/core::invoke` 调用下面这些命令：
//!
//! - 目录数据：[`commands::get_albums`]、[`commands::get_album_detail`]、
//!   [`commands::get_song_detail`]、[`commands::get_song_lyrics`]
//! - 播放控制：[`commands::play_song`]、[`commands::pause_playback`]、
//!   [`commands::resume_playback`]、[`commands::seek_current_playback`]、
//!   [`commands::play_next`]、[`commands::play_previous`]、
//!   [`commands::get_player_state`]、
//!   [`commands::set_playback_volume`]
//! - 下载和工具：[`commands::get_default_output_dir`]、
//!   [`commands::clear_audio_cache`]、[`commands::extract_image_theme`]
//!
//! # 事件
//!
//! - [`player::events::PLAYER_STATE_CHANGED`] 会在播放状态、队列能力或音量
//!   变化时发出完整的 [`player::PlayerState`] 快照。
//! - [`player::events::PLAYER_PROGRESS`] 会在播放推进过程中持续发出完整的
//!   [`player::PlayerState`] 快照。
//!
//! # 生成 rustdoc
//!
//! 当前 `main.rs` 只负责 Tauri 启动与 wiring；命令与后端模块定义位于 library target。
//! 如需查看二进制入口文档，请使用：
//!
//! ```bash
//! cargo doc -p harubble --bin harubble --no-deps --document-private-items
//! ```
//!
//! 如需查看后端模块与命令定义，请使用：
//!
//! ```bash
//! cargo doc -p harubble --lib --no-deps --document-private-items
//! ```

use anyhow::Context;
use harubble::{
    commands,
    desktop_lifecycle::{self, DesktopLifecycleState},
    initialize_download_bridge, spawn_belong_warmup, spawn_inventory_scan, spawn_network_monitor,
    spawn_tag_registry_sync, AppState, LogLevel, LogPayload,
};
#[cfg(target_os = "macos")]
use tauri::WebviewWindowBuilder;
use tauri::{LogicalSize, Manager, RunEvent, WebviewWindow};

const PLAYER_BAR_SAFE_WINDOW_WIDTH: f64 = 1120.0;
const MIN_LAYOUT_WINDOW_WIDTH: f64 = 1120.0;
const DEFAULT_WINDOW_HEIGHT: f64 = 800.0;
const MIN_WINDOW_HEIGHT: f64 = 600.0;
const WINDOW_MARGIN_X: f64 = 48.0;
const WINDOW_MARGIN_Y: f64 = 72.0;

fn fit_main_window_to_monitor<R: tauri::Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let monitor = window.current_monitor()?.or(window.primary_monitor()?);
    let Some(monitor) = monitor else {
        return Ok(());
    };

    let work_area = monitor.work_area();
    let scale_factor = monitor.scale_factor().max(1.0);
    let available_width = work_area.size.width as f64 / scale_factor;
    let available_height = work_area.size.height as f64 / scale_factor;
    if available_width <= 0.0 || available_height <= 0.0 {
        return Ok(());
    }

    let max_width = if available_width > WINDOW_MARGIN_X {
        available_width - WINDOW_MARGIN_X
    } else {
        available_width
    };
    let max_height = if available_height > WINDOW_MARGIN_Y {
        available_height - WINDOW_MARGIN_Y
    } else {
        available_height
    };

    let width = PLAYER_BAR_SAFE_WINDOW_WIDTH.min(max_width).round();
    let height = DEFAULT_WINDOW_HEIGHT.min(max_height).round();
    let min_width = MIN_LAYOUT_WINDOW_WIDTH.min(width).round();
    let min_height = MIN_WINDOW_HEIGHT.min(height).round();

    window.set_min_size(Some(LogicalSize::new(min_width, min_height)))?;
    window.set_size(LogicalSize::new(width, height))?;
    window.center()?;

    Ok(())
}

fn attach_main_window_lifecycle<R: tauri::Runtime>(window: &WebviewWindow<R>) {
    let lifecycle_window = window.clone();
    window.on_window_event(move |event| {
        desktop_lifecycle::handle_main_window_event(&lifecycle_window, event);
    });
}

fn configure_main_window<R: tauri::Runtime>(window: &WebviewWindow<R>, state: &AppState) {
    attach_main_window_lifecycle(window);
    if let Err(error) = fit_main_window_to_monitor(window) {
        state.record_log(
            LogPayload::new(
                LogLevel::Warn,
                "window",
                "window.fit_monitor_failed",
                "Failed to fit main window to monitor",
            )
            .details(error.to_string()),
        );
    }

    #[cfg(debug_assertions)]
    {
        window.open_devtools();
    }
}

#[cfg(target_os = "macos")]
fn create_main_window<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> anyhow::Result<WebviewWindow<R>> {
    if let Some(window) = app.get_webview_window(desktop_lifecycle::MAIN_WINDOW_LABEL) {
        return Ok(window);
    }

    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|config| config.label == desktop_lifecycle::MAIN_WINDOW_LABEL)
        .context("Failed to locate main window config")?;
    WebviewWindowBuilder::from_config(app, config)?
        .build()
        .context("Failed to create main window")
}

#[cfg(target_os = "macos")]
fn restore_or_create_main_window<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> anyhow::Result<()> {
    if app
        .get_webview_window(desktop_lifecycle::MAIN_WINDOW_LABEL)
        .is_some()
    {
        desktop_lifecycle::restore_main_window(app).context("Failed to restore main window")?;
        return Ok(());
    }

    let window = create_main_window(app)?;
    if let Some(state) = app.try_state::<AppState>() {
        configure_main_window(&window, &state);
    } else {
        attach_main_window_lifecycle(&window);
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn handle_macos_dock_reopen<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    has_visible_windows: bool,
) {
    let main_window = app.get_webview_window(desktop_lifecycle::MAIN_WINDOW_LABEL);
    let main_window_focused = main_window
        .as_ref()
        .is_some_and(|window| window.is_focused().unwrap_or(false));

    match desktop_lifecycle::dock_reopen_action(
        desktop_lifecycle::current_platform(),
        has_visible_windows,
        main_window_focused,
    ) {
        desktop_lifecycle::DockReopenAction::Ignore => {}
        desktop_lifecycle::DockReopenAction::FocusVisibleWindow
        | desktop_lifecycle::DockReopenAction::RestoreOrCreateWindow => {
            if let Err(error) = restore_or_create_main_window(app) {
                record_desktop_lifecycle_log(
                    app,
                    "desktop_lifecycle.dock_reopen_failed",
                    "Failed to reopen main window from Dock",
                    error.to_string(),
                );
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn record_desktop_lifecycle_log<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    event: &'static str,
    message: &'static str,
    details: String,
) {
    if let Some(state) = app.try_state::<AppState>() {
        state.record_log(
            LogPayload::new(LogLevel::Warn, "desktop-lifecycle", event, message).details(details),
        );
    }
}

fn flush_logs_on_exit<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Err(error) = state.flush_logs_on_exit() {
            eprintln!("[logging] failed to flush session logs: {error}");
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let _ =
                tauri_plugin_notification::NotificationExt::notification(app).request_permission();
            let window = app
                .get_webview_window("main")
                .context("Failed to locate main window")?;
            let state =
                AppState::new(app.handle().clone()).expect("Failed to initialize app state");
            app.manage(DesktopLifecycleState::default());
            configure_main_window(&window, &state);
            if let Err(error) = state.bind_media_controls() {
                state.record_log(
                    LogPayload::new(
                        LogLevel::Warn,
                        "media-session",
                        "media_session.bind_failed",
                        "Failed to bind media controls",
                    )
                    .details(error),
                );
            }
            initialize_download_bridge(app.handle(), &state);
            spawn_inventory_scan(
                app.handle().clone(),
                state.clone(),
                state.output_dir(),
                None,
            );
            spawn_belong_warmup(app.handle().clone(), &state);
            spawn_tag_registry_sync(&state);
            spawn_network_monitor(&state);
            app.manage(state);
            if let Err(error) = desktop_lifecycle::install_background_entrypoint(app.handle()) {
                if let Some(state) = app.handle().try_state::<AppState>() {
                    state.record_log(
                        LogPayload::new(
                            LogLevel::Warn,
                            "desktop-lifecycle",
                            "desktop_lifecycle.entrypoint_install_failed",
                            "Failed to install desktop background entrypoint",
                        )
                        .details(error.to_string()),
                    );
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::collection::list_collections,
            commands::collection::get_collection,
            commands::collection::create_collection,
            commands::collection::update_collection,
            commands::collection::delete_collection,
            commands::collection::add_songs_to_collection,
            commands::collection::remove_songs_from_collection,
            commands::collection::reorder_collection_songs,
            commands::collection::export_collection,
            commands::collection::import_collection,
            commands::library::get_albums,
            commands::library::get_album_detail,
            commands::library::get_song_detail,
            commands::library::get_song_lyrics,
            commands::library::extract_image_theme,
            commands::library::get_image_data_url,
            commands::library::get_default_output_dir,
            commands::search::search_library,
            commands::playback::play_song,
            commands::playback::pause_playback,
            commands::playback::resume_playback,
            commands::playback::seek_current_playback,
            commands::playback::play_next,
            commands::playback::play_previous,
            commands::playback::get_player_state,
            commands::playback::set_playback_volume,
            commands::window::show_main_window,
            commands::preferences::get_preferences,
            commands::preferences::set_preferences,
            commands::preferences::export_preferences,
            commands::preferences::import_preferences,
            commands::local_inventory::get_local_inventory_snapshot,
            commands::local_inventory::rescan_local_inventory,
            commands::local_inventory::cancel_local_inventory_scan,
            commands::local_inventory::get_audio_metadata,
            commands::preferences::get_notification_permission_state,
            commands::preferences::send_test_notification,
            commands::logging::list_log_records,
            commands::logging::get_log_file_status,
            commands::downloads::clear_audio_cache,
            commands::downloads::clear_response_cache,
            commands::downloads::reset_http_client,
            commands::downloads::create_download_job,
            commands::downloads::list_download_jobs,
            commands::downloads::get_download_job,
            commands::downloads::cancel_download_job,
            commands::downloads::cancel_download_task,
            commands::downloads::retry_download_job,
            commands::downloads::retry_download_task,
            commands::downloads::clear_download_history,
            commands::homepage::get_latest_albums,
            commands::homepage::get_albums_by_series,
            commands::homepage::get_recent_history,
            commands::homepage::clear_listening_history,
            commands::homepage::get_homepage_status,
            commands::tag_registry::get_tag_dimensions,
            commands::tag_registry::get_albums_by_tag_dimension,
            commands::tag_editor::get_tag_editor_merged,
            commands::tag_editor::get_tag_editor_local_overlay,
            commands::tag_editor::set_tag_editor_entity_tag,
            commands::tag_editor::remove_tag_editor_entity_tag,
            commands::tag_editor::add_tag_editor_dimension,
            commands::tag_editor::remove_tag_editor_dimension,
            commands::tag_editor::apply_tag_editor_remote_update,
            commands::tag_editor::resolve_tag_editor_conflict,
            commands::tag_editor::export_tag_editor_registry,
            commands::tag_editor::import_tag_editor_registry,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            #[cfg(target_os = "macos")]
            RunEvent::Reopen {
                has_visible_windows,
                ..
            } => {
                handle_macos_dock_reopen(app_handle, has_visible_windows);
            }
            RunEvent::ExitRequested { api, .. } => {
                if desktop_lifecycle::should_prevent_exit_after_window_close(
                    desktop_lifecycle::current_platform(),
                    app_handle.try_state::<DesktopLifecycleState>().as_deref(),
                ) {
                    api.prevent_exit();
                    return;
                }
                flush_logs_on_exit(app_handle);
            }
            RunEvent::Exit => {
                flush_logs_on_exit(app_handle);
            }
            _ => {}
        });
}
