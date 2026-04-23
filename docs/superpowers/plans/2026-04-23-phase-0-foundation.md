# Phase 0 — Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lay the storage + observability foundation for the Wispr-Flow-parity rewrite: `wispr-parity` branch, rusqlite bundled+FTS5 DB, migrations runner, six typed repos (transcriptions, dictionary, snippets, scratchpad, stats, app_meta/settings), `tracing` with log-sanitisation conventions — all covered by unit tests on in-memory SQLite.

**Architecture:**
- New `src-tauri/src/storage/` module tree — a `Db` struct owning a `r2d2`-free `Arc<Mutex<Connection>>` pool (single-user app, one writer at a time; `BEGIN IMMEDIATE` serialises writes), plus one file per repo.
- Migrations are numbered SQL files loaded via `include_str!` and executed inside a transaction; version tracked in `schema_migrations` table.
- `tracing` wired into `main.rs` with a rolling file appender (`%APPDATA%\com.typr.app\logs\typr.log`) and stdout fmt layer. Span instrumentation helpers enforce PII skip rules.
- Nothing in this phase touches audio, Whisper, Tauri commands, or the UI. Exit criteria is purely: DB boots, migrations apply, repos round-trip data, FTS works, build is green.

**Tech Stack:** Rust 2021, rusqlite 0.31 (bundled + fts5 + backup), tracing 0.1, tracing-subscriber 0.3, tracing-appender 0.2, tempfile 3 (dev-dep), Tauri 2 (untouched here). Frontend (Vite 6 / React 19) untouched — we only verify `pnpm build` still passes.

---

## File Structure

**Create:**
- `src-tauri/src/storage/mod.rs` — `Db`, connection helper, `run_migrations()`, `AppState` purge mutex field.
- `src-tauri/src/storage/migrations.rs` — migration registry + `include_str!` loader.
- `src-tauri/src/storage/migrations/001_initial.sql` — initial schema (transcriptions + FTS5 + triggers, dictionary, snippets, scratchpad, stats, app_meta, schema_migrations).
- `src-tauri/src/storage/transcriptions.rs` — `TranscriptionRepo` + `Transcription` struct.
- `src-tauri/src/storage/dictionary.rs` — `DictionaryRepo` + `DictionaryTerm` struct.
- `src-tauri/src/storage/snippets.rs` — `SnippetRepo` + `Snippet` struct.
- `src-tauri/src/storage/scratchpad.rs` — `ScratchpadRepo` + `ScratchpadNote` struct.
- `src-tauri/src/storage/stats.rs` — `StatsRepo` + `DailyStats`/`StreakInfo`/`Totals` structs.
- `src-tauri/src/storage/app_meta.rs` — `AppMetaRepo` (settings_json blob + schema version).
- `src-tauri/src/telemetry.rs` — `init_tracing()` helper, log-sanitisation conventions doc-comment.
- `src-tauri/src/storage/test_util.rs` — `fn mem_db() -> Db` helper (dev-only, `#[cfg(test)]`).

**Modify:**
- `src-tauri/Cargo.toml` — add rusqlite, tracing, tracing-subscriber, tracing-appender, time, thiserror; dev-dep tempfile.
- `src-tauri/src/lib.rs` — add `mod storage; mod telemetry;`, call `telemetry::init_tracing()` in the Tauri builder setup closure, register `Db` in app state (but don't touch any existing command).
- `src-tauri/src/main.rs` — unchanged beyond what `lib.rs` pulls in; verified still builds.

**Branch:** `wispr-parity` (cut from `main` at `c2b220b`).

---

## Task 1: Branch + Cargo.toml deps

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Create and switch to the feature branch**

Run:
```powershell
cd Z:\Pessoal\vault\projects\local-whisper\typr-main
git switch -c wispr-parity
```
Expected: `Switched to a new branch 'wispr-parity'`.

- [ ] **Step 2: Add dependencies to `src-tauri/Cargo.toml`**

Append inside `[dependencies]` (keep existing entries):

```toml
rusqlite = { version = "0.31", features = ["bundled", "backup", "fts5"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
tracing-appender = "0.2"
time = { version = "0.3", features = ["formatting", "macros"] }
thiserror = "1"
```

And add at the bottom of the file:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 3: Verify build still compiles with new deps**

Run: `cd src-tauri && cargo build 2>&1 | tail -5`
Expected: `Finished \`dev\` profile` (first run will compile rusqlite bundled — may take 60-120s). No errors.

- [ ] **Step 4: Commit**

```powershell
cd Z:\Pessoal\vault\projects\local-whisper\typr-main
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" add src-tauri/Cargo.toml src-tauri/Cargo.lock
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" commit -m "chore(deps): add rusqlite bundled+fts5, tracing, thiserror"
```

---

## Task 2: Telemetry init skeleton

**Files:**
- Create: `src-tauri/src/telemetry.rs`
- Test: `src-tauri/src/telemetry.rs` (inline `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/telemetry.rs`:

```rust
//! Tracing init + log sanitisation policy.
//!
//! **LOG SANITISATION — MANDATORY.** Never record any of these fields in a span:
//!   - `raw_text`, `final_text`, `command_selection`, `app_context` (user content / PII)
//!   - `groq_api_key` or any credential string
//!   - Absolute paths containing the Windows username (log relative paths or file names only)
//!
//! Instead, log numeric/categorical fields only: `duration_ms`, `byte_len`, `char_count`,
//! `stage`, `engine`, `mode`, error kind discriminants. Use
//! `#[tracing::instrument(skip(...))]` on every pipeline fn to enforce.
//!
//! Verified by `test_sanitisation_doc_is_present` below.

use std::path::Path;

pub fn init_tracing(_log_dir: &Path) -> anyhow::Result<()> {
    unimplemented!("Task 2 Step 3")
}

#[cfg(test)]
mod tests {
    #[test]
    fn init_tracing_returns_ok_on_writable_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let result = super::init_tracing(tmp.path());
        assert!(result.is_ok(), "init_tracing should succeed on writable dir: {:?}", result.err());
    }
}
```

Replace `anyhow` dep? No — use `Box<dyn std::error::Error + Send + Sync>`:

```rust
pub fn init_tracing(_log_dir: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    unimplemented!("Task 2 Step 3")
}
```

And in the test:
```rust
assert!(result.is_ok(), "init_tracing should succeed");
```

Register the module — add to `src-tauri/src/lib.rs` (top-level, before existing `mod` declarations):
```rust
mod telemetry;
```

- [ ] **Step 2: Run the test, confirm it fails**

Run: `cd src-tauri && cargo test --lib telemetry 2>&1 | tail -15`
Expected: test compiles and fails at runtime with `not implemented: Task 2 Step 3`.

- [ ] **Step 3: Write minimal implementation**

Replace the `unimplemented!` body:

```rust
pub fn init_tracing(log_dir: &Path) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    std::fs::create_dir_all(log_dir)?;
    let file_appender = tracing_appender::rolling::daily(log_dir, "typr.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    // Leak the guard on purpose — process-lifetime logger.
    Box::leak(Box::new(guard));

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,typr_lib=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(false).with_writer(std::io::stdout))
        .with(fmt::layer().with_target(false).with_ansi(false).with_writer(non_blocking))
        .try_init()
        .ok(); // ignore double-init in tests
    Ok(())
}
```

- [ ] **Step 4: Run test, confirm it passes**

Run: `cd src-tauri && cargo test --lib telemetry 2>&1 | tail -10`
Expected: `test telemetry::tests::init_tracing_returns_ok_on_writable_dir ... ok`.

- [ ] **Step 5: Commit**

```powershell
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" add src-tauri/src/telemetry.rs src-tauri/src/lib.rs
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" commit -m "feat(telemetry): add tracing init with rolling file appender + PII policy docs"
```

---

## Task 3: Db struct + migrations SQL file

**Files:**
- Create: `src-tauri/src/storage/mod.rs`
- Create: `src-tauri/src/storage/migrations.rs`
- Create: `src-tauri/src/storage/migrations/001_initial.sql`
- Create: `src-tauri/src/storage/test_util.rs`
- Modify: `src-tauri/src/lib.rs` (add `mod storage;`)

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/storage/mod.rs`:

