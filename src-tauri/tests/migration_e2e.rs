//! Phase 1 e2e: plant a v1 `config.json` on disk, run the loader with a
//! mock keyring + in-memory DB, assert the full lifecycle.

use tempfile::TempDir;
use typr_lib::settings;
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

    // v3 shape on disk (v1 → v2 → v3 chained in one boot).
    let v3_raw = std::fs::read_to_string(dir.path().join("config.json")).unwrap();
    assert!(v3_raw.contains("\"schemaVersion\": 3"));
    assert!(v3_raw.contains("\"whisperModel\": \"turbo\""));
    assert!(!v3_raw.contains("groqApiKey"), "secret must not leak to JSON");

    // Struct in memory.
    assert_eq!(out.settings.schema_version, 3);
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
        Some("3")
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
fn load_v2_with_medium_writes_v3_and_remaps_to_turbo() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.json");
    std::fs::write(&path, r#"{
        "schemaVersion": 2,
        "microphone": "default",
        "transcription": {
            "engine": "local", "whisperModel": "medium",
            "languages": ["pt","en"], "autoDetect": true,
            "gpuAcceleration": "auto", "vadEnabled": true, "noSpeechThreshold": 0.6
        },
        "hotkeys": {"dictation":"F24","commandMode":"Shift+F24","recordingMode":"push-to-talk"},
        "overlay": {"style":"pill","position":"near-cursor"},
        "formatting": {"enhanceEnabled":false,"removeFillers":true,"fillerWords":[],"explicitCommands":true},
        "dictionary": {"autoAdd":false},
        "stats": {"enabled":true,"milestoneNotifications":true},
        "data": {"wordCountCap":500000,"purgeOnExceed":true},
        "system": {"launchAtLogin":false,"closeToTray":true,"dictationSounds":true,"muteMusicOnDictate":false},
        "ui": {"language":"en","theme":"system","accent":"indigo"}
    }"#).unwrap();

    let s = settings::load_for_test(&path).expect("load v2");
    assert_eq!(s.schema_version, 3);
    assert_eq!(s.transcription.whisper_model, "turbo");

    let on_disk: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(on_disk["schemaVersion"], 3);
    assert_eq!(on_disk["transcription"]["whisperModel"], "turbo");
}

#[test]
fn load_v3_passes_through_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.json");
    let mut def = settings::Settings::default();
    def.transcription.whisper_model = "large-v3".into();
    std::fs::write(&path, serde_json::to_string_pretty(&def).unwrap()).unwrap();
    let mtime_before = std::fs::metadata(&path).unwrap().modified().unwrap();

    let s = settings::load_for_test(&path).unwrap();
    assert_eq!(s.transcription.whisper_model, "large-v3");
    let mtime_after = std::fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(mtime_before, mtime_after, "v3 load must not rewrite file");
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
