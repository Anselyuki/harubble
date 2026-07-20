//! 桌面应用菜单栏骨架。
//!
//! 这个模块负责在应用启动时把一份系统默认的菜单挂到 Tauri 运行时上。
//! 在 macOS 上，Tauri 会自动把菜单挂载到系统顶部 NSMenu；在 Windows / Linux
//! 上则表现为窗口内的传统菜单。
//!
//! 菜单分为两类项：
//!
//! - **预定义系统项**：走 Tauri 的 `PredefinedMenuItem`，例如 About / Quit /
//!   Copy / Minimize，标签由 AppKit 按 Bundle 声明的本地化渲染，本模块不干预。
//! - **应用自定义项**：使用 `MenuItemBuilder::with_id(...)`，稳定 ID 定义在
//!   [`MENU_COMMAND_EVENT`] 事件的 `id` 字段中；点击时后端不直接执行动作，而是
//!   通过 [`MENU_COMMAND_EVENT`] 事件把 ID 广播到前端，由前端在
//!   `appRuntime` 的统一监听器里 dispatch 到对应 controller。
//!
//! 菜单标题走 Fluent i18n（`appmenu-*` 键），因此偏好中的 `locale` 变化后应重新
//! 调用 [`install`] 以刷新标签；预定义项不会随此改变，仍由系统渲染。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::menu::{
    CheckMenuItem, CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Wry};

use crate::i18n::tr;
use crate::preferences::Locale;

static MENU_EVENT_INSTALLED: AtomicBool = AtomicBool::new(false);

/// 需要跟随播放器状态刷新勾选态的菜单项句柄。
///
/// 每次调用 [`install`] 都会重建整个菜单，这些引用会随之更新；
/// [`sync_playback_menu_state`] 从此处读取当前引用后调用 `set_checked`。
struct PlaybackMenuHandles {
    repeat_off: CheckMenuItem<Wry>,
    repeat_all: CheckMenuItem<Wry>,
    repeat_one: CheckMenuItem<Wry>,
    shuffle: CheckMenuItem<Wry>,
}

static PLAYBACK_HANDLES: Mutex<Option<PlaybackMenuHandles>> = Mutex::new(None);

/// 自定义菜单项点击后广播给前端的事件名。
///
/// 载荷是 [`MenuCommandPayload`]，前端应在唯一入口订阅一次，避免多组件重复监听。
pub const MENU_COMMAND_EVENT: &str = "app-menu-command";

/// [`MENU_COMMAND_EVENT`] 事件载荷。
///
/// 只包含菜单项的稳定 ID，前端按 ID 查找 dispatch 表；参数化命令后续再扩展。
#[derive(Debug, Clone, serde::Serialize)]
pub struct MenuCommandPayload {
    pub id: String,
}

/// 稳定菜单 ID 命名空间：`app.<group>.<action>`。
mod ids {
    pub const PREFERENCES: &str = "app.app.preferences";
    pub const TEST_NOTIFICATION: &str = "app.app.test_notification";

    pub const FILE_NEW_COLLECTION: &str = "app.file.new_collection";
    pub const FILE_IMPORT_COLLECTION: &str = "app.file.import_collection";
    pub const FILE_EXPORT_COLLECTION: &str = "app.file.export_collection";
    pub const FILE_IMPORT_TAG_REGISTRY: &str = "app.file.import_tag_registry";
    pub const FILE_EXPORT_TAG_REGISTRY: &str = "app.file.export_tag_registry";
    pub const FILE_CLEAR_LISTENING_HISTORY: &str = "app.file.clear_listening_history";
    pub const FILE_CLEAR_DOWNLOAD_HISTORY: &str = "app.file.clear_download_history";
    pub const FILE_IMPORT_PREFERENCES: &str = "app.file.import_preferences";
    pub const FILE_EXPORT_PREFERENCES: &str = "app.file.export_preferences";

    pub const VIEW_LOGS: &str = "app.view.logs";
    pub const VIEW_RESCAN_INVENTORY: &str = "app.view.rescan_inventory";

