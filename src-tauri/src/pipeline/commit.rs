//! Final commit stage of the dictation pipeline.
//!
//! Writes the finished transcription row and bumps the corresponding
//! `stats_daily` bucket inside a single SQLite transaction. After the
//! transaction commits, performs a best-effort word-cap purge OUTSIDE
//! the transaction so the purge cannot deadlock against the same
//! `Mutex<Connection>` (the repo's `delete_to_fit_word_cap` takes its
//! own `with_conn_mut` lock).
//!
//! Phase 2: Dictation arm only. Command Mode lands in Phase 4.

use crate::settings::Settings;
use crate::storage::transcriptions::TranscriptionRepo;
use crate::storage::{Db, DbError};
use rusqlite::params;
use time::format_description::well_known::Iso8601;
use time::OffsetDateTime;

/// Plain data carrier for one finished session, ready to persist.
///
/// String fields use `String` (not `Option<String>`) because the
/// `transcriptions` table declares `language TEXT NOT NULL` and Phase 2
/// always knows the engine/model/mode. Use empty strings for unknowns
/// (e.g. `language = ""` when language detection produced nothing).
#[derive(Debug, Clone)]
pub struct TranscriptionRecord {
    pub raw_text: String,
    pub final_text: String,
    pub word_count: i64,
    pub duration_ms: i64,
    /// ISO-639-1 code, or empty string if unknown.
    pub language: String,
    /// "local" | "cloud".
    pub engine: String,
    /// "turbo", "groq:whisper-large-v3", etc. Always non-empty in Phase 2.
    pub model: String,
    /// Foreground app identifier; empty in Phase 2 (stored as NULL).
    pub app_context: String,
    /// "dictation" in Phase 2.
    pub mode: String,
    /// Whether the LLM enhancement pass ran. Phase 2 always `false`.
    pub enhanced: bool,
}

