-- migrations/002_auto_add_candidates.sql
CREATE TABLE auto_add_candidates (
    term          TEXT    PRIMARY KEY,
    seen_count    INTEGER NOT NULL DEFAULT 0,
    last_seen_at  INTEGER NOT NULL
);

CREATE INDEX idx_auto_add_candidates_seen_count
  ON auto_add_candidates(seen_count DESC);
