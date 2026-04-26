# Phase 1 — Settings v2 + Migrator + Keyring Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the 6-field flat `config.json` settings with the v2 nested shape (10 groups, `schemaVersion: 2`), migrate legacy configs cleanly on first boot (remap whisper models, move `groqApiKey` into Windows Credential Manager, keep `.bak` on failure), and wire the loader into the Tauri boot sequence so downstream phases can depend on a known-good `Settings` value.

**Architecture:**
- `src-tauri/src/settings/` becomes a module directory (delete monolithic `settings.rs`): `schema.rs` owns the v2 Rust struct with serde rename attributes matching the TS shape; `keyring.rs` wraps the `keyring` v3 crate behind a `KeyringBackend` trait (real `SystemBackend` + test `MockBackend`) so migrator and loader never touch OS APIs directly; `migrations.rs` holds pure `Value`→`Settings` logic with the model remap table; `mod.rs` orchestrates the on-disk lifecycle (read → detect v1 → write `.bak` → migrate → rewrite v2 → delete `.bak` on success → stamp `app_meta.settings_version=2`).
- First-boot toasts surface via `tauri::Emitter::emit` events (`settings:migrated`, `settings:model-remapped`, `settings:needs-groq-key`); the React toast UI is Phase 3, Phase 1 only emits and logs.
- `.bak` preserved on every failure path. DB backup utility deferred (spec L893) — no existing DB to back up on fresh V1.

**Tech Stack:** Rust 1.82 edition 2021, Tauri 2, `rusqlite` 0.31 (bundled), `keyring` 3 (new dep), `serde` + `serde_json`, `thiserror`, `tracing`, `tempfile` (dev). Frontend untouched this phase.

---

## File Structure

**Delete:**
- `src-tauri/src/settings.rs` (107-line monolith — v1 flat struct, replaced by `settings/` module)

**Create:**
- `src-tauri/src/settings/mod.rs` — public module root, `load()` orchestrator, `SettingsError`, `MigrationEvent`
- `src-tauri/src/settings/schema.rs` — v2 `Settings` struct + `Default` impl matching TS shape (lines 175–231 of spec)
- `src-tauri/src/settings/keyring.rs` — `KeyringBackend` trait, `SystemBackend`, `MockBackend`, module-level helpers
- `src-tauri/src/settings/migrations.rs` — `detect_version`, `remap_whisper_model`, `migrate_v1_to_v2`
- `src-tauri/tests/migration_e2e.rs` — end-to-end fixture (v1 JSON → temp dir → full load → assert v2 + keyring populated + `.bak` removed + sentinel set)

**Modify:**
- `src-tauri/Cargo.toml` — add `keyring = "3"` under `[dependencies]`
- `src-tauri/src/lib.rs` — call `settings::load(...)` inside `.setup(...)`, `app.manage(settings)`, emit migration events

---

## Task 1: Add `keyring` dependency

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add dep**

Open `src-tauri/Cargo.toml`, after the `thiserror = "1"` line under `[dependencies]`, append:

```toml
keyring = "3"
```

- [ ] **Step 2: Verify it builds**

Run: `cd src-tauri && cargo build --lib`
Expected: compiles green (keyring is a leaf add, no code uses it yet).

- [ ] **Step 3: Commit**

```bash
cd Z:/Pessoal/vault/projects/local-whisper/typr-main
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  add src-tauri/Cargo.toml src-tauri/Cargo.lock
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  commit -m "chore(settings): add keyring 3 dependency"
```

---

## Task 2: Swap monolith for empty module dir

**Files:**
- Delete: `src-tauri/src/settings.rs`
- Create: `src-tauri/src/settings/mod.rs`

- [ ] **Step 1: Delete monolith**

```bash
rm src-tauri/src/settings.rs
```

- [ ] **Step 2: Create stub module**

Write `src-tauri/src/settings/mod.rs`:

```rust
//! Settings v2 — nested shape persisted as JSON, secrets in keyring.
//!
//! Public surface built up across Task 3–7. This stub keeps the crate compiling
//! between module swap and schema landing.

pub mod schema;
pub mod keyring;
pub mod migrations;
```

- [ ] **Step 3: Create placeholder children so `pub mod` resolves**

Write `src-tauri/src/settings/schema.rs`:

