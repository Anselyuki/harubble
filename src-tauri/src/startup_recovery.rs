use crate::logging::{LogCenter, LogLevel, LogPayload};
use rusqlite::Connection;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const LOCAL_DATABASE_FILE_NAME: &str = "harubble_local.db";
pub(crate) const LOCAL_DATABASE_SCHEMA_VERSION: u32 = 1;

/// 准备本地 SQLite 数据库，确保启动阶段拿到的是可打开、schema 可用的数据库文件。
pub(crate) fn prepare_local_database(
    app_data_dir: &Path,
    log_center: Option<&LogCenter>,
) -> Result<PathBuf, String> {
    fs::create_dir_all(app_data_dir)
        .map_err(|error| format!("failed to create {}: {error}", app_data_dir.display()))?;
    let db_path = app_data_dir.join(LOCAL_DATABASE_FILE_NAME);
    match try_prepare_database(&db_path) {
        Ok(()) => Ok(db_path),
        Err(error) => {
            record_recovery_log(
                log_center,
                LogLevel::Warn,
                "startup-recovery",
                "startup-recovery.db_migration_failed",
                "Failed to prepare local database; recreating from backup",
                format!("path={} error={error}", db_path.display()),
            );
            backup_and_recreate_database(&db_path, log_center)?;
            Ok(db_path)
        }
    }
}

fn try_prepare_database(db_path: &Path) -> Result<(), String> {
    let conn = Connection::open(db_path)
        .map_err(|error| format!("failed to open {}: {error}", db_path.display()))?;
    check_database_health(&conn)?;
    initialize_target_schema(&conn)?;
    validate_and_migrate_existing_tables(&conn)?;
    let user_version = read_user_version(&conn)?;
    if user_version > LOCAL_DATABASE_SCHEMA_VERSION {
        return Err(format!(
            "unsupported local database schema version {user_version}, current {LOCAL_DATABASE_SCHEMA_VERSION}"
        ));
    }
    conn.pragma_update(None, "user_version", LOCAL_DATABASE_SCHEMA_VERSION)
        .map_err(|error| format!("failed to set user_version: {error}"))?;
    Ok(())
}

fn check_database_health(conn: &Connection) -> Result<(), String> {
    let quick_check: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|error| format!("failed to run quick_check: {error}"))?;
    if quick_check.eq_ignore_ascii_case("ok") {
        Ok(())
    } else {
        Err(format!("quick_check failed: {quick_check}"))
    }
}

fn read_user_version(conn: &Connection) -> Result<u32, String> {
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|error| format!("failed to read user_version: {error}"))?;
    u32::try_from(version).map_err(|_| format!("invalid user_version: {version}"))
}

fn initialize_target_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS listening_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            song_cid TEXT NOT NULL,
            song_name TEXT NOT NULL,
            album_cid TEXT NOT NULL,
            album_name TEXT NOT NULL,
            cover_url TEXT,
            artists TEXT NOT NULL,
            played_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_listening_played_at
            ON listening_history(played_at DESC);

        CREATE TABLE IF NOT EXISTS album_metadata_cache (
            album_cid TEXT PRIMARY KEY,
            belong TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

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
    .map_err(|error| format!("failed to initialize local database schema: {error}"))?;
    Ok(())
}

fn validate_and_migrate_existing_tables(conn: &Connection) -> Result<(), String> {
    ensure_columns(
        conn,
        "listening_history",
        &[
            "id",
            "song_cid",
            "song_name",
            "album_cid",
            "album_name",
            "cover_url",
            "artists",
            "played_at",
        ],
        &[("cover_url", "TEXT")],
    )?;
    ensure_columns(
        conn,
        "album_metadata_cache",
        &["album_cid", "belong", "updated_at"],
        &[],
    )?;
    ensure_columns(
        conn,
        "collections",
        &[
            "id",
            "name",
            "description",
            "cover",
            "created_at",
            "updated_at",
        ],
        &[("description", "TEXT NOT NULL DEFAULT ''")],
    )?;
    ensure_columns(
        conn,
        "collection_songs",
        &["collection_id", "song_id", "position", "added_at"],
        &[],
    )?;
    Ok(())
}

fn ensure_columns(
    conn: &Connection,
    table_name: &str,
    required_columns: &[&str],
    safe_add_columns: &[(&str, &str)],
) -> Result<(), String> {
    let existing = table_columns(conn, table_name)?;
    for column in required_columns {
        if existing.contains(*column) {
            continue;
        }
        if let Some((_, definition)) = safe_add_columns
            .iter()
            .find(|(safe_column, _)| safe_column == column)
        {
            conn.execute_batch(&format!(
                "ALTER TABLE {table_name} ADD COLUMN {column} {definition};"
            ))
            .map_err(|error| {
                format!("failed to add {table_name}.{column} during migration: {error}")
            })?;
        } else {
            return Err(format!(
                "required column {table_name}.{column} is missing and cannot be recovered"
            ));
        }
    }
    Ok(())
}