    pub const VIEW_HOME: &str = "app.view.home";
    pub const VIEW_SEARCH: &str = "app.view.search";
    pub const VIEW_OVERVIEW: &str = "app.view.overview";
    pub const VIEW_LIBRARY: &str = "app.view.library";
    pub const VIEW_COLLECTION: &str = "app.view.collection";
    pub const VIEW_TAG_EDITOR: &str = "app.view.tag_editor";
    pub const VIEW_GO_BACK: &str = "app.view.go_back";
    pub const VIEW_TOGGLE_SIDEBAR: &str = "app.view.toggle_sidebar";
    pub const VIEW_TOGGLE_DOWNLOADS: &str = "app.view.toggle_downloads";
    pub const VIEW_REFRESH: &str = "app.view.refresh";
    pub const VIEW_APPEARANCE_AUTO: &str = "app.view.appearance.auto";
    pub const VIEW_APPEARANCE_LIGHT: &str = "app.view.appearance.light";
    pub const VIEW_APPEARANCE_DARK: &str = "app.view.appearance.dark";

    pub const PLAYBACK_TOGGLE: &str = "app.playback.toggle";
    pub const PLAYBACK_NEXT: &str = "app.playback.next";
    pub const PLAYBACK_PREVIOUS: &str = "app.playback.previous";
    pub const PLAYBACK_SEEK_FORWARD: &str = "app.playback.seek_forward";
    pub const PLAYBACK_SEEK_BACKWARD: &str = "app.playback.seek_backward";
    pub const PLAYBACK_VOLUME_UP: &str = "app.playback.volume_up";
    pub const PLAYBACK_VOLUME_DOWN: &str = "app.playback.volume_down";
    pub const PLAYBACK_TOGGLE_MUTE: &str = "app.playback.toggle_mute";
    pub const PLAYBACK_TOGGLE_SHUFFLE: &str = "app.playback.toggle_shuffle";
    pub const PLAYBACK_REPEAT_OFF: &str = "app.playback.repeat.off";
    pub const PLAYBACK_REPEAT_ALL: &str = "app.playback.repeat.all";
    pub const PLAYBACK_REPEAT_ONE: &str = "app.playback.repeat.one";
    pub const PLAYBACK_TOGGLE_LYRICS: &str = "app.playback.toggle_lyrics";
    pub const PLAYBACK_TOGGLE_PLAYLIST: &str = "app.playback.toggle_playlist";
    pub const PLAYBACK_TOGGLE_FULLSCREEN: &str = "app.playback.toggle_fullscreen";
}

/// 按给定语言重新构建并挂载菜单。
///
/// 首次调用应发生在 `tauri::Builder::setup` 中，偏好里的 `locale` 变化后应再次调用
/// 以刷新自定义项标题。重复调用会替换现有菜单。macOS 上首个子菜单被系统当作应用
/// 菜单，其显示名由 Info.plist 决定，因此传入的 `appmenu-app` 文案仅用于占位，
/// 实际不会显示。
///
/// # 错误
///
/// 当底层菜单构建失败时返回 [`tauri::Error`]；调用方应把错误写入应用日志，
/// 但不必因此中断启动流程 —— 菜单缺失不影响其余功能。
pub fn install(app: &AppHandle<Wry>, locale: Locale) -> tauri::Result<()> {
    let (menu, handles) = build(app, locale)?;
    app.set_menu(menu)?;
    if let Ok(mut slot) = PLAYBACK_HANDLES.lock() {
        *slot = Some(handles);
    }
    if !MENU_EVENT_INSTALLED.swap(true, Ordering::AcqRel) {
        let handle = app.clone();
        app.on_menu_event(move |_, event| {
            let id = event.id().as_ref().to_string();
            if !id.starts_with("app.") {
                return;
            }
            if let Err(error) = handle.emit(MENU_COMMAND_EVENT, MenuCommandPayload { id }) {
                eprintln!("[menu] failed to emit menu command: {error}");
            }
        });
    }
    Ok(())
}

/// 按当前播放器状态刷新循环 / 随机的勾选态。
///
/// 前端在 `player.repeatMode` / `player.shuffleEnabled` 变化时调用；
/// 若菜单尚未安装或 UI 线程调用 `set_checked` 失败，仅返回 `Ok(())`，不阻断上层。
pub fn sync_playback_state(repeat_mode: &str, shuffle_enabled: bool) -> tauri::Result<()> {
    let slot = PLAYBACK_HANDLES.lock().map_err(|e| {
        tauri::Error::Anyhow(anyhow::anyhow!("menu playback handles mutex poisoned: {e}"))
    })?;
    let Some(handles) = slot.as_ref() else {
        return Ok(());
    };
    let repeat_off_checked = repeat_mode == "off";
    let repeat_all_checked = repeat_mode == "all";
    let repeat_one_checked = repeat_mode == "one";
    handles.repeat_off.set_checked(repeat_off_checked)?;
    handles.repeat_all.set_checked(repeat_all_checked)?;
    handles.repeat_one.set_checked(repeat_one_checked)?;
    handles.shuffle.set_checked(shuffle_enabled)?;
    Ok(())
}

