//! Command 与后台入口调度域声明。
//!
//! Tauri command 的作用域、优先级和取消策略由 [`crate::command_registry`] 的同一
//! 条目生成；本模块只额外维护非 Tauri 的内部后台入口，并提供统一查询与架构守卫，
//! 避免播放相关入口被误接到普通 runtime / API client，或让普通后台任务反向占用
//! 播放资源域。

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

macro_rules! collect_command_specs {
    ( $( ($path:path, $name:literal, $domain:ident, $priority:ident, $cancel:ident) ),* $(,)? ) => {
        &[$(CommandSpec {
            name: $name,
            domain: CommandDomain::$domain,
            priority: CommandPriority::$priority,
            cancel_policy: CancelPolicy::$cancel,
        }),*]
    };
}

const TAURI_COMMAND_SPECS: &[CommandSpec] = crate::for_each_tauri_command!(collect_command_specs);

const INTERNAL_COMMAND_SPECS: &[CommandSpec] = &[
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
    TAURI_COMMAND_SPECS
        .iter()
        .chain(INTERNAL_COMMAND_SPECS)
        .find(|spec| spec.name == name)
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
    use super::{
        command_spec, CancelPolicy, CommandDomain, CommandPriority, INTERNAL_COMMAND_SPECS,
        TAURI_COMMAND_SPECS,
    };
    use std::collections::HashSet;

    macro_rules! extract_names {
        ( $( ($path:path, $name:literal, $domain:ident, $priority:ident, $cancel:ident) ),* $(,)? ) => {
            &[ $($name),* ]
        }
    }
    const ALL_TAURI_COMMAND_NAMES: &[&str] = crate::for_each_tauri_command!(extract_names);

    macro_rules! extract_paths_and_names {
        ( $( ($path:path, $name:literal, $domain:ident, $priority:ident, $cancel:ident) ),* $(,)? ) => {
            &[ $((stringify!($path), $name)),* ]
        }
    }
    const ALL_TAURI_COMMAND_PATHS_AND_NAMES: &[(&str, &str)] =
        crate::for_each_tauri_command!(extract_paths_and_names);

    const INTERNAL_SCHEDULED_ENTRIES: &[&str] = &[
        "download_execution_loop",
        "local_inventory_scan",
        "library_search_rebuild",
        "belong_warmup",
        "tag_registry_sync",
    ];

    #[test]
    fn command_registry_covers_all_tauri_commands() {
        let mut seen = HashSet::new();
        for spec in TAURI_COMMAND_SPECS.iter().chain(INTERNAL_COMMAND_SPECS) {
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

        for spec in TAURI_COMMAND_SPECS.iter().chain(INTERNAL_COMMAND_SPECS) {
            assert!(
                ALL_TAURI_COMMAND_NAMES.contains(&spec.name)
                    || INTERNAL_SCHEDULED_ENTRIES.contains(&spec.name),
                "scheduling spec {} is neither registered with Tauri nor declared as an internal entry",
                spec.name
            );
        }
    }

    #[test]
    fn command_registry_paths_match_declared_names() {
        for (path, name) in ALL_TAURI_COMMAND_PATHS_AND_NAMES {
            assert_eq!(
                path.split_whitespace().last(),
                Some(*name),
                "command handler path `{path}` does not match declared name `{name}`"
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
    fn playback_side_effect_commands_use_side_effect_domain() {
        for name in ["record_song_heat", "clear_listening_history"] {
            let spec = command_spec(name).expect("spec");
            assert_eq!(spec.domain, CommandDomain::PlaybackSideEffect);
            assert_eq!(spec.priority, CommandPriority::CriticalSideEffect);
            assert_eq!(spec.cancel_policy, CancelPolicy::Cooperative);
        }
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
