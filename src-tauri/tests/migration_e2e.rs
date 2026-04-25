//! Phase 1 e2e: plant a v1 `config.json` on disk, run the loader with a
//! mock keyring + in-memory DB, assert the full lifecycle.

use tempfile::TempDir;
use typr_lib::settings::keyring::MockBackend;
use typr_lib::settings::{load, MigrationEvent, SETTINGS_VERSION_KEY};
use typr_lib::storage::{app_meta::AppMetaRepo, Db};

const V1_FIXTURE: &str = r#"{
    "microphone": "Stream Deck Mic",
    "engine": "groq",
    "whisperModel": "medium",
    "groqApiKey": "sk-legacy-42",
    "recordingMode": "push-to-talk",
    "hotkey": "F13"
}"#;

#[test]
fn v1_fixture_migrates_full_lifecycle() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("config.json"), V1_FIXTURE).unwrap();

    let db = Db::open_in_memory().unwrap();
    let kr = MockBackend::new();

    let out = load(dir.path(), &db, &kr).expect("load succeeds");

    // v2 shape on disk.
    let v2_raw = std::fs::read_to_string(dir.path().join("config.json")).unwrap();
    assert!(v2_raw.contains("\"schemaVersion\": 2"));
    assert!(v2_raw.contains("\"whisperModel\": \"turbo\""));
    assert!(!v2_raw.contains("groqApiKey"), "secret must not leak to JSON");

    // Struct in memory.
    assert_eq!(out.settings.schema_version, 2);
    assert_eq!(out.settings.microphone, "Stream Deck Mic");
    assert_eq!(out.settings.transcription.engine, "groq");
    assert_eq!(out.settings.transcription.whisper_model, "turbo");
    assert_eq!(out.settings.hotkeys.dictation, "F13");
    assert_eq!(out.settings.hotkeys.recording_mode, "push-to-talk");

    // Keyring populated, .bak cleaned.
    assert_eq!(kr.peek().as_deref(), Some("sk-legacy-42"));
    assert!(!dir.path().join("config.json.v1.bak").exists());

    // Sentinel stamped.
    assert_eq!(
        AppMetaRepo::new(&db).get(SETTINGS_VERSION_KEY).unwrap().as_deref(),
        Some("2")
    );

    // Events include Migrated, ModelRemapped(medium→turbo), NeedsGroqKey.
    assert!(out.events.contains(&MigrationEvent::Migrated));
    assert!(out.events.iter().any(|e| matches!(
        e,
        MigrationEvent::ModelRemapped { from, to } if from == "medium" && to == "turbo"
    )));
    assert!(out.events.contains(&MigrationEvent::NeedsGroqKey));

    // Re-run → idempotent, no new events.
    let out2 = load(dir.path(), &db, &kr).expect("second load");
    assert!(out2.events.is_empty());
    assert_eq!(out2.settings, out.settings);
}

#[test]
fn failed_migration_preserves_backup() {
    // A malformed v1 where root is a JSON array → migrator returns Malformed.
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("config.json"), "[1,2,3]").unwrap();
    let db = Db::open_in_memory().unwrap();
    let kr = MockBackend::new();

    let out = load(dir.path(), &db, &kr).expect("load returns Ok with event");
    assert!(matches!(
        out.events.first(),
        Some(MigrationEvent::MigrationFailed(_))
    ));
    assert!(
        dir.path().join("config.json.v1.bak").exists(),
        ".bak preserved on migration failure"
    );
}
