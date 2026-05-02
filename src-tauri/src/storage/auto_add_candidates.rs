use super::{Db, DbError};
use rusqlite::params;
use rusqlite::OptionalExtension;

#[derive(Debug, Clone, PartialEq)]
pub struct AutoAddCandidate {
    pub term: String,
    pub seen_count: i64,
    pub last_seen_at: i64,
}

pub struct AutoAddCandidatesRepo<'a> {
    db: &'a Db,
}

impl<'a> AutoAddCandidatesRepo<'a> {
    pub fn new(db: &'a Db) -> Self {
        Self { db }
    }

    /// Upsert: increments `seen_count` by 1 and stamps `last_seen_at = now`.
    /// Returns the new count.
    pub fn observe(&self, now: i64, term: &str) -> Result<i64, DbError> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO auto_add_candidates (term, seen_count, last_seen_at)
                 VALUES (?1, 1, ?2)
                 ON CONFLICT(term) DO UPDATE SET
                   seen_count = seen_count + 1,
                   last_seen_at = excluded.last_seen_at",
                params![term, now],
            )?;
            c.query_row(
                "SELECT seen_count FROM auto_add_candidates WHERE term = ?1",
                [term],
                |r| r.get(0),
            )
        })
    }

    pub fn get(&self, term: &str) -> Result<Option<AutoAddCandidate>, DbError> {
        self.db.with_conn(|c| {
            c.query_row(
                "SELECT term, seen_count, last_seen_at FROM auto_add_candidates WHERE term = ?1",
                [term],
                |r| {
                    Ok(AutoAddCandidate {
                        term: r.get(0)?,
                        seen_count: r.get(1)?,
                        last_seen_at: r.get(2)?,
                    })
                },
            )
            .optional()
        })
    }

    pub fn delete(&self, term: &str) -> Result<usize, DbError> {
        self.db.with_conn(|c| {
            c.execute("DELETE FROM auto_add_candidates WHERE term = ?1", [term])
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    #[test]
    fn observe_increments_count() {
        let db = mem_db();
        let repo = AutoAddCandidatesRepo::new(&db);
        assert_eq!(repo.observe(100, "tauri").unwrap(), 1);
        assert_eq!(repo.observe(200, "tauri").unwrap(), 2);
        assert_eq!(repo.observe(300, "tauri").unwrap(), 3);
        let row = repo.get("tauri").unwrap().unwrap();
        assert_eq!(row.seen_count, 3);
        assert_eq!(row.last_seen_at, 300);
    }

    #[test]
    fn get_missing_returns_none() {
        let db = mem_db();
        let repo = AutoAddCandidatesRepo::new(&db);
        assert!(repo.get("anything").unwrap().is_none());
    }

    #[test]
    fn delete_removes_row() {
        let db = mem_db();
        let repo = AutoAddCandidatesRepo::new(&db);
        repo.observe(1, "x").unwrap();
        assert_eq!(repo.delete("x").unwrap(), 1);
        assert!(repo.get("x").unwrap().is_none());
    }
}