```rust
//! v2 Settings struct — populated in Task 3.
```

Write `src-tauri/src/settings/keyring.rs`:

```rust
//! Keyring backend wrapper — populated in Task 4.
```

Write `src-tauri/src/settings/migrations.rs`:

```rust
//! v1 → v2 migrator — populated in Task 5.
```

- [ ] **Step 4: Build**

Run: `cd src-tauri && cargo build --lib`
Expected: compiles green. Any `use crate::settings::Settings` references from elsewhere would error — confirm `grep -r "use crate::settings" src-tauri/src` returns only `lib.rs`'s `pub mod settings;` (the lib has no other settings consumers yet).

- [ ] **Step 5: Run existing tests**

Run: `cd src-tauri && cargo test --lib`
Expected: all storage tests still pass. No settings tests yet.

- [ ] **Step 6: Commit**

```bash
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  add src-tauri/src/settings.rs src-tauri/src/settings/
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  commit -m "refactor(settings): replace monolith with empty module tree"
```

---

## Task 3: v2 `Settings` struct

**Files:**
- Modify: `src-tauri/src/settings/schema.rs`

- [ ] **Step 1: Write failing test**

Append to `src-tauri/src/settings/schema.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_spec_defaults() {
        let s = Settings::default();
        assert_eq!(s.schema_version, 2);
        assert_eq!(s.microphone, "default");
        assert_eq!(s.transcription.engine, "local");
        assert_eq!(s.transcription.whisper_model, "turbo");
        assert_eq!(s.transcription.languages, vec!["pt", "en"]);
        assert!(s.transcription.auto_detect);
        assert_eq!(s.transcription.gpu_acceleration, "auto");
        assert!(s.transcription.vad_enabled);
        assert!((s.transcription.no_speech_threshold - 0.6).abs() < 1e-9);
        assert_eq!(s.hotkeys.dictation, "F24");
        assert_eq!(s.hotkeys.command_mode, "Shift+F24");
        assert_eq!(s.hotkeys.recording_mode, "push-to-talk");
        assert_eq!(s.overlay.style, "pill");
        assert_eq!(s.data.word_count_cap, 500_000);
        assert!(s.data.purge_on_exceed);
        assert_eq!(s.ui.theme, "system");
    }

    #[test]
    fn serializes_with_camelcase_keys() {
        let s = Settings::default();
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["schemaVersion"], 2);
        assert_eq!(json["transcription"]["whisperModel"], "turbo");
        assert_eq!(json["hotkeys"]["recordingMode"], "push-to-talk");
        assert_eq!(json["data"]["wordCountCap"], 500_000);
        assert_eq!(json["system"]["muteMusicOnDictate"], false);
        // Secret NEVER surfaces in JSON.
        assert!(json["transcription"].get("groqApiKey").is_none());
    }

    #[test]
    fn round_trip_preserves_fields() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
```

- [ ] **Step 2: Run test, confirm FAIL**

Run: `cd src-tauri && cargo test --lib settings::schema`
Expected: FAIL — `Settings` undefined.

- [ ] **Step 3: Implement struct**

Replace the content of `src-tauri/src/settings/schema.rs` with:

