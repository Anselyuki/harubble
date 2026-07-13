//! 合集服务——用户自建合集与官方合集的统一管理层。
//!
//! 该模块提供合集的增删改查、歌曲管理（添加、移除、排序）以及导入导出能力。
//! 官方合集从内嵌 JSON 文件加载，用户合集持久化到 SQLite 数据库。
//!
//! # 主要能力
//!
//! - 列出所有合集（官方 + 用户），支持 locale 本地化。
//! - 查询单个合集详情，包含歌曲 ID 列表（按 position 排序）。
//! - 创建、更新、删除用户合集（官方合集只读）。
//! - 向合集添加/移除歌曲，以及对歌曲重新排序。
//! - 导出合集为 JSON 字符串，从 JSON 字符串导入合集。
//!
//! # 设计约束
//!
//! - 官方合集 ID 以 `"official:"` 为前缀，不可写入。
//! - 用户合集 ID 为 UUID v4，存储于 SQLite。
//! - 所有写操作在修改前调用 `guard_not_official` 防止误改官方合集。
//! - `update()` 在调用 `self.get()` 前必须 `drop(conn)` 以避免 Mutex 死锁。

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// ─── 常量 ─────────────────────────────────────────────────────────────────────

/// 官方合集 ID 前缀，带此前缀的合集不可修改。
const OFFICIAL_PREFIX: &str = "official:";

// ─── 错误类型 ─────────────────────────────────────────────────────────────────

/// 合集操作错误。
///
/// 实现 `Serialize`，可直接通过 Tauri IPC 序列化返回前端。
/// 序列化格式：`{ "code": "<variant>", "detail": <payload> }`。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "code", content = "detail")]
pub enum CollectionError {
    /// 合集不存在。
    NotFound { id: String },
    /// 官方合集不可修改。
    ReadOnly,
    /// 数据库操作失败。
    Database(String),
    /// 序列化 / 反序列化失败。
    Serialization(String),
    /// 不支持的导入格式版本。
    UnsupportedVersion { version: u32 },
}

impl fmt::Display for CollectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CollectionError::NotFound { id } => write!(f, "合集不存在: {id}"),
            CollectionError::ReadOnly => write!(f, "官方合集不可修改"),
            CollectionError::Database(msg) => write!(f, "{msg}"),
            CollectionError::Serialization(msg) => write!(f, "{msg}"),
            CollectionError::UnsupportedVersion { version } => {
                write!(f, "不支持的导入格式版本: {version}，当前最高支持版本为 1")
            }
        }
    }
}

impl std::error::Error for CollectionError {}

// ─── 数据结构 ─────────────────────────────────────────────────────────────────

/// 合集摘要，用于列表展示。
///
/// 包含合集基本信息与歌曲数量，不含完整歌曲 ID 列表。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionSummary {
    /// 合集唯一 ID（官方合集以 `"official:"` 为前缀，用户合集为 UUID v4）。
    pub id: String,
    /// 合集名称（已按 locale 解析）。
    pub name: String,
    /// 合集描述（已按 locale 解析）。
    pub description: String,
    /// 封面图路径或 URL，可为空。
    pub cover: Option<String>,
    /// 合集中的歌曲数量。
    pub song_count: i64,
    /// 是否为官方合集。
    pub is_official: bool,
    /// 创建时间戳（毫秒，Unix epoch）。
    pub created_at: i64,
    /// 最后更新时间戳（毫秒，Unix epoch）。
    pub updated_at: i64,
}

/// 合集分段（已按 locale 解析名称），用于前端展示。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionSection {
    /// 分段名称（已按 locale 解析），为空时前端不渲染标题行。
    pub name: Option<String>,
    /// 该分段包含的歌曲 ID 列表。
    pub song_ids: Vec<String>,
}

/// 合集详情，用于合集详情页展示。
///
/// 歌曲通过 `sections` 组织；前端从 sections 中提取完整播放队列。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    /// 合集唯一 ID。
    pub id: String,
    /// 合集名称（已按 locale 解析）。
    pub name: String,
    /// 合集描述（已按 locale 解析）。
    pub description: String,
    /// 封面图路径或 URL，可为空。
    pub cover: Option<String>,
    /// 分段列表，歌曲 ID 挂载在各分段下。
    pub sections: Vec<CollectionSection>,
    /// 是否为官方合集。
    pub is_official: bool,
    /// 创建时间戳（毫秒，Unix epoch）。
    pub created_at: i64,
    /// 最后更新时间戳（毫秒，Unix epoch）。
    pub updated_at: i64,
}

