use crate::i18n::tr;
use crate::logging::{LogCenter, LogLevel, LogPayload};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 应用支持的界面语言
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Locale {
    #[default]
    #[serde(rename = "zh-CN")]
    ZhCN,
    #[serde(rename = "en-US")]
    EnUS,
}

/// 统一应用偏好模型（TOML 序列化格式：camelCase 字段名）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    /// 预留给未来迁移使用的模式版本；缺失时默认按 `1` 处理。
    #[serde(default)]
    pub(crate) schema_version: i32,
    pub(crate) output_format: String,
    pub(crate) output_dir: String,
    pub(crate) download_lyrics: bool,
    pub(crate) notify_on_download_complete: bool,
    pub(crate) notify_on_playback_change: bool,
    #[serde(default = "default_log_level")]
    pub(crate) log_level: String,
    #[serde(default)]
    pub(crate) locale: Locale,
}

impl AppPreferences {
    /// 验证偏好是否合法
    pub(crate) fn validate(&self, locale: Locale) -> Result<(), String> {
        match self.output_format.as_str() {
            "flac" | "wav" | "mp3" => {}
            _ => {
                let args = crate::i18n::fluent_args!("format" => self.output_format.clone());
                return Err(crate::i18n::tr_args(
                    locale,
                    "preferences-unsupported-format",
                    &args,
                ));
            }
        }
        if LogLevel::parse(&self.log_level).is_none() {
            let args = crate::i18n::fluent_args!("level" => self.log_level.clone());
            return Err(crate::i18n::tr_args(
                locale,
                "preferences-unsupported-log-level",
                &args,
            ));
        }
        let path = Path::new(&self.output_dir);
        if path.as_os_str().is_empty() || !path.is_absolute() {
            return Err(tr(locale, "preferences-output-dir-must-be-absolute"));
        }
        if !path.exists() {
            return Err(tr(locale, "preferences-output-dir-not-exists"));
        }
        ensure_not_symlink(path, &tr(locale, "preferences-output-dir-is-symlink"))?;
        if !path.is_dir() {
            return Err(tr(locale, "preferences-output-dir-not-directory"));
        }
        Ok(())
    }
}

fn default_log_level() -> String {
    LogLevel::Error.as_str().to_string()
}

fn validate_explicit_export_path(path: &Path, locale: Locale) -> Result<(), String> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err(tr(locale, "preferences-export-path-must-be-absolute"));
    }
    if path.exists() {
        ensure_not_symlink(path, &tr(locale, "preferences-export-path-is-symlink"))?;
    }
    if path.exists() && path.is_dir() {
        return Err(tr(locale, "preferences-export-path-is-directory"));
    }
    Ok(())
}

fn validate_explicit_import_path(path: &Path, locale: Locale) -> Result<(), String> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err(tr(locale, "preferences-import-path-must-be-absolute"));
    }
    if !path.exists() {
        return Err(tr(locale, "preferences-import-file-not-exists"));
    }
    ensure_not_symlink(path, &tr(locale, "preferences-import-path-is-symlink"))?;
    if !path.is_file() {
        return Err(tr(locale, "preferences-import-path-not-file"));
    }
    Ok(())
}

fn ensure_not_symlink(path: &Path, message: &str) -> Result<(), String> {
    if fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(message.to_string());
    }
    Ok(())
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            schema_version: 1,
            output_format: "flac".to_string(),
            output_dir: String::new(),
            download_lyrics: true,
            notify_on_download_complete: true,
            notify_on_playback_change: true,
            log_level: default_log_level(),
            locale: Locale::default(),
        }
    }
}

/// 偏好持久化管理器
#[derive(Clone)]
pub(crate) struct PreferencesStore {
    path: PathBuf,
}

impl PreferencesStore {
    pub(crate) fn new(app_data_dir: PathBuf) -> Self {
        let path = app_data_dir.join("preferences.toml");
        Self { path }
    }

