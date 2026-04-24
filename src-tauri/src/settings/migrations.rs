//! Legacy `config.json` migrator.
//!
//! Detection: absence of `schemaVersion` → v1. Migration maps the six flat v1
//! fields into the nested v2 tree, remaps Whisper model per spec table, and
//! moves `groqApiKey` out of JSON into the keyring.

use crate::settings::keyring::{KeyringBackend, KeyringError};
use crate::settings::schema::Settings;
use serde_json::Value;

#[derive(thiserror::Error, Debug)]
pub enum MigrationError {
    #[error("settings JSON is malformed: {0}")]
    Malformed(String),
    #[error("keyring write failed: {0}")]
    Keyring(#[from] KeyringError),
}

/// Outcome of a single migration run. Consumed by the loader to decide which
/// toasts / events to emit.
#[derive(Debug, Clone, PartialEq)]
pub struct MigrationOutcome {
    pub settings: Settings,
    pub remapped_model: Option<(String, String)>, // (from, to)
    pub groq_key_migrated: bool,
    pub had_groq_key_in_json: bool,
}

/// Returns `1` for legacy flat shape (no `schemaVersion`), otherwise the value
/// of `schemaVersion` (clamped to `u32`). Unknown/non-numeric → `0` so callers
/// can reject it.
pub fn detect_version(root: &Value) -> u32 {
    match root.get("schemaVersion") {
        None => 1,
        Some(Value::Number(n)) => n.as_u64().map(|v| v as u32).unwrap_or(0),
        _ => 0,
    }
}

/// v1 whisper model → v2 (per spec §7 table). Unknown values default to
/// `turbo` so users on deleted models don't brick.
pub fn remap_whisper_model(old: &str) -> &'static str {
    match old {
        "tiny" => "turbo",
        "base" => "base",
        "small" => "turbo",
        "medium" => "turbo",
        "large-v3" => "large-v3",
        "turbo" => "turbo",
        _ => "turbo",
    }
}

