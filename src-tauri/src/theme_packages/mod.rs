//! 主题包（Theme Package）后端子系统。
//!
//! # 模块职责
//!
//! 该模块承载主题包系统的核心后端能力：编译期内置包注册、主题包文档模型、磁盘状态机、
//! 字段级校验与清洗、受限 URL 下载，以及组合这些能力的服务入口。激活主题包时的
//! preferences CAS 与跨窗口事件广播由 Tauri command 层协调；具体 command 定义在
//! [`crate::commands`] 与 [`crate::command_registry`] 中。
//!
//! # 子模块导航
//!
//! - [`builtin`]：编译期内置主题包注册表。
//! - [`types`]：主题包核心数据结构（`ThemePackageDocument` / `ThemePackageManifest` / `ThemePackageStatus`）。
//! - [`store`]：`PackageStore` 磁盘状态机（staging/committed/pending-delete + `<id>.sha256` sidecar）。
//! - [`sanitizer`]：导入与存量文档的 schema 校验、字段级 clamp、CSS 值 sanitize。
//! - [`downloader`]：HTTPS 下载、SSRF 防护、响应类型与大小限制。
//! - [`service`]：`ThemePackageService` 组合根，注入 `AppState`。
//!
//! # 稳定性
//!
//! 主题包 v1 契约通过 `src/lib/types.ts` 与 `ipc-contract.test.ts` 双向校验。
//! 新增或修改字段必须同步两端类型、sanitizer 与契约测试。

#![allow(dead_code)]

pub(crate) mod builtin;
pub(crate) mod downloader;
pub(crate) mod sanitizer;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod types;

// AppState 组合根与 Tauri command 边界均通过此处 re-export。

pub(crate) use service::ThemePackageService;
pub use types::{ThemePackageDocument, ThemePackageSummary};
// ThemePackageStatus 通过 ThemePackageSummary.status 字段间接暴露给前端；
// 若命令层未来需要独立返回状态枚举，取消下行注释。
// pub use types::ThemePackageStatus;