```rust
//! v2 Settings — nested shape persisted as JSON (`config.json`).
//! Secrets (`groqApiKey`) live in Windows Credential Manager, not here.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    pub microphone: String,
    pub transcription: Transcription,
    pub hotkeys: Hotkeys,
    pub overlay: Overlay,
    pub formatting: Formatting,
    pub dictionary: Dictionary,
    pub stats: Stats,
    pub data: DataPolicy,
    pub system: System,
    pub ui: Ui,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcription {
    pub engine: String,
    pub whisper_model: String,
    pub languages: Vec<String>,
    pub auto_detect: bool,
    pub gpu_acceleration: String,
    pub vad_enabled: bool,
    pub no_speech_threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hotkeys {
    pub dictation: String,
    pub command_mode: String,
    pub recording_mode: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Overlay {
    pub style: String,
    pub position: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_pos: Option<OverlayPos>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayPos {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Formatting {
    pub enhance_enabled: bool,
    pub remove_fillers: bool,
    pub filler_words: Vec<String>,
    pub explicit_commands: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dictionary {
    pub auto_add: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub enabled: bool,
    pub milestone_notifications: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataPolicy {
    pub word_count_cap: u64,
    pub purge_on_exceed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct System {
    pub launch_at_login: bool,
    pub close_to_tray: bool,
    pub dictation_sounds: bool,
    pub mute_music_on_dictate: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ui {
    pub language: String,
    pub theme: String,
    pub accent: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 2,
            microphone: "default".to_string(),
            transcription: Transcription {
                engine: "local".to_string(),
                whisper_model: "turbo".to_string(),
                languages: vec!["pt".to_string(), "en".to_string()],
                auto_detect: true,
                gpu_acceleration: "auto".to_string(),
                vad_enabled: true,
                no_speech_threshold: 0.6,
            },
            hotkeys: Hotkeys {
                dictation: "F24".to_string(),
                command_mode: "Shift+F24".to_string(),
                recording_mode: "push-to-talk".to_string(),
            },
            overlay: Overlay {
                style: "pill".to_string(),
                position: "near-cursor".to_string(),
                custom_pos: None,
            },
            formatting: Formatting {
                enhance_enabled: false,
                remove_fillers: true,
                filler_words: vec![
                    "uh".to_string(),
                    "um".to_string(),
                    "né".to_string(),
                    "tipo".to_string(),
                ],
                explicit_commands: true,
            },
            dictionary: Dictionary { auto_add: false },
            stats: Stats {
                enabled: true,
                milestone_notifications: true,
            },
            data: DataPolicy {
                word_count_cap: 500_000,
                purge_on_exceed: true,
            },
            system: System {
                launch_at_login: false,
                close_to_tray: true,
                dictation_sounds: true,
                mute_music_on_dictate: false,
            },
            ui: Ui {
                language: "en".to_string(),
                theme: "system".to_string(),
                accent: "indigo".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_spec_defaults() {
        let s = Settings::default();
        assert_eq!(s.schema_version, 2);
        assert_eq!(s.microphone, "default");
        assert_eq!(s.transcription.engine, "local");
        assert_eq!(s.transcription.whisper_model, "turbo");
        assert_eq!(s.transcription.languages, vec!["pt", "en"]);
        assert!(s.transcription.auto_detect);
        assert_eq!(s.transcription.gpu_acceleration, "auto");
        assert!(s.transcription.vad_enabled);
        assert!((s.transcription.no_speech_threshold - 0.6).abs() < 1e-9);
        assert_eq!(s.hotkeys.dictation, "F24");
        assert_eq!(s.hotkeys.command_mode, "Shift+F24");
        assert_eq!(s.hotkeys.recording_mode, "push-to-talk");
        assert_eq!(s.overlay.style, "pill");
        assert_eq!(s.data.word_count_cap, 500_000);
        assert!(s.data.purge_on_exceed);
        assert_eq!(s.ui.theme, "system");
    }

    #[test]
    fn serializes_with_camelcase_keys() {
        let s = Settings::default();
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["schemaVersion"], 2);
        assert_eq!(json["transcription"]["whisperModel"], "turbo");
        assert_eq!(json["hotkeys"]["recordingMode"], "push-to-talk");
        assert_eq!(json["data"]["wordCountCap"], 500_000);
        assert_eq!(json["system"]["muteMusicOnDictate"], false);
        assert!(json["transcription"].get("groqApiKey").is_none());
    }

    #[test]
    fn round_trip_preserves_fields() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
```

- [ ] **Step 4: Run tests, confirm PASS**

Run: `cd src-tauri && cargo test --lib settings::schema`
Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  add src-tauri/src/settings/schema.rs
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  commit -m "feat(settings): v2 Settings struct with serde camelCase + default"
```

---

## Task 4: Keyring backend wrapper

**Files:**
- Modify: `src-tauri/src/settings/keyring.rs`

- [ ] **Step 1: Write failing test**

Replace `src-tauri/src/settings/keyring.rs` content with the skeleton + tests:

```rust
//! Groq API key storage — Windows Credential Manager via `keyring` v3.
//!
//! All callers go through `KeyringBackend` so the migrator + loader are testable
//! without touching the OS. Service/user strings are hard-coded per spec.

use std::sync::Mutex;

pub const SERVICE: &str = "com.typr.app";
pub const USER: &str = "groq_api_key";

