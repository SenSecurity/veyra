//! Pipeline transcription dispatcher.
//!
//! Selects the local (whisper.cpp sidecar) or cloud (Groq) engine based on
//! `settings.transcription.engine` and returns a typed [`TranscriptionResult`].
//!
//! Phase 2 keeps `language` as `None` in both arms — neither engine is wired
//! to surface it yet (local uses `-otxt`, cloud uses `response_format: json`).
//! `duration_ms` is wall-clock pipeline cost measured by the engine modules,
//! not upstream-reported audio duration.

use std::path::Path;

use tauri::AppHandle;

use crate::settings::Settings;
use crate::{transcribe_groq, transcribe_local};

/// Local whisper-cpp models that Phase 2 still ships. Anything else (medium,
/// small, tiny, …) is rejected by the dispatcher because Phase 2 retired
/// those file paths during settings migration.
pub const ALLOWED_LOCAL_MODELS: &[&str] = &["turbo", "large-v3", "base"];

#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub duration_ms: u64,
    pub model: String,
}

#[derive(Debug)]
pub enum TranscribeError {
    /// User's settings still reference a whisper model that Phase 2 no longer
    /// allows. Carries the offending model name.
    ModelRetired(String),
    /// Selected model is allowed but the `ggml-<model>.bin` file is missing
    /// from `app_dir`. Carries the absolute path that was probed.
    ModelFileMissing(String),
    /// Any other failure surfaced by the underlying engine module.
    Engine(String),
}

impl std::fmt::Display for TranscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranscribeError::ModelRetired(m) => {
                write!(f, "whisper model `{m}` is retired; please pick turbo, large-v3, or base")
            }
            TranscribeError::ModelFileMissing(p) => {
                write!(f, "whisper model file missing at {p}; download it from settings")
            }
            TranscribeError::Engine(m) => write!(f, "transcription engine error: {m}"),
        }
    }
}

impl std::error::Error for TranscribeError {}

/// Dispatch to the appropriate transcription engine based on
/// `settings.transcription.engine`. Returns a typed [`TranscriptionResult`].
///
/// `groq_key` is only consulted for the cloud engine; `None` is fine when the
/// caller knows the local engine is selected.
pub async fn dispatch(
    app: &AppHandle,
    app_dir: &Path,
    wav_path: &Path,
    settings: &Settings,
    groq_key: Option<&str>,
) -> Result<TranscriptionResult, TranscribeError> {
    match settings.transcription.engine.as_str() {
        "local" => {
            let model = settings.transcription.whisper_model.as_str();
            if !ALLOWED_LOCAL_MODELS.contains(&model) {
                return Err(TranscribeError::ModelRetired(model.to_string()));
            }
            let model_path = app_dir.join(transcribe_local::model_filename(model));
            if !model_path.exists() {
                return Err(TranscribeError::ModelFileMissing(
                    model_path.to_string_lossy().to_string(),
                ));
            }
            transcribe_local::transcribe_local(app, &model_path, &wav_path.to_path_buf())
                .await
                .map_err(TranscribeError::Engine)
        }
        "cloud" | "groq" => {
            let key = groq_key
                .filter(|k| !k.is_empty())
                .ok_or_else(|| TranscribeError::Engine("groq key missing".into()))?;
            transcribe_groq::transcribe_groq(key, &wav_path.to_path_buf())
                .await
                .map_err(TranscribeError::Engine)
        }
        other => Err(TranscribeError::Engine(format!("unknown engine: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_models_list_matches_spec() {
        assert!(ALLOWED_LOCAL_MODELS.contains(&"turbo"));
        assert!(ALLOWED_LOCAL_MODELS.contains(&"large-v3"));
        assert!(ALLOWED_LOCAL_MODELS.contains(&"base"));
        assert!(!ALLOWED_LOCAL_MODELS.contains(&"medium"));
        assert!(!ALLOWED_LOCAL_MODELS.contains(&"small"));
        assert!(!ALLOWED_LOCAL_MODELS.contains(&"tiny"));
    }

    #[test]
    fn dispatch_rejects_retired_model_without_io() {
        // The dispatch function itself needs an AppHandle which is hard to fake
        // in a unit test. The retirement gate is a const slice, so we assert
        // the gate behavior directly: a v3-migrated `medium` setting must fail
        // the membership check before any IO is attempted.
        let retired = "medium";
        assert!(!ALLOWED_LOCAL_MODELS.contains(&retired));
    }

    #[test]
    fn transcription_result_round_trip() {
        let r = TranscriptionResult {
            text: "olá mundo".into(),
            language: Some("pt".into()),
            duration_ms: 1234,
            model: "turbo".into(),
        };
        let cloned = r.clone();
        assert_eq!(r, cloned);
    }

    #[test]
    fn transcribe_error_display_mentions_inputs() {
        let e = TranscribeError::ModelRetired("medium".into());
        assert!(format!("{e}").contains("medium"));
        let e = TranscribeError::ModelFileMissing("C:/x/ggml-turbo.bin".into());
        assert!(format!("{e}").contains("ggml-turbo.bin"));
        let e = TranscribeError::Engine("boom".into());
        assert!(format!("{e}").contains("boom"));
    }
}