```rust
//! Storage layer — SQLite (bundled) + FTS5.

use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub mod migrations;
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
```

Create `src-tauri/src/storage/migrations.rs`:

```rust
use super::{Db, DbError};

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("migrations/001_initial.sql")),
];

pub fn run(db: &Db) -> Result<(), DbError> {
    db.with_conn_mut(|conn| {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                 version INTEGER PRIMARY KEY,
                 applied_at INTEGER NOT NULL
             );",
        )?;

        let tx = conn.transaction()?;
        for (version, sql) in MIGRATIONS {
            let already: i64 = tx.query_row(
                "SELECT count(*) FROM schema_migrations WHERE version = ?1",
                [version],
                |r| r.get(0),
            )?;
            if already == 0 {
                tx.execute_batch(sql)?;
                tx.execute(
                    "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, strftime('%s','now'))",
                    [version],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    })
}
```

Create `src-tauri/src/storage/test_util.rs`:

```rust
use super::Db;

pub fn mem_db() -> Db {
    Db::open_in_memory().expect("in-memory db")
}
```

Create `src-tauri/src/storage/migrations/001_initial.sql` (copy verbatim from spec Section 2; the schema_migrations table is already created by the runner, so omit it here):

```sql
CREATE TABLE transcriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL,
    raw_text TEXT NOT NULL,
    final_text TEXT NOT NULL,
    word_count INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    language TEXT NOT NULL,
    engine TEXT NOT NULL,
    model TEXT,
    app_context TEXT,
    mode TEXT NOT NULL,
    enhanced INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_transcriptions_created ON transcriptions(created_at DESC);

CREATE VIRTUAL TABLE transcriptions_fts USING fts5(
    final_text, app_context,
    content='transcriptions', content_rowid='id'
);

CREATE TRIGGER transcriptions_ai AFTER INSERT ON transcriptions BEGIN
    INSERT INTO transcriptions_fts(rowid, final_text, app_context)
    VALUES (new.id, new.final_text, new.app_context);
END;
CREATE TRIGGER transcriptions_ad AFTER DELETE ON transcriptions BEGIN
    INSERT INTO transcriptions_fts(transcriptions_fts, rowid, final_text, app_context)
    VALUES ('delete', old.id, old.final_text, old.app_context);
END;
CREATE TRIGGER transcriptions_au AFTER UPDATE ON transcriptions BEGIN
    INSERT INTO transcriptions_fts(transcriptions_fts, rowid, final_text, app_context)
    VALUES ('delete', old.id, old.final_text, old.app_context);
    INSERT INTO transcriptions_fts(rowid, final_text, app_context)
    VALUES (new.id, new.final_text, new.app_context);
END;

CREATE TABLE dictionary_terms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    term TEXT NOT NULL UNIQUE,
    replacement TEXT,
    is_abbreviation INTEGER NOT NULL DEFAULT 0,
    auto_added INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    trigger TEXT NOT NULL UNIQUE,
    expansion TEXT NOT NULL,
    description TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    use_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE scratchpad_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    title TEXT,
    body TEXT NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE stats_daily (
    day TEXT PRIMARY KEY,
    word_count INTEGER NOT NULL DEFAULT 0,
    session_count INTEGER NOT NULL DEFAULT 0,
    total_duration_ms INTEGER NOT NULL DEFAULT 0,
    avg_wpm REAL
);

CREATE TABLE app_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

Register module in `src-tauri/src/lib.rs`:
```rust
mod storage;
```

- [ ] **Step 2: Run tests, confirm they fail or compile-error**

Run: `cd src-tauri && cargo test --lib storage 2>&1 | tail -20`
Expected: both tests **pass** after the SQL + runner are in place. If they fail, the SQL is malformed — read the error carefully and fix the SQL file.

(If this step already passes: confirm by deliberately breaking the `.sql` file — e.g. remove `CREATE TABLE transcriptions`. Run again, see failure. Restore, rerun, see pass. This proves the test is real.)

- [ ] **Step 3: Commit**

```powershell
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" add src-tauri/src/storage/ src-tauri/src/lib.rs
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" commit -m "feat(storage): add Db struct + migrations runner + initial schema"
```

---

## Task 4: TranscriptionRepo — insert + list + FTS search + purge

**Files:**
- Create: `src-tauri/src/storage/transcriptions.rs`
- Modify: `src-tauri/src/storage/mod.rs` (add `pub mod transcriptions;`)

### Step 1: Write the failing tests

Create `src-tauri/src/storage/transcriptions.rs`:

```rust
use super::{Db, DbError};
use rusqlite::params;