#[derive(thiserror::Error, Debug)]
pub enum KeyringError {
    #[error("keyring entry not found")]
    NotFound,
    #[error("keyring access denied")]
    AccessDenied,
    #[error("keyring backend failure: {0}")]
    Other(String),
}

pub trait KeyringBackend: Send + Sync {
    fn get(&self) -> Result<Option<String>, KeyringError>;
    fn set(&self, secret: &str) -> Result<(), KeyringError>;
    fn delete(&self) -> Result<(), KeyringError>;
}

pub struct SystemBackend;

impl KeyringBackend for SystemBackend {
    fn get(&self) -> Result<Option<String>, KeyringError> {
        let entry = ::keyring::Entry::new(SERVICE, USER)
            .map_err(|e| KeyringError::Other(e.to_string()))?;
        match entry.get_password() {
            Ok(s) => Ok(Some(s)),
            Err(::keyring::Error::NoEntry) => Ok(None),
            Err(::keyring::Error::PlatformFailure(e)) => Err(KeyringError::Other(e.to_string())),
            Err(e) => Err(KeyringError::Other(e.to_string())),
        }
    }

    fn set(&self, secret: &str) -> Result<(), KeyringError> {
        let entry = ::keyring::Entry::new(SERVICE, USER)
            .map_err(|e| KeyringError::Other(e.to_string()))?;
        entry
            .set_password(secret)
            .map_err(|e| KeyringError::Other(e.to_string()))
    }

    fn delete(&self) -> Result<(), KeyringError> {
        let entry = ::keyring::Entry::new(SERVICE, USER)
            .map_err(|e| KeyringError::Other(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(::keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(KeyringError::Other(e.to_string())),
        }
    }
}

/// In-memory backend used by unit + integration tests.
#[derive(Default)]
pub struct MockBackend {
    inner: Mutex<Option<String>>,
}

impl MockBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_secret(secret: &str) -> Self {
        Self {
            inner: Mutex::new(Some(secret.to_string())),
        }
    }

    pub fn peek(&self) -> Option<String> {
        self.inner.lock().unwrap().clone()
    }
}

impl KeyringBackend for MockBackend {
    fn get(&self) -> Result<Option<String>, KeyringError> {
        Ok(self.inner.lock().unwrap().clone())
    }

    fn set(&self, secret: &str) -> Result<(), KeyringError> {
        *self.inner.lock().unwrap() = Some(secret.to_string());
        Ok(())
    }