fn build(app: &AppHandle<Wry>, locale: Locale) -> tauri::Result<(Menu<Wry>, PlaybackMenuHandles)> {
    let app_submenu = SubmenuBuilder::new(app, tr(locale, "appmenu-app"))
        .about(None)
        .separator()
        .item(
            &MenuItemBuilder::with_id(ids::PREFERENCES, tr(locale, "appmenu-app-preferences"))
                .accelerator("CmdOrCtrl+,")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::TEST_NOTIFICATION,
                tr(locale, "appmenu-app-test-notification"),
            )
            .build(app)?,
        )
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let file_submenu = SubmenuBuilder::new(app, tr(locale, "appmenu-file"))
        .item(
            &MenuItemBuilder::with_id(
                ids::FILE_NEW_COLLECTION,
                tr(locale, "appmenu-file-new-collection"),
            )
            .accelerator("CmdOrCtrl+N")
            .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id(
                ids::FILE_IMPORT_COLLECTION,
                tr(locale, "appmenu-file-import-collection"),
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::FILE_EXPORT_COLLECTION,
                tr(locale, "appmenu-file-export-collection"),
            )
            .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id(
                ids::FILE_IMPORT_TAG_REGISTRY,
                tr(locale, "appmenu-file-import-tag-registry"),
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::FILE_EXPORT_TAG_REGISTRY,
                tr(locale, "appmenu-file-export-tag-registry"),
            )
            .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id(
                ids::FILE_IMPORT_PREFERENCES,
                tr(locale, "appmenu-file-import-preferences"),
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::FILE_EXPORT_PREFERENCES,
                tr(locale, "appmenu-file-export-preferences"),
            )
            .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id(
                ids::FILE_CLEAR_LISTENING_HISTORY,
                tr(locale, "appmenu-file-clear-listening-history"),
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::FILE_CLEAR_DOWNLOAD_HISTORY,
                tr(locale, "appmenu-file-clear-download-history"),
            )
            .build(app)?,
        )
        .build()?;

    let edit_submenu = SubmenuBuilder::new(app, tr(locale, "appmenu-edit"))
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let repeat_off_item = CheckMenuItemBuilder::with_id(
        ids::PLAYBACK_REPEAT_OFF,
        tr(locale, "appmenu-playback-repeat-off"),
    )
    .build(app)?;
    let repeat_all_item = CheckMenuItemBuilder::with_id(
        ids::PLAYBACK_REPEAT_ALL,
        tr(locale, "appmenu-playback-repeat-all"),
    )
    .build(app)?;
    let repeat_one_item = CheckMenuItemBuilder::with_id(
        ids::PLAYBACK_REPEAT_ONE,
        tr(locale, "appmenu-playback-repeat-one"),
    )
    .build(app)?;
    let repeat_submenu = SubmenuBuilder::new(app, tr(locale, "appmenu-playback-repeat"))
        .item(&repeat_off_item)
        .item(&repeat_all_item)
        .item(&repeat_one_item)
        .build()?;

    let shuffle_item = CheckMenuItemBuilder::with_id(
        ids::PLAYBACK_TOGGLE_SHUFFLE,
        tr(locale, "appmenu-playback-toggle-shuffle"),
    )
    .build(app)?;

    let playback_submenu = SubmenuBuilder::new(app, tr(locale, "appmenu-playback"))
        .item(
            &MenuItemBuilder::with_id(ids::PLAYBACK_TOGGLE, tr(locale, "appmenu-playback-toggle"))
                .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id(ids::PLAYBACK_NEXT, tr(locale, "appmenu-playback-next"))
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::PLAYBACK_PREVIOUS,
                tr(locale, "appmenu-playback-previous"),
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::PLAYBACK_SEEK_FORWARD,
                tr(locale, "appmenu-playback-seek-forward"),
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::PLAYBACK_SEEK_BACKWARD,
                tr(locale, "appmenu-playback-seek-backward"),
            )
            .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id(
                ids::PLAYBACK_VOLUME_UP,
                tr(locale, "appmenu-playback-volume-up"),
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::PLAYBACK_VOLUME_DOWN,
                tr(locale, "appmenu-playback-volume-down"),
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::PLAYBACK_TOGGLE_MUTE,
                tr(locale, "appmenu-playback-toggle-mute"),
            )
            .build(app)?,
        )
        .separator()
        .item(&repeat_submenu)
        .item(&shuffle_item)
        .separator()
        .item(
            &MenuItemBuilder::with_id(
                ids::PLAYBACK_TOGGLE_LYRICS,
                tr(locale, "appmenu-playback-toggle-lyrics"),
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::PLAYBACK_TOGGLE_PLAYLIST,
                tr(locale, "appmenu-playback-toggle-playlist"),
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::PLAYBACK_TOGGLE_FULLSCREEN,
                tr(locale, "appmenu-playback-toggle-fullscreen"),
            )
            .accelerator("Ctrl+CmdOrCtrl+F")
            .build(app)?,
        )
        .build()?;

    let appearance_submenu = SubmenuBuilder::new(app, tr(locale, "appmenu-view-appearance"))
        .item(
            &MenuItemBuilder::with_id(
                ids::VIEW_APPEARANCE_AUTO,
                tr(locale, "appmenu-view-appearance-auto"),
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::VIEW_APPEARANCE_LIGHT,
                tr(locale, "appmenu-view-appearance-light"),
            )
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::VIEW_APPEARANCE_DARK,
                tr(locale, "appmenu-view-appearance-dark"),
            )
            .build(app)?,
        )
        .build()?;

    let view_submenu = SubmenuBuilder::new(app, tr(locale, "appmenu-view"))
        .item(
            &MenuItemBuilder::with_id(ids::VIEW_HOME, tr(locale, "appmenu-view-home"))
                .accelerator("CmdOrCtrl+1")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(ids::VIEW_SEARCH, tr(locale, "appmenu-view-search"))
                .accelerator("CmdOrCtrl+2")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(ids::VIEW_OVERVIEW, tr(locale, "appmenu-view-overview"))
                .accelerator("CmdOrCtrl+3")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(ids::VIEW_LIBRARY, tr(locale, "appmenu-view-library"))
                .accelerator("CmdOrCtrl+4")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(ids::VIEW_COLLECTION, tr(locale, "appmenu-view-collection"))
                .accelerator("CmdOrCtrl+5")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(ids::VIEW_TAG_EDITOR, tr(locale, "appmenu-view-tag-editor"))
                .accelerator("CmdOrCtrl+6")
                .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id(ids::VIEW_GO_BACK, tr(locale, "appmenu-view-go-back"))
                .accelerator("CmdOrCtrl+[")
                .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id(
                ids::VIEW_TOGGLE_SIDEBAR,
                tr(locale, "appmenu-view-toggle-sidebar"),
            )
            .accelerator("Ctrl+CmdOrCtrl+S")
            .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::VIEW_TOGGLE_DOWNLOADS,
                tr(locale, "appmenu-view-toggle-downloads"),
            )
            .accelerator("Shift+CmdOrCtrl+D")
            .build(app)?,
        )
        .separator()
        .item(&appearance_submenu)
        .separator()
        .item(
            &MenuItemBuilder::with_id(ids::VIEW_REFRESH, tr(locale, "appmenu-view-refresh"))
                .accelerator("CmdOrCtrl+R")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::with_id(
                ids::VIEW_RESCAN_INVENTORY,
                tr(locale, "appmenu-view-rescan-inventory"),
            )
            .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::with_id(ids::VIEW_LOGS, tr(locale, "appmenu-view-logs"))
                .build(app)?,
        )
        .build()?;

    let window_submenu = SubmenuBuilder::new(app, tr(locale, "appmenu-window"))
        .minimize()
        .maximize()
        .separator()
        .close_window()
        .build()?;

    let menu = MenuBuilder::new(app)
        .items(&[
            &app_submenu,
            &file_submenu,
            &edit_submenu,
            &view_submenu,
            &playback_submenu,
            &window_submenu,
        ])
        .build()?;

    Ok((
        menu,
        PlaybackMenuHandles {
            repeat_off: repeat_off_item,
            repeat_all: repeat_all_item,
            repeat_one: repeat_one_item,
            shuffle: shuffle_item,
        },
    ))
}
