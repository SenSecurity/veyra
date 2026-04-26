//! Settings v2 loader — owns the on-disk lifecycle.
//!
//! Flow per boot:
//! 1. Read `<app_dir>/config.json`. Absent → return default settings, write v2 JSON, stamp sentinel.
//! 2. Parse to `serde_json::Value`. Detect version.
//!     - v2 → deserialize into `schema::Settings`, return.
//!     - v1 → write `config.json.v1.bak` → run migrator → write v2 JSON → on success delete `.bak`
//!       + set `app_meta.settings_version=2`. On failure keep `.bak`, surface error, fall back to defaults.
//!     - unknown → keep user's file untouched, return defaults with `UnknownVersion` event.
//!
//! Phase 1.5 cutover: `main.rs` now drives migration directly via [`load`] and
//! exposes v2 [`schema::Settings`] in `AppState`. The frontend continues to
//! speak the v1-shaped JSON via [`adapter`], which projects the in-memory v2
//! tree + the OS keyring secret into [`legacy_v1::Settings`] for serialisation.

pub mod adapter;
pub mod keyring;
pub mod legacy_v1;
pub mod migrations;
pub mod schema;

use crate::settings::keyring::KeyringBackend;
use crate::settings::migrations::{detect_version, migrate_v1_to_v2, MigrationError};
use crate::storage::{app_meta::AppMetaRepo, Db, DbError};
use std::path::{Path, PathBuf};

pub const CONFIG_FILENAME: &str = "config.json";
pub const BACKUP_FILENAME: &str = "config.json.v1.bak";
pub const SETTINGS_VERSION_KEY: &str = "settings_version";

