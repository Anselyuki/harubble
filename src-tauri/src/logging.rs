use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

const APP_ERROR_RECORDED: &str = "app-error-recorded";
const SESSION_LOG_PREFIX: &str = "session-";
const SESSION_LOG_SUFFIX: &str = ".jsonl";
const PERSISTENT_LOG_FILE: &str = "persistent.jsonl";

/// 结构化日志等级。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warn" => Some(Self::Warn),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

impl PartialOrd for LogLevel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LogLevel {
    fn cmp(&self, other: &Self) -> Ordering {
        let rank = |level: LogLevel| match level {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
        };
        rank(*self).cmp(&rank(*other))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum FrontendExposure {
    None,
    EventOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LogRecord {
    pub(crate) id: String,
    pub(crate) ts: String,
    pub(crate) level: LogLevel,
    pub(crate) domain: String,
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) user_message: Option<String>,
    pub(crate) details: Option<String>,
    pub(crate) context: Option<Value>,
    pub(crate) frontend_exposure: FrontendExposure,
    pub(crate) cause_chain: Vec<String>,
    pub(crate) session_id: String,
    pub(crate) source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppErrorEvent {
    pub(crate) id: String,
    pub(crate) ts: String,
    pub(crate) level: LogLevel,
    pub(crate) domain: String,
    pub(crate) code: String,
    pub(crate) message: String,
}

/// 日志查看器要读取的日志文件类型。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogFileKind {
    Session,
    Persistent,
}

/// 日志查看器查询参数。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogViewerQuery {
    pub(crate) kind: LogFileKind,
    pub(crate) level: Option<LogLevel>,
    pub(crate) domain: Option<String>,
    pub(crate) search: Option<String>,
    pub(crate) limit: Option<usize>,
    pub(crate) offset: Option<usize>,
}

/// 日志面板使用的单条日志记录视图。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogViewerRecord {
    pub(crate) id: String,
    pub(crate) ts: String,
    pub(crate) level: LogLevel,
    pub(crate) domain: String,
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) details: Option<String>,
}

/// 日志查看器分页结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogViewerPage {
    pub(crate) records: Vec<LogViewerRecord>,
    pub(crate) total: usize,
    pub(crate) kind: LogFileKind,
}

/// 会话日志与持久化日志文件的可用状态摘要。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFileStatus {
    pub(crate) has_session_log: bool,
    pub(crate) has_persistent_log: bool,
}

/// 后端内部记录日志时使用的结构化载荷。
#[derive(Debug, Clone)]
pub struct LogPayload {
    pub(crate) level: LogLevel,
    pub(crate) domain: String,
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) user_message: Option<String>,
    pub(crate) details: Option<String>,
    pub(crate) context: Option<Value>,
    pub(crate) frontend_exposure: FrontendExposure,
    pub(crate) cause_chain: Vec<String>,
    pub(crate) source: String,
}

impl LogPayload {
    /// 创建一条新的结构化日志载荷。
    ///
    /// 入参 `level`、`domain`、`code` 与 `message` 组成最小必需日志字段；返回值可继续
    /// 链式附加面向用户的消息或诊断细节。
    pub fn new(
        level: LogLevel,
        domain: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            level,
            domain: domain.into(),
            code: code.into(),
            message: message.into(),
            user_message: None,
            details: None,
            context: None,
            frontend_exposure: FrontendExposure::None,
            cause_chain: Vec::new(),
            source: "backend".to_string(),
        }
    }

    /// 设置面向用户展示的消息文本。
    pub fn user_message(mut self, value: impl Into<String>) -> Self {
        self.user_message = Some(value.into());
        self
    }

    /// 设置附加诊断细节。
    pub fn details(mut self, value: impl Into<String>) -> Self {
        self.details = Some(value.into());
        self
    }
}

#[derive(Debug)]
struct LogCenterState {
    session_path: PathBuf,
    persistent_path: PathBuf,
    flushed: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct LogCenter {
    app: AppHandle,
    session_id: String,
    state: Arc<Mutex<LogCenterState>>,
}

impl LogCenter {
    pub(crate) fn new(app: AppHandle) -> Result<Self, String> {
        let cache_dir = app
            .path()
            .app_cache_dir()
            .map_err(|error| format!("failed to get app cache dir: {error}"))?
            .join("logs");
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| format!("failed to get app data dir: {error}"))?
            .join("logs");
        fs::create_dir_all(&cache_dir).map_err(|error| {
            format!(
                "failed to create session log directory {}: {error}",
                cache_dir.display()
            )
        })?;
        fs::create_dir_all(&app_data_dir).map_err(|error| {
            format!(
                "failed to create persistent log directory {}: {error}",
                app_data_dir.display()
            )
        })?;

        let session_id = next_id();
        let session_path = cache_dir.join(format!(
            "{}{}{}",
            SESSION_LOG_PREFIX, session_id, SESSION_LOG_SUFFIX
        ));
        File::create(&session_path).map_err(|error| {
            format!(
                "failed to create session log file {}: {error}",
                session_path.display()
            )
        })?;

        Ok(Self {
            app,
            session_id,
            state: Arc::new(Mutex::new(LogCenterState {
                session_path,
                persistent_path: app_data_dir.join(PERSISTENT_LOG_FILE),
                flushed: false,
            })),
        })
    }

    pub(crate) fn record(&self, payload: LogPayload) {
        if let Err(error) = self.record_impl(payload) {
            eprintln!("[logging] failed to record log entry: {error:#}");
        }
    }