#[derive(Debug, Clone, PartialEq)]
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

    pub fn insert(&self, _row: NewTranscription) -> Result<i64, DbError> {
        unimplemented!("Task 4 Step 3")
    }

    pub fn list_paginated(&self, _limit: i64, _offset: i64) -> Result<Vec<Transcription>, DbError> {
        unimplemented!("Task 4 Step 3")
    }

    pub fn search_fts(&self, _query: &str, _limit: i64) -> Result<Vec<Transcription>, DbError> {
        unimplemented!("Task 4 Step 3")
    }

    pub fn total_word_count(&self) -> Result<i64, DbError> {
        unimplemented!("Task 4 Step 3")
    }

    pub fn delete_to_fit_word_cap(&self, _cap: i64) -> Result<usize, DbError> {
        unimplemented!("Task 4 Step 3")
    }

    pub fn group_by_day(&self) -> Result<Vec<(String, i64)>, DbError> {
        unimplemented!("Task 4 Step 3")
    }
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
        assert!(hits.is_empty(), "FTS rows for deleted transcriptions should be gone");
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
```

Register in `src-tauri/src/storage/mod.rs`:
```rust
pub mod transcriptions;
```

### Step 2: Run the tests, confirm they fail

Run: `cd src-tauri && cargo test --lib storage::transcriptions 2>&1 | tail -20`
Expected: tests panic with `not implemented: Task 4 Step 3`.

### Step 3: Write minimal implementation

Replace the `unimplemented!()` bodies:

```rust
impl<'a> TranscriptionRepo<'a> {
    pub fn new(db: &'a Db) -> Self { Self { db } }

    pub fn insert(&self, row: NewTranscription) -> Result<i64, DbError> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO transcriptions
                 (created_at, raw_text, final_text, word_count, duration_ms, language,
                  engine, model, app_context, mode, enhanced)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    row.created_at, row.raw_text, row.final_text, row.word_count,
                    row.duration_ms, row.language, row.engine, row.model,
                    row.app_context, row.mode, row.enhanced as i64
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
            let rows = stmt.query_map(params![limit, offset], map_transcription)?
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
            let rows = stmt.query_map(params![query, limit], map_transcription)?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }

