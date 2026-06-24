use harubble::desktop_lifecycle::{
    dock_reopen_action, should_close_to_background, should_close_to_background_with_state,
    should_hide_on_focus_loss, should_install_background_entrypoint,
    should_install_mini_player_window, should_prevent_exit_after_window_close,
    DesktopLifecycleState, DockReopenAction, MINI_PLAYER_WINDOW_LABEL,
};

#[test]
fn desktop_background_entrypoint_is_enabled_only_on_windows_and_macos() {
    assert!(should_install_background_entrypoint("windows"));
    assert!(!should_install_background_entrypoint("macos"));
    assert!(!should_install_background_entrypoint("linux"));
    assert!(!should_install_background_entrypoint("freebsd"));
}

#[test]
fn close_to_background_is_enabled_only_for_main_window_on_supported_platforms() {
    assert!(should_close_to_background("main", "windows"));
    assert!(!should_close_to_background("main", "macos"));
    assert!(!should_close_to_background("main", "linux"));
    assert!(!should_close_to_background("settings", "windows"));
    assert!(!should_close_to_background("", "macos"));
}

#[test]
fn mini_player_window_is_installed_only_on_windows() {
    assert!(should_install_mini_player_window("windows"));
    assert!(!should_install_mini_player_window("macos"));
    assert!(!should_install_mini_player_window("linux"));
}

#[test]
fn mini_player_hides_on_focus_loss_only_where_it_is_installed() {
    assert!(should_hide_on_focus_loss(
        MINI_PLAYER_WINDOW_LABEL,
        "windows"
    ));
    assert!(!should_hide_on_focus_loss(
        MINI_PLAYER_WINDOW_LABEL,
        "macos"
    ));
    assert!(!should_hide_on_focus_loss(
        MINI_PLAYER_WINDOW_LABEL,
        "linux"
    ));
    assert!(!should_hide_on_focus_loss("main", "macos"));
}

#[test]
fn dock_reopen_uses_macos_window_visibility_and_focus() {
    assert_eq!(
        dock_reopen_action("macos", true, true),
        DockReopenAction::Ignore
    );
    assert_eq!(
        dock_reopen_action("macos", true, false),
        DockReopenAction::FocusVisibleWindow
    );
    assert_eq!(
        dock_reopen_action("macos", false, false),
        DockReopenAction::RestoreOrCreateWindow
    );
    assert_eq!(
        dock_reopen_action("windows", false, false),
        DockReopenAction::Ignore
    );
}

#[test]
fn lifecycle_state_marks_real_quit() {
    let state = DesktopLifecycleState::default();
    assert!(!state.is_quitting());
    state.mark_quitting();
    assert!(state.is_quitting());
}

#[test]
fn real_quit_disables_close_to_background() {
    let state = DesktopLifecycleState::default();
    assert!(should_close_to_background_with_state(
        "main",
        "windows",
        Some(&state)
    ));

    state.mark_quitting();

    assert!(!should_close_to_background_with_state(
        "main",
        "windows",
        Some(&state)
    ));
}

#[test]
fn macos_window_close_prevents_only_the_immediate_implicit_exit() {
    let state = DesktopLifecycleState::default();

    assert!(!should_prevent_exit_after_window_close(
        "macos",
        Some(&state)
    ));

    state.mark_main_window_close_requested();

    assert!(should_prevent_exit_after_window_close(
        "macos",
        Some(&state)
    ));
    assert!(!should_prevent_exit_after_window_close(
        "macos",
        Some(&state)
    ));

    state.mark_main_window_close_requested();
    state.mark_quitting();

    assert!(!should_prevent_exit_after_window_close(
        "macos",
        Some(&state)
    ));
    assert!(!should_prevent_exit_after_window_close(
        "windows",
        Some(&state)
    ));
}
