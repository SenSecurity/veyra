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
