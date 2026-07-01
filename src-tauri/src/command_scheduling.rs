//! Tauri command 调度域声明。
//!
//! 该模块把 command 的作用域、优先级和取消策略显式化，避免播放相关入口在后续维护中
//! 被误接到普通 runtime / API client，或让普通后台任务反向占用播放资源域。

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CommandDomain {
    PlaybackControl,
    PlaybackTransition,
    PlaybackSideEffect,
    InteractiveUi,
    VisualAux,
    BackgroundIo,
    Maintenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CommandPriority {
    Playback,
    CriticalSideEffect,
    Interactive,
    Visual,
    Background,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CancelPolicy {
    NeverCancel,
    LatestWins,
    SupersedePlaybackSession,
    Cooperative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CommandSpec {
    pub(crate) name: &'static str,
    pub(crate) domain: CommandDomain,
    pub(crate) priority: CommandPriority,
    pub(crate) cancel_policy: CancelPolicy,
}

pub(crate) const COMMAND_SPECS: &[CommandSpec] = &[
    spec(
        "play_song",
        CommandDomain::PlaybackTransition,
        CommandPriority::Playback,
        CancelPolicy::SupersedePlaybackSession,
    ),
    spec(
        "seek_current_playback",
        CommandDomain::PlaybackTransition,
        CommandPriority::Playback,
        CancelPolicy::LatestWins,
    ),
    spec(
        "play_next",
        CommandDomain::PlaybackTransition,
        CommandPriority::Playback,
        CancelPolicy::SupersedePlaybackSession,
    ),
    spec(
        "play_previous",
        CommandDomain::PlaybackTransition,
        CommandPriority::Playback,
        CancelPolicy::SupersedePlaybackSession,
    ),
    spec(
        "pause_playback",
        CommandDomain::PlaybackControl,
        CommandPriority::Playback,
        CancelPolicy::NeverCancel,
    ),
    spec(
        "resume_playback",
        CommandDomain::PlaybackControl,
        CommandPriority::Playback,
        CancelPolicy::NeverCancel,
    ),
    spec(
        "get_player_state",
        CommandDomain::PlaybackControl,
        CommandPriority::Playback,
        CancelPolicy::NeverCancel,
    ),
    spec(
        "set_playback_volume",
        CommandDomain::PlaybackControl,
        CommandPriority::Playback,
        CancelPolicy::NeverCancel,
    ),
    spec(
        "get_albums",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_album_detail",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_song_detail",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "search_library",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_latest_albums",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_albums_by_series",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_recent_history",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_homepage_status",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "list_collections",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_collection",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "create_collection",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "update_collection",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "delete_collection",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "add_songs_to_collection",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "remove_songs_from_collection",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "reorder_collection_songs",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "export_collection",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "import_collection",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_tag_dimensions",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_albums_by_tag_dimension",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_tag_editor_merged",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_tag_editor_local_overlay",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "set_tag_editor_entity_tag",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "remove_tag_editor_entity_tag",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "add_tag_editor_dimension",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "remove_tag_editor_dimension",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "apply_tag_editor_remote_update",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "resolve_tag_editor_conflict",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "export_tag_editor_registry",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "import_tag_editor_registry",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_preferences",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "set_preferences",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "export_preferences",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "import_preferences",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_notification_permission_state",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "send_test_notification",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "show_main_window",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::NeverCancel,
    ),
    spec(
        "list_log_records",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_log_file_status",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_default_output_dir",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::NeverCancel,
    ),
    spec(
        "get_local_inventory_snapshot",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "rescan_local_inventory",
        CommandDomain::BackgroundIo,
        CommandPriority::Background,
        CancelPolicy::Cooperative,
    ),
    spec(
        "cancel_local_inventory_scan",
        CommandDomain::BackgroundIo,
        CommandPriority::Interactive,
        CancelPolicy::NeverCancel,
    ),
    spec(
        "get_audio_metadata",
        CommandDomain::BackgroundIo,
        CommandPriority::Background,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_song_lyrics",
        CommandDomain::VisualAux,
        CommandPriority::Visual,
        CancelPolicy::LatestWins,
    ),
    spec(
        "extract_image_theme",
        CommandDomain::VisualAux,
        CommandPriority::Visual,
        CancelPolicy::LatestWins,
    ),
    spec(
        "get_image_data_url",
        CommandDomain::VisualAux,
        CommandPriority::Visual,
        CancelPolicy::LatestWins,
    ),
    spec(
        "create_download_job",
        CommandDomain::BackgroundIo,
        CommandPriority::Background,
        CancelPolicy::Cooperative,
    ),
    spec(
        "list_download_jobs",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "get_download_job",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "cancel_download_job",
        CommandDomain::BackgroundIo,
        CommandPriority::Interactive,
        CancelPolicy::NeverCancel,
    ),
    spec(
        "cancel_download_task",
        CommandDomain::BackgroundIo,
        CommandPriority::Interactive,
        CancelPolicy::NeverCancel,
    ),
    spec(
        "retry_download_job",
        CommandDomain::BackgroundIo,
        CommandPriority::Background,
        CancelPolicy::Cooperative,
    ),
    spec(
        "retry_download_task",
        CommandDomain::BackgroundIo,
        CommandPriority::Background,
        CancelPolicy::Cooperative,
    ),
    spec(
        "clear_download_history",
        CommandDomain::BackgroundIo,
        CommandPriority::Background,
        CancelPolicy::Cooperative,
    ),
    spec(
        "clear_audio_cache",
        CommandDomain::Maintenance,
        CommandPriority::CriticalSideEffect,
        CancelPolicy::Cooperative,
    ),
    spec(
        "clear_response_cache",
        CommandDomain::Maintenance,
        CommandPriority::CriticalSideEffect,
        CancelPolicy::Cooperative,
    ),
    spec(
        "reset_http_client",
        CommandDomain::Maintenance,
        CommandPriority::CriticalSideEffect,
        CancelPolicy::Cooperative,
    ),
    spec(
        "clear_listening_history",
        CommandDomain::PlaybackSideEffect,
        CommandPriority::CriticalSideEffect,
        CancelPolicy::Cooperative,
    ),
];

const fn spec(
    name: &'static str,
    domain: CommandDomain,
    priority: CommandPriority,
    cancel_policy: CancelPolicy,
) -> CommandSpec {
    CommandSpec {
        name,
        domain,
        priority,
        cancel_policy,
    }
}

pub(crate) fn command_spec(name: &str) -> Option<&'static CommandSpec> {
    COMMAND_SPECS.iter().find(|spec| spec.name == name)
}

pub(crate) fn debug_assert_command_domain(name: &str, domain: CommandDomain) {
    debug_assert_eq!(
        command_spec(name).map(|spec| spec.domain),
        Some(domain),
        "command `{name}` is not registered in the expected scheduling domain"
    );
}

#[cfg(test)]
mod tests {
    use super::{command_spec, CancelPolicy, CommandDomain, CommandPriority, COMMAND_SPECS};
    use std::collections::HashSet;

    const REGISTERED_TAURI_COMMANDS: &[&str] = &[
        "list_collections",
        "get_collection",
        "create_collection",
        "update_collection",
        "delete_collection",
        "add_songs_to_collection",
        "remove_songs_from_collection",
        "reorder_collection_songs",
        "export_collection",
        "import_collection",
        "get_albums",
        "get_album_detail",
        "get_song_detail",
        "get_song_lyrics",
        "extract_image_theme",
        "get_image_data_url",
        "get_default_output_dir",
        "search_library",
        "play_song",
        "pause_playback",
        "resume_playback",
        "seek_current_playback",
        "play_next",
        "play_previous",
        "get_player_state",
        "set_playback_volume",
        "show_main_window",
        "get_preferences",
        "set_preferences",
        "export_preferences",
        "import_preferences",
        "get_local_inventory_snapshot",
        "rescan_local_inventory",
        "cancel_local_inventory_scan",
        "get_audio_metadata",
        "get_notification_permission_state",
        "send_test_notification",
        "list_log_records",
        "get_log_file_status",
        "clear_audio_cache",
        "clear_response_cache",
        "reset_http_client",
        "create_download_job",
        "list_download_jobs",
        "get_download_job",
        "cancel_download_job",
        "cancel_download_task",
        "retry_download_job",
        "retry_download_task",
        "clear_download_history",
        "get_latest_albums",
        "get_albums_by_series",
        "get_recent_history",
        "clear_listening_history",
        "get_homepage_status",
        "get_tag_dimensions",
        "get_albums_by_tag_dimension",
        "get_tag_editor_merged",
        "get_tag_editor_local_overlay",
        "set_tag_editor_entity_tag",
        "remove_tag_editor_entity_tag",
        "add_tag_editor_dimension",
        "remove_tag_editor_dimension",
        "apply_tag_editor_remote_update",
        "resolve_tag_editor_conflict",
        "export_tag_editor_registry",
        "import_tag_editor_registry",
    ];

    #[test]
    fn command_registry_covers_all_tauri_commands() {
        let mut seen = HashSet::new();
        for spec in COMMAND_SPECS {
            assert!(
                seen.insert(spec.name),
                "duplicate command spec {}",
                spec.name
            );
        }

        for command in REGISTERED_TAURI_COMMANDS {
            assert!(
                command_spec(command).is_some(),
                "missing scheduling spec for {command}"
            );
        }

        for spec in COMMAND_SPECS {
            assert!(
                REGISTERED_TAURI_COMMANDS.contains(&spec.name),
                "scheduling spec {} is not registered with Tauri",
                spec.name
            );
        }
    }

    #[test]
    fn playback_transition_commands_supersede_or_coalesce() {
        for name in [
            "play_song",
            "seek_current_playback",
            "play_next",
            "play_previous",
        ] {
            let spec = command_spec(name).expect("spec");
            assert_eq!(spec.domain, CommandDomain::PlaybackTransition);
            assert_eq!(spec.priority, CommandPriority::Playback);
            assert!(matches!(
                spec.cancel_policy,
                CancelPolicy::SupersedePlaybackSession | CancelPolicy::LatestWins
            ));
        }
    }

    #[test]
    fn visual_and_background_commands_do_not_enter_playback_priority() {
        for name in [
            "get_song_lyrics",
            "extract_image_theme",
            "get_image_data_url",
            "create_download_job",
            "retry_download_job",
            "rescan_local_inventory",
        ] {
            let spec = command_spec(name).expect("spec");
            assert_ne!(spec.priority, CommandPriority::Playback);
            assert_ne!(spec.domain, CommandDomain::PlaybackTransition);
            assert_ne!(spec.domain, CommandDomain::PlaybackControl);
        }
    }

    #[test]
    fn playback_resources_are_only_used_from_allowed_modules() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let playback_allowed = [
            "app_state/mod.rs",
            "app_state/playback.rs",
            "app_state/media_controls.rs",
            "commands/playback.rs",
            "network_monitor.rs",
            "player/controller.rs",
            "playback_load_gate.rs",
        ];

        assert_identifier_is_limited_to(&root, "playback_api", &playback_allowed);
        assert_identifier_is_limited_to(&root, "playback_runtime", &playback_allowed);

        let image_allowed = [
            "app_state/mod.rs",
            "commands/library.rs",
            "network_monitor.rs",
            "notification/cover.rs",
        ];
        assert_identifier_is_limited_to(&root, "image_api", &image_allowed);

        let download_allowed = [
            "app_state/mod.rs",
            "commands/downloads.rs",
            "downloads/bridge.rs",
            "network_monitor.rs",
        ];
        assert_identifier_is_limited_to(&root, "download_api", &download_allowed);
    }

    fn assert_identifier_is_limited_to(
        root: &std::path::Path,
        identifier: &str,
        allowed_relative_paths: &[&str],
    ) {
        for path in rust_files(root) {
            let relative = path.strip_prefix(root).expect("path under root");
            let relative = relative.to_string_lossy().replace('\\', "/");
            if relative == "command_scheduling.rs" {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("read source");
            if source.contains(identifier) && !allowed_relative_paths.contains(&relative.as_str()) {
                panic!(
                    "`{identifier}` is used from `{relative}`, which is outside the playback resource domain"
                );
            }
        }
    }

    fn rust_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut files = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(path) = stack.pop() {
            let entries = std::fs::read_dir(&path).expect("read dir");
            for entry in entries {
                let entry = entry.expect("dir entry");
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|extension| extension == "rs") {
                    files.push(path);
                }
            }
        }
        files
    }
}
