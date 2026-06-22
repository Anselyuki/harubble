//! Desktop lifecycle helpers for background playback entrypoints.
//!
//! This module keeps platform-specific tray/menu-bar behavior out of `main.rs`.

use crate::{AppState, LogLevel, LogPayload};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime, WebviewWindow, WindowEvent};

pub const MAIN_WINDOW_LABEL: &str = "main";

#[cfg(any(target_os = "windows", target_os = "macos"))]
const TRAY_ID: &str = "harubble-background";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const MENU_SHOW: &str = "show";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const MENU_TOGGLE_PLAYBACK: &str = "toggle-playback";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const MENU_QUIT: &str = "quit";

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
    matches!(os, "windows" | "macos")
}

pub fn should_close_to_background(window_label: &str, os: &str) -> bool {
    window_label == MAIN_WINDOW_LABEL && should_install_background_entrypoint(os)
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

pub fn current_platform() -> &'static str {
    std::env::consts::OS
}

pub fn restore_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
    }

    Ok(())
}

pub fn hide_main_window<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    window.hide()
}

pub fn request_real_quit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(lifecycle) = app.try_state::<DesktopLifecycleState>() {
        lifecycle.mark_quitting();
    }
    app.exit(0);
}

pub fn handle_main_window_event<R: Runtime>(window: &WebviewWindow<R>, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };

    let app_handle = window.app_handle();
    let lifecycle = app_handle.try_state::<DesktopLifecycleState>();
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

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub fn install_background_entrypoint<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

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
                ..
            } = event
            {
                let _ = restore_main_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SHOW => {
                let _ = restore_main_window(app);
            }
            MENU_TOGGLE_PLAYBACK => {
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
            MENU_QUIT => request_real_quit(app),
            _ => {}
        })
        .build(app)?;

    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory)?;

    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
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