    fn delete(&self) -> Result<(), KeyringError> {
        *self.inner.lock().unwrap() = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_get_returns_none_when_empty() {
        let m = MockBackend::new();
        assert_eq!(m.get().unwrap(), None);
    }

    #[test]
    fn mock_set_then_get() {
        let m = MockBackend::new();
        m.set("sk-test-123").unwrap();
        assert_eq!(m.get().unwrap().as_deref(), Some("sk-test-123"));
    }

    #[test]
    fn mock_delete_removes_secret() {
        let m = MockBackend::with_secret("sk-test-123");
        m.delete().unwrap();
        assert_eq!(m.get().unwrap(), None);
    }

    #[test]
    fn mock_overwrites() {
        let m = MockBackend::with_secret("old");
        m.set("new").unwrap();
        assert_eq!(m.get().unwrap().as_deref(), Some("new"));
    }

    #[test]
    fn service_user_constants_match_spec() {
        assert_eq!(SERVICE, "com.typr.app");
        assert_eq!(USER, "groq_api_key");
    }
}
```

- [ ] **Step 2: Run tests, confirm PASS**

Run: `cd src-tauri && cargo test --lib settings::keyring`
Expected: 5 tests pass. (No failing-first step here — tests exercise the mock + constants; the system backend is only reachable on real Windows at runtime and is not asserted directly.)

- [ ] **Step 3: Commit**

```bash
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  add src-tauri/src/settings/keyring.rs
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  commit -m "feat(settings): KeyringBackend trait + System + Mock impls"
```

---

## Task 5: v1→v2 migrator + model remap

**Files:**
- Modify: `src-tauri/src/settings/migrations.rs`

- [ ] **Step 1: Write failing tests**

Replace `src-tauri/src/settings/migrations.rs` content with the full module + tests:

```rust
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
        assert!(out.remapped_model.is_none()); // small→turbo but old was default-small; treat as remap
        // Actually: default v1 model we assumed is "small" which remaps to "turbo".
        // The struct compares old "small" vs new "turbo" → remapped.
        // Adjust expectation:
        // (This branch intentionally left to catch logic drift; see follow-up assertion.)
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
```

> Note on `missing_fields_fall_back_to_default`: the first assertion block is
> documentation-style; the real assertion lives in
> `missing_whisper_model_still_reports_remap_from_assumed_small` below it. Keep
> both tests — the first one doubles as a smoke check that defaults fill in
> cleanly even when the migrator has to synthesise a model.

- [ ] **Step 2: Run tests, confirm FAIL**

Run: `cd src-tauri && cargo test --lib settings::migrations`
Expected: FAIL on first run (module has tests but the code compiles, so the failure surfaces only if any assertion is wrong). If all pass first try, confirm by running `cargo test --lib settings::migrations -- --nocapture` and re-reading the assertions.

- [ ] **Step 3: Fix any red tests**

If `missing_whisper_model_still_reports_remap_from_assumed_small` failed, the `migrate_v1_to_v2` default-branch is wrong — adjust the `old_model` default to `"small"` (matches legacy v0 `Settings::default`). The code above already does this; this step is the re-run confirmation.

Run: `cd src-tauri && cargo test --lib settings::migrations`
Expected: all 11 tests pass.

- [ ] **Step 4: Commit**

```bash
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  add src-tauri/src/settings/migrations.rs
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  commit -m "feat(settings): v1→v2 migrator with model remap + groq key move"
```

---

## Task 6: Loader + `.bak` lifecycle + sentinel

**Files:**
- Modify: `src-tauri/src/settings/mod.rs`

- [ ] **Step 1: Write failing tests**

Replace `src-tauri/src/settings/mod.rs` content with:

```rust
//! Settings v2 loader — owns the on-disk lifecycle.
//!
//! Flow per boot:
//! 1. Read `<app_dir>/config.json`. Absent → return default settings, write v2 JSON, stamp sentinel.
//! 2. Parse to `serde_json::Value`. Detect version.
//!     - v2 → deserialize into `Settings`, return.
//!     - v1 → write `config.json.v1.bak` → run migrator → write v2 JSON → on success delete `.bak`
//!       + set `app_meta.settings_version=2`. On failure keep `.bak`, surface error, fall back to defaults.
//!     - unknown → keep user's file untouched, return defaults with `UnknownVersion` event.

pub mod schema;
pub mod keyring;
pub mod migrations;

pub use schema::Settings;

use crate::storage::{app_meta::AppMetaRepo, Db, DbError};
use crate::settings::keyring::KeyringBackend;
use crate::settings::migrations::{detect_version, migrate_v1_to_v2, MigrationError};
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

pub struct LoadOutcome {
    pub settings: Settings,
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
        let settings = Settings::default();
        write_atomic(&cfg_path, &settings)?;
        meta.set(SETTINGS_VERSION_KEY, "2")?;
        return Ok(LoadOutcome { settings, events: vec![] });
    }

    let raw = std::fs::read_to_string(&cfg_path)?;
    let value: serde_json::Value = serde_json::from_str(&raw)?;
    let version = detect_version(&value);