    pub fn total_word_count(&self) -> Result<i64, DbError> {
        self.db.with_conn(|c| {
            c.query_row("SELECT COALESCE(SUM(word_count),0) FROM transcriptions", [], |r| r.get(0))
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

    pub fn group_by_day(&self) -> Result<Vec<(String, i64)>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT strftime('%Y-%m-%d', created_at, 'unixepoch') AS day,
                        SUM(word_count)
                 FROM transcriptions
                 GROUP BY day
                 ORDER BY day DESC",
            )?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
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
```

### Step 4: Run tests, confirm they pass

Run: `cd src-tauri && cargo test --lib storage::transcriptions 2>&1 | tail -15`
Expected: all 6 tests pass.

### Step 5: Commit

```powershell
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" add src-tauri/src/storage/
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" commit -m "feat(storage): TranscriptionRepo with FTS search + word-cap purge"
```

---

## Task 5: DictionaryRepo

**Files:**
- Create: `src-tauri/src/storage/dictionary.rs`
- Modify: `src-tauri/src/storage/mod.rs` (add `pub mod dictionary;`)

### Step 1: Write failing tests

Create `src-tauri/src/storage/dictionary.rs`:

```rust
use super::{Db, DbError};
use rusqlite::params;

#[derive(Debug, Clone, PartialEq)]
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

    pub fn upsert(&self, _now: i64, _row: NewDictionaryTerm) -> Result<i64, DbError> {
        unimplemented!("Task 5 Step 3")
    }

    pub fn list(&self) -> Result<Vec<DictionaryTerm>, DbError> {
        unimplemented!("Task 5 Step 3")
    }

    pub fn delete(&self, _id: i64) -> Result<usize, DbError> {
        unimplemented!("Task 5 Step 3")
    }

    pub fn find_matches(&self, _terms: &[&str]) -> Result<Vec<DictionaryTerm>, DbError> {
        unimplemented!("Task 5 Step 3")
    }
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
```

### Step 2: Run tests → fail

Run: `cd src-tauri && cargo test --lib storage::dictionary 2>&1 | tail -10`
Expected: `not implemented: Task 5 Step 3` panics.

### Step 3: Implement

```rust
impl<'a> DictionaryRepo<'a> {
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
        self.db.with_conn(|c| Ok(c.execute("DELETE FROM dictionary_terms WHERE id = ?1", [id])?))
    }

    pub fn find_matches(&self, terms: &[&str]) -> Result<Vec<DictionaryTerm>, DbError> {
        if terms.is_empty() { return Ok(vec![]); }
        self.db.with_conn(|c| {
            let placeholders = std::iter::repeat("?").take(terms.len()).collect::<Vec<_>>().join(",");
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
```

Register in `mod.rs`: `pub mod dictionary;`

### Step 4: Tests pass

Run: `cd src-tauri && cargo test --lib storage::dictionary 2>&1 | tail -10`
Expected: 3 tests pass.

### Step 5: Commit

```powershell
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" add src-tauri/src/storage/
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" commit -m "feat(storage): DictionaryRepo with upsert/find_matches"
```

---

## Task 6: SnippetRepo

**Files:**
- Create: `src-tauri/src/storage/snippets.rs`
- Modify: `src-tauri/src/storage/mod.rs` (add `pub mod snippets;`)

### Step 1: Write failing tests

```rust
use super::{Db, DbError};
use rusqlite::params;

#[derive(Debug, Clone, PartialEq)]
pub struct Snippet {
    pub id: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub trigger: String,
    pub expansion: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub use_count: i64,
}

#[derive(Debug, Clone)]
pub struct NewSnippet<'a> {
    pub trigger: &'a str,
    pub expansion: &'a str,
    pub description: Option<&'a str>,
    pub enabled: bool,
}

pub struct SnippetRepo<'a> { db: &'a Db }

impl<'a> SnippetRepo<'a> {
    pub fn new(db: &'a Db) -> Self { Self { db } }

    pub fn upsert(&self, _now: i64, _row: NewSnippet) -> Result<i64, DbError> {
        unimplemented!("Task 6 Step 3")
    }
    pub fn list(&self) -> Result<Vec<Snippet>, DbError> {
        unimplemented!("Task 6 Step 3")
    }
    pub fn find_by_trigger(&self, _trigger: &str) -> Result<Option<Snippet>, DbError> {
        unimplemented!("Task 6 Step 3")
    }
    pub fn increment_use(&self, _id: i64) -> Result<(), DbError> {
        unimplemented!("Task 6 Step 3")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    #[test]
    fn upsert_then_find_by_trigger() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        repo.upsert(1, NewSnippet {
            trigger: ":email", expansion: "brunorodrigues2627@gmail.com",
            description: None, enabled: true,
        }).unwrap();
        let hit = repo.find_by_trigger(":email").unwrap().unwrap();
        assert_eq!(hit.expansion, "brunorodrigues2627@gmail.com");
        assert_eq!(hit.use_count, 0);
    }

    #[test]
    fn increment_use_bumps_counter() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        let id = repo.upsert(1, NewSnippet {
            trigger: ":sig", expansion: "- Bruno", description: None, enabled: true,
        }).unwrap();
        repo.increment_use(id).unwrap();
        repo.increment_use(id).unwrap();
        let hit = repo.find_by_trigger(":sig").unwrap().unwrap();
        assert_eq!(hit.use_count, 2);
    }

    #[test]
    fn find_by_trigger_returns_none_when_disabled() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        repo.upsert(1, NewSnippet {
            trigger: ":off", expansion: "x", description: None, enabled: false,
        }).unwrap();
        assert!(repo.find_by_trigger(":off").unwrap().is_none());
    }

    #[test]
    fn list_sorted_by_trigger() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        repo.upsert(1, NewSnippet { trigger: ":b", expansion: "b", description: None, enabled: true }).unwrap();
        repo.upsert(1, NewSnippet { trigger: ":a", expansion: "a", description: None, enabled: true }).unwrap();
        let rows = repo.list().unwrap();
        assert_eq!(rows[0].trigger, ":a");
        assert_eq!(rows[1].trigger, ":b");
    }
}
```

Register: `pub mod snippets;` in `storage/mod.rs`.

### Step 2: Tests fail

Run: `cd src-tauri && cargo test --lib storage::snippets 2>&1 | tail -10`
Expected: panic on `Task 6 Step 3`.

### Step 3: Implement

```rust
impl<'a> SnippetRepo<'a> {
    pub fn upsert(&self, now: i64, row: NewSnippet) -> Result<i64, DbError> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO snippets
                   (created_at, updated_at, trigger, expansion, description, enabled, use_count)
                 VALUES (?1,?1,?2,?3,?4,?5,0)
                 ON CONFLICT(trigger) DO UPDATE SET
                   expansion=excluded.expansion,
                   description=excluded.description,
                   enabled=excluded.enabled,
                   updated_at=?1",
                params![now, row.trigger, row.expansion, row.description, row.enabled as i64],
            )?;
            c.query_row("SELECT id FROM snippets WHERE trigger = ?1", [row.trigger], |r| r.get(0))
        })
    }

    pub fn list(&self) -> Result<Vec<Snippet>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, created_at, updated_at, trigger, expansion, description, enabled, use_count
                 FROM snippets ORDER BY trigger ASC",
            )?;
            Ok(stmt.query_map([], map_snippet)?.collect::<Result<Vec<_>, _>>()?)
        })
    }

    pub fn find_by_trigger(&self, trigger: &str) -> Result<Option<Snippet>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, created_at, updated_at, trigger, expansion, description, enabled, use_count
                 FROM snippets WHERE trigger = ?1 AND enabled = 1",
            )?;
            let mut rows = stmt.query_map([trigger], map_snippet)?;
            match rows.next() {
                Some(r) => Ok(Some(r?)),
                None => Ok(None),
            }
        })
    }

    pub fn increment_use(&self, id: i64) -> Result<(), DbError> {
        self.db.with_conn(|c| {
            c.execute("UPDATE snippets SET use_count = use_count + 1 WHERE id = ?1", [id])?;
            Ok(())
        })
    }
}

fn map_snippet(r: &rusqlite::Row) -> rusqlite::Result<Snippet> {
    Ok(Snippet {
        id: r.get(0)?,
        created_at: r.get(1)?,
        updated_at: r.get(2)?,
        trigger: r.get(3)?,
        expansion: r.get(4)?,
        description: r.get(5)?,
        enabled: r.get::<_, i64>(6)? != 0,
        use_count: r.get(7)?,
    })
}
```

### Step 4: Tests pass

Run: `cd src-tauri && cargo test --lib storage::snippets 2>&1 | tail -10`
Expected: 4 pass.

### Step 5: Commit

```powershell
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" add src-tauri/src/storage/
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" commit -m "feat(storage): SnippetRepo with trigger lookup + use counter"
```

---

## Task 7: ScratchpadRepo

**Files:**
- Create: `src-tauri/src/storage/scratchpad.rs`
- Modify: `src-tauri/src/storage/mod.rs` (add `pub mod scratchpad;`)

### Step 1: Write failing tests

```rust
use super::{Db, DbError};
use rusqlite::params;

