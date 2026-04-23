//! Storage layer — SQLite (bundled) + FTS5.

use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub mod migrations;
pub mod transcriptions;
#[cfg(test)]
pub mod test_util;

/// Shared DB handle. Single-writer; `BEGIN IMMEDIATE` serialises writes.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        let db = Db { conn: Arc::new(Mutex::new(conn)) };
        migrations::run(&db)?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let db = Db { conn: Arc::new(Mutex::new(conn)) };
        migrations::run(&db)?;
        Ok(db)
    }

    pub fn with_conn<R>(&self, f: impl FnOnce(&Connection) -> Result<R, rusqlite::Error>) -> Result<R, DbError> {
        let guard = self.conn.lock().expect("db mutex poisoned");
        Ok(f(&guard)?)
    }

    pub fn with_conn_mut<R>(&self, f: impl FnOnce(&mut Connection) -> Result<R, rusqlite::Error>) -> Result<R, DbError> {
        let mut guard = self.conn.lock().expect("db mutex poisoned");
        Ok(f(&mut guard)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_runs_migrations() {
        let db = Db::open_in_memory().expect("open");
        let tables: i64 = db.with_conn(|c| {
            c.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='transcriptions'",
                [],
                |r| r.get(0),
            )
        }).unwrap();
        assert_eq!(tables, 1, "transcriptions table should exist after migrations");
    }

    #[test]
    fn fts_table_exists() {
        let db = Db::open_in_memory().expect("open");
        let count: i64 = db.with_conn(|c| {
            c.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='transcriptions_fts'",
                [],
                |r| r.get(0),
            )
        }).unwrap();
        assert_eq!(count, 1);
    }
}
