use harubble::desktop_lifecycle::{
    should_close_to_background, should_close_to_background_with_state,
    should_install_background_entrypoint, DesktopLifecycleState,
};

#[test]
fn desktop_background_entrypoint_is_enabled_only_on_windows_and_macos() {
    assert!(should_install_background_entrypoint("windows"));
    assert!(should_install_background_entrypoint("macos"));
    assert!(!should_install_background_entrypoint("linux"));
    assert!(!should_install_background_entrypoint("freebsd"));
}

#[test]
fn close_to_background_is_enabled_only_for_main_window_on_supported_platforms() {
    assert!(should_close_to_background("main", "windows"));
    assert!(should_close_to_background("main", "macos"));
    assert!(!should_close_to_background("main", "linux"));
    assert!(!should_close_to_background("settings", "windows"));
    assert!(!should_close_to_background("", "macos"));
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