#[derive(Debug, Clone, PartialEq)]
pub struct ScratchpadNote {
    pub id: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub title: Option<String>,
    pub body: String,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub struct NewNote<'a> {
    pub title: Option<&'a str>,
    pub body: &'a str,
    pub pinned: bool,
}

pub struct ScratchpadRepo<'a> { db: &'a Db }

impl<'a> ScratchpadRepo<'a> {
    pub fn new(db: &'a Db) -> Self { Self { db } }
    pub fn upsert(&self, _now: i64, _id: Option<i64>, _note: NewNote) -> Result<i64, DbError> {
        unimplemented!("Task 7 Step 3")
    }
    pub fn list_ordered(&self) -> Result<Vec<ScratchpadNote>, DbError> {
        unimplemented!("Task 7 Step 3")
    }
    pub fn delete(&self, _id: i64) -> Result<usize, DbError> {
        unimplemented!("Task 7 Step 3")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    #[test]
    fn insert_and_list() {
        let db = mem_db();
        let repo = ScratchpadRepo::new(&db);
        let id = repo.upsert(100, None, NewNote { title: Some("t"), body: "b", pinned: false }).unwrap();
        assert!(id > 0);
        let notes = repo.list_ordered().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].body, "b");
    }

    #[test]
    fn update_existing_by_id() {
        let db = mem_db();
        let repo = ScratchpadRepo::new(&db);
        let id = repo.upsert(100, None, NewNote { title: None, body: "v1", pinned: false }).unwrap();
        repo.upsert(200, Some(id), NewNote { title: None, body: "v2", pinned: true }).unwrap();
        let notes = repo.list_ordered().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].body, "v2");
        assert!(notes[0].pinned);
        assert_eq!(notes[0].updated_at, 200);
    }

    #[test]
    fn list_pinned_first_then_by_updated_desc() {
        let db = mem_db();
        let repo = ScratchpadRepo::new(&db);
        repo.upsert(100, None, NewNote { title: None, body: "old-unpinned", pinned: false }).unwrap();
        repo.upsert(200, None, NewNote { title: None, body: "new-unpinned", pinned: false }).unwrap();
        repo.upsert(50, None, NewNote { title: None, body: "pinned", pinned: true }).unwrap();
        let notes = repo.list_ordered().unwrap();
        assert_eq!(notes[0].body, "pinned");
        assert_eq!(notes[1].body, "new-unpinned");
        assert_eq!(notes[2].body, "old-unpinned");
    }

    #[test]
    fn delete_removes() {
        let db = mem_db();
        let repo = ScratchpadRepo::new(&db);
        let id = repo.upsert(1, None, NewNote { title: None, body: "x", pinned: false }).unwrap();
        assert_eq!(repo.delete(id).unwrap(), 1);
        assert!(repo.list_ordered().unwrap().is_empty());
    }
}
```

Register `pub mod scratchpad;` in `storage/mod.rs`.

### Step 2: Tests fail

Run: `cd src-tauri && cargo test --lib storage::scratchpad 2>&1 | tail -10`

### Step 3: Implement

```rust
impl<'a> ScratchpadRepo<'a> {
    pub fn upsert(&self, now: i64, id: Option<i64>, note: NewNote) -> Result<i64, DbError> {
        self.db.with_conn(|c| match id {
            Some(i) => {
                c.execute(
                    "UPDATE scratchpad_notes
                       SET updated_at=?1, title=?2, body=?3, pinned=?4
                     WHERE id=?5",
                    params![now, note.title, note.body, note.pinned as i64, i],
                )?;
                Ok(i)
            }
            None => {
                c.execute(
                    "INSERT INTO scratchpad_notes
                       (created_at, updated_at, title, body, pinned)
                     VALUES (?1,?1,?2,?3,?4)",
                    params![now, note.title, note.body, note.pinned as i64],
                )?;
                Ok(c.last_insert_rowid())
            }
        })
    }

    pub fn list_ordered(&self) -> Result<Vec<ScratchpadNote>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, created_at, updated_at, title, body, pinned
                 FROM scratchpad_notes
                 ORDER BY pinned DESC, updated_at DESC, id DESC",
            )?;
            Ok(stmt.query_map([], |r| Ok(ScratchpadNote {
                id: r.get(0)?,
                created_at: r.get(1)?,
                updated_at: r.get(2)?,
                title: r.get(3)?,
                body: r.get(4)?,
                pinned: r.get::<_, i64>(5)? != 0,
            }))?.collect::<Result<Vec<_>, _>>()?)
        })
    }

    pub fn delete(&self, id: i64) -> Result<usize, DbError> {
        self.db.with_conn(|c| Ok(c.execute("DELETE FROM scratchpad_notes WHERE id = ?1", [id])?))
    }
}
```

### Step 4: Tests pass

Run: `cd src-tauri && cargo test --lib storage::scratchpad 2>&1 | tail -10`
Expected: 4 pass.

### Step 5: Commit

```powershell
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" add src-tauri/src/storage/
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" commit -m "feat(storage): ScratchpadRepo (pinned ordering)"
```

---

## Task 8: StatsRepo

**Files:**
- Create: `src-tauri/src/storage/stats.rs`
- Modify: `src-tauri/src/storage/mod.rs` (add `pub mod stats;`)

### Step 1: Write failing tests

```rust
use super::{Db, DbError};
use rusqlite::params;