fn table_columns(conn: &Connection, table_name: &str) -> Result<HashSet<String>, String> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table_name})"))
        .map_err(|error| format!("failed to inspect table {table_name}: {error}"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| format!("failed to query columns for {table_name}: {error}"))?
        .collect::<Result<HashSet<_>, _>>()
        .map_err(|error| format!("failed to read columns for {table_name}: {error}"))?;
    Ok(columns)
}

fn backup_and_recreate_database(
    db_path: &Path,
    log_center: Option<&LogCenter>,
) -> Result<(), String> {
    let backup_suffix = recovered_backup_suffix();
    backup_path_if_exists(db_path, &backup_suffix, log_center);
    backup_path_if_exists(
        &sqlite_sidecar_path(db_path, "wal"),
        &backup_suffix,
        log_center,
    );
    backup_path_if_exists(
        &sqlite_sidecar_path(db_path, "shm"),
        &backup_suffix,
        log_center,
    );

    let conn = Connection::open(db_path).map_err(|error| {
        format!(
            "failed to create replacement database {}: {error}",
            db_path.display()
        )
    })?;
    initialize_target_schema(&conn)?;
    conn.pragma_update(None, "user_version", LOCAL_DATABASE_SCHEMA_VERSION)
        .map_err(|error| format!("failed to set replacement user_version: {error}"))?;
    record_recovery_log(
        log_center,
        LogLevel::Warn,
        "startup-recovery",
        "startup-recovery.db_recreated_from_backup",
        "Recreated local database from backup",
        format!("path={}", db_path.display()),
    );
    Ok(())
}

fn sqlite_sidecar_path(db_path: &Path, suffix: &str) -> PathBuf {
    db_path.with_file_name(format!(
        "{}-{suffix}",
        db_path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| LOCAL_DATABASE_FILE_NAME.into())
    ))
}

pub(crate) fn backup_path_if_exists(
    path: &Path,
    backup_suffix: &str,
    log_center: Option<&LogCenter>,
) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    let backup_path = path.with_file_name(format!(
        "{}.{backup_suffix}",
        path.file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| "unknown".into())
    ));
    match fs::rename(path, &backup_path) {
        Ok(()) => Some(backup_path),
        Err(error) => {
            record_recovery_log(
                log_center,
                LogLevel::Error,
                "startup-recovery",
                "startup-recovery.backup_failed",
                "Failed to backup startup data file",
                format!(
                    "path={} backupPath={} error={error}",
                    path.display(),
                    backup_path.display()
                ),
            );
            None
        }
    }
}

pub(crate) fn recovered_backup_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("recovered-backup-{nanos}")
}