/// Consume a v1 `Value`, write the Groq key (if any) to `backend`, and return
/// the fully-populated v2 `Settings` plus telemetry on what changed.
pub fn migrate_v1_to_v2(
    v1: &Value,
    backend: &dyn KeyringBackend,
) -> Result<MigrationOutcome, MigrationError> {
    let obj = v1
        .as_object()
        .ok_or_else(|| MigrationError::Malformed("root is not an object".into()))?;

    let mut out = Settings::default();

    if let Some(Value::String(mic)) = obj.get("microphone") {
        out.microphone = mic.clone();
    }
    if let Some(Value::String(engine)) = obj.get("engine") {
        // v1 only allowed 'local' | 'groq'; anything else falls back to default.
        if engine == "local" || engine == "groq" {
            out.transcription.engine = engine.clone();
        }
    }

    let (old_model, new_model) = match obj.get("whisperModel") {
        Some(Value::String(m)) => {
            let mapped = remap_whisper_model(m);
            (m.clone(), mapped.to_string())
        }
        _ => ("small".to_string(), remap_whisper_model("small").to_string()),
    };
    out.transcription.whisper_model = new_model.clone();
    let remapped_model = if old_model != new_model {
        Some((old_model, new_model))
    } else {
        None
    };

    if let Some(Value::String(mode)) = obj.get("recordingMode") {
        if mode == "toggle" || mode == "push-to-talk" {
            out.hotkeys.recording_mode = mode.clone();
        }
    }
    if let Some(Value::String(hk)) = obj.get("hotkey") {
        out.hotkeys.dictation = hk.clone();
    }

    let mut had_groq_key_in_json = false;
    let mut groq_key_migrated = false;
    if let Some(Value::String(key)) = obj.get("groqApiKey") {
        if !key.trim().is_empty() {
            had_groq_key_in_json = true;
            backend.set(key)?;
            groq_key_migrated = true;
        }
    }

    Ok(MigrationOutcome {
        settings: out,
        remapped_model,
        groq_key_migrated,
        had_groq_key_in_json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::keyring::MockBackend;
    use serde_json::json;

    #[test]
    fn detect_v1_when_schema_version_absent() {
        assert_eq!(detect_version(&json!({ "microphone": "default" })), 1);
    }

    #[test]
    fn detect_v2_when_schema_version_present() {
        assert_eq!(detect_version(&json!({ "schemaVersion": 2 })), 2);
    }

    #[test]
    fn detect_zero_when_schema_version_is_string() {
        assert_eq!(detect_version(&json!({ "schemaVersion": "2" })), 0);
    }

    #[test]
    fn remap_table_matches_spec() {
        assert_eq!(remap_whisper_model("tiny"), "turbo");
        assert_eq!(remap_whisper_model("base"), "base");
        assert_eq!(remap_whisper_model("small"), "turbo");
        assert_eq!(remap_whisper_model("medium"), "turbo");
        assert_eq!(remap_whisper_model("large-v3"), "large-v3");
        assert_eq!(remap_whisper_model("turbo"), "turbo");
        assert_eq!(remap_whisper_model("unknown-garbage"), "turbo");
    }

    #[test]
    fn migrates_full_v1_shape() {
        let v1 = json!({
            "microphone": "Stream Deck Mic",
            "engine": "groq",
            "whisperModel": "small",
            "groqApiKey": "sk-abc",
            "recordingMode": "toggle",
            "hotkey": "F13"
        });
        let kr = MockBackend::new();
        let out = migrate_v1_to_v2(&v1, &kr).unwrap();
        assert_eq!(out.settings.microphone, "Stream Deck Mic");
        assert_eq!(out.settings.transcription.engine, "groq");
        assert_eq!(out.settings.transcription.whisper_model, "turbo");
        assert_eq!(out.settings.hotkeys.dictation, "F13");
        assert_eq!(out.settings.hotkeys.recording_mode, "toggle");
        assert_eq!(
            out.remapped_model,
            Some(("small".into(), "turbo".into()))
        );
        assert!(out.had_groq_key_in_json);
        assert!(out.groq_key_migrated);
        assert_eq!(kr.peek().as_deref(), Some("sk-abc"));
    }

    #[test]
    fn missing_fields_fall_back_to_default() {
        let v1 = json!({});
        let kr = MockBackend::new();
        let out = migrate_v1_to_v2(&v1, &kr).unwrap();
        let def = Settings::default();
        assert_eq!(out.settings.microphone, def.microphone);
        assert_eq!(out.settings.hotkeys.dictation, def.hotkeys.dictation);
        // Absent `whisperModel` is synthesised as legacy "small" which remaps to
        // "turbo" — so `remapped_model` is Some. Dedicated check in
        // `missing_whisper_model_still_reports_remap_from_assumed_small`.
        assert!(out.remapped_model.is_some());
    }

    #[test]
    fn missing_whisper_model_still_reports_remap_from_assumed_small() {
        let v1 = json!({});
        let kr = MockBackend::new();
        let out = migrate_v1_to_v2(&v1, &kr).unwrap();
        assert_eq!(
            out.remapped_model,
            Some(("small".into(), "turbo".into())),
            "absent whisperModel is treated as legacy default 'small' per v0 code"
        );
    }

    #[test]
    fn empty_groq_key_does_not_touch_keyring() {
        let v1 = json!({ "groqApiKey": "   " });
        let kr = MockBackend::new();
        let out = migrate_v1_to_v2(&v1, &kr).unwrap();
        assert!(!out.had_groq_key_in_json);
        assert!(!out.groq_key_migrated);
        assert_eq!(kr.peek(), None);
    }

    #[test]
    fn absent_groq_key_does_not_touch_keyring() {
        let v1 = json!({ "microphone": "default" });
        let kr = MockBackend::new();
        let out = migrate_v1_to_v2(&v1, &kr).unwrap();
        assert!(!out.groq_key_migrated);
        assert_eq!(kr.peek(), None);
    }

    #[test]
    fn rejects_non_object_root() {
        let v1 = json!([1, 2, 3]);
        let kr = MockBackend::new();
        let err = migrate_v1_to_v2(&v1, &kr).unwrap_err();
        assert!(matches!(err, MigrationError::Malformed(_)));
    }

    #[test]
    fn unknown_engine_falls_back_to_default() {
        let v1 = json!({ "engine": "whatever" });
        let kr = MockBackend::new();
        let out = migrate_v1_to_v2(&v1, &kr).unwrap();
        assert_eq!(out.settings.transcription.engine, "local"); // default
    }

    #[test]
    fn no_remap_when_model_is_turbo() {
        let v1 = json!({ "whisperModel": "turbo" });
        let kr = MockBackend::new();
        let out = migrate_v1_to_v2(&v1, &kr).unwrap();
        assert!(out.remapped_model.is_none());
        assert_eq!(out.settings.transcription.whisper_model, "turbo");
    }
}
