//! Desktop lifecycle helpers for background playback entrypoints.
//!
//! This module keeps platform-specific tray and Dock behavior out of `main.rs`.

use crate::{AppState, LogLevel, LogPayload};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Rect, Runtime, WebviewWindow, WindowEvent};

#[cfg(target_os = "windows")]
use tauri::{LogicalPosition, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

pub const MAIN_WINDOW_LABEL: &str = "main";
pub const MINI_PLAYER_WINDOW_LABEL: &str = "mini-player";
#[cfg(target_os = "windows")]
const MINI_PLAYER_WIDTH: f64 = 336.0;
#[cfg(target_os = "windows")]
const MINI_PLAYER_HEIGHT: f64 = 168.0;
#[cfg(target_os = "windows")]
const MINI_PLAYER_MARGIN: f64 = 12.0;

#[cfg(target_os = "windows")]
const TRAY_ID: &str = "harubble-background";
#[cfg(target_os = "windows")]
const MENU_SHOW: &str = "show";
#[cfg(target_os = "windows")]
const MENU_TOGGLE_PLAYBACK: &str = "toggle-playback";
#[cfg(target_os = "windows")]
const MENU_QUIT: &str = "quit";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DockReopenAction {
    Ignore,
    FocusVisibleWindow,
    RestoreOrCreateWindow,
}

#[derive(Clone, Default)]
pub struct DesktopLifecycleState {
    quitting: Arc<AtomicBool>,
}

impl DesktopLifecycleState {
    pub fn mark_quitting(&self) {
        self.quitting.store(true, Ordering::SeqCst);
    }

    pub fn is_quitting(&self) -> bool {
        self.quitting.load(Ordering::SeqCst)
    }
}

pub fn should_install_background_entrypoint(os: &str) -> bool {
    matches!(os, "windows")
}

pub fn should_close_to_background(window_label: &str, os: &str) -> bool {
    window_label == MAIN_WINDOW_LABEL && matches!(os, "windows")
}

pub fn should_minimize_to_dock_on_close(window_label: &str, os: &str) -> bool {
    window_label == MAIN_WINDOW_LABEL && matches!(os, "macos")
}

pub fn should_install_mini_player_window(os: &str) -> bool {
    matches!(os, "windows")
}

pub fn should_hide_on_focus_loss(window_label: &str, os: &str) -> bool {
    window_label == MINI_PLAYER_WINDOW_LABEL && should_install_mini_player_window(os)
}

pub fn should_close_to_background_with_state(
    window_label: &str,
    os: &str,
    lifecycle: Option<&DesktopLifecycleState>,
) -> bool {
    if lifecycle.is_some_and(DesktopLifecycleState::is_quitting) {
        return false;
    }

    should_close_to_background(window_label, os)
}

pub fn should_minimize_to_dock_on_close_with_state(
    window_label: &str,
    os: &str,
    lifecycle: Option<&DesktopLifecycleState>,
) -> bool {
    if lifecycle.is_some_and(DesktopLifecycleState::is_quitting) {
        return false;
    }

    should_minimize_to_dock_on_close(window_label, os)
}

pub fn current_platform() -> &'static str {
    std::env::consts::OS
}

pub fn dock_reopen_action(
    os: &str,
    has_visible_windows: bool,
    main_window_focused: bool,
) -> DockReopenAction {
    if !matches!(os, "macos") {
        return DockReopenAction::Ignore;
    }

    if !has_visible_windows {
        return DockReopenAction::RestoreOrCreateWindow;
    }

    if main_window_focused {
        DockReopenAction::Ignore
    } else {
        DockReopenAction::FocusVisibleWindow
    }
}

pub fn restore_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    hide_mini_player_window(app)?;
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.unminimize()?;
        window.show()?;
        window.set_focus()?;
    }

    Ok(())
}

pub fn hide_main_window<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    window.hide()
}

pub fn minimize_main_window<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    window.minimize()
}