/// 多语种本地化值，key 为 BCP 47 语言标签（如 `"zh-CN"`、`"en-US"`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizedValue(pub HashMap<String, String>);

/// 官方合集分段条目（JSON 侧，可选多语种名称 + 歌曲列表）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialCollectionSection {
    /// 分段名称，多语种。无名分段不显示标题行。
    pub name: Option<LocalizedValue>,
    /// 该分段包含的歌曲 ID 列表。
    pub song_ids: Vec<String>,
}

/// 官方合集 JSON 文件中的单个合集条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialCollectionEntry {
    /// 合集 ID（不含 `"official:"` 前缀，加载时自动补全）。
    pub id: String,
    /// 合集名称，多语种。
    pub name: LocalizedValue,
    /// 合集描述，多语种。
    pub description: LocalizedValue,
    /// 封面图路径或 URL，可为空。
    pub cover: Option<String>,
    /// 分段列表，每个分段自带歌曲 ID。
    pub sections: Vec<OfficialCollectionSection>,
}

/// 官方合集 JSON 文件根结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialCollectionsFile {
    /// JSON schema 版本，当前为 1。
    pub schema_version: u32,
    /// 官方合集列表。
    pub collections: Vec<OfficialCollectionEntry>,
}

/// 导出合集的根结构。
///
/// 用于将合集序列化为可移植的 JSON 字符串，或从 JSON 字符串恢复合集。
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedCollection {
    /// 导出格式 schema 版本，当前为 1。
    pub schema_version: u32,
    /// 导出的合集数据。
    pub collection: ExportedCollectionData,
}

/// 导出合集的数据体。
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportedCollectionData {
    /// 合集 ID（导入时忽略，重新生成）。
    pub id: String,
    /// 合集名称，多语种。
    pub name: LocalizedValue,
    /// 合集描述，多语种。
    pub description: LocalizedValue,
    /// 封面图路径或 URL，可为空。
    pub cover: Option<String>,
    /// 合集中的歌曲 ID 列表，按顺序排列。
    pub song_ids: Vec<String>,
}

/// 合集服务，管理官方合集（内存）与用户合集（SQLite）。
///
/// 实现 `Clone`，可在 Tauri 状态中共享。
#[derive(Clone)]
pub struct CollectionService {
    /// 官方合集列表（内存，只读）。
    official: Arc<Vec<OfficialCollectionEntry>>,
    /// SQLite 连接（用户合集持久化）。
    conn: Arc<Mutex<Connection>>,
}

impl CollectionService {
    /// 创建合集服务实例。
    ///
    /// # 参数
    ///
    /// - `db_path`：SQLite 数据库文件路径，不存在时自动创建。
    /// - `official_json`：官方合集 JSON 文件内容（字节切片），通常由 `include_bytes!` 嵌入。
    ///
    /// # 返回值
    ///
    /// 成功返回 `CollectionService`，失败返回中文错误描述。
    ///
    /// # 副作用
    ///
    /// 调用 `initialize_schema()` 确保数据库表结构存在。
    pub fn new(db_path: &Path, official_json: &[u8]) -> Result<Self, String> {
        let conn = Connection::open(db_path)
            .map_err(|e| format!("failed to open collection database: {e}"))?;

        let file: OfficialCollectionsFile = serde_json::from_slice(official_json)
            .map_err(|e| format!("failed to parse official collections JSON: {e}"))?;

        let official: Vec<OfficialCollectionEntry> = file
            .collections
            .into_iter()
            .map(|mut entry| {
                if !entry.id.starts_with(OFFICIAL_PREFIX) {
                    entry.id = format!("{}{}", OFFICIAL_PREFIX, entry.id);
                }
                entry
            })
            .collect();

        let service = Self {
            official: Arc::new(official),
            conn: Arc::new(Mutex::new(conn)),
        };
        service.initialize_schema()?;
        Ok(service)
    }

