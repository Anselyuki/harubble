//! 主题包（Theme Package）后端子系统。
//!
//! # 模块职责
//!
//! 该模块承载主题包系统的核心后端能力：主题包文档模型、磁盘状态机存储、schema 校验、
//! CAS 版本比对与内容检查器（Inspector）的 LRU 缓存。前端通过 Tauri command 层
//! 访问这些能力，具体 command 定义在 [`crate::commands`] 与
//! [`crate::command_registry`] 中。
//!
//! # 子模块导航
//!
//! - [`types`]：主题包核心数据结构（`ThemePackageDocument` / `ThemePackageManifest` / `ThemePackageStatus`）。
//! - [`store`]：`PackageStore` 磁盘状态机（staging/committed/pending-delete + `.sha256.sidecar`）。
//! - [`sanitizer`]：`PackageSanitizer` schema 校验、字段级 clamp、CSS 值 sanitize。
//! - [`service`]：`ThemePackageService` 组合根，注入 `AppState`。
//!
//! # 稳定性
//!
//! 当前处于 Phase 1 MVP 阶段，公开 API 尚未冻结。前端契约通过
//! `src/lib/types.ts` 与 `ipc-contract.test.ts` 双向校验，禁止绕过。

#![allow(dead_code)]

pub(crate) mod downloader;
pub(crate) mod sanitizer;
pub(crate) mod service;
pub(crate) mod store;
pub(crate) mod types;

// Phase 1 Step 1.c 起 command 层会消费这些类型；AppState 组合根 + Tauri command 边界均通过此处 re-export。

pub(crate) use service::ThemePackageService;
pub use types::{ThemePackageDocument, ThemePackageSummary};
// ThemePackageStatus 通过 ThemePackageSummary.status 字段间接暴露给前端；
// 若命令层未来需要独立返回状态枚举，取消下行注释。
// pub use types::ThemePackageStatus;