pub fn hide_mini_player_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MINI_PLAYER_WINDOW_LABEL) {
        window.hide()?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn install_mini_player_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if app.get_webview_window(MINI_PLAYER_WINDOW_LABEL).is_some() {
        return Ok(());
    }

    let window = WebviewWindowBuilder::new(
        app,
        MINI_PLAYER_WINDOW_LABEL,
        WebviewUrl::App("index.html?window=mini-player".into()),
    )
    .title("Harubble Mini Player")
    .inner_size(MINI_PLAYER_WIDTH, MINI_PLAYER_HEIGHT)
    .resizable(false)
    .decorations(false)
    .shadow(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .focused(false)
    .build()?;

    window.on_window_event({
        let mini_window = window.clone();
        move |event| handle_mini_player_window_event(&mini_window, event)
    });

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn install_mini_player_window<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn toggle_mini_player_window<R: Runtime>(
    app: &AppHandle<R>,
    anchor: Option<Rect>,
) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(MINI_PLAYER_WINDOW_LABEL) else {
        return restore_main_window(app);
    };

    if window.is_visible().unwrap_or(false) {
        window.hide()?;
        return Ok(());
    }

    position_mini_player_window(app, &window, anchor)?;
    window.show()?;
    window.set_focus()?;

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn toggle_mini_player_window<R: Runtime>(
    app: &AppHandle<R>,
    _anchor: Option<Rect>,
) -> tauri::Result<()> {
    restore_main_window(app)
}

pub fn handle_mini_player_window_event<R: Runtime>(window: &WebviewWindow<R>, event: &WindowEvent) {
    let WindowEvent::Focused(false) = event else {
        return;
    };

    if should_hide_on_focus_loss(window.label(), current_platform()) {
        let _ = window.hide();
    }
}

pub fn request_real_quit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(lifecycle) = app.try_state::<DesktopLifecycleState>() {
        lifecycle.mark_quitting();
    }
    app.exit(0);
}

#[cfg(target_os = "windows")]
fn toggle_playback_from_lifecycle<R: Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Err(error) = state.toggle_playback_from_lifecycle() {
            state.record_log(
                LogPayload::new(
                    LogLevel::Warn,
                    "desktop-lifecycle",
                    "desktop_lifecycle.toggle_playback_failed",
                    "Failed to toggle playback from tray",
                )
                .details(error),
            );
        }
    }
}

pub fn handle_main_window_event<R: Runtime>(window: &WebviewWindow<R>, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };

    let app_handle = window.app_handle();
    let lifecycle = app_handle.try_state::<DesktopLifecycleState>();
    if should_minimize_to_dock_on_close_with_state(
        window.label(),
        current_platform(),
        lifecycle.as_deref(),
    ) {
        api.prevent_close();
        if let Err(error) = minimize_main_window(window) {
            record_lifecycle_log(
                &app_handle,
                LogLevel::Warn,
                "desktop_lifecycle.minimize_failed",
                "Failed to minimize main window for background playback",
                error.to_string(),
            );
        }
        return;
    }

    if !should_close_to_background_with_state(
        window.label(),
        current_platform(),
        lifecycle.as_deref(),
    ) {
        return;
    }

    api.prevent_close();
    if let Err(error) = hide_main_window(window) {
        record_lifecycle_log(
            &app_handle,
            LogLevel::Warn,
            "desktop_lifecycle.hide_failed",
            "Failed to hide main window for background playback",
            error.to_string(),
        );
    }
}