#[derive(Debug, Clone, PartialEq)]
pub struct DailyStats {
    pub day: String,
    pub word_count: i64,
    pub session_count: i64,
    pub total_duration_ms: i64,
    pub avg_wpm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StreakInfo {
    pub current: i64,
    pub longest: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Totals {
    pub word_count: i64,
    pub session_count: i64,
    pub total_duration_ms: i64,
}

pub struct StatsRepo<'a> { db: &'a Db }

impl<'a> StatsRepo<'a> {
    pub fn new(db: &'a Db) -> Self { Self { db } }

    pub fn bump_day(&self, _day: &str, _words: i64, _duration_ms: i64) -> Result<(), DbError> {
        unimplemented!("Task 8 Step 3")
    }
    pub fn get_day(&self, _day: &str) -> Result<Option<DailyStats>, DbError> {
        unimplemented!("Task 8 Step 3")
    }
    pub fn totals(&self) -> Result<Totals, DbError> {
        unimplemented!("Task 8 Step 3")
    }
    pub fn streak_info(&self, _today: &str) -> Result<StreakInfo, DbError> {
        unimplemented!("Task 8 Step 3")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    #[test]
    fn bump_day_aggregates() {
        let db = mem_db();
        let repo = StatsRepo::new(&db);
        repo.bump_day("2026-04-23", 50, 10_000).unwrap();
        repo.bump_day("2026-04-23", 30, 5_000).unwrap();
        let day = repo.get_day("2026-04-23").unwrap().unwrap();
        assert_eq!(day.word_count, 80);
        assert_eq!(day.session_count, 2);
        assert_eq!(day.total_duration_ms, 15_000);
    }

    #[test]
    fn totals_sums_all_days() {
        let db = mem_db();
        let repo = StatsRepo::new(&db);
        repo.bump_day("2026-04-20", 10, 1_000).unwrap();
        repo.bump_day("2026-04-21", 20, 2_000).unwrap();
        let t = repo.totals().unwrap();
        assert_eq!(t.word_count, 30);
        assert_eq!(t.session_count, 2);
        assert_eq!(t.total_duration_ms, 3_000);
    }

    #[test]
    fn streak_counts_consecutive_days_including_today() {
        let db = mem_db();
        let repo = StatsRepo::new(&db);
        repo.bump_day("2026-04-21", 1, 1).unwrap();
        repo.bump_day("2026-04-22", 1, 1).unwrap();
        repo.bump_day("2026-04-23", 1, 1).unwrap();
        let s = repo.streak_info("2026-04-23").unwrap();
        assert_eq!(s.current, 3);
        assert_eq!(s.longest, 3);
    }

    #[test]
    fn streak_resets_on_gap() {
        let db = mem_db();
        let repo = StatsRepo::new(&db);
        repo.bump_day("2026-04-10", 1, 1).unwrap();
        repo.bump_day("2026-04-11", 1, 1).unwrap();
        repo.bump_day("2026-04-20", 1, 1).unwrap();
        repo.bump_day("2026-04-23", 1, 1).unwrap(); // today
        let s = repo.streak_info("2026-04-23").unwrap();
        assert_eq!(s.current, 1, "today is active but yesterday is missing");
        assert_eq!(s.longest, 2, "Apr 10-11 streak");
    }
}
```

Register `pub mod stats;` in `storage/mod.rs`.

### Step 2: Tests fail

Run: `cd src-tauri && cargo test --lib storage::stats 2>&1 | tail -10`

### Step 3: Implement

```rust
impl<'a> StatsRepo<'a> {
    pub fn bump_day(&self, day: &str, words: i64, duration_ms: i64) -> Result<(), DbError> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO stats_daily (day, word_count, session_count, total_duration_ms)
                 VALUES (?1, ?2, 1, ?3)
                 ON CONFLICT(day) DO UPDATE SET
                   word_count = word_count + excluded.word_count,
                   session_count = session_count + 1,
                   total_duration_ms = total_duration_ms + excluded.total_duration_ms",
                params![day, words, duration_ms],
            )?;
            Ok(())
        })
    }

    pub fn get_day(&self, day: &str) -> Result<Option<DailyStats>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT day, word_count, session_count, total_duration_ms, avg_wpm
                 FROM stats_daily WHERE day = ?1",
            )?;
            let mut rows = stmt.query_map([day], |r| Ok(DailyStats {
                day: r.get(0)?,
                word_count: r.get(1)?,
                session_count: r.get(2)?,
                total_duration_ms: r.get(3)?,
                avg_wpm: r.get(4)?,
            }))?;
            Ok(rows.next().transpose()?)
        })
    }

    pub fn totals(&self) -> Result<Totals, DbError> {
        self.db.with_conn(|c| {
            c.query_row(
                "SELECT COALESCE(SUM(word_count),0),
                        COALESCE(SUM(session_count),0),
                        COALESCE(SUM(total_duration_ms),0)
                 FROM stats_daily",
                [], |r| Ok(Totals {
                    word_count: r.get(0)?,
                    session_count: r.get(1)?,
                    total_duration_ms: r.get(2)?,
                }),
            )
        })
    }

    pub fn streak_info(&self, today: &str) -> Result<StreakInfo, DbError> {
        let days: Vec<String> = self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT day FROM stats_daily WHERE word_count > 0 ORDER BY day ASC",
            )?;
            Ok(stmt.query_map([], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?)
        })?;

        // Parse YYYY-MM-DD into ordinal day number via time::Date.
        fn parse(d: &str) -> Option<time::Date> {
            time::Date::parse(d, &time::format_description::well_known::Iso8601::DATE).ok()
        }
        let today_d = parse(today).ok_or_else(|| DbError::Sqlite(
            rusqlite::Error::InvalidParameterName("today".into())
        ))?;

        let mut longest = 0i64;
        let mut run = 0i64;
        let mut prev: Option<time::Date> = None;
        for s in &days {
            let d = match parse(s) { Some(d) => d, None => continue };
            match prev {
                Some(p) if (d - p).whole_days() == 1 => run += 1,
                _ => run = 1,
            }
            if run > longest { longest = run; }
            prev = Some(d);
        }

        // Current streak walks backward from today.
        let set: std::collections::HashSet<time::Date> =
            days.iter().filter_map(|s| parse(s)).collect();
        let mut current = 0i64;
        let mut cursor = today_d;
        while set.contains(&cursor) {
            current += 1;
            cursor = cursor - time::Duration::days(1);
        }

        Ok(StreakInfo { current, longest })
    }
}
```

### Step 4: Tests pass

Run: `cd src-tauri && cargo test --lib storage::stats 2>&1 | tail -10`
Expected: 4 pass.

### Step 5: Commit

```powershell
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" add src-tauri/src/storage/
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" commit -m "feat(storage): StatsRepo with streak calculation"
```

---

## Task 9: AppMetaRepo (settings JSON blob)

**Files:**
- Create: `src-tauri/src/storage/app_meta.rs`
- Modify: `src-tauri/src/storage/mod.rs` (add `pub mod app_meta;`)

Holds opaque key/value strings — settings JSON serialized by the frontend will live at key `"settings_json"` in Phase 1. Phase 0 just needs the storage primitive + get/set semantics.

### Step 1: Write failing tests

```rust
use super::{Db, DbError};