/// Persist `record` and bump today's stats atomically, then run a
/// best-effort word-cap purge if `settings.data.purge_on_exceed`.
///
/// Returns the new `transcriptions.id`.
///
/// The cap purge is intentionally OUTSIDE the insert transaction:
/// `TranscriptionRepo::delete_to_fit_word_cap` uses its own
/// `with_conn_mut`, so calling it from inside another `with_conn_mut`
/// would deadlock on the same connection mutex. As a consequence the
/// purge is not atomic with the insert; an interrupted purge just
/// leaves over-cap rows for the next session to clean up.
pub fn commit_session(
    db: &Db,
    record: TranscriptionRecord,
    settings: &Settings,
) -> Result<i64, DbError> {
    let now_dt = OffsetDateTime::now_utc();
    let now = now_dt.unix_timestamp();
    let today = now_dt.format(&Iso8601::DATE).map_err(|e| {
        DbError::Sqlite(rusqlite::Error::InvalidParameterName(format!(
            "date format: {e}"
        )))
    })?;

    // app_context column is nullable; pass NULL for empty to keep rows
    // tidy and to avoid storing the literal "" sentinel.
    let app_context_opt: Option<&str> = if record.app_context.is_empty() {
        None
    } else {
        Some(record.app_context.as_str())
    };
    // model column is nullable but Phase 2 always supplies a model.
    let model_opt: Option<&str> = if record.model.is_empty() {
        None
    } else {
        Some(record.model.as_str())
    };

    let row_id = db.with_conn_mut(|c| {
        let tx = c.transaction()?;

        // Insert SQL mirrors `TranscriptionRepo::insert` so we stay
        // within the same transaction without re-acquiring the mutex.
        tx.execute(
            "INSERT INTO transcriptions
             (created_at, raw_text, final_text, word_count, duration_ms, language,
              engine, model, app_context, mode, enhanced)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                now,
                record.raw_text,
                record.final_text,
                record.word_count,
                record.duration_ms,
                record.language,
                record.engine,
                model_opt,
                app_context_opt,
                record.mode,
                record.enhanced as i64,
            ],
        )?;
        let id = tx.last_insert_rowid();

        // Stats bump SQL mirrors `StatsRepo::bump_day`. We cannot call
        // the repo here — it would re-lock the same mutex.
        tx.execute(
            "INSERT INTO stats_daily (day, word_count, session_count, total_duration_ms)
             VALUES (?1, ?2, 1, ?3)
             ON CONFLICT(day) DO UPDATE SET
               word_count = word_count + excluded.word_count,
               session_count = session_count + 1,
               total_duration_ms = total_duration_ms + excluded.total_duration_ms",
            params![today, record.word_count, record.duration_ms],
        )?;

        tx.commit()?;
        Ok(id)
    })?;

    // Best-effort cap purge OUTSIDE the transaction. The cast from
    // u64 → i64 is safe in practice: word_count_cap defaults to
    // 500_000 and the UI clamps it well below 2^63 - 1.
    if settings.data.purge_on_exceed {
        let cap = settings.data.word_count_cap as i64;
        let repo = TranscriptionRepo::new(db);
        let total = repo.total_word_count()?;
        if total > cap {
            repo.delete_to_fit_word_cap(cap)?;
        }
    }

    Ok(row_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::stats::StatsRepo;
    use crate::storage::test_util::mem_db;
    use crate::storage::transcriptions::TranscriptionRepo;

    fn record(text: &str, words: i64, duration_ms: i64) -> TranscriptionRecord {
        TranscriptionRecord {
            raw_text: text.to_string(),
            final_text: text.to_string(),
            word_count: words,
            duration_ms,
            language: "pt".to_string(),
            engine: "local".to_string(),
            model: "turbo".to_string(),
            app_context: String::new(),
            mode: "dictation".to_string(),
            enhanced: false,
        }
    }

    #[test]
    fn inserts_row_and_bumps_stats() {
        let db = mem_db();
        let settings = Settings::default();

        let id = commit_session(&db, record("ola mundo", 2, 1500), &settings).unwrap();
        assert!(id > 0, "returned row id must be positive");

        let totals = StatsRepo::new(&db).totals().unwrap();
        assert_eq!(totals.word_count, 2);
        assert_eq!(totals.session_count, 1);
        assert_eq!(totals.total_duration_ms, 1500);

        let rows = TranscriptionRepo::new(&db).list_paginated(10, 0).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].final_text, "ola mundo");
    }

    #[test]
    fn purges_when_cap_exceeded() {
        let db = mem_db();
        let mut settings = Settings::default();
        settings.data.word_count_cap = 5;
        settings.data.purge_on_exceed = true;

        for i in 0..10 {
            let text = format!("a b {i}"); // not used as cap input
            commit_session(&db, record(&text, 2, 100), &settings).unwrap();
        }

        let total = TranscriptionRepo::new(&db).total_word_count().unwrap();
        assert!(total <= 5, "purge should keep total under cap, got {total}");
    }

    #[test]
    fn skips_purge_when_disabled() {
        let db = mem_db();
        let mut settings = Settings::default();
        settings.data.word_count_cap = 5;
        settings.data.purge_on_exceed = false;

        for i in 0..10 {
            let text = format!("a b {i}");
            commit_session(&db, record(&text, 2, 100), &settings).unwrap();
        }

        let total = TranscriptionRepo::new(&db).total_word_count().unwrap();
        assert!(total > 5, "purge disabled, expected total > 5, got {total}");
        assert_eq!(total, 20, "all 10 rows of 2 words each should remain");
    }

    #[test]
    fn empty_app_context_stored_as_null() {
        let db = mem_db();
        let settings = Settings::default();
        commit_session(&db, record("hello", 1, 100), &settings).unwrap();

        let rows = TranscriptionRepo::new(&db).list_paginated(10, 0).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(
            rows[0].app_context.is_none(),
            "empty app_context should map to SQL NULL, got {:?}",
            rows[0].app_context
        );
    }
}
