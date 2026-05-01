use super::{Db, DbError};
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryTerm {
    pub id: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub term: String,
    pub replacement: Option<String>,
    pub is_abbreviation: bool,
    pub auto_added: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct NewDictionaryTerm<'a> {
    pub term: &'a str,
    pub replacement: Option<&'a str>,
    pub is_abbreviation: bool,
    pub auto_added: bool,
    pub enabled: bool,
}

pub struct DictionaryRepo<'a> { db: &'a Db }

impl<'a> DictionaryRepo<'a> {
    pub fn new(db: &'a Db) -> Self { Self { db } }

    pub fn upsert(&self, now: i64, row: NewDictionaryTerm) -> Result<i64, DbError> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO dictionary_terms
                   (created_at, updated_at, term, replacement, is_abbreviation, auto_added, enabled)
                 VALUES (?1,?1,?2,?3,?4,?5,?6)
                 ON CONFLICT(term) DO UPDATE SET
                   replacement=excluded.replacement,
                   is_abbreviation=excluded.is_abbreviation,
                   auto_added=excluded.auto_added,
                   enabled=excluded.enabled,
                   updated_at=?1",
                params![
                    now, row.term, row.replacement,
                    row.is_abbreviation as i64, row.auto_added as i64, row.enabled as i64
                ],
            )?;
            c.query_row(
                "SELECT id FROM dictionary_terms WHERE term = ?1",
                [row.term], |r| r.get(0),
            )
        })
    }

    pub fn list(&self) -> Result<Vec<DictionaryTerm>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, created_at, updated_at, term, replacement,
                        is_abbreviation, auto_added, enabled
                 FROM dictionary_terms ORDER BY term ASC",
            )?;
            let rows = stmt.query_map([], map_term)?.collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }

    pub fn delete(&self, id: i64) -> Result<usize, DbError> {
        self.db.with_conn(|c| c.execute("DELETE FROM dictionary_terms WHERE id = ?1", [id]))
    }

    pub fn find_matches(&self, terms: &[&str]) -> Result<Vec<DictionaryTerm>, DbError> {
        if terms.is_empty() { return Ok(vec![]); }
        self.db.with_conn(|c| {
            let placeholders = std::iter::repeat_n("?", terms.len()).collect::<Vec<_>>().join(",");
            let sql = format!(
                "SELECT id, created_at, updated_at, term, replacement,
                        is_abbreviation, auto_added, enabled
                 FROM dictionary_terms
                 WHERE enabled = 1 AND term IN ({placeholders})"
            );
            let mut stmt = c.prepare(&sql)?;
            let params = rusqlite::params_from_iter(terms.iter());
            let rows = stmt.query_map(params, map_term)?.collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }
}

fn map_term(r: &rusqlite::Row) -> rusqlite::Result<DictionaryTerm> {
    Ok(DictionaryTerm {
        id: r.get(0)?,
        created_at: r.get(1)?,
        updated_at: r.get(2)?,
        term: r.get(3)?,
        replacement: r.get(4)?,
        is_abbreviation: r.get::<_, i64>(5)? != 0,
        auto_added: r.get::<_, i64>(6)? != 0,
        enabled: r.get::<_, i64>(7)? != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    #[test]
    fn upsert_inserts_then_updates() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        let id1 = repo.upsert(100, NewDictionaryTerm {
            term: "tauri", replacement: Some("Tauri"),
            is_abbreviation: false, auto_added: false, enabled: true,
        }).unwrap();
        let id2 = repo.upsert(200, NewDictionaryTerm {
            term: "tauri", replacement: Some("TAURI"),
            is_abbreviation: false, auto_added: false, enabled: true,
        }).unwrap();
        assert_eq!(id1, id2, "same term should upsert same row");
        let rows = repo.list().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].replacement.as_deref(), Some("TAURI"));
        assert_eq!(rows[0].updated_at, 200);
    }

    #[test]
    fn delete_removes_row() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        let id = repo.upsert(1, NewDictionaryTerm {
            term: "foo", replacement: None, is_abbreviation: false,
            auto_added: false, enabled: true,
        }).unwrap();
        let n = repo.delete(id).unwrap();
        assert_eq!(n, 1);
        assert!(repo.list().unwrap().is_empty());
    }

    #[test]
    fn find_matches_returns_only_requested_terms() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        repo.upsert(1, NewDictionaryTerm {
            term: "alpha", replacement: Some("Alpha"),
            is_abbreviation: false, auto_added: false, enabled: true,
        }).unwrap();
        repo.upsert(1, NewDictionaryTerm {
            term: "beta", replacement: Some("Beta"),
            is_abbreviation: false, auto_added: false, enabled: true,
        }).unwrap();
        repo.upsert(1, NewDictionaryTerm {
            term: "gamma", replacement: None,
            is_abbreviation: false, auto_added: false, enabled: false,
        }).unwrap();
        let hits = repo.find_matches(&["alpha", "gamma"]).unwrap();
        assert_eq!(hits.len(), 1, "disabled terms must be excluded");
        assert_eq!(hits[0].term, "alpha");
    }
}
