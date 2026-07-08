use std::path::Path;
use std::sync::Mutex;

use harubble_core::homepage::{HistoryEntry, ListeningEvent};
use rusqlite::Connection;

/// 歌曲热度持久化服务。
///
/// 基于 SQLite 存储歌曲播放热度，每首歌一行，记录有效收听次数与最近播放时间。
/// 当播放进度达到阈值（由调用方判断）时，通过 UPSERT 增加热度或插入新记录。
/// 内部持有 `Mutex<Connection>` 以保证线程安全。
pub(crate) struct ListeningHistoryService {
    conn: Mutex<Connection>,
}

impl ListeningHistoryService {
    pub(crate) fn new(db_path: &Path) -> Result<Self, String> {
        let conn = Connection::open(db_path).map_err(|e| format!("打开收听历史数据库失败: {e}"))?;
        let service = Self {
            conn: Mutex::new(conn),
        };
        service.initialize_schema()?;
        Ok(service)
    }

    #[cfg(test)]
    fn new_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| format!("创建内存数据库失败: {e}"))?;
        let service = Self {
            conn: Mutex::new(conn),
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
            "CREATE TABLE IF NOT EXISTS song_heat (
                song_cid TEXT PRIMARY KEY,
                song_name TEXT NOT NULL,
                album_cid TEXT NOT NULL,
                album_name TEXT NOT NULL,
                cover_url TEXT,
                artists TEXT NOT NULL,
                heat INTEGER NOT NULL DEFAULT 1,
                last_played_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_song_heat_last_played
                ON song_heat(last_played_at DESC);",
        )
        .map_err(|e| format!("初始化歌曲热度表失败: {e}"))?;
        Ok(())
    }

    pub(crate) fn record(&self, event: &ListeningEvent) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("获取数据库锁失败: {e}"))?;

        let artists_json = serde_json::to_string(&event.artists)
            .map_err(|e| format!("序列化 artists 失败: {e}"))?;
        let now = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Iso8601::DEFAULT)
            .map_err(|e| format!("格式化时间失败: {e}"))?;

        // UPSERT: 如果 song_cid 已存在则增加 heat 并更新 last_played_at，否则插入新记录
        conn.execute(
            "INSERT INTO song_heat (song_cid, song_name, album_cid, album_name, cover_url, artists, heat, last_played_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7)
             ON CONFLICT(song_cid) DO UPDATE SET
                heat = heat + 1,
                last_played_at = excluded.last_played_at",
            rusqlite::params![
                event.song_cid,
                event.song_name,
                event.album_cid,
                event.album_name,
                event.cover_url,
                artists_json,
                now,
            ],
        )
        .map_err(|e| format!("记录歌曲热度失败: {e}"))?;

        Ok(())
    }

    pub(crate) fn get_recent(&self, limit: u32) -> Result<Vec<HistoryEntry>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("获取数据库锁失败: {e}"))?;
        let mut stmt = conn
            .prepare(
                "SELECT song_cid, song_name, album_cid, album_name, cover_url, artists, heat, last_played_at
                 FROM song_heat ORDER BY last_played_at DESC LIMIT ?1",
            )
            .map_err(|e| format!("准备查询语句失败: {e}"))?;

        let entries = stmt
            .query_map([limit], |row| {
                let artists_json: String = row.get(5)?;
                let artists: Vec<String> = serde_json::from_str(&artists_json).unwrap_or_default();
                Ok(HistoryEntry {
                    song_cid: row.get(0)?,
                    song_name: row.get(1)?,
                    album_cid: row.get(2)?,
                    album_name: row.get(3)?,
                    cover_url: row.get(4)?,
                    artists,
                    heat: row.get(6)?,
                    played_at: row.get(7)?,
                })
            })
            .map_err(|e| format!("查询歌曲热度失败: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("读取歌曲热度行失败: {e}"))?;

        Ok(entries)
    }

    pub(crate) fn clear(&self) -> Result<u32, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| format!("获取数据库锁失败: {e}"))?;
        let deleted = conn
            .execute("DELETE FROM song_heat", [])
            .map_err(|e| format!("清除歌曲热度失败: {e}"))?;
        Ok(deleted as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(song_cid: &str, album_cid: &str) -> ListeningEvent {
        ListeningEvent {
            song_cid: song_cid.to_string(),
            song_name: format!("Song {song_cid}"),
            album_cid: album_cid.to_string(),
            album_name: format!("Album {album_cid}"),
            cover_url: Some("https://example.com/cover.jpg".to_string()),
            artists: vec!["Artist A".to_string()],
        }
    }

    #[test]
    fn accumulates_heat_for_same_song() {
        let service = ListeningHistoryService::new_in_memory().unwrap();
        service.record(&make_event("s1", "a1")).unwrap();
        service.record(&make_event("s2", "a1")).unwrap();
        service.record(&make_event("s1", "a1")).unwrap();
        // s1 出现两次，s2 出现一次，共两行
        let entries = service.get_recent(10).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn unique_songs_each_get_one_row() {
        let service = ListeningHistoryService::new_in_memory().unwrap();
        for i in 0..5 {
            service.record(&make_event(&format!("s{i}"), "a1")).unwrap();
        }
        let entries = service.get_recent(10).unwrap();
        assert_eq!(entries.len(), 5);
    }

    #[test]
    fn clear_removes_all_and_returns_count() {
        let service = ListeningHistoryService::new_in_memory().unwrap();
        service.record(&make_event("s1", "a1")).unwrap();
        service.record(&make_event("s2", "a1")).unwrap();
        let deleted = service.clear().unwrap();
        assert_eq!(deleted, 2);
        let entries = service.get_recent(10).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn get_recent_respects_limit() {
        let service = ListeningHistoryService::new_in_memory().unwrap();
        for i in 0..10 {
            service.record(&make_event(&format!("s{i}"), "a1")).unwrap();
        }
        let entries = service.get_recent(3).unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn get_recent_orders_by_last_played_desc() {
        let service = ListeningHistoryService::new_in_memory().unwrap();
        service.record(&make_event("s1", "a1")).unwrap();
        service.record(&make_event("s2", "a1")).unwrap();
        service.record(&make_event("s1", "a1")).unwrap(); // s1 最近播放更新
        let entries = service.get_recent(10).unwrap();
        // s1 最近播放，排在首位
        assert_eq!(entries[0].song_cid, "s1");
    }
}
