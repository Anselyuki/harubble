//! Command 与后台入口调度域声明。
//!
//! 该模块把 Tauri command 和内部后台入口的作用域、优先级和取消策略显式化，避免播放相关入口在后续维护中
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

impl CommandDomain {
    pub(crate) const fn as_label(self) -> &'static str {
        match self {
            Self::PlaybackControl => "PlaybackControl",
            Self::PlaybackTransition => "PlaybackTransition",
            Self::PlaybackSideEffect => "PlaybackSideEffect",
            Self::InteractiveUi => "InteractiveUi",
            Self::VisualAux => "VisualAux",
            Self::BackgroundIo => "BackgroundIo",
            Self::Maintenance => "Maintenance",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CommandPriority {
    Playback,
    CriticalSideEffect,
    Interactive,
    Visual,
    Background,
}

impl CommandPriority {
    pub(crate) const fn as_label(self) -> &'static str {
        match self {
            Self::Playback => "Playback",
            Self::CriticalSideEffect => "CriticalSideEffect",
            Self::Interactive => "Interactive",
            Self::Visual => "Visual",
            Self::Background => "Background",
        }
    }
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
        "get_album_catalog",
        CommandDomain::InteractiveUi,
        CommandPriority::Interactive,
        CancelPolicy::Cooperative,
    ),
    spec(
        "refresh_album_catalog",
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
        "refresh_album_detail",
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
        "sync_playback_menu_state",
        CommandDomain::VisualAux,
        CommandPriority::Visual,
        CancelPolicy::LatestWins,
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
        "get_cached_image_path",
        CommandDomain::VisualAux,
        CommandPriority::Visual,
        CancelPolicy::Cooperative,
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
    spec(
        "record_song_heat",
        CommandDomain::PlaybackSideEffect,
        CommandPriority::CriticalSideEffect,
        CancelPolicy::Cooperative,
    ),
    spec(
        "record_listening_history",
        CommandDomain::PlaybackSideEffect,
        CommandPriority::CriticalSideEffect,
        CancelPolicy::Cooperative,
    ),
    spec(
        "download_execution_loop",
        CommandDomain::BackgroundIo,
        CommandPriority::Background,
        CancelPolicy::Cooperative,
    ),
    spec(
        "local_inventory_scan",
        CommandDomain::BackgroundIo,
        CommandPriority::Background,
        CancelPolicy::Cooperative,
    ),
    spec(
        "library_search_rebuild",
        CommandDomain::BackgroundIo,
        CommandPriority::Background,
        CancelPolicy::Cooperative,
    ),
    spec(
        "belong_warmup",
        CommandDomain::BackgroundIo,
        CommandPriority::Background,
        CancelPolicy::Cooperative,
    ),
    spec(
        "tag_registry_sync",
        CommandDomain::BackgroundIo,
        CommandPriority::Background,
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

    macro_rules! extract_names {
        ( $( ($path:path, $name:literal, $domain:ident, $priority:ident, $cancel:ident) ),* $(,)? ) => {
            &[ $($name),* ]
        }
    }
    const ALL_TAURI_COMMAND_NAMES: &[&str] = crate::for_each_tauri_command!(extract_names);

    const INTERNAL_SCHEDULED_ENTRIES: &[&str] = &[
        "record_listening_history",
        "download_execution_loop",
        "local_inventory_scan",
        "library_search_rebuild",
        "belong_warmup",
        "tag_registry_sync",
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

        for command in ALL_TAURI_COMMAND_NAMES {
            assert!(
                command_spec(command).is_some(),
                "missing scheduling spec for {command}"
            );
        }

        for spec in COMMAND_SPECS {
            assert!(
                ALL_TAURI_COMMAND_NAMES.contains(&spec.name)
                    || INTERNAL_SCHEDULED_ENTRIES.contains(&spec.name),
                "scheduling spec {} is neither registered with Tauri nor declared as an internal entry",
                spec.name
            );
        }
    }

    #[test]
    fn internal_scheduled_entries_have_specs() {
        for entry in INTERNAL_SCHEDULED_ENTRIES {
            assert!(
                command_spec(entry).is_some(),
                "missing scheduling spec for internal entry {entry}"
            );
        }
    }

    #[test]
    fn command_metric_labels_are_stable() {
        assert_eq!(
            CommandDomain::PlaybackTransition.as_label(),
            "PlaybackTransition"
        );
        assert_eq!(
            CommandDomain::PlaybackSideEffect.as_label(),
            "PlaybackSideEffect"
        );
        assert_eq!(CommandPriority::Playback.as_label(), "Playback");
        assert_eq!(
            CommandPriority::CriticalSideEffect.as_label(),
            "CriticalSideEffect"
        );
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
    fn playback_side_effect_entries_use_side_effect_domain() {
        let spec = command_spec("record_listening_history").expect("spec");
        assert_eq!(spec.domain, CommandDomain::PlaybackSideEffect);
        assert_eq!(spec.priority, CommandPriority::CriticalSideEffect);
        assert_eq!(spec.cancel_policy, CancelPolicy::Cooperative);
    }

    #[test]
    fn background_internal_entries_use_background_domain() {
        for name in [
            "download_execution_loop",
            "local_inventory_scan",
            "library_search_rebuild",
            "belong_warmup",
            "tag_registry_sync",
        ] {
            let spec = command_spec(name).expect("spec");
            assert_eq!(spec.domain, CommandDomain::BackgroundIo);
            assert_eq!(spec.priority, CommandPriority::Background);
            assert_eq!(spec.cancel_policy, CancelPolicy::Cooperative);
        }
    }

    #[test]
    fn visual_and_background_commands_do_not_enter_playback_priority() {
        for name in [
            "get_song_lyrics",
            "extract_image_theme",
            "get_cached_image_path",
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
    fn cached_image_requests_do_not_supersede_each_other() {
        let spec = command_spec("get_cached_image_path").expect("spec");
        assert_eq!(spec.domain, CommandDomain::VisualAux);
        assert_eq!(spec.priority, CommandPriority::Visual);
        assert_eq!(spec.cancel_policy, CancelPolicy::Cooperative);
    }

    #[test]
    fn playback_resources_are_only_used_from_allowed_modules() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let playback_allowed = [
            "app_state/mod.rs",
            "app_state/playback.rs",
            "app_state/media_controls.rs",
            "app_state/api_clients.rs",
            "commands/playback.rs",
            "network_monitor.rs",
            "playback_actor.rs",
            "player/controller.rs",
            "playback_load_gate.rs",
        ];

        assert_identifier_is_limited_to(&root, "playback_api", &playback_allowed);
        assert_identifier_is_limited_to(&root, "playback_runtime", &playback_allowed);
        assert_identifier_is_limited_to(&root, "PlaybackActor", &playback_allowed);

        let image_allowed = [
            "app_state/mod.rs",
            "app_state/api_clients.rs",
            "commands/library.rs",
            "network_monitor.rs",
            "notification/cover.rs",
        ];
        assert_identifier_is_limited_to(&root, "image_api", &image_allowed);

        let download_allowed = [
            "app_state/mod.rs",
            "app_state/api_clients.rs",
            "commands/downloads.rs",
            "downloads/bridge.rs",
            "network_monitor.rs",
        ];
        assert_identifier_is_limited_to(&root, "download_api", &download_allowed);
    }

    /// 阻止 command 模块直接访问 AppState 的领域字段。
    ///
    /// command 层应通过窄化 accessor 方法访问领域服务（如 state.player() 而非 state.player），
    /// 以便未来可以拆分为独立 Tauri State 或按领域建立 Facade。app_state 内部与 playback / downloads
    /// 桥接层的直接字段访问是允许的，不受此测试限制。
    #[test]
    fn command_layer_uses_appstate_accessors() {
        let commands_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("commands");

        // 禁止在 commands/*.rs 中出现的直接字段访问模式。
        // 这里只列窄化后本应通过 accessor 访问的领域字段；
        // preferences / prefs 等仍通过 method 面向外部访问。
        //
        // 注意：所有模式统一以 `.` 结尾，这样只会匹配直接字段访问链
        // `state.foo.bar`，不会误判为同名 accessor 方法调用 `state.foo()`。
        // 去除所有空白字符后，`state.player.pause()` 变为 `state.player.pause()`
        // 而 `state.player().pause()` 变为 `state.player().pause()`，二者不同，
        // 因此增加 `state.player.` 不会误判 accessor 方法调用。
        let forbidden_patterns = [
            "state.api_clients.",
            "state.download.download_",
            "state.tag_registry.",
            "state.tag_editor.",
            "state.collection.",
            "state.library_search_service.",
            "state.listening_history.",
            "state.album_metadata_cache.",
            "state.local_inventory_service.",
            "state.log_center.",
            "state.player.",
        ];

        for path in rust_files(&commands_dir) {
            let relative = path
                .strip_prefix(&commands_dir)
                .expect("path under commands");
            let source = std::fs::read_to_string(&path).expect("read source");
            // 去除所有空白字符，使多行字段访问链（如 `state\n    .api_clients\n    .image_api`）
            // 也能被 `state.api_clients.` 模式匹配到。
            let compact: String = source.chars().filter(|c| !c.is_whitespace()).collect();
            for pattern in &forbidden_patterns {
                if compact.contains(pattern) {
                    panic!(
                        "command file `commands/{}` contains direct AppState field access `{}`; \
                         use the corresponding accessor method (e.g. state.player() instead of state.player)",
                        relative.display(),
                        pattern
                    );
                }
            }
        }
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