    /// 初始化数据库 schema。
    ///
    /// 创建 `collections` 与 `collection_songs` 表（若不存在），并启用外键约束。
    /// 幂等操作，可安全重复调用。
    fn initialize_schema(&self) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("获取合集数据库锁失败: {e}"))?;

        conn.execute_batch(
            "PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS collections (
                id          TEXT PRIMARY KEY NOT NULL,
                name        TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                cover       TEXT,
                created_at  INTEGER NOT NULL,
                updated_at  INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS collection_songs (
                collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
                song_id       TEXT NOT NULL,
                position      INTEGER NOT NULL DEFAULT 0,
                added_at      INTEGER NOT NULL,
                PRIMARY KEY (collection_id, song_id)
            );

            CREATE INDEX IF NOT EXISTS idx_collection_songs_collection
                ON collection_songs(collection_id, position);",
        )
        .map_err(|e| format!("初始化合集表失败: {e}"))?;

        Ok(())
    }

    // ─── 查询方法 ─────────────────────────────────────────────────────────────

    /// 列出所有合集（官方 + 用户），按 locale 解析名称与描述。
    ///
    /// # 参数
    ///
    /// - `locale`：BCP 47 语言标签（如 `"zh-CN"`），用于本地化官方合集名称。
    ///
    /// # 返回值
    ///
    /// 官方合集在前，用户合集在后，均包含歌曲数量。
    pub fn list_all(&self, locale: &str) -> Result<Vec<CollectionSummary>, CollectionError> {
        let mut result: Vec<CollectionSummary> = Vec::new();

        // 官方合集：从内存映射，歌曲数量从 sections 计算
        for entry in self.official.iter() {
            let song_count: usize = entry.sections.iter().map(|s| s.song_ids.len()).sum();
            result.push(CollectionSummary {
                id: entry.id.clone(),
                name: resolve_locale(&entry.name, locale),
                description: resolve_locale(&entry.description, locale),
                cover: entry.cover.clone(),
                song_count: song_count as i64,
                is_official: true,
                created_at: 0,
                updated_at: 0,
            });
        }

        // 用户合集：从 SQLite 查询，LEFT JOIN 统计歌曲数量
        let conn = self
            .conn
            .lock()
            .map_err(|e| CollectionError::Database(format!("获取合集数据库锁失败: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT c.id, c.name, c.description, c.cover, c.created_at, c.updated_at,
                        COUNT(cs.song_id) AS song_count
                 FROM collections c
                 LEFT JOIN collection_songs cs ON cs.collection_id = c.id
                 GROUP BY c.id
                 ORDER BY c.updated_at DESC",
            )
            .map_err(|e| CollectionError::Database(format!("准备合集查询语句失败: {e}")))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(CollectionSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    cover: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    song_count: row.get(6)?,
                    is_official: false,
                })
            })
            .map_err(|e| CollectionError::Database(format!("查询合集列表失败: {e}")))?;

        for row in rows {
            result
                .push(row.map_err(|e| CollectionError::Database(format!("读取合集行失败: {e}")))?);
        }

        Ok(result)
    }

    /// 查询单个合集详情，包含完整歌曲 ID 列表。
    ///
    /// # 参数
    ///
    /// - `id`：合集 ID（官方合集以 `"official:"` 为前缀）。
    /// - `locale`：BCP 47 语言标签，用于本地化官方合集名称。
    ///
    /// # 返回值
    ///
    /// 成功返回 `Collection`，合集不存在时返回错误。
    pub fn get(&self, id: &str, locale: &str) -> Result<Collection, CollectionError> {
        if id.starts_with(OFFICIAL_PREFIX) {
            // 官方合集：从内存查找
            let entry = self
                .official
                .iter()
                .find(|e| e.id == id)
                .ok_or_else(|| CollectionError::NotFound { id: id.to_string() })?;

            let sections: Vec<CollectionSection> = entry
                .sections
                .iter()
                .map(|s| CollectionSection {
                    name: s.name.as_ref().map(|n| resolve_locale(n, locale)),
                    song_ids: s.song_ids.clone(),
                })
                .collect();

            return Ok(Collection {
                id: entry.id.clone(),
                name: resolve_locale(&entry.name, locale),
                description: resolve_locale(&entry.description, locale),
                cover: entry.cover.clone(),
                sections,
                is_official: true,
                created_at: 0,
                updated_at: 0,
            });
        }

        // 用户合集：从 SQLite 查询
        let conn = self
            .conn
            .lock()
            .map_err(|e| CollectionError::Database(format!("获取合集数据库锁失败: {e}")))?;

        let (name, description, cover, created_at, updated_at): (String, String, Option<String>, i64, i64) = conn
            .query_row(
                "SELECT name, description, cover, created_at, updated_at FROM collections WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .map_err(|e| CollectionError::Database(format!("查询合集失败: {e}")))?;

        let mut song_stmt = conn
            .prepare(
                "SELECT song_id FROM collection_songs
                 WHERE collection_id = ?1
                 ORDER BY position ASC",
            )
            .map_err(|e| CollectionError::Database(format!("准备歌曲查询语句失败: {e}")))?;

        let song_ids: Vec<String> = song_stmt
            .query_map(params![id], |row| row.get(0))
            .map_err(|e| CollectionError::Database(format!("查询合集歌曲失败: {e}")))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| CollectionError::Database(format!("读取合集歌曲行失败: {e}")))?;

        Ok(Collection {
            id: id.to_string(),
            name,
            description,
            cover,
            sections: vec![CollectionSection {
                name: None,
                song_ids,
            }],
            is_official: false,
            created_at,
            updated_at,
        })
    }

    // ─── 写入方法 ─────────────────────────────────────────────────────────────

    /// 创建用户合集。
    ///
    /// # 参数
    ///
    /// - `name`：合集名称。
    /// - `description`：合集描述。
    /// - `cover_path`：封面图路径或 URL，可为 `None`。
    ///
    /// # 返回值
    ///
    /// 成功返回新建的 `Collection`（歌曲列表为空）。
    pub fn create(
        &self,
        name: &str,
        description: &str,
        cover_path: Option<&str>,
    ) -> Result<Collection, CollectionError> {
        let id = Uuid::new_v4().to_string();
        let now = now_millis();

        let conn = self
            .conn
            .lock()
            .map_err(|e| CollectionError::Database(format!("获取合集数据库锁失败: {e}")))?;

        conn.execute(
            "INSERT INTO collections (id, name, description, cover, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, name, description, cover_path, now, now],
        )
        .map_err(|e| CollectionError::Database(format!("创建合集失败: {e}")))?;

        drop(conn);
        self.get(&id, "zh-CN")
    }

    /// 更新用户合集的名称、描述或封面（部分更新）。
    ///
    /// # 参数
    ///
    /// - `id`：合集 ID，不可为官方合集。
    /// - `name`：新名称，`None` 表示不修改。
    /// - `description`：新描述，`None` 表示不修改。
    /// - `cover_path`：新封面，`None` 表示不修改；`Some(None)` 表示清除封面。
    ///
    /// # 返回值
    ///
    /// 成功返回更新后的 `Collection`。
    pub fn update(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
        cover_path: Option<Option<&str>>,
    ) -> Result<Collection, CollectionError> {
        guard_not_official(id)?;

        let now = now_millis();

        let conn = self
            .conn
            .lock()
            .map_err(|e| CollectionError::Database(format!("获取合集数据库锁失败: {e}")))?;

        if let Some(n) = name {
            conn.execute(
                "UPDATE collections SET name = ?1, updated_at = ?2 WHERE id = ?3",
                params![n, now, id],
            )
            .map_err(|e| CollectionError::Database(format!("更新合集名称失败: {e}")))?;
        }

        if let Some(d) = description {
            conn.execute(
                "UPDATE collections SET description = ?1, updated_at = ?2 WHERE id = ?3",
                params![d, now, id],
            )
            .map_err(|e| CollectionError::Database(format!("更新合集描述失败: {e}")))?;
        }

        if let Some(c) = cover_path {
            conn.execute(
                "UPDATE collections SET cover = ?1, updated_at = ?2 WHERE id = ?3",
                params![c, now, id],
            )
            .map_err(|e| CollectionError::Database(format!("更新合集封面失败: {e}")))?;
        }

        // 必须 drop conn，避免 Mutex 死锁（Mutex 不可重入）
        drop(conn);
        self.get(id, "zh-CN")
    }

    /// 删除用户合集（级联删除关联歌曲记录）。
    ///
    /// # 参数
    ///
    /// - `id`：合集 ID，不可为官方合集。
    ///
    /// # 返回值
    ///
    /// 成功返回 `()`，合集不存在时返回错误。
    pub fn delete(&self, id: &str) -> Result<(), CollectionError> {
        guard_not_official(id)?;

        let conn = self
            .conn
            .lock()
            .map_err(|e| CollectionError::Database(format!("获取合集数据库锁失败: {e}")))?;

        let affected = conn
            .execute("DELETE FROM collections WHERE id = ?1", params![id])
            .map_err(|e| CollectionError::Database(format!("删除合集失败: {e}")))?;

        if affected == 0 {
            return Err(CollectionError::NotFound { id: id.to_string() });
        }

        Ok(())
    }

    // ─── 歌曲管理方法 ─────────────────────────────────────────────────────────

    /// 向合集添加歌曲（已存在的歌曲忽略，不报错）。
    ///
    /// # 参数
    ///
    /// - `id`：合集 ID，不可为官方合集。
    /// - `song_ids`：要添加的歌曲 ID 列表。
    ///
    /// # 副作用
    ///
    /// 更新合集的 `updated_at` 时间戳。
    pub fn add_songs(&self, id: &str, song_ids: &[String]) -> Result<(), CollectionError> {
        guard_not_official(id)?;

        let conn = self
            .conn
            .lock()
            .map_err(|e| CollectionError::Database(format!("获取合集数据库锁失败: {e}")))?;

        let max_pos: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(position), -1) FROM collection_songs WHERE collection_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| CollectionError::Database(format!("查询最大 position 失败: {e}")))?;

        let mut pos = max_pos + 1;
        let now = now_millis();
        for song_id in song_ids {
            conn.execute(
                "INSERT OR IGNORE INTO collection_songs (collection_id, song_id, position, added_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, song_id, pos, now],
            )
            .map_err(|e| CollectionError::Database(format!("添加歌曲失败: {e}")))?;
            pos += 1;
        }

        conn.execute(
            "UPDATE collections SET updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| CollectionError::Database(format!("更新合集时间戳失败: {e}")))?;

        Ok(())
    }

    /// 从合集移除歌曲。
    ///
    /// # 参数
    ///
    /// - `id`：合集 ID，不可为官方合集。
    /// - `song_ids`：要移除的歌曲 ID 列表。
    ///
    /// # 副作用
    ///
    /// 更新合集的 `updated_at` 时间戳。
    pub fn remove_songs(&self, id: &str, song_ids: &[String]) -> Result<(), CollectionError> {
        guard_not_official(id)?;

        let conn = self
            .conn
            .lock()
            .map_err(|e| CollectionError::Database(format!("获取合集数据库锁失败: {e}")))?;

        for song_id in song_ids {
            conn.execute(
                "DELETE FROM collection_songs WHERE collection_id = ?1 AND song_id = ?2",
                params![id, song_id],
            )
            .map_err(|e| CollectionError::Database(format!("移除歌曲失败: {e}")))?;
        }

        let now = now_millis();
        conn.execute(
            "UPDATE collections SET updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| CollectionError::Database(format!("更新合集时间戳失败: {e}")))?;

        Ok(())
    }

    /// 对合集中的歌曲重新排序。
    ///
    /// # 参数
    ///
    /// - `id`：合集 ID，不可为官方合集。
    /// - `song_ids`：按新顺序排列的歌曲 ID 列表，列表中的 index 即为新 position。
    ///
    /// # 副作用
    ///
    /// 在事务中批量更新 position，并更新合集的 `updated_at` 时间戳。
    pub fn reorder_songs(&self, id: &str, song_ids: &[String]) -> Result<(), CollectionError> {
        guard_not_official(id)?;

        let conn = self
            .conn
            .lock()
            .map_err(|e| CollectionError::Database(format!("获取合集数据库锁失败: {e}")))?;

        // SAFETY: unchecked_transaction 在 Mutex 保护下安全使用
        let tx = conn
            .unchecked_transaction()
            .map_err(|e| CollectionError::Database(format!("开启事务失败: {e}")))?;

        for (pos, song_id) in song_ids.iter().enumerate() {
            tx.execute(
                "UPDATE collection_songs SET position = ?1
                 WHERE collection_id = ?2 AND song_id = ?3",
                params![pos as i64, id, song_id],
            )
            .map_err(|e| CollectionError::Database(format!("更新歌曲排序失败: {e}")))?;
        }

        let now = now_millis();
        tx.execute(
            "UPDATE collections SET updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .map_err(|e| CollectionError::Database(format!("更新合集时间戳失败: {e}")))?;

        tx.commit()
            .map_err(|e| CollectionError::Database(format!("提交排序事务失败: {e}")))?;

        Ok(())
    }

    // ─── 导出 / 导入方法 ──────────────────────────────────────────────────────

    /// 将合集导出为 JSON 字符串。
    ///
    /// # 参数
    ///
    /// - `id`：合集 ID（官方或用户均可导出）。
    /// - `locale`：BCP 47 语言标签，用于解析官方合集名称（用户合集名称直接存储）。
    ///
    /// # 返回值
    ///
    /// 成功返回格式化的 JSON 字符串（pretty-printed）。
    pub fn export(&self, id: &str, locale: &str) -> Result<String, CollectionError> {
        let collection = self.get(id, locale)?;

        let mut name_map = HashMap::new();
        name_map.insert("zh-CN".to_string(), collection.name.clone());
        let mut desc_map = HashMap::new();
        desc_map.insert("zh-CN".to_string(), collection.description.clone());

        let exported = ExportedCollection {
            schema_version: 1,
            collection: ExportedCollectionData {
                id: collection.id,
                name: LocalizedValue(name_map),
                description: LocalizedValue(desc_map),
                cover: collection.cover,
                song_ids: collection
                    .sections
                    .into_iter()
                    .flat_map(|s| s.song_ids)
                    .collect(),
            },
        };

        serde_json::to_string_pretty(&exported)
            .map_err(|e| CollectionError::Serialization(format!("序列化合集失败: {e}")))
    }

    /// 从 JSON 字符串导入合集，创建新的用户合集。
    ///
    /// # 参数
    ///
    /// - `json`：由 `export()` 生成的 JSON 字符串。
    ///
    /// # 返回值
    ///
    /// 成功返回新建的 `Collection`（ID 为新生成的 UUID v4）。
    ///
    /// # 错误场景
    ///
    /// - JSON 格式不合法。
    /// - `schema_version` 大于 1（不兼容的未来版本）。
    pub fn import(&self, json: &str) -> Result<Collection, CollectionError> {
        let exported: ExportedCollection = serde_json::from_str(json)
            .map_err(|e| CollectionError::Serialization(format!("解析导入 JSON 失败: {e}")))?;

        if exported.schema_version > 1 {
            return Err(CollectionError::UnsupportedVersion {
                version: exported.schema_version,
            });
        }

        let data = exported.collection;
        let name = resolve_locale(&data.name, "zh-CN");
        let description = resolve_locale(&data.description, "zh-CN");

        let new_collection = self.create(&name, &description, data.cover.as_deref())?;

        if !data.song_ids.is_empty() {
            self.add_songs(&new_collection.id, &data.song_ids)?;
        }

        self.get(&new_collection.id, "zh-CN")
    }
}

// ─── 辅助函数 ─────────────────────────────────────────────────────────────────

/// 按 locale 解析本地化值，回退链：locale → zh-CN → en-US → 第一个可用项 → 空字符串。
///
/// # 参数
///
/// - `value`：多语种本地化值。
/// - `locale`：目标语言标签（如 `"zh-CN"`）。
fn resolve_locale(value: &LocalizedValue, locale: &str) -> String {
    let map = &value.0;
    if let Some(v) = map.get(locale) {
        return v.clone();
    }
    if let Some(v) = map.get("zh-CN") {
        return v.clone();
    }
    if let Some(v) = map.get("en-US") {
        return v.clone();
    }
    if let Some(v) = map.values().next() {
        return v.clone();
    }
    String::new()
}

/// 防止对官方合集执行写操作。
///
/// # 参数
///
/// - `id`：合集 ID。
///
/// # 返回值
///
/// 若 ID 以 `OFFICIAL_PREFIX` 开头，返回错误；否则返回 `Ok(())`。
fn guard_not_official(id: &str) -> Result<(), CollectionError> {
    if id.starts_with(OFFICIAL_PREFIX) {
        return Err(CollectionError::ReadOnly);
    }
    Ok(())
}

/// 获取当前时间戳（毫秒，Unix epoch）。
fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
