use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

const URL_CACHE_DATABASE: &str = "Cache.db";

/// Repairs the process-wide macOS URL cache before WebKit opens it.
///
/// Removing the application cache directory while Harubble is running can
/// leave behind an empty SQLite container on the next launch. CFNetwork sees
/// the file, assumes its schema exists, and then repeatedly fails to prepare
/// its cache insert statement. The database is disposable, so removing only
/// an unreadable or schema-less database lets CFNetwork recreate it normally.
pub fn repair_macos_url_cache() -> Result<bool, String> {
    let cache_root = dirs::cache_dir()
        .ok_or_else(|| "failed to resolve the macOS cache directory".to_string())?
        .join(env!("CARGO_PKG_NAME"));
    repair_macos_url_cache_at(&cache_root)
}

fn repair_macos_url_cache_at(cache_root: &Path) -> Result<bool, String> {
    std::fs::create_dir_all(cache_root)
        .map_err(|error| format!("failed to create {}: {error}", cache_root.display()))?;

    let database = cache_root.join(URL_CACHE_DATABASE);
    if !database.exists() {
        return Ok(false);
    }

    if url_cache_database_has_schema(&database) {
        return Ok(false);
    }

    for path in url_cache_database_files(&database) {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!("failed to remove {}: {error}", path.display()));
            }
        }
    }

    Ok(true)
}

fn url_cache_database_has_schema(database: &Path) -> bool {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let Ok(connection) = Connection::open_with_flags(database, flags) else {
        return false;
    };

    connection
        .query_row(
            "SELECT EXISTS(\
                 SELECT 1 FROM sqlite_master \
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%'\
             )",
            [],
            |row| row.get::<_, bool>(0),
        )
        .unwrap_or(false)
}

fn url_cache_database_files(database: &Path) -> [PathBuf; 4] {
    [
        database.to_path_buf(),
        database.with_file_name(format!("{URL_CACHE_DATABASE}-wal")),
        database.with_file_name(format!("{URL_CACHE_DATABASE}-shm")),
        database.with_file_name(format!("{URL_CACHE_DATABASE}-journal")),
    ]
}

#[cfg(test)]
mod tests {
    use super::repair_macos_url_cache_at;
    use rusqlite::Connection;

    #[test]
    fn preserves_missing_database_and_creates_cache_root() {
        let temp = tempfile::tempdir().expect("temp dir");
        let cache_root = temp.path().join("harubble");

        let repaired = repair_macos_url_cache_at(&cache_root).expect("repair cache");

        assert!(!repaired);
        assert!(cache_root.is_dir());
        assert!(!cache_root.join("Cache.db").exists());
    }

    #[test]
    fn removes_schema_less_database() {
        let temp = tempfile::tempdir().expect("temp dir");
        let database = temp.path().join("Cache.db");
        let connection = Connection::open(&database).expect("create empty sqlite database");
        connection
            .execute_batch("VACUUM")
            .expect("flush sqlite header");
        drop(connection);
        assert!(database.metadata().expect("database metadata").len() > 0);

        let repaired = repair_macos_url_cache_at(temp.path()).expect("repair cache");

        assert!(repaired);
        assert!(!database.exists());
    }

    #[test]
    fn removes_unreadable_database_and_sidecars() {
        let temp = tempfile::tempdir().expect("temp dir");
        let database = temp.path().join("Cache.db");
        std::fs::write(&database, b"not sqlite").expect("write invalid database");
        std::fs::write(temp.path().join("Cache.db-wal"), b"wal").expect("write wal");
        std::fs::write(temp.path().join("Cache.db-shm"), b"shm").expect("write shm");
        std::fs::write(temp.path().join("Cache.db-journal"), b"journal").expect("write journal");

        let repaired = repair_macos_url_cache_at(temp.path()).expect("repair cache");

        assert!(repaired);
        assert!(!database.exists());
        assert!(!temp.path().join("Cache.db-wal").exists());
        assert!(!temp.path().join("Cache.db-shm").exists());
        assert!(!temp.path().join("Cache.db-journal").exists());
    }

    #[test]
    fn preserves_database_with_schema() {
        let temp = tempfile::tempdir().expect("temp dir");
        let database = temp.path().join("Cache.db");
        let connection = Connection::open(&database).expect("create sqlite database");
        connection
            .execute(
                "CREATE TABLE cfurl_cache_response (entry_ID INTEGER PRIMARY KEY)",
                [],
            )
            .expect("create cache schema");
        drop(connection);

        let repaired = repair_macos_url_cache_at(temp.path()).expect("repair cache");

        assert!(!repaired);
        assert!(database.exists());
    }
}
