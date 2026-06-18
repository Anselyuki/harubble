//! 网络配置变更监听器。
//!
//! 在 macOS 上通过 `SCDynamicStore` 监听系统代理与网络配置变化，
//! 变更发生时自动重建 HTTP 客户端以适应新的网络环境。
//! 其他平台当前不提供后台监听，依赖前端 `online` 事件触发重建。

use crate::app_state::AppState;

/// 启动网络配置变更监听后台任务。
///
/// 在 macOS 上会启动一个专用线程运行 CFRunLoop，监听系统代理与网络路由变化；
/// 检测到变更后自动调用 `reset_http_client` 重建 HTTP 客户端。
/// 其他平台为空实现。
pub fn spawn_network_monitor(state: &AppState) {
    #[cfg(target_os = "macos")]
    macos::spawn(state);

    #[cfg(not(target_os = "macos"))]
    let _ = state;
}

#[cfg(target_os = "macos")]
mod macos {
    use crate::logging::{LogLevel, LogPayload};

    use super::*;
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use system_configuration::core_foundation::array::CFArray;
    use system_configuration::core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
    use system_configuration::core_foundation::string::CFString;
    use system_configuration::dynamic_store::{
        SCDynamicStore, SCDynamicStoreBuilder, SCDynamicStoreCallBackContext,
    };

    const DEBOUNCE_DURATION: Duration = Duration::from_millis(500);
    const SETTLE_DELAY: Duration = Duration::from_millis(500);

    struct CallbackState {
        api: Arc<harubble_core::ApiClient>,
        log_center: Arc<crate::logging::LogCenter>,
        last_reset: std::sync::Mutex<Instant>,
    }

    fn on_network_change(
        _store: SCDynamicStore,
        _changed_keys: CFArray<CFString>,
        state: &mut CallbackState,
    ) {
        let now = Instant::now();
        {
            let mut last = state.last_reset.lock().unwrap_or_else(|e| e.into_inner());
            if now.duration_since(*last) < DEBOUNCE_DURATION {
                return;
            }
            *last = now;
        }

        // Delay to let the system finish applying new settings.
        // Increased from 200ms to 500ms to handle slower proxy configuration updates.
        std::thread::sleep(SETTLE_DELAY);

        match state.api.reset_http_client() {
            Ok(()) => {
                state.log_center.record(LogPayload::new(
                    LogLevel::Info,
                    "network",
                    "network.client_reset",
                    "网络配置变更，已重建 HTTP 客户端",
                ));
            }
            Err(e) => {
                state.log_center.record(
                    LogPayload::new(
                        LogLevel::Warn,
                        "network",
                        "network.reset_client_failed",
                        "网络配置变更后重建 HTTP 客户端失败",
                    )
                    .details(e.to_string()),
                );
            }
        }
    }

    pub(super) fn spawn(state: &AppState) {
        let api = state.api.clone();
        let log_center = state.log_center.clone();

        std::thread::Builder::new()
            .name("network-monitor".into())
            .spawn(move || {
                let callback_state = CallbackState {
                    api,
                    log_center,
                    last_reset: std::sync::Mutex::new(Instant::now() - DEBOUNCE_DURATION),
                };

                let context = SCDynamicStoreCallBackContext {
                    callout: on_network_change,
                    info: callback_state,
                };

                let store = SCDynamicStoreBuilder::new("harubble-network-monitor")
                    .callback_context(context)
                    .build();

                let Some(store) = store else {
                    return;
                };

                let watch_patterns = CFArray::from_CFTypes(&[
                    // Active network state (route, DNS, proxies)
                    CFString::from("State:/Network/Global/.*"),
                    // Proxy configuration changes (System Preferences / networksetup)
                    CFString::from("Setup:/Network/Global/Proxies"),
                    CFString::from("Setup:/Network/Service/.*/Proxies"),
                    // Interface state changes (link up/down, VPN connect/disconnect)
                    CFString::from("State:/Network/Interface/.*/Link"),
                    CFString::from("State:/Network/Interface/.*/IPv4"),
                    CFString::from("State:/Network/Interface/.*/IPv6"),
                ]);

                if !store.set_notification_keys(
                    &CFArray::from_CFTypes(&[] as &[CFString]),
                    &watch_patterns,
                ) {
                    return;
                }

                let Some(source) = store.create_run_loop_source() else {
                    return;
                };

                let run_loop = CFRunLoop::get_current();
                run_loop.add_source(&source, unsafe { kCFRunLoopCommonModes });
                CFRunLoop::run_current();
            })
            .ok();
    }
}