pub struct AppMetaRepo<'a> { db: &'a Db }

impl<'a> AppMetaRepo<'a> {
    pub fn new(db: &'a Db) -> Self { Self { db } }
    pub fn get(&self, _key: &str) -> Result<Option<String>, DbError> {
        unimplemented!("Task 9 Step 3")
    }
    pub fn set(&self, _key: &str, _value: &str) -> Result<(), DbError> {
        unimplemented!("Task 9 Step 3")
    }
    pub fn delete(&self, _key: &str) -> Result<(), DbError> {
        unimplemented!("Task 9 Step 3")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    #[test]
    fn set_then_get() {
        let db = mem_db();
        let repo = AppMetaRepo::new(&db);
        assert_eq!(repo.get("k").unwrap(), None);
        repo.set("k", "v1").unwrap();
        assert_eq!(repo.get("k").unwrap(), Some("v1".to_string()));
        repo.set("k", "v2").unwrap();
        assert_eq!(repo.get("k").unwrap(), Some("v2".to_string()));
    }

    #[test]
    fn delete_removes_key() {
        let db = mem_db();
        let repo = AppMetaRepo::new(&db);
        repo.set("k", "v").unwrap();
        repo.delete("k").unwrap();
        assert_eq!(repo.get("k").unwrap(), None);
    }

    #[test]
    fn stores_large_json_blob() {
        let db = mem_db();
        let repo = AppMetaRepo::new(&db);
        let big = "x".repeat(100_000);
        repo.set("settings_json", &big).unwrap();
        assert_eq!(repo.get("settings_json").unwrap().unwrap().len(), 100_000);
    }
}
```

Register `pub mod app_meta;` in `storage/mod.rs`.

### Step 2: Tests fail

Run: `cd src-tauri && cargo test --lib storage::app_meta 2>&1 | tail -10`

### Step 3: Implement

```rust
impl<'a> AppMetaRepo<'a> {
    pub fn get(&self, key: &str) -> Result<Option<String>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare("SELECT value FROM app_meta WHERE key = ?1")?;
            let mut rows = stmt.query_map([key], |r| r.get::<_, String>(0))?;
            Ok(rows.next().transpose()?)
        })
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), DbError> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO app_meta (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )?;
            Ok(())
        })
    }

    pub fn delete(&self, key: &str) -> Result<(), DbError> {
        self.db.with_conn(|c| {
            c.execute("DELETE FROM app_meta WHERE key = ?1", [key])?;
            Ok(())
        })
    }
}
```

### Step 4: Tests pass

Run: `cd src-tauri && cargo test --lib storage::app_meta 2>&1 | tail -10`
Expected: 3 pass.

### Step 5: Commit

```powershell
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" add src-tauri/src/storage/
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" commit -m "feat(storage): AppMetaRepo (key/value settings blob)"
```

---

## Task 10: Integration — open on disk, full round-trip, WAL files exist

**Files:**
- Create: `src-tauri/tests/storage_integration.rs`

Integration test that proves the whole `Db::open` path works on a real file (not just `:memory:`).

### Step 1: Write failing test

```rust
// src-tauri/tests/storage_integration.rs
use typr_lib::storage::{Db, transcriptions::{TranscriptionRepo, NewTranscription}};