    /// 从 TOML 文件加载偏好，缺失或损坏时用默认值初始化并写入。
    ///
    /// `load_locale` 强制使用 `Locale::default()`（`zh-CN`），因为在偏好文件解析成功前
    /// 无法得知用户实际选择的语言。这意味着文件损坏或缺失时的日志/错误消息始终使用中文。
    /// 这是一个有意的设计取舍——偏好加载是 locale 来源的上游，不可能自引用。
    pub(crate) fn load(&self, log_center: Option<&LogCenter>) -> AppPreferences {
        // 偏好文件尚未解析前不知道用户选择的语言，强制使用默认 locale
        let load_locale = Locale::default();
        if self.path.exists() {
            match fs::read_to_string(&self.path) {
                Ok(content) => match toml::from_str::<AppPreferences>(&content) {
                    Ok(prefs) => match prefs.validate(prefs.locale) {
                        Ok(()) => return prefs,
                        Err(error) => {
                            if let Some(log_center) = log_center {
                                log_center.record(
                                    LogPayload::new(
                                        LogLevel::Error,
                                        "preferences",
                                        "preferences.invalid_persisted",
                                        "Persisted preferences are invalid",
                                    )
                                    .user_message(tr(load_locale, "preferences-load-invalid"))
                                    .details(error.clone()),
                                );
                            }
                            eprintln!("[preferences] invalid persisted preferences: {error}");
                        }
                    },
                    Err(e) => {
                        if let Some(log_center) = log_center {
                            log_center.record(
                                LogPayload::new(
                                    LogLevel::Error,
                                    "preferences",
                                    "preferences.parse_failed",
                                    "Failed to parse persisted preferences",
                                )
                                .user_message(tr(load_locale, "preferences-load-corrupted"))
                                .details(e.to_string()),
                            );
                        }
                        eprintln!("[preferences] failed to parse TOML: {e}");
                    }
                },
                Err(e) => {
                    if let Some(log_center) = log_center {
                        log_center.record(
                            LogPayload::new(
                                LogLevel::Error,
                                "preferences",
                                "preferences.read_failed",
                                "Failed to read persisted preferences",
                            )
                            .user_message(tr(load_locale, "preferences-load-read-failed"))
                            .details(e.to_string()),
                        );
                    }
                    eprintln!("[preferences] failed to read file: {e}");
                }
            }
        }
        // 缺失或损坏时写入默认值（output_dir 使用下载目录兜底）
        let default_output_dir = dirs::download_dir()
            .unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"))
            })
            .join("Harubble");
        let resolved_output_dir = if fs::create_dir_all(&default_output_dir).is_ok() {
            default_output_dir
        } else {
            self.path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| std::path::PathBuf::from("/"))
        };
        let default_prefs = AppPreferences {
            schema_version: 1,
            output_format: "flac".to_string(),
            output_dir: resolved_output_dir.to_string_lossy().to_string(),
            download_lyrics: true,
            notify_on_download_complete: true,
            notify_on_playback_change: true,
            log_level: default_log_level(),
            locale: Locale::default(),
        };
        if let Err(e) = self.save(&default_prefs, load_locale) {
            if let Some(log_center) = log_center {
                log_center.record(
                    LogPayload::new(
                        LogLevel::Error,
                        "preferences",
                        "preferences.write_default_failed",
                        "Failed to write default preferences",
                    )
                    .details(e.clone()),
                );
            }
            eprintln!("[preferences] failed to write default preferences: {e}");
        }
        default_prefs
    }

    /// 原子写入偏好到 TOML 文件
    pub(crate) fn save(&self, prefs: &AppPreferences, locale: Locale) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| tr(locale, "preferences-dir-invalid"))?;
        fs::create_dir_all(parent).map_err(|_| tr(locale, "preferences-dir-create-failed"))?;
        let content = toml::to_string_pretty(prefs)
            .map_err(|e| format!("failed to serialize preferences: {e}"))?;

        let mut tmp = tempfile::NamedTempFile::new_in(parent)
            .map_err(|_| tr(locale, "preferences-file-write-failed"))?;
        std::io::Write::write_all(&mut tmp, content.as_bytes())
            .map_err(|_| tr(locale, "preferences-file-write-failed"))?;
        tmp.persist(&self.path)
            .map_err(|_| tr(locale, "preferences-file-write-failed"))?;
        Ok(())
    }

    /// 导出偏好到指定路径
    pub(crate) fn export_to(
        &self,
        prefs: &AppPreferences,
        path: &Path,
        locale: Locale,
    ) -> Result<(), String> {
        validate_explicit_export_path(path, locale)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|_| tr(locale, "preferences-export-dir-create-failed"))?;
            }
        }
        let content = toml::to_string_pretty(prefs)
            .map_err(|e| format!("failed to serialize preferences: {e}"))?;
        fs::write(path, content.as_bytes())
            .map_err(|_| tr(locale, "preferences-export-file-write-failed"))?;
        Ok(())
    }

    /// 从指定路径导入偏好（读取后验证）
    pub(crate) fn import_from(
        &self,
        path: &Path,
        locale: Locale,
    ) -> Result<AppPreferences, String> {
        validate_explicit_import_path(path, locale)?;
        let content = fs::read_to_string(path)
            .map_err(|_| tr(locale, "preferences-import-file-read-failed"))?;
        let prefs: AppPreferences =
            toml::from_str(&content).map_err(|e| format!("failed to parse TOML: {e}"))?;
        prefs.validate(locale)?;
        Ok(prefs)
    }
}