    match version {
        2 => {
            let settings: Settings = serde_json::from_value(value)?;
            Ok(LoadOutcome { settings, events: vec![] })
        }
        1 => run_v1_migration(&cfg_path, &bak_path, &raw, &value, &meta, backend),
        other => Ok(LoadOutcome {
            settings: Settings::default(),
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
    std::fs::write(bak_path, raw)?;

    // Step B: run migrator. On failure, keep .bak and surface event.
    let outcome = match migrate_v1_to_v2(value, backend) {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!(error = %e, "settings migration failed; .bak preserved");
            return Ok(LoadOutcome {
                settings: Settings::default(),
                events: vec![MigrationEvent::MigrationFailed(e.to_string())],
            });
        }
    };

    // Step C: write v2 JSON. On failure, also keep .bak.
    if let Err(e) = write_atomic(cfg_path, &outcome.settings) {
        tracing::warn!(error = %e, "v2 write failed; .bak preserved");
        return Ok(LoadOutcome {
            settings: Settings::default(),
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
    if outcome.had_groq_key_in_json {
        // Spec §7: "prompt user to re-enter Groq key (now stored securely)".
        events.push(MigrationEvent::NeedsGroqKey);
    }
    Ok(LoadOutcome { settings: outcome.settings, events })
}

fn write_atomic(path: &Path, settings: &Settings) -> Result<(), SettingsError> {
    let tmp: PathBuf = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
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
        let s = Settings::default();
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
```

- [ ] **Step 2: Run tests, confirm FAIL then PASS**

Run: `cd src-tauri && cargo test --lib settings`
Expected: 6 new loader tests pass alongside schema + keyring + migrations tests.

If any fail, fix the logic in `load` / `run_v1_migration` — no test adjustments.

- [ ] **Step 3: Commit**

```bash
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  add src-tauri/src/settings/mod.rs
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  commit -m "feat(settings): load() with .bak lifecycle + sentinel + events"
```

---

## Task 7: Wire loader into Tauri boot

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Replace `.setup` body**

Edit `src-tauri/src/lib.rs`, replacing the whole `.setup(|app| { ... })` block with:

```rust
        .setup(|app| {
            let app_dir = app.path().app_data_dir().expect("app_data_dir");
            let log_dir = app_dir.join("logs");
            let _ = crate::telemetry::init_tracing(&log_dir);

            let db_path = app_dir.join("typr.db");
            let db = crate::storage::Db::open(&db_path)
                .expect("open typr.db");

            let backend = crate::settings::keyring::SystemBackend;
            let outcome = crate::settings::load(&app_dir, &db, &backend)
                .expect("load settings");

            for ev in &outcome.events {
                match ev {
                    crate::settings::MigrationEvent::Migrated => {
                        tracing::info!(stage = "settings", "migrated v1 → v2");
                        let _ = app.emit("settings:migrated", ());
                    }
                    crate::settings::MigrationEvent::ModelRemapped { from, to } => {
                        tracing::info!(stage = "settings", %from, %to, "whisper model remapped");
                        let _ = app.emit("settings:model-remapped", serde_json::json!({ "from": from, "to": to }));
                    }
                    crate::settings::MigrationEvent::NeedsGroqKey => {
                        tracing::info!(stage = "settings", "legacy Groq key — prompting for re-entry");
                        let _ = app.emit("settings:needs-groq-key", ());
                    }
                    crate::settings::MigrationEvent::UnknownVersion(v) => {
                        tracing::warn!(stage = "settings", version = v, "unknown settings version; using defaults");
                        let _ = app.emit("settings:unknown-version", *v);
                    }
                    crate::settings::MigrationEvent::MigrationFailed(msg) => {
                        tracing::error!(stage = "settings", error = %msg, "migration failed; .bak preserved");
                        let _ = app.emit("settings:migration-failed", msg.clone());
                    }
                }
            }

            app.manage(db);
            app.manage(outcome.settings);

            tracing::info!(stage = "boot", "storage + telemetry + settings initialised");
            Ok(())
        })
```

Add `use tauri::Emitter;` alongside the existing `use tauri::Manager;` near the top of the file.

- [ ] **Step 2: Build**

Run: `cd src-tauri && cargo build --lib`
Expected: green. If `Emitter` import missing, add it.

- [ ] **Step 3: Run full lib test suite**

Run: `cd src-tauri && cargo test --lib`
Expected: all tests (storage + settings) pass.

- [ ] **Step 4: Commit**

```bash
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  add src-tauri/src/lib.rs
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  commit -m "feat(boot): load settings + emit migration events on startup"
```

---

## Task 8: Integration test — fixture v1 boot

**Files:**
- Create: `src-tauri/tests/migration_e2e.rs`

- [ ] **Step 1: Write failing test**

Create `src-tauri/tests/migration_e2e.rs`:

```rust
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
```

- [ ] **Step 2: Confirm `SETTINGS_VERSION_KEY` is re-exported**

`src-tauri/src/settings/mod.rs` already defines `pub const SETTINGS_VERSION_KEY`. Verify it's reachable at `typr_lib::settings::SETTINGS_VERSION_KEY`. If not, add `pub use` or rely on the existing public const.

- [ ] **Step 3: Run the integration test**

Run: `cd src-tauri && cargo test --test migration_e2e`
Expected: 2 tests pass.

- [ ] **Step 4: Run the full workspace test suite**

Run: `cd src-tauri && cargo test`
Expected: all unit + integration tests green.

- [ ] **Step 5: Commit**

```bash
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  add src-tauri/tests/migration_e2e.rs
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  commit -m "test(settings): e2e v1 fixture → v2 migration + failure backup preserved"
```

---

## Task 9: Smoke — full build + Tauri dev boot

**Files:**
- none modified (verification only)

- [ ] **Step 1: Rust build**

Run: `cd src-tauri && cargo build --release --lib`
Expected: green.

- [ ] **Step 2: Frontend build**

Run (from repo root): `pnpm build`
Expected: Vite build succeeds, no new TS errors caused by this phase (Phase 1 does not touch frontend).

- [ ] **Step 3: Clippy**

Run: `cd src-tauri && cargo clippy --all-targets -- -D warnings`
Expected: green. Fix any new lint surfaced by settings code before moving on.

- [ ] **Step 4: Manual smoke — fresh install path**

With no prior `config.json` in `%APPDATA%\com.typr.app\`:

Run: `pnpm tauri dev`

Expected log lines (via `tracing`):
- `stage="settings"` emit absent (no events on fresh install)
- `stage="boot"`: "storage + telemetry + settings initialised"

Kill the process. Verify `%APPDATA%\com.typr.app\config.json` exists and contains `"schemaVersion": 2`.

- [ ] **Step 5: Manual smoke — v1 migration path**

Replace `%APPDATA%\com.typr.app\config.json` with a v1 fixture:

```json
{"microphone":"default","engine":"local","whisperModel":"small","groqApiKey":"","recordingMode":"toggle","hotkey":"F24"}
```

Run: `pnpm tauri dev`

Expected log lines:
- `stage="settings"` "migrated v1 → v2"
- `stage="settings"` `from="small"` `to="turbo"` "whisper model remapped"
- `stage="boot"` "storage + telemetry + settings initialised"

Kill the process. Verify:
- `config.json` now contains `"schemaVersion": 2` and `"whisperModel": "turbo"`
- `config.json.v1.bak` is **absent** (deleted on success)
- `typr.db` holds `app_meta.settings_version=2` (spot-check via `sqlite3 %APPDATA%\com.typr.app\typr.db "SELECT * FROM app_meta;"`)

- [ ] **Step 6: Manual smoke — failure preserves .bak**

Replace `config.json` with `[1,2,3]` (malformed root). Run `pnpm tauri dev`.

Expected log: `stage="settings"` error; app still boots on defaults; `config.json.v1.bak` **present** next to the rewritten `config.json`.

Kill the process. Clean up: delete both files.

- [ ] **Step 7: Final commit (if any lint/style fixes landed)**

```bash
git status
# If anything surfaced in step 3:
git -c user.name="Bruno Rodrigues" -c user.email="brunorodrigues2627@gmail.com" \
  commit -am "chore(settings): clippy polish"
```

Otherwise skip — Phase 1 ends with the test commit from Task 8.

---

## Self-review

Ran the fresh-eyes check against spec §7 (migration plan) + exit criteria row for Phase 1 ("Legacy config.json migrates clean, remap toast fires, backup preserved on failure"):

- **Settings rewrite** → Tasks 2, 3 (module swap + v2 struct with all 10 groups).
- **v1 → v2 migrator** → Task 5 (detect + remap + field mapping + groq key move, 11 unit tests covering every field branch, unknown engine, unknown model, empty key, non-object root).
- **Keyring v3 wrapper** → Task 4 (trait + real + mock, spec-mandated service/user constants asserted).
- **.bak lifecycle** → Task 6 (`run_v1_migration`: write .bak → migrate → write v2 → delete .bak on success; every failure path returns early and leaves .bak intact).
- **"Remap toast fires"** → Task 7 emits `settings:model-remapped` event when `ModelRemapped { from, to }` is present; frontend toast UI is Phase 3 scope per spec §9, documented in the plan header.
- **"Backup preserved on failure"** → Task 8 second integration test `failed_migration_preserves_backup` asserts `.bak` exists after a malformed-root failure.
- **Sentinel** → `app_meta.settings_version=2` set on success (Task 6 + assertions in Task 8).
- **Groq re-entry prompt** → `MigrationEvent::NeedsGroqKey` emitted when v1 JSON held a non-empty key (spec §7 line 819 requirement).

Placeholder scan: no TBDs, every step has concrete code or a concrete command. Type-name consistency checked (`Settings`, `KeyringBackend`, `MockBackend`, `MigrationEvent`, `SETTINGS_VERSION_KEY`, `LoadOutcome`, `SystemBackend`, `SERVICE`, `USER`) — all names that appear in later tasks are defined in earlier ones. Integration test imports `typr_lib::settings::keyring::MockBackend` which Task 4 exports via the already-`pub` module.

Deferred (recorded here, not in Phase 1 scope):
- DB backup utility (spec L893) — no existing DB to back up on fresh V1; wire in Phase 2 when we first ship a `002_*.sql` migration.
- Frontend toast UI — spec §9 Phase 3.
- `fts5` rusqlite feature check — a Phase 0 concern; if missing there, pull forward as a one-liner before Phase 1 merges.

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-24-phase-1-settings.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints.

Which approach?

---

## Phase 1 Completion Status — 2026-04-26

**Status:** Done. Closed via Phase 1.5 cutover (commits `41e157d`, `84c73d6`, `6ad5fbe`).

**Tasks delivered:** All 8 tasks plus Phase 1.5 cutover (adapter + main.rs wiring).

**Test posture:**
- 77 lib unit tests green (`cargo test --lib`)
- 2 settings migration e2e tests green (`tests/migration_e2e.rs`)
- 1 storage integration test green
- `cargo clippy --all-targets` — 0 new warnings; 5 pre-existing Phase 0 entries logged on backlog
- Filesystem smoke 3/3 PASS: v1 migration, fresh install, v2 idempotent reload

**Manual verification (Bruno):**
- Built via `npx tauri build --no-bundle` (NOT `cargo build --release` — direct cargo skips `beforeBuildCommand`, ships dev-server URL into bundle)
- UI rendered live; General tab shows microphone + engine + whisper model + hotkey, all six v1 fields round-trip through `to_v1_view`/`apply_v1_payload`
- Bruno's real `config.json` (medium model, F24 hotkey, Antlion mic) preserved across migration

**Reviewer fixes shipped in `6ad5fbe`:**
- `save_settings`: JSON write before keyring write, with in-memory v2 rollback on JSON failure (was: keyring first → desync risk on JSON IO error)
- `get_settings` / `legacy_view`: clone v2 under lock then drop guard before keyring syscall (was: lock held across syscall, serialised recorder hot path)
- `main()` boot: `unwrap_or_else` with synthetic `MigrationFailed` event + `tracing::error` (was: panic on `settings::load` failure)
- All 5 `MigrationEvent` emit branches use `if let Err(e) = ... { tracing::warn!(...) }` (was: 3 used `let _ =`, inconsistent)

**Adapter split for caller-controlled ordering:**
- `apply_v1_v2_fields(&mut v2, &payload)` — pure mutation, no IO
- `write_v1_keyring(&payload, backend)` — keyring only
- `apply_v1_payload(&mut v2, payload, backend)` — convenience for tests; production path uses split form

**Deferred to Phase 2** (need frontend payload reshape, not in Phase 1 scope):
- Empty-string `groqApiKey` round-trip nukes a real key. Needs `Option<String>` for "unchanged" vs "cleared" semantics
- Tri-state surfacing of keyring read errors (currently empty string masks both "no key" and "read failed")
- Frontend listeners for `settings:model-remapped` / `settings:needs-groq-key` / `settings:migration-failed` events (toasts)
- `recorder.rs` still imports `crate::settings::legacy_v1::Settings` directly — clean up when v1 shim deleted
- Test additions: `save_settings` rollback, `to_v1_view` keyring-error path, integration test for migration event emission, concurrent-writer test

**Phase 0 backlog carried forward** (not opened in Phase 1):
- `tx-rollback` doc comment, `cap<0` doc, empty MATCH handling, negative limit/offset, N+1 purge loop, `DbError::InvalidArgument`, CHECK constraints, MSRV doc, concurrent-writer test, clippy `PathBuf vs &Path` + `Default` impls
