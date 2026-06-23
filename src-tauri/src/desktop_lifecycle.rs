//! Desktop lifecycle helpers for background playback entrypoints.
//!
//! This module keeps platform-specific tray/menu-bar behavior out of `main.rs`.

#[cfg(target_os = "macos")]
use crate::i18n;
#[cfg(target_os = "macos")]
use crate::player::PlayerState;
#[cfg(target_os = "macos")]
use crate::preferences::Locale;
use crate::{AppState, LogLevel, LogPayload};
#[cfg(target_os = "macos")]
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Rect, Runtime, WebviewWindow, WindowEvent};

#[cfg(target_os = "macos")]
use tauri::menu::MenuItem;
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

#[cfg(any(target_os = "windows", target_os = "macos"))]
const TRAY_ID: &str = "harubble-background";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const MENU_SHOW: &str = "show";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const MENU_NOW_PLAYING: &str = "now-playing";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const MENU_PREVIOUS: &str = "previous";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const MENU_TOGGLE_PLAYBACK: &str = "toggle-playback";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const MENU_NEXT: &str = "next";
#[cfg(any(target_os = "windows", target_os = "macos"))]
const MENU_QUIT: &str = "quit";
#[cfg(target_os = "macos")]
static NATIVE_MENU_REFRESH_ID: AtomicU64 = AtomicU64::new(0);

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

pub fn should_install_mini_player_window(os: &str) -> bool {
    matches!(os, "windows")
}

pub fn should_hide_on_focus_loss(window_label: &str, os: &str) -> bool {
    window_label == MINI_PLAYER_WINDOW_LABEL && should_install_mini_player_window(os)
}

pub fn should_refresh_native_menu_asynchronously(os: &str) -> bool {
    matches!(os, "macos")
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
    hide_mini_player_window(app)?;
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

#[cfg(target_os = "macos")]
fn native_playback_summary(state: Option<&PlayerState>, locale: Locale) -> String {
    match state.and_then(|state| state.song_name.as_deref()) {
        Some(song_name) if !song_name.trim().is_empty() => song_name.to_string(),
        _ => i18n::tr(locale, "desktop-menu-not-playing"),
    }
}

#[cfg(target_os = "macos")]
fn native_playback_toggle_label(state: Option<&PlayerState>, locale: Locale) -> String {
    let is_playing = state.is_some_and(|state| state.is_playing && !state.is_loading);
    let key = if is_playing {
        "desktop-menu-pause"
    } else {
        "desktop-menu-play"
    };

    i18n::tr(locale, key)
}

#[cfg(target_os = "macos")]
fn native_playback_toggle_enabled(state: Option<&PlayerState>) -> bool {
    state.is_some_and(|state| {
        state.song_cid.is_some() && !state.is_loading && (state.is_playing || state.is_paused)
    })
}

#[cfg(target_os = "macos")]
struct NativePlaybackMenu<R: Runtime> {
    now_playing: MenuItem<R>,
    previous: MenuItem<R>,
    toggle: MenuItem<R>,
    next: MenuItem<R>,
    show: MenuItem<R>,
    quit: MenuItem<R>,
}

#[cfg(target_os = "macos")]
struct NativePlaybackMenuSnapshot {
    now_playing: String,
    previous: String,
    previous_enabled: bool,
    toggle: String,
    toggle_enabled: bool,
    next: String,
    next_enabled: bool,
    show: String,
    quit: String,
}

#[cfg(target_os = "macos")]
impl<R: Runtime> Clone for NativePlaybackMenu<R> {
    fn clone(&self) -> Self {
        Self {
            now_playing: self.now_playing.clone(),
            previous: self.previous.clone(),
            toggle: self.toggle.clone(),
            next: self.next.clone(),
            show: self.show.clone(),
            quit: self.quit.clone(),
        }
    }
}

#[cfg(target_os = "macos")]
fn native_playback_menu_snapshot<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: Option<&PlayerState>,
) -> NativePlaybackMenuSnapshot {
    let locale = app
        .try_state::<AppState>()
        .map(|state| state.preferences().locale)
        .unwrap_or_default();
    let state = snapshot.cloned().or_else(|| {
        app.try_state::<AppState>()
            .map(|state| state.player_snapshot())
    });
    let state = state.as_ref();
    NativePlaybackMenuSnapshot {
        now_playing: native_playback_summary(state, locale),
        previous: i18n::tr(locale, "desktop-menu-previous"),
        previous_enabled: state.is_some_and(|state| state.has_previous),
        toggle: native_playback_toggle_label(state, locale),
        toggle_enabled: native_playback_toggle_enabled(state),
        next: i18n::tr(locale, "desktop-menu-next"),
        next_enabled: state.is_some_and(|state| state.has_next),
        show: i18n::tr(locale, "desktop-menu-show"),
        quit: i18n::tr(locale, "desktop-menu-quit"),
    }
}

#[cfg(target_os = "macos")]
fn apply_native_playback_menu<R: Runtime>(
    menu: &NativePlaybackMenu<R>,
    snapshot: NativePlaybackMenuSnapshot,
) {
    let _ = menu.now_playing.set_text(snapshot.now_playing);
    let _ = menu.previous.set_text(snapshot.previous);
    let _ = menu.previous.set_enabled(snapshot.previous_enabled);
    let _ = menu.toggle.set_text(snapshot.toggle);
    let _ = menu.toggle.set_enabled(snapshot.toggle_enabled);
    let _ = menu.next.set_text(snapshot.next);
    let _ = menu.next.set_enabled(snapshot.next_enabled);
    let _ = menu.show.set_text(snapshot.show);
    let _ = menu.quit.set_text(snapshot.quit);
}