    fn record_impl(&self, payload: LogPayload) -> Result<()> {
        let record = LogRecord {
            id: next_id(),
            ts: iso_timestamp_now(),
            level: payload.level,
            domain: payload.domain,
            code: payload.code,
            message: payload.message,
            user_message: payload.user_message,
            details: payload.details,
            context: payload.context,
            frontend_exposure: payload.frontend_exposure,
            cause_chain: payload.cause_chain,
            session_id: self.session_id.clone(),
            source: payload.source,
        };

        let session_path = self.state.lock().unwrap().session_path.clone();
        append_json_line(&session_path, &record)
            .with_context(|| format!("failed to append session log {}", session_path.display()))?;

        if matches!(record.frontend_exposure, FrontendExposure::EventOnly) {
            let event = AppErrorEvent {
                id: record.id.clone(),
                ts: record.ts.clone(),
                level: record.level,
                domain: record.domain.clone(),
                code: record.code.clone(),
                message: record
                    .user_message
                    .clone()
                    .unwrap_or_else(|| record.message.clone()),
            };
            let _ = self.app.emit(APP_ERROR_RECORDED, event);
        }

        Ok(())
    }

    pub(crate) fn flush_session_to_persistent(&self, threshold: LogLevel) -> Result<()> {
        let (session_path, persistent_path) = {
            let mut state = self.state.lock().unwrap();
            if state.flushed {
                return Ok(());
            }
            state.flushed = true;
            (state.session_path.clone(), state.persistent_path.clone())
        };

        let records = read_records(&session_path)?;
        let selected = records
            .into_iter()
            .filter(|record| record.level >= threshold)
            .collect::<Vec<_>>();

        if !selected.is_empty() {
            if let Some(parent) = persistent_path.parent() {
                fs::create_dir_all(parent)?;
            }
            for record in &selected {
                append_json_line(&persistent_path, record)?;
            }
        }

        if session_path.exists() {
            fs::remove_file(&session_path)?;
        }
        Ok(())
    }

    pub(crate) fn list_records(&self, query: LogViewerQuery) -> Result<LogViewerPage, String> {
        let path = match query.kind {
            LogFileKind::Session => self.state.lock().unwrap().session_path.clone(),
            LogFileKind::Persistent => self.state.lock().unwrap().persistent_path.clone(),
        };

        let mut records = if path.exists() {
            read_records(&path).map_err(|error| error.to_string())?
        } else {
            Vec::new()
        };

        if let Some(level) = query.level {
            records.retain(|record| record.level >= level);
        }
        if let Some(domain) = query.domain.as_ref().filter(|value| !value.is_empty()) {
            records.retain(|record| record.domain == *domain);
        }
        if let Some(search) = query.search.as_ref().filter(|value| !value.is_empty()) {
            let term = search.to_lowercase();
            records.retain(|record| {
                let display_message = record
                    .user_message
                    .as_ref()
                    .unwrap_or(&record.message)
                    .to_lowercase();
                display_message.contains(&term)
                    || record.code.to_lowercase().contains(&term)
                    || record.domain.to_lowercase().contains(&term)
            });
        }

        records.sort_by(|left, right| right.ts.cmp(&left.ts));

        let total = records.len();
        let offset = query.offset.unwrap_or(0).min(total);
        let limit = query.limit.unwrap_or(100).min(500);
        let page_records = records
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|record| LogViewerRecord {
                id: record.id,
                ts: record.ts,
                level: record.level,
                domain: record.domain,
                code: record.code,
                message: record.user_message.unwrap_or(record.message),
                details: None,
            })
            .collect::<Vec<_>>();

        Ok(LogViewerPage {
            records: page_records,
            total,
            kind: query.kind,
        })
    }

    pub(crate) fn file_status(&self) -> LogFileStatus {
        let state = self.state.lock().unwrap();
        LogFileStatus {
            has_session_log: state.session_path.exists(),
            has_persistent_log: state.persistent_path.exists(),
        }
    }
}

fn append_json_line(path: &Path, record: &LogRecord) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    serde_json::to_writer(&mut file, record)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn read_records(path: &Path) -> Result<Vec<LogRecord>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record = serde_json::from_str::<LogRecord>(&line)?;
        records.push(record);
    }
    Ok(records)
}

fn iso_timestamp_now() -> String {
    OffsetDateTime::now_utc()
        .format(&Iso8601::DEFAULT)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn next_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}")
}

#[cfg(test)]
mod tests {
    use super::{append_json_line, read_records, FrontendExposure, LogLevel};
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn reads_and_filters_records() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("session.jsonl");
        append_json_line(
            &path,
            &super::LogRecord {
                id: "1".to_string(),
                ts: "2026-01-01T00:00:00Z".to_string(),
                level: LogLevel::Warn,
                domain: "player".to_string(),
                code: "player.failed".to_string(),
                message: "Player failed".to_string(),
                user_message: Some("播放失败".to_string()),
                details: Some("detail".to_string()),
                context: Some(json!({"songCid":"x"})),
                frontend_exposure: FrontendExposure::None,
                cause_chain: Vec::new(),
                session_id: "session".to_string(),
                source: "backend".to_string(),
            },
        )
        .unwrap();
        let records = read_records(&path).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].domain, "player");
    }

    #[test]
    fn log_level_ordering_matches_thresholds() {
        assert!(LogLevel::Error > LogLevel::Warn);
        assert!(LogLevel::Warn > LogLevel::Info);
        assert!(LogLevel::Info > LogLevel::Debug);
    }

    #[test]
    fn parses_log_level_values() {
        assert_eq!(LogLevel::parse("debug"), Some(LogLevel::Debug));
        assert_eq!(LogLevel::parse("info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::parse("warn"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::parse("error"), Some(LogLevel::Error));
        assert_eq!(LogLevel::parse("bogus"), None);
    }
}