#[derive(thiserror::Error, Debug)]
pub enum SettingsError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("migration: {0}")]
    Migration(#[from] MigrationError),
    #[error("db: {0}")]
    Db(#[from] DbError),
}

/// One-shot record of what the loader did. Consumed by the Tauri boot wiring
/// to emit `settings:*` events to the frontend.
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationEvent {
    Migrated,
    ModelRemapped { from: String, to: String },
    NeedsGroqKey,
    UnknownVersion(u32),
    MigrationFailed(String),
}

#[derive(Debug)]
pub struct LoadOutcome {
    pub settings: schema::Settings,
    pub events: Vec<MigrationEvent>,
}

pub fn load(
    app_dir: &Path,
    db: &Db,
    backend: &dyn KeyringBackend,
) -> Result<LoadOutcome, SettingsError> {
    std::fs::create_dir_all(app_dir)?;
    let cfg_path = app_dir.join(CONFIG_FILENAME);
    let bak_path = app_dir.join(BACKUP_FILENAME);
    let meta = AppMetaRepo::new(db);

    if !cfg_path.exists() {
        let settings = schema::Settings::default();
        write_atomic(&cfg_path, &settings)?;
        meta.set(SETTINGS_VERSION_KEY, "2")?;
        return Ok(LoadOutcome { settings, events: vec![] });
    }

    let raw = std::fs::read_to_string(&cfg_path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    let version = detect_version(&value);

    match version {
        2 => {
            let settings: schema::Settings = serde_json::from_value(value)?;
            Ok(LoadOutcome { settings, events: vec![] })
        }
        1 => run_v1_migration(&cfg_path, &bak_path, &raw, &value, &meta, backend),
        other => Ok(LoadOutcome {
            settings: schema::Settings::default(),
            events: vec![MigrationEvent::UnknownVersion(other)],
        }),
    }
}

fn run_v1_migration(
    cfg_path: &Path,
    bak_path: &Path,
    raw: &str,
    value: &serde_json::Value,
    meta: &AppMetaRepo<'_>,
    backend: &dyn KeyringBackend,
) -> Result<LoadOutcome, SettingsError> {
    // Step A: write backup BEFORE anything else.
    // Don't clobber an existing .bak from a prior failed migration — that would
    // destroy the only intact copy of the user's v1 config.
    if !bak_path.exists() {
        std::fs::write(bak_path, raw)?;
    }

    // Step B: run migrator. On failure, keep .bak and surface event.
    let outcome = match migrate_v1_to_v2(value, backend) {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(error = %e, "settings migration failed; .bak preserved");
            return Ok(LoadOutcome {
                settings: schema::Settings::default(),
                events: vec![MigrationEvent::MigrationFailed(e.to_string())],
            });
        }
    };

    // Step C: write v2 JSON. On failure, also keep .bak.
    if let Err(e) = write_atomic(cfg_path, &outcome.settings) {
        tracing::warn!(error = %e, "v2 write failed; .bak preserved");
        return Ok(LoadOutcome {
            settings: schema::Settings::default(),
            events: vec![MigrationEvent::MigrationFailed(e.to_string())],
        });
    }

    // Step D: delete backup + stamp sentinel.
    if let Err(e) = std::fs::remove_file(bak_path) {
        tracing::warn!(error = %e, "failed to delete .bak after success — harmless");
    }
    meta.set(SETTINGS_VERSION_KEY, "2")?;

    let mut events = vec![MigrationEvent::Migrated];
    if let Some((from, to)) = outcome.remapped_model {
        events.push(MigrationEvent::ModelRemapped { from, to });
    }
    if outcome.groq_key_migrated {
        // Spec §7: "prompt user to re-enter Groq key (now stored securely)".
        // Gated on actual keyring write (not just JSON presence) so a v1 file with
        // an empty groqApiKey doesn't pester the user.
        events.push(MigrationEvent::NeedsGroqKey);
    }
    Ok(LoadOutcome { settings: outcome.settings, events })
}

/// Persist a fresh v2 `Settings` snapshot at the canonical `config.json` path
/// inside `app_dir`. Wraps [`write_atomic`] so callers outside this module
/// (e.g. the `save_settings` Tauri command) get the same durable-rename
/// guarantees without depending on the private helper.
pub fn save(app_dir: &Path, settings: &schema::Settings) -> Result<(), SettingsError> {
    std::fs::create_dir_all(app_dir)?;
    let cfg_path = app_dir.join(CONFIG_FILENAME);
    write_atomic(&cfg_path, settings)
}

/// Write `settings` to `path` via a `.tmp` sibling + rename. NOTE: this is
/// durable-rename, not crash-atomic — a power loss between `write` and
/// `rename` (or before the directory entry hits disk) can still leave the
/// caller seeing the old file. Good enough for settings; if we ever store
/// data we cannot rebuild, switch to fsync(file) + fsync(dir) + rename.
fn write_atomic(path: &Path, settings: &schema::Settings) -> Result<(), SettingsError> {
    let tmp: PathBuf = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(&tmp, &json)?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        // Don't let an orphaned .tmp shadow future runs.
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::schema::Settings as SettingsV2;
    use super::*;
    use crate::settings::keyring::MockBackend;
    use crate::storage::Db;
    use tempfile::TempDir;

    fn test_db() -> Db {
        Db::open_in_memory().unwrap()
    }

    #[test]
    fn fresh_install_writes_default_v2() {
        let dir = TempDir::new().unwrap();
        let db = test_db();
        let kr = MockBackend::new();
        let out = load(dir.path(), &db, &kr).unwrap();
        assert_eq!(out.settings.schema_version, 2);
        assert!(dir.path().join("config.json").exists());
        assert!(!dir.path().join("config.json.v1.bak").exists());
        assert_eq!(
            AppMetaRepo::new(&db).get(SETTINGS_VERSION_KEY).unwrap().as_deref(),
            Some("2")
        );
        assert!(out.events.is_empty());
    }

    #[test]
    fn v1_file_is_migrated_and_backup_removed() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"microphone":"default","engine":"local","whisperModel":"small","groqApiKey":"sk-x","recordingMode":"toggle","hotkey":"F24"}"#,
        ).unwrap();
        let db = test_db();
        let kr = MockBackend::new();
        let out = load(dir.path(), &db, &kr).unwrap();
        assert_eq!(out.settings.schema_version, 2);
        assert_eq!(out.settings.transcription.whisper_model, "turbo");
        assert_eq!(kr.peek().as_deref(), Some("sk-x"));
        assert!(!dir.path().join("config.json.v1.bak").exists(), ".bak deleted on success");
        // Sentinel set.
        assert_eq!(
            AppMetaRepo::new(&db).get(SETTINGS_VERSION_KEY).unwrap().as_deref(),
            Some("2")
        );
        // Events: Migrated + ModelRemapped(small→turbo) + NeedsGroqKey.
        assert!(out.events.contains(&MigrationEvent::Migrated));
        assert!(out.events.iter().any(|e| matches!(e, MigrationEvent::ModelRemapped { from, to } if from == "small" && to == "turbo")));
        assert!(out.events.contains(&MigrationEvent::NeedsGroqKey));
        // Reload now reads v2 without re-migrating.
        let out2 = load(dir.path(), &db, &kr).unwrap();
        assert!(out2.events.is_empty());
    }

    #[test]
    fn v2_file_loads_without_events() {
        let dir = TempDir::new().unwrap();
        let s = SettingsV2::default();
        std::fs::write(dir.path().join("config.json"), serde_json::to_string(&s).unwrap()).unwrap();
        let db = test_db();
        let kr = MockBackend::new();
        let out = load(dir.path(), &db, &kr).unwrap();
        assert_eq!(out.settings, s);
        assert!(out.events.is_empty());
    }

    #[test]
    fn v1_without_groq_key_skips_needs_groq_event() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"whisperModel":"turbo","hotkey":"F24"}"#,
        ).unwrap();
        let db = test_db();
        let kr = MockBackend::new();
        let out = load(dir.path(), &db, &kr).unwrap();
        assert!(!out.events.contains(&MigrationEvent::NeedsGroqKey));
        assert_eq!(kr.peek(), None);
    }

    #[test]
    fn malformed_json_bubbles_as_error() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("config.json"), "{ not json").unwrap();
        let db = test_db();
        let kr = MockBackend::new();
        let err = load(dir.path(), &db, &kr).unwrap_err();
        assert!(matches!(err, SettingsError::Json(_)));
    }

    #[test]
    fn unknown_version_keeps_file_and_emits_event() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"schemaVersion":99}"#,
        ).unwrap();
        let db = test_db();
        let kr = MockBackend::new();
        let out = load(dir.path(), &db, &kr).unwrap();
        assert!(out.events.contains(&MigrationEvent::UnknownVersion(99)));
        // Original file untouched.
        let raw = std::fs::read_to_string(dir.path().join("config.json")).unwrap();
        assert!(raw.contains("99"));
    }
}