pub(crate) fn record_recovery_log(
    log_center: Option<&LogCenter>,
    level: LogLevel,
    domain: &str,
    code: &str,
    message: &str,
    details: String,
) {
    if let Some(log_center) = log_center {
        log_center.record(LogPayload::new(level, domain, code, message).details(details));
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;
    use tempfile::tempdir;

    #[test]
    fn prepares_new_database_with_current_schema() {
        let dir = tempdir().expect("temp dir");

        let db_path = super::prepare_local_database(dir.path(), None).expect("database prepared");

        assert_eq!(db_path, dir.path().join(super::LOCAL_DATABASE_FILE_NAME));
        let conn = Connection::open(&db_path).expect("open prepared database");
        let user_version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read user_version");
        assert_eq!(user_version, super::LOCAL_DATABASE_SCHEMA_VERSION as i64);

        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN (
                    'listening_history',
                    'album_metadata_cache',
                    'collections',
                    'collection_songs'
                )",
                [],
                |row| row.get(0),
            )
            .expect("count tables");
        assert_eq!(table_count, 4);
    }

    #[test]
    fn backs_up_invalid_database_and_recreates_empty_schema() {
        let dir = tempdir().expect("temp dir");
        let db_path = dir.path().join(super::LOCAL_DATABASE_FILE_NAME);
        std::fs::write(&db_path, b"not sqlite").expect("write invalid db");

        let prepared = super::prepare_local_database(dir.path(), None).expect("database prepared");

        assert_eq!(prepared, db_path);
        let backups = std::fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.starts_with("harubble_local.db.recovered-backup-"))
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);

        let conn = Connection::open(&prepared).expect("open recreated database");
        let user_version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .expect("read user_version");
        assert_eq!(user_version, super::LOCAL_DATABASE_SCHEMA_VERSION as i64);
    }

    #[test]
    fn migrates_user_version_zero_database_without_backup() {
        let dir = tempdir().expect("temp dir");
        let db_path = dir.path().join(super::LOCAL_DATABASE_FILE_NAME);
        let conn = Connection::open(&db_path).expect("open old db");
        conn.execute_batch(
            "CREATE TABLE listening_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                song_cid TEXT NOT NULL,
                song_name TEXT NOT NULL,
                album_cid TEXT NOT NULL,
                album_name TEXT NOT NULL,
                cover_url TEXT,
                artists TEXT NOT NULL,
                played_at TEXT NOT NULL
            );
            INSERT INTO listening_history (
                song_cid,
                song_name,
                album_cid,
                album_name,
                artists,
                played_at
            ) VALUES (
                'song-1',
                'Song',
                'album-1',
                'Album',
                '[]',
                '2026-06-17T00:00:00Z'
            );",
        )
        .expect("seed old db");
        drop(conn);

        super::prepare_local_database(dir.path(), None).expect("database prepared");

        let backups = std::fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.starts_with("harubble_local.db.recovered-backup-"))
            .collect::<Vec<_>>();
        assert!(backups.is_empty());

        let conn = Connection::open(&db_path).expect("open migrated database");
        let history_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM listening_history", [], |row| {
                row.get(0)
            })
            .expect("count history");
        assert_eq!(history_count, 1);

        let collection_table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'collections'",
                [],
                |row| row.get(0),
            )
            .expect("collections table exists");
        assert_eq!(collection_table_count, 1);
    }

    #[test]
    fn adds_safe_missing_collection_description_column() {
        let dir = tempdir().expect("temp dir");
        let db_path = dir.path().join(super::LOCAL_DATABASE_FILE_NAME);
        let conn = Connection::open(&db_path).expect("open old db");
        conn.execute_batch(
            "CREATE TABLE collections (
                id          TEXT PRIMARY KEY NOT NULL,
                name        TEXT NOT NULL,
                cover       TEXT,
                created_at  INTEGER NOT NULL,
                updated_at  INTEGER NOT NULL
            );
            INSERT INTO collections (id, name, created_at, updated_at)
            VALUES ('collection-1', 'Favorites', 1, 1);",
        )
        .expect("seed old collections");
        drop(conn);

        super::prepare_local_database(dir.path(), None).expect("database prepared");

        let conn = Connection::open(&db_path).expect("open migrated database");
        let description: String = conn
            .query_row(
                "SELECT description FROM collections WHERE id = 'collection-1'",
                [],
                |row| row.get(0),
            )
            .expect("read migrated description");
        assert_eq!(description, "");
    }

    #[test]
    fn backs_up_database_when_required_column_cannot_be_recovered() {
        let dir = tempdir().expect("temp dir");
        let db_path = dir.path().join(super::LOCAL_DATABASE_FILE_NAME);
        let conn = Connection::open(&db_path).expect("open old db");
        conn.execute_batch(
            "CREATE TABLE listening_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                song_cid TEXT NOT NULL,
                song_name TEXT NOT NULL,
                album_cid TEXT NOT NULL,
                album_name TEXT NOT NULL,
                played_at TEXT NOT NULL
            );
            INSERT INTO listening_history (
                song_cid,
                song_name,
                album_cid,
                album_name,
                played_at
            ) VALUES (
                'song-1',
                'Song',
                'album-1',
                'Album',
                '2026-06-17T00:00:00Z'
            );",
        )
        .expect("seed unrecoverable old history");
        drop(conn);

        super::prepare_local_database(dir.path(), None).expect("database prepared");

        let backups = std::fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.starts_with("harubble_local.db.recovered-backup-"))
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);

        let conn = Connection::open(&db_path).expect("open recreated database");
        let history_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM listening_history", [], |row| {
                row.get(0)
            })
            .expect("count history");
        assert_eq!(history_count, 0);
    }

    #[test]
    fn adds_safe_missing_nullable_history_cover_column() {
        let dir = tempdir().expect("temp dir");
        let db_path = dir.path().join(super::LOCAL_DATABASE_FILE_NAME);
        let conn = Connection::open(&db_path).expect("open old db");
        conn.execute_batch(
            "CREATE TABLE listening_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                song_cid TEXT NOT NULL,
                song_name TEXT NOT NULL,
                album_cid TEXT NOT NULL,
                album_name TEXT NOT NULL,
                artists TEXT NOT NULL,
                played_at TEXT NOT NULL
            );
            INSERT INTO listening_history (
                song_cid,
                song_name,
                album_cid,
                album_name,
                artists,
                played_at
            ) VALUES (
                'song-1',
                'Song',
                'album-1',
                'Album',
                '[]',
                '2026-06-17T00:00:00Z'
            );",
        )
        .expect("seed old history without cover_url");
        drop(conn);

        super::prepare_local_database(dir.path(), None).expect("database prepared");

        let backups = std::fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.starts_with("harubble_local.db.recovered-backup-"))
            .collect::<Vec<_>>();
        assert!(backups.is_empty());

        let conn = Connection::open(&db_path).expect("open migrated database");
        let cover_url: Option<String> = conn
            .query_row(
                "SELECT cover_url FROM listening_history WHERE song_cid = 'song-1'",
                [],
                |row| row.get(0),
            )
            .expect("read migrated cover_url");
        assert!(cover_url.is_none());
    }
}