#[cfg(target_os = "windows")]
pub fn install_background_entrypoint<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    install_mini_player_window(app)?;

    let show = MenuItem::with_id(app, MENU_SHOW, "Show Harubble", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, MENU_TOGGLE_PLAYBACK, "Play/Pause", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Harubble", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &toggle, &quit])?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tauri::include_image!("icons/32x32.png"))
        .tooltip("Harubble")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                position,
                ..
            } = event
            {
                let anchor = Some(rect).filter(|rect| has_non_zero_rect(rect));
                let anchor = anchor.or_else(|| fallback_anchor_from_position(position));
                let _ = toggle_mini_player_window(tray.app_handle(), anchor);
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SHOW => {
                let _ = restore_main_window(app);
            }
            MENU_TOGGLE_PLAYBACK => toggle_playback_from_lifecycle(app),
            MENU_QUIT => request_real_quit(app),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn install_background_entrypoint<R: Runtime>(_app: &AppHandle<R>) -> tauri::Result<()> {
    Ok(())
}

fn record_lifecycle_log<R: Runtime>(
    app: &AppHandle<R>,
    level: LogLevel,
    event: &'static str,
    message: &'static str,
    details: String,
) {
    if let Some(state) = app.try_state::<AppState>() {
        state.record_log(
            LogPayload::new(level, "desktop-lifecycle", event, message).details(details),
        );
    }
}

#[cfg(target_os = "windows")]
fn position_mini_player_window<R: Runtime>(
    app: &AppHandle<R>,
    window: &WebviewWindow<R>,
    anchor: Option<Rect>,
) -> tauri::Result<()> {
    let target = anchor
        .and_then(|rect| position_from_anchor(app, rect))
        .or_else(|| fallback_position_from_primary_monitor(app))
        .unwrap_or(LogicalPosition::new(MINI_PLAYER_MARGIN, MINI_PLAYER_MARGIN));

    window.set_position(target)
}

#[cfg(target_os = "windows")]
fn position_from_anchor<R: Runtime>(
    app: &AppHandle<R>,
    anchor: Rect,
) -> Option<LogicalPosition<f64>> {
    let anchor_position = anchor.position.to_physical::<f64>(1.0);
    let anchor_size = anchor.size.to_physical::<f64>(1.0);
    let anchor_center_x = anchor_position.x + anchor_size.width / 2.0;
    let anchor_bottom_y = anchor_position.y + anchor_size.height;
    let monitor = app
        .monitor_from_point(anchor_center_x, anchor_position.y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let scale_factor = monitor.scale_factor().max(1.0);
    let work_area = monitor.work_area();
    let work_left = work_area.position.x as f64 / scale_factor;
    let work_top = work_area.position.y as f64 / scale_factor;
    let work_width = work_area.size.width as f64 / scale_factor;
    let work_height = work_area.size.height as f64 / scale_factor;
    let anchor_x = anchor_center_x / scale_factor;
    let anchor_y = anchor_bottom_y / scale_factor;
    let x = (anchor_x - MINI_PLAYER_WIDTH / 2.0)
        .max(work_left + MINI_PLAYER_MARGIN)
        .min(work_left + work_width - MINI_PLAYER_WIDTH - MINI_PLAYER_MARGIN);
    let y_below = anchor_y + MINI_PLAYER_MARGIN;
    let y = if y_below + MINI_PLAYER_HEIGHT <= work_top + work_height - MINI_PLAYER_MARGIN {
        y_below
    } else {
        work_top + MINI_PLAYER_MARGIN
    };

    Some(LogicalPosition::new(x, y))
}

#[cfg(target_os = "windows")]
fn fallback_position_from_primary_monitor<R: Runtime>(
    app: &AppHandle<R>,
) -> Option<LogicalPosition<f64>> {
    let monitor = app.primary_monitor().ok().flatten()?;
    let scale_factor = monitor.scale_factor().max(1.0);
    let work_area = monitor.work_area();
    let work_left = work_area.position.x as f64 / scale_factor;
    let work_top = work_area.position.y as f64 / scale_factor;
    let work_width = work_area.size.width as f64 / scale_factor;

    Some(LogicalPosition::new(
        work_left + work_width - MINI_PLAYER_WIDTH - MINI_PLAYER_MARGIN,
        work_top + MINI_PLAYER_MARGIN,
    ))
}

#[cfg(target_os = "windows")]
fn has_non_zero_rect(rect: &Rect) -> bool {
    let size = rect.size.to_physical::<f64>(1.0);
    size.width > 0.0 && size.height > 0.0
}

#[cfg(target_os = "windows")]
fn fallback_anchor_from_position(position: PhysicalPosition<f64>) -> Option<Rect> {
    if position.x == 0.0 && position.y == 0.0 {
        return None;
    }

    Some(Rect {
        position: position.into(),
        size: tauri::PhysicalSize::new(1.0, 1.0).into(),
    })
}