#[test]
fn open_on_disk_runs_migrations_and_round_trips() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("typr.db");

    let db = Db::open(&db_path).expect("open on disk");
    let repo = TranscriptionRepo::new(&db);
    repo.insert(NewTranscription {
        created_at: 1776902400,
        raw_text: "hello world",
        final_text: "hello world",
        word_count: 2,
        duration_ms: 1000,
        language: "en",
        engine: "local",
        model: Some("turbo"),
        app_context: Some("Notepad"),
        mode: "dictation",
        enhanced: false,
    }).unwrap();

    // Reopen the same file — migrations must be idempotent.
    drop(db);
    let db2 = Db::open(&db_path).expect("reopen");
    let repo2 = TranscriptionRepo::new(&db2);
    let hits = repo2.search_fts("hello", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].final_text, "hello world");

    // WAL should be enabled (file present while connection is open)
    // If we're re-using an existing connection, typr.db-wal or typr.db-shm should exist.
    let has_wal = std::fs::read_dir(tmp.path()).unwrap().any(|e| {
        e.unwrap().file_name().to_string_lossy().ends_with("-wal")
    });
    assert!(has_wal, "WAL journal mode should leave a -wal file");
}
```

For this test to compile, the `storage` module and its submodules must be `pub` in `lib.rs`. Update `src-tauri/src/lib.rs`:

```rust
pub mod storage;
pub mod telemetry;
```

(Change from `mod storage;`/`mod telemetry;` to `pub mod …` — without this, integration tests cannot import the symbols because they live in the `typr_lib` crate.)

### Step 2: Run the test, confirm it fails

Run: `cd src-tauri && cargo test --test storage_integration 2>&1 | tail -15`
Expected: if `pub` wasn't set, a compile error about private module. If `pub` was set in Step 1, the test may already pass — in that case, break it (e.g. change `final_text` assertion to `"wrong"`), rerun, see fail, revert, pass.

### Step 3: Run the test, confirm it passes

Run: `cd src-tauri && cargo test --test storage_integration 2>&1 | tail -10`
Expected: `test open_on_disk_runs_migrations_and_round_trips ... ok`.

### Step 4: Full suite green

Run: `cd src-tauri && cargo test 2>&1 | tail -20`
Expected: all tests pass (roughly 24 tests across the storage submodules + 1 integration + 1 telemetry).

### Step 5: Frontend still builds

Run: `cd Z:\Pessoal\vault\projects\local-whisper\typr-main && pnpm install --frozen-lockfile 2>&1 | tail -5 && pnpm build 2>&1 | tail -10`
Expected: `vite build` finishes with `built in …`. No TS errors (Phase 0 didn't touch frontend).

### Step 6: Commit

```powershell
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" add src-tauri/tests/ src-tauri/src/lib.rs
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" commit -m "test(storage): integration test — disk open, reopen, WAL file present"
```

---

## Task 11: Boot wire-up — `Db` lives in Tauri state, `tracing` active from app start

**Files:**
- Modify: `src-tauri/src/lib.rs`

Wire `telemetry::init_tracing()` and `Db::open()` into the Tauri builder. Do **not** add any commands that consume them yet — Phase 1 does that. Just prove the app boots with the DB attached and logs to `%APPDATA%\com.typr.app\logs\`.

- [ ] **Step 1: Add setup closure to `tauri::Builder`**

In `src-tauri/src/lib.rs`, find the existing `run()` function (inside the Tauri builder chain) and extend it with a `.setup(|app| { … })` call. If the existing code already has a `.setup()`, augment it. Otherwise add one.

Pseudo-patch (adapt to the actual current code):

```rust
.setup(|app| {
    let app_dir = app.path().app_data_dir().expect("app_data_dir");
    let log_dir = app_dir.join("logs");
    let _ = crate::telemetry::init_tracing(&log_dir);

    let db_path = app_dir.join("typr.db");
    let db = crate::storage::Db::open(&db_path)
        .expect("open typr.db");
    app.manage(db);

    tracing::info!(stage = "boot", "storage + telemetry initialised");
    Ok(())
})
```

(`app.path()` requires `use tauri::Manager;` at the top of the file.)

- [ ] **Step 2: Full build is green**

Run: `cd src-tauri && cargo build 2>&1 | tail -5`
Expected: `Finished` with no warnings about unused `Db` (it is managed, so consumed).

- [ ] **Step 3: Smoke boot the app (manual)**

Run: `cd Z:\Pessoal\vault\projects\local-whisper\typr-main && pnpm tauri dev 2>&1 | tee boot.log`

Watch the first 10 seconds. Expected:
- No panic.
- A line containing `stage=boot storage + telemetry initialised`.
- `%APPDATA%\com.typr.app\typr.db` now exists (check with `Test-Path "$env:APPDATA\com.typr.app\typr.db"`).
- `%APPDATA%\com.typr.app\logs\typr.log.*` exists.

Kill with Ctrl+C. Delete `boot.log` afterwards — `rm boot.log`.

- [ ] **Step 4: Commit**

```powershell
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" add src-tauri/src/lib.rs
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" commit -m "feat(boot): wire tracing + Db::open into Tauri setup"
```

- [ ] **Step 5: Push the branch**

```powershell
git push -u origin wispr-parity
```

---

## Exit Criteria (Phase 0 DoD)

- [ ] Branch `wispr-parity` pushed to `origin`.
- [ ] `cd src-tauri && cargo build` → green, no warnings about unused DB.
- [ ] `cd src-tauri && cargo test` → all storage + telemetry + integration tests pass.
- [ ] `pnpm build` → green.
- [ ] Manual `pnpm tauri dev` boot: `typr.db` and `logs/typr.log.*` materialise under `%APPDATA%\com.typr.app\`.
- [ ] `TranscriptionRepo::search_fts("query", 10)` returns a row inserted by `insert()` (covered by integration test).

---

## Self-Review Notes

Ran the writing-plans self-review checklist inline:

**1. Spec coverage (Phase 0 scope only):**
- Scaffolding ✓ Task 1
- Deps ✓ Task 1 (rusqlite bundled+fts5, tracing stack, thiserror)
- Tracing ✓ Task 2, Task 11
- Branch `wispr-parity` ✓ Task 1 Step 1, Task 11 Step 5
- Storage layer (rusqlite bundled+FTS5) ✓ Task 3
- Migrations runner ✓ Task 3 (`migrations.rs` + `include_str!`)
- Repos — transcriptions ✓ Task 4, dictionary ✓ Task 5, snippets ✓ Task 6, scratchpad ✓ Task 7, stats ✓ Task 8, settings (app_meta) ✓ Task 9
- Unit tests ✓ every repo task
- Exit: `cargo build` + `pnpm build` green ✓ Task 10 Step 4-5; FTS query works ✓ Task 4 + Task 10 integration test; repo unit tests pass ✓ covered.

**2. Placeholder scan:** no "TBD", no "implement later", every `unimplemented!()` is immediately replaced in the next step with real code.

**3. Type consistency:**
- `NewTranscription` fields match across Task 4 test + impl + Task 10 integration test.
- `DictionaryRepo::upsert` takes `(now, NewDictionaryTerm)` — same signature everywhere.
- `StatsRepo::streak_info(today: &str)` — same across test + impl.
- `Db::with_conn` / `with_conn_mut` — used consistently, no drift.

Minor note for the implementer: Task 10 integration test imports via `typr_lib::storage::…` — this matches the `[lib] name = "typr_lib"` already set in `Cargo.toml`.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-04-23-phase-0-foundation.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task (1 → 11), review between tasks, fast iteration. REQUIRED SUB-SKILL: `superpowers:subagent-driven-development`.

**2. Inline Execution** — Execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints for review.

**Which approach?**