#[cfg(target_os = "macos")]
fn schedule_native_playback_menu_refresh<R: Runtime>(
    app: AppHandle<R>,
    menu: NativePlaybackMenu<R>,
) {
    let refresh_id = NATIVE_MENU_REFRESH_ID.fetch_add(1, Ordering::SeqCst) + 1;

    tauri::async_runtime::spawn_blocking(move || {
        let snapshot = native_playback_menu_snapshot(&app, None);
        if NATIVE_MENU_REFRESH_ID.load(Ordering::SeqCst) != refresh_id {
            return;
        }

        apply_native_playback_menu(&menu, snapshot);
    });
}

#[cfg(target_os = "macos")]
fn handle_native_playback_menu_event<R: Runtime>(
    app: &AppHandle<R>,
    menu_id: &str,
    menu: &NativePlaybackMenu<R>,
) {
    match menu_id {
        MENU_SHOW => {
            let _ = restore_main_window(app);
        }
        MENU_PREVIOUS => play_previous_from_lifecycle(app),
        MENU_TOGGLE_PLAYBACK => toggle_playback_from_lifecycle_async(app),
        MENU_NEXT => play_next_from_lifecycle(app),
        MENU_QUIT => request_real_quit(app),
        _ => {}
    }

    schedule_native_playback_menu_refresh(app.clone(), menu.clone());
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

#[cfg(target_os = "macos")]
fn toggle_playback_from_lifecycle_async<R: Runtime>(app: &AppHandle<R>) {
    let Some(state) = app
        .try_state::<AppState>()
        .map(|state| state.inner().clone())
    else {
        return;
    };

    tauri::async_runtime::spawn(async move {
        if let Err(error) = state.toggle_playback_from_lifecycle() {
            state.record_log(
                LogPayload::new(
                    LogLevel::Warn,
                    "desktop-lifecycle",
                    "desktop_lifecycle.toggle_playback_failed",
                    "Failed to toggle playback from menu bar",
                )
                .details(error),
            );
        }
    });
}

#[cfg(target_os = "macos")]
fn play_next_from_lifecycle<R: Runtime>(app: &AppHandle<R>) {
    let Some(state) = app
        .try_state::<AppState>()
        .map(|state| state.inner().clone())
    else {
        return;
    };

    tauri::async_runtime::spawn(async move {
        if let Err(error) = state.play_next_from_lifecycle().await {
            state.record_log(
                LogPayload::new(
                    LogLevel::Warn,
                    "desktop-lifecycle",
                    "desktop_lifecycle.next_track_failed",
                    "Failed to play next track from menu bar",
                )
                .details(error),
            );
        }
    });
}

#[cfg(target_os = "macos")]
fn play_previous_from_lifecycle<R: Runtime>(app: &AppHandle<R>) {
    let Some(state) = app
        .try_state::<AppState>()
        .map(|state| state.inner().clone())
    else {
        return;
    };

    tauri::async_runtime::spawn(async move {
        if let Err(error) = state.play_previous_from_lifecycle().await {
            state.record_log(
                LogPayload::new(
                    LogLevel::Warn,
                    "desktop-lifecycle",
                    "desktop_lifecycle.previous_track_failed",
                    "Failed to play previous track from menu bar",
                )
                .details(error),
            );
        }
    });
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

#[cfg(target_os = "macos")]
pub fn install_background_entrypoint<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{TrayIconBuilder, TrayIconEvent};
    use tauri::Listener;

    let now_playing = MenuItem::with_id(app, MENU_NOW_PLAYING, "Harubble", false, None::<&str>)?;
    let previous = MenuItem::with_id(app, MENU_PREVIOUS, "Previous", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, MENU_TOGGLE_PLAYBACK, "Play/Pause", true, None::<&str>)?;
    let next = MenuItem::with_id(app, MENU_NEXT, "Next", true, None::<&str>)?;
    let show = MenuItem::with_id(app, MENU_SHOW, "Show Harubble", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Harubble", true, None::<&str>)?;
    let controls = NativePlaybackMenu {
        now_playing,
        previous,
        toggle,
        next,
        show,
        quit,
    };
    schedule_native_playback_menu_refresh(app.clone(), controls.clone());

    let playback_separator = PredefinedMenuItem::separator(app)?;
    let app_separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &controls.now_playing,
            &playback_separator,
            &controls.previous,
            &controls.toggle,
            &controls.next,
            &app_separator,
            &controls.show,
            &controls.quit,
        ],
    )?;

    let controls_for_tray = controls.clone();
    let controls_for_menu = controls.clone();
    let controls_for_event = controls.clone();
    app.listen_any(crate::player::events::PLAYER_STATE_CHANGED, {
        let app = app.clone();
        move |_event| {
            schedule_native_playback_menu_refresh(app.clone(), controls_for_event.clone());
        }
    });

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tauri::include_image!("icons/32x32.png"))
        .icon_as_template(true)
        .tooltip("Harubble")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_tray_icon_event(move |tray, event| {
            if matches!(event, TrayIconEvent::Click { .. }) {
                schedule_native_playback_menu_refresh(
                    tray.app_handle().clone(),
                    controls_for_tray.clone(),
                );
            }
        })
        .on_menu_event(move |app, event| {
            handle_native_playback_menu_event(app, event.id().as_ref(), &controls_for_menu);
        })
        .build(app)?;

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
