use super::{Db, DbError};
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcription {
    pub id: i64,
    pub created_at: i64,
    pub raw_text: String,
    pub final_text: String,
    pub word_count: i64,
    pub duration_ms: i64,
    pub language: String,
    pub engine: String,
    pub model: Option<String>,
    pub app_context: Option<String>,
    pub mode: String,
    pub enhanced: bool,
}

#[derive(Debug, Clone)]
pub struct NewTranscription<'a> {
    pub created_at: i64,
    pub raw_text: &'a str,
    pub final_text: &'a str,
    pub word_count: i64,
    pub duration_ms: i64,
    pub language: &'a str,
    pub engine: &'a str,
    pub model: Option<&'a str>,
    pub app_context: Option<&'a str>,
    pub mode: &'a str,
    pub enhanced: bool,
}

pub struct TranscriptionRepo<'a> {
    db: &'a Db,
}

impl<'a> TranscriptionRepo<'a> {
    pub fn new(db: &'a Db) -> Self {
        Self { db }
    }

    pub fn insert(&self, row: NewTranscription) -> Result<i64, DbError> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO transcriptions
                 (created_at, raw_text, final_text, word_count, duration_ms, language,
                  engine, model, app_context, mode, enhanced)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    row.created_at,
                    row.raw_text,
                    row.final_text,
                    row.word_count,
                    row.duration_ms,
                    row.language,
                    row.engine,
                    row.model,
                    row.app_context,
                    row.mode,
                    row.enhanced as i64
                ],
            )?;
            Ok(c.last_insert_rowid())
        })
    }

    pub fn list_paginated(&self, limit: i64, offset: i64) -> Result<Vec<Transcription>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, created_at, raw_text, final_text, word_count, duration_ms,
                        language, engine, model, app_context, mode, enhanced
                 FROM transcriptions
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?1 OFFSET ?2",
            )?;
            let rows = stmt
                .query_map(params![limit, offset], map_transcription)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }

    pub fn list_by_mode(
        &self,
        mode: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Transcription>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, created_at, raw_text, final_text, word_count, duration_ms,
                        language, engine, model, app_context, mode, enhanced
                 FROM transcriptions
                 WHERE mode = ?1
                 ORDER BY created_at DESC, id DESC
                 LIMIT ?2 OFFSET ?3",
            )?;
            let rows = stmt
                .query_map(params![mode, limit, offset], map_transcription)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }

    pub fn search_fts(&self, query: &str, limit: i64) -> Result<Vec<Transcription>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT t.id, t.created_at, t.raw_text, t.final_text, t.word_count, t.duration_ms,
                        t.language, t.engine, t.model, t.app_context, t.mode, t.enhanced
                 FROM transcriptions_fts f
                 JOIN transcriptions t ON t.id = f.rowid
                 WHERE transcriptions_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )?;
            let rows = stmt
                .query_map(params![query, limit], map_transcription)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }

    pub fn total_word_count(&self) -> Result<i64, DbError> {
        self.db.with_conn(|c| {
            c.query_row(
                "SELECT COALESCE(SUM(word_count),0) FROM transcriptions",
                [],
                |r| r.get(0),
            )
        })
    }

    pub fn delete_to_fit_word_cap(&self, cap: i64) -> Result<usize, DbError> {
        self.db.with_conn_mut(|c| {
            let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let mut total: i64 = tx.query_row(
                "SELECT COALESCE(SUM(word_count),0) FROM transcriptions",
                [], |r| r.get(0),
            )?;
            if total <= cap {
                tx.commit()?;
                return Ok(0);
            }
            let mut deleted = 0usize;
            loop {
                let oldest: Option<(i64, i64)> = tx.query_row(
                    "SELECT id, word_count FROM transcriptions ORDER BY created_at ASC, id ASC LIMIT 1",
                    [], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
                ).ok();
                match oldest {
                    Some((id, wc)) if total > cap => {
                        tx.execute("DELETE FROM transcriptions WHERE id = ?1", [id])?;
                        total -= wc;
                        deleted += 1;
                    }
                    _ => break,
                }
            }
            tx.commit()?;
            Ok(deleted)
        })
    }

    pub fn delete_by_id(&self, id: i64) -> Result<usize, DbError> {
        self.db
            .with_conn(|c| c.execute("DELETE FROM transcriptions WHERE id = ?1", [id]))
    }

    pub fn group_by_day(&self) -> Result<Vec<(String, i64)>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT strftime('%Y-%m-%d', created_at, 'unixepoch') AS day,
                        SUM(word_count)
                 FROM transcriptions
                 GROUP BY day
                 ORDER BY day DESC",
            )?;
            let rows = stmt
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }
}

