//! 网络配置变更监听器。
//!
//! 在 macOS 上通过 `SCDynamicStore` 监听系统代理与网络配置变化，
//! 变更发生时自动重建 HTTP 客户端以适应新的网络环境。
//! 其他平台当前不提供后台监听，依赖前端 `online` 事件触发重建。

use crate::app_state::AppState;
use crate::logging::{LogLevel, LogPayload};

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
    use super::*;
    use system_configuration::core_foundation::array::CFArray;
    use system_configuration::core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
    use system_configuration::core_foundation::string::CFString;
    use system_configuration::dynamic_store::{
        SCDynamicStore, SCDynamicStoreBuilder, SCDynamicStoreCallBackContext,
    };

    struct CallbackState {
        api: std::sync::Arc<harubble_core::ApiClient>,
        log_center: std::sync::Arc<crate::logging::LogCenter>,
    }

    fn on_network_change(
        _store: SCDynamicStore,
        _changed_keys: CFArray<CFString>,
        state: &mut CallbackState,
    ) {
        if let Err(e) = state.api.reset_http_client() {
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

    pub(super) fn spawn(state: &AppState) {
        let api = state.api.clone();
        let log_center = state.log_center.clone();

        std::thread::Builder::new()
            .name("network-monitor".into())
            .spawn(move || {
                let callback_state = CallbackState { api, log_center };

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

                let watch_patterns =
                    CFArray::from_CFTypes(&[CFString::from("State:/Network/Global/.*")]);

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
