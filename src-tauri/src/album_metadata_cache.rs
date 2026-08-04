use std::sync::{Arc, Mutex};

use rusqlite::Connection;

/// 专辑 belong 缓存记录。
pub(crate) struct AlbumBelongRecord {
    pub album_cid: String,
    pub belong: String,
}

/// 专辑元数据缓存服务。
///
/// 基于 SQLite 缓存专辑的 belong（系列归属）信息，用于首页"按系列浏览"分组。
/// 使用 `Arc<Mutex<Connection>>` 以支持 `Clone` 和线程安全。
#[derive(Clone)]
pub(crate) struct AlbumMetadataCacheService {
    conn: Arc<Mutex<Connection>>,
}

impl AlbumMetadataCacheService {
    pub(crate) fn new(db_path: &std::path::Path) -> Result<Self, String> {
        let conn = Connection::open(db_path)
            .map_err(|e| format!("failed to open metadata cache database: {e}"))?;
        let service = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        service.initialize_schema()?;
        Ok(service)
    }

    #[cfg(test)]
    fn new_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| format!("创建内存数据库失败: {e}"))?;
        let service = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        service.initialize_schema()?;
        Ok(service)
    }

    fn initialize_schema(&self) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("获取数据库锁失败: {e}"))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS album_metadata_cache (
                album_cid TEXT PRIMARY KEY,
                belong TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )
        .map_err(|e| format!("初始化元数据缓存表失败: {e}"))?;
        Ok(())
    }

    pub(crate) fn upsert_belong(&self, album_cid: &str, belong: &str) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("获取数据库锁失败: {e}"))?;
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .unwrap_or_default();
        conn.execute(
            "INSERT INTO album_metadata_cache (album_cid, belong, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(album_cid) DO UPDATE SET belong = excluded.belong, updated_at = excluded.updated_at",
            rusqlite::params![album_cid, belong, now],
        )
        .map_err(|e| format!("写入 belong 缓存失败: {e}"))?;
        Ok(())
    }

    pub(crate) fn get_all_belongs(&self) -> Result<Vec<AlbumBelongRecord>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("获取数据库锁失败: {e}"))?;
        let mut stmt = conn
            .prepare("SELECT album_cid, belong FROM album_metadata_cache")
            .map_err(|e| format!("准备查询语句失败: {e}"))?;
        let records = stmt
            .query_map([], |row| {
                Ok(AlbumBelongRecord {
                    album_cid: row.get(0)?,
                    belong: row.get(1)?,
                })
            })
            .map_err(|e| format!("查询元数据缓存失败: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("读取元数据缓存行失败: {e}"))?;
        Ok(records)
    }

    pub(crate) fn get_missing_album_cids(
        &self,
        all_cids: &[String],
    ) -> Result<Vec<String>, String> {
        let cached = self.get_all_belongs()?;
        let cached_set: std::collections::HashSet<&str> =
            cached.iter().map(|r| r.album_cid.as_str()).collect();
        let missing = all_cids
            .iter()
            .filter(|cid| !cached_set.contains(cid.as_str()))
            .cloned()
            .collect();
        Ok(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upserts_and_retrieves_belong() {
        let service = AlbumMetadataCacheService::new_in_memory().unwrap();
        service.upsert_belong("a1", "Arknights").unwrap();
        let records = service.get_all_belongs().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].album_cid, "a1");
        assert_eq!(records[0].belong, "Arknights");
    }

    #[test]
    fn upsert_overwrites_existing_belong() {
        let service = AlbumMetadataCacheService::new_in_memory().unwrap();
        service.upsert_belong("a1", "Arknights").unwrap();
        service.upsert_belong("a1", "Endfield").unwrap();
        let records = service.get_all_belongs().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].belong, "Endfield");
    }

    #[test]
    fn get_missing_album_cids_returns_only_uncached() {
        let service = AlbumMetadataCacheService::new_in_memory().unwrap();
        service.upsert_belong("a1", "Arknights").unwrap();
        let all_cids = vec!["a1".to_string(), "a2".to_string(), "a3".to_string()];
        let missing = service.get_missing_album_cids(&all_cids).unwrap();
        assert_eq!(missing, vec!["a2", "a3"]);
    }
}