fn map_transcription(r: &rusqlite::Row) -> rusqlite::Result<Transcription> {
    Ok(Transcription {
        id: r.get(0)?,
        created_at: r.get(1)?,
        raw_text: r.get(2)?,
        final_text: r.get(3)?,
        word_count: r.get(4)?,
        duration_ms: r.get(5)?,
        language: r.get(6)?,
        engine: r.get(7)?,
        model: r.get(8)?,
        app_context: r.get(9)?,
        mode: r.get(10)?,
        enhanced: r.get::<_, i64>(11)? != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    fn sample(created: i64, text: &str, words: i64) -> NewTranscription<'_> {
        NewTranscription {
            created_at: created,
            raw_text: text,
            final_text: text,
            word_count: words,
            duration_ms: 1000,
            language: "pt",
            engine: "local",
            model: Some("turbo"),
            app_context: Some("Notepad"),
            mode: "dictation",
            enhanced: false,
        }
    }

    #[test]
    fn insert_then_list() {
        let db = mem_db();
        let repo = TranscriptionRepo::new(&db);
        let id = repo.insert(sample(1000, "ola mundo", 2)).unwrap();
        assert!(id > 0);
        let rows = repo.list_paginated(10, 0).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].final_text, "ola mundo");
    }

    #[test]
    fn list_is_descending_by_created_at() {
        let db = mem_db();
        let repo = TranscriptionRepo::new(&db);
        repo.insert(sample(100, "older", 1)).unwrap();
        repo.insert(sample(200, "newer", 1)).unwrap();
        let rows = repo.list_paginated(10, 0).unwrap();
        assert_eq!(rows[0].final_text, "newer");
        assert_eq!(rows[1].final_text, "older");
    }

    #[test]
    fn list_by_mode_returns_only_matching_rows() {
        let db = mem_db();
        let repo = TranscriptionRepo::new(&db);
        repo.insert(sample(100, "dictation", 1)).unwrap();
        let mut command = sample(200, "draft", 1);
        command.mode = "command";
        command.enhanced = true;
        repo.insert(command).unwrap();

        let rows = repo.list_by_mode("command", 10, 0).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].final_text, "draft");
        assert_eq!(rows[0].mode, "command");
    }

    #[test]
    fn fts_search_matches_final_text() {
        let db = mem_db();
        let repo = TranscriptionRepo::new(&db);
        repo.insert(sample(1, "hello world", 2)).unwrap();
        repo.insert(sample(2, "unrelated", 1)).unwrap();
        let hits = repo.search_fts("hello", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].final_text, "hello world");
    }

    #[test]
    fn fts_search_matches_app_context() {
        let db = mem_db();
        let repo = TranscriptionRepo::new(&db);
        let mut row = sample(1, "body", 1);
        row.app_context = Some("VSCode");
        repo.insert(row).unwrap();
        let hits = repo.search_fts("VSCode", 10).unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn purge_deletes_oldest_until_cap() {
        let db = mem_db();
        let repo = TranscriptionRepo::new(&db);
        repo.insert(sample(1, "a", 100)).unwrap();
        repo.insert(sample(2, "b", 100)).unwrap();
        repo.insert(sample(3, "c", 100)).unwrap();
        assert_eq!(repo.total_word_count().unwrap(), 300);
        let deleted = repo.delete_to_fit_word_cap(150).unwrap();
        assert_eq!(deleted, 2); // oldest two removed, 100 words left
        let remaining = repo.list_paginated(10, 0).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].final_text, "c");
        // FTS must also be purged via trigger
        let hits = repo.search_fts("a", 10).unwrap();
        assert!(
            hits.is_empty(),
            "FTS rows for deleted transcriptions should be gone"
        );
    }

    #[test]
    fn delete_by_id_removes_row_and_fts() {
        let db = mem_db();
        let repo = TranscriptionRepo::new(&db);
        let id = repo.insert(sample(1, "doomed", 1)).unwrap();
        repo.insert(sample(2, "keeper", 1)).unwrap();
        let n = repo.delete_by_id(id).unwrap();
        assert_eq!(n, 1);
        let rows = repo.list_paginated(10, 0).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].final_text, "keeper");
        let hits = repo.search_fts("doomed", 10).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn group_by_day_buckets_correctly() {
        let db = mem_db();
        let repo = TranscriptionRepo::new(&db);
        // 2026-04-23 00:00 UTC = 1776902400
        repo.insert(sample(1776902400, "a", 1)).unwrap();
        repo.insert(sample(1776902500, "b", 1)).unwrap();
        // next day
        repo.insert(sample(1776988800, "c", 1)).unwrap();
        let groups = repo.group_by_day().unwrap();
        assert_eq!(groups.len(), 2);
    }
}
