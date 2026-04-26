//! v1 ↔ v2 projection used by the Tauri command surface during the Phase 1.5
//! cutover. The frontend still speaks the legacy six-field shape, so we read
//! the in-memory v2 [`schema::Settings`] + the OS keyring secret into a
//! [`legacy_v1::Settings`] for `get_settings`, and apply the inverse mapping
//! in `save_settings`.
//!
//! Field mapping (v1 → v2):
//! - `microphone`     → `Settings.microphone`
//! - `engine`         → `Settings.transcription.engine`
//! - `whisperModel`   → `Settings.transcription.whisper_model`
//! - `groqApiKey`     → keyring (`com.typr.app` / `groq_api_key`)
//! - `recordingMode`  → `Settings.hotkeys.recording_mode`
//! - `hotkey`         → `Settings.hotkeys.dictation`
//!
//! Fields outside the v1 shape (`overlay`, `formatting`, `dictionary`, `stats`,
//! `data`, `system`, `ui`, plus `transcription.languages` etc.) are left
//! untouched on apply — they are owned by Phase 2 UI surfaces.

use crate::settings::keyring::{KeyringBackend, KeyringError};
use crate::settings::legacy_v1;
use crate::settings::schema::Settings;

/// Project the in-memory v2 settings + keyring secret into a v1-shaped struct
/// suitable for `serde_json` serialisation back to the existing frontend.
///
/// A keyring read failure is treated as "no key on file" — we surface the empty
/// string rather than failing the whole `get_settings` call. The boot-time
/// migration already logs and emits a `settings:needs-groq-key` event when the
/// secret is missing, so the frontend can prompt independently.
pub fn to_v1_view(v2: &Settings, backend: &dyn KeyringBackend) -> legacy_v1::Settings {
    let groq_api_key = match backend.get() {
        Ok(Some(s)) => s,
        Ok(None) => String::new(),
        Err(e) => {
            tracing::warn!(error = %e, "keyring read failed in to_v1_view; surfacing empty key");
            String::new()
        }
    };

    legacy_v1::Settings {
        microphone: v2.microphone.clone(),
        engine: v2.transcription.engine.clone(),
        whisper_model: v2.transcription.whisper_model.clone(),
        groq_api_key,
        recording_mode: v2.hotkeys.recording_mode.clone(),
        hotkey: v2.hotkeys.dictation.clone(),
    }
}

/// Apply a v1-shaped payload from the frontend onto an existing v2 settings
/// tree. Keyring side-effect: if `groq_api_key` is non-empty we `set` it; if
/// empty we `delete` it (so users can clear the credential through the UI).
///
/// Any error from the keyring is returned — the caller is expected to refuse
/// to persist the JSON in that case so the on-disk and OS-stored states stay
/// in sync.
pub fn apply_v1_payload(
    target: &mut Settings,
    payload: legacy_v1::Settings,
    backend: &dyn KeyringBackend,
) -> Result<(), KeyringError> {
    target.microphone = payload.microphone;
    target.transcription.engine = payload.engine;
    target.transcription.whisper_model = payload.whisper_model;
    target.hotkeys.recording_mode = payload.recording_mode;
    target.hotkeys.dictation = payload.hotkey;

    if payload.groq_api_key.is_empty() {
        backend.delete()?;
    } else {
        backend.set(&payload.groq_api_key)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::keyring::MockBackend;

    #[test]
    fn to_v1_view_with_empty_keyring_reports_empty_key() {
        let v2 = Settings::default();
        let kr = MockBackend::new();
        let v1 = to_v1_view(&v2, &kr);
        assert_eq!(v1.groq_api_key, "");
        assert_eq!(v1.microphone, v2.microphone);
        assert_eq!(v1.engine, v2.transcription.engine);
        assert_eq!(v1.whisper_model, v2.transcription.whisper_model);
        assert_eq!(v1.recording_mode, v2.hotkeys.recording_mode);
        assert_eq!(v1.hotkey, v2.hotkeys.dictation);
    }

    #[test]
    fn to_v1_view_pulls_secret_from_keyring() {
        let v2 = Settings::default();
        let kr = MockBackend::with_secret("sk-live-42");
        let v1 = to_v1_view(&v2, &kr);
        assert_eq!(v1.groq_api_key, "sk-live-42");
    }

    #[test]
    fn apply_v1_payload_writes_secret_when_present() {
        let mut v2 = Settings::default();
        let kr = MockBackend::new();
        let payload = legacy_v1::Settings {
            microphone: "Built-in".to_string(),
            engine: "cloud".to_string(),
            whisper_model: "large-v3".to_string(),
            groq_api_key: "sk-new".to_string(),
            recording_mode: "toggle".to_string(),
            hotkey: "F9".to_string(),
        };
        apply_v1_payload(&mut v2, payload, &kr).unwrap();
        assert_eq!(v2.microphone, "Built-in");
        assert_eq!(v2.transcription.engine, "cloud");
        assert_eq!(v2.transcription.whisper_model, "large-v3");
        assert_eq!(v2.hotkeys.recording_mode, "toggle");
        assert_eq!(v2.hotkeys.dictation, "F9");
        assert_eq!(kr.peek().as_deref(), Some("sk-new"));
    }

    #[test]
    fn apply_v1_payload_with_empty_key_clears_keyring() {
        let mut v2 = Settings::default();
        let kr = MockBackend::with_secret("sk-stale");
        let payload = legacy_v1::Settings {
            microphone: v2.microphone.clone(),
            engine: v2.transcription.engine.clone(),
            whisper_model: v2.transcription.whisper_model.clone(),
            groq_api_key: String::new(),
            recording_mode: v2.hotkeys.recording_mode.clone(),
            hotkey: v2.hotkeys.dictation.clone(),
        };
        apply_v1_payload(&mut v2, payload, &kr).unwrap();
        assert_eq!(kr.peek(), None);
    }

    #[test]
    fn apply_v1_payload_preserves_v2_only_fields() {
        let mut v2 = Settings::default();
        // Mutate fields outside the v1 shape so we can confirm they survive a round-trip.
        v2.overlay.style = "bar".to_string();
        v2.data.word_count_cap = 12_345;
        v2.ui.theme = "dark".to_string();
        v2.transcription.languages = vec!["fr".to_string()];
        let kr = MockBackend::new();

        let v1_view = to_v1_view(&v2, &kr);
        // Round-trip the v1 projection back through apply.
        let mut v2_after = v2.clone();
        apply_v1_payload(&mut v2_after, v1_view, &kr).unwrap();

        assert_eq!(v2_after.overlay.style, "bar");
        assert_eq!(v2_after.data.word_count_cap, 12_345);
        assert_eq!(v2_after.ui.theme, "dark");
        assert_eq!(v2_after.transcription.languages, vec!["fr"]);
        assert_eq!(v2_after, v2);
    }

    #[test]
    fn round_trip_preserves_v1_fields() {
        let mut v2 = Settings::default();
        let kr = MockBackend::new();

        // Seed with a non-default v1 payload.
        let original = legacy_v1::Settings {
            microphone: "MicArray".to_string(),
            engine: "groq".to_string(),
            whisper_model: "base".to_string(),
            groq_api_key: "sk-rt".to_string(),
            recording_mode: "push-to-talk".to_string(),
            hotkey: "Shift+F12".to_string(),
        };
        apply_v1_payload(&mut v2, original.clone(), &kr).unwrap();

        // Now re-project and assert equality.
        let projected = to_v1_view(&v2, &kr);
        assert_eq!(projected, original);
    }
}
