//! Pipeline orchestrator and stage modules.
//!
//! Phase 2 wires only the Dictation arm. Command Mode is Phase 4.

pub mod capture;
pub mod commit;
pub mod format;
pub mod inject;
pub mod tmp;
pub mod transcribe;

use std::path::Path;
use std::sync::Mutex;

use tauri::AppHandle;
use uuid::Uuid;

use crate::audio::AudioRecorder;
use crate::settings::Settings;
use crate::storage::Db;

/// Shared inputs that every stage needs at runtime. Borrowed for the
/// lifetime of one `run_session` call — the orchestrator owns nothing,
/// it just composes existing handles.
pub struct PipelineDeps<'a> {
    pub db: &'a Db,
    pub settings: &'a Settings,
    pub audio: &'a Mutex<AudioRecorder>,
    pub app: &'a AppHandle,
    pub app_dir: &'a Path,
    pub groq_key: Option<&'a str>,
}

/// Which arm of the pipeline to run. Phase 2 only implements `Dictation`;
/// `Command` is rejected with a `StageError::Capture` so the variant exists
/// in the type system today and Phase 4 can flip the branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineMode {
    Dictation,
    Command,
}

/// Per-stage failure tag. Carries a free-form message rather than a typed
/// inner error so we can fan in heterogeneous failures (string from inject,
/// `TranscribeError::Display` from transcribe, `Debug` rendering of
/// `FormatError` / `DbError` from format/persist) without growing a giant
/// enum hierarchy. The string is intended for logs, not pattern matching.
#[derive(Debug)]
pub enum StageError {
    Capture(String),
    Transcribe(String),
    Format(String),
    Inject(String),
    Persist(String),
}

/// Top-level error returned by [`run_session`]. The wrapping struct keeps
/// the public API stable in case we add fields (e.g. a session UUID) later.
#[derive(Debug)]
pub struct PipelineError {
    pub stage: StageError,
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.stage {
            StageError::Capture(m) => write!(f, "capture: {m}"),
            StageError::Transcribe(m) => write!(f, "transcribe: {m}"),
            StageError::Format(m) => write!(f, "format: {m}"),
            StageError::Inject(m) => write!(f, "inject: {m}"),
            StageError::Persist(m) => write!(f, "persist: {m}"),
        }
    }
}

impl std::error::Error for PipelineError {}

/// Run one Dictation session end-to-end: capture → transcribe → format →
/// inject → persist → cleanup. Returns the new `transcriptions.id`.
///
/// Stage semantics:
/// - **Capture** stops the recorder, writes the WAV, and short-circuits
///   with `StageError::Capture("zero speech captured")` when the file is
///   under ~1 KiB (no recognisable audio). The WAV is removed in that case.
/// - **Transcribe** dispatches to local whisper.cpp or Groq depending on
///   `settings.transcription.engine`.
/// - **Format** runs the four format passes plus auto-add observation.
/// - **Inject** is best-effort: clipboard write failure is fatal, but a
///   keystroke failure degrades to `ClipboardOnly` and we keep going so
///   the row is still persisted. Empty `final_text` skips the actual
///   keystroke but we still persist the row — Phase 2 keeps zero-word
///   sessions visible in stats; tightening this is a Phase 4 concern.
/// - **Persist** runs `commit::commit_session` inside `spawn_blocking` so
///   the SQLite write does not stall the async runtime. Both `Db` and
///   `Settings` derive `Clone` (cheap — `Db` is `Arc<Mutex<Connection>>`).
/// - **Cleanup** removes the on-disk WAV; failure is logged but ignored.
#[tracing::instrument(skip(deps), fields(mode = ?mode))]
pub async fn run_session(
    deps: PipelineDeps<'_>,
    mode: PipelineMode,
) -> Result<i64, PipelineError> {
    if mode != PipelineMode::Dictation {
        return Err(PipelineError {
            stage: StageError::Capture("command mode is Phase 4".into()),
        });
    }

    // Session id is logged manually (not recorded into the span) so we
    // don't have to thread `Empty` field declarations through the
    // `instrument` macro. Functionally equivalent for log correlation.
    let session_id = Uuid::new_v4();
    tracing::info!(session_id = %session_id, "pipeline session start");

    // 1. Capture
    let cap = capture::stop_and_save(deps.audio)
        .map_err(|e| PipelineError { stage: StageError::Capture(e) })?;
    if cap.byte_size < 1024 {
        // ~1 KiB minimum for a non-empty 16kHz mono WAV header + a few
        // frames. Short-circuit before transcribing silence.
        let _ = std::fs::remove_file(&cap.wav_path);
        return Err(PipelineError {
            stage: StageError::Capture("zero speech captured".into()),
        });
    }
    // Log only the basename so the rotating log doesn't embed
    // %LOCALAPPDATA%\<username>\... paths. Spec §9 (telemetry) bans
    // user-identifying paths in the rotating log.
    tracing::info!(
        wav_name = %cap.wav_path.file_name().and_then(|s| s.to_str()).unwrap_or("<no-name>"),
        duration_ms = cap.duration_ms,
        bytes = cap.byte_size,
        "capture done",
    );

    // 2. Transcribe
    let tx_result = transcribe::dispatch(
        deps.app,
        deps.app_dir,
        &cap.wav_path,
        deps.settings,
        deps.groq_key,
    )
    .await
    .map_err(|e| PipelineError {
        stage: StageError::Transcribe(e.to_string()),
    })?;
    tracing::info!(
        engine = %deps.settings.transcription.engine,
        model = %tx_result.model,
        language = ?tx_result.language,
        duration_ms = tx_result.duration_ms,
        "transcribe done",
    );

    // 3. Format
    let final_text = format::run_format(&tx_result.text, deps.settings, deps.db)
        .map_err(|e| PipelineError {
            stage: StageError::Format(format!("{e:?}")),
        })?;
    tracing::info!(
        words = final_text.split_whitespace().count(),
        "format done",
    );

    // 4. Inject (best-effort; an empty final_text skips the keystroke but
    // still proceeds to persist so stats reflect the empty session).
    let inject_method = if !final_text.is_empty() {
        inject::paste(&final_text)
            .map_err(|e| PipelineError { stage: StageError::Inject(e) })?
    } else {
        inject::InjectMethod::Enigo
    };
    tracing::info!(method = ?inject_method, "inject done");

    // 5. Persist — SQLite write off the async runtime.
    // `language` is `Option<String>` upstream; `TranscriptionRecord.language`
    // is `String NOT NULL` in storage. Map missing language to empty string,
    // matching the Phase 2 contract documented in `commit.rs`.
    let record = commit::TranscriptionRecord {
        raw_text: tx_result.text.clone(),
        final_text: final_text.clone(),
        word_count: final_text.split_whitespace().count() as i64,
        duration_ms: cap.duration_ms as i64,
        language: tx_result.language.clone().unwrap_or_default(),
        engine: deps.settings.transcription.engine.clone(),
        model: tx_result.model.clone(),
        app_context: String::new(),
        mode: "dictation".into(),
        enhanced: false,
    };
    let row_id = {
        let db = deps.db.clone();
        let settings = deps.settings.clone();
        tokio::task::spawn_blocking(move || commit::commit_session(&db, record, &settings))
            .await
            .map_err(|e| PipelineError {
                stage: StageError::Persist(format!("join: {e}")),
            })?
            .map_err(|e| PipelineError {
                stage: StageError::Persist(format!("{e:?}")),
            })?
    };
    tracing::info!(row_id, "persist done");

    // 6. Cleanup — best-effort. The tmp sweep added in T15 will mop up
    // any WAV we leak on the rare failure path here.
    if let Err(e) = std::fs::remove_file(&cap.wav_path) {
        tracing::warn!(
            error = %e,
            wav_name = %cap.wav_path.file_name().and_then(|s| s.to_str()).unwrap_or("<no-name>"),
            "tmp wav cleanup failed",
        );
    }

    Ok(row_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_error_display_includes_stage_name() {
        let err = PipelineError {
            stage: StageError::Capture("zero speech".into()),
        };
        let s = format!("{err}");
        assert!(s.starts_with("capture:"), "got {s}");
        assert!(s.contains("zero speech"), "got {s}");
    }

    #[test]
    fn pipeline_error_display_covers_each_stage() {
        // Every variant must render with its tag prefix so log greps stay
        // useful. If anyone adds a new variant they have to extend the
        // match in `Display` and this test will guide them.
        let cases = [
            (
                PipelineError { stage: StageError::Transcribe("x".into()) },
                "transcribe:",
            ),
            (
                PipelineError { stage: StageError::Format("x".into()) },
                "format:",
            ),
            (
                PipelineError { stage: StageError::Inject("x".into()) },
                "inject:",
            ),
            (
                PipelineError { stage: StageError::Persist("x".into()) },
                "persist:",
            ),
        ];
        for (err, prefix) in cases {
            let s = format!("{err}");
            assert!(s.starts_with(prefix), "got {s}, expected prefix {prefix}");
        }
    }

    #[test]
    fn inject_method_clones_correctly() {
        use super::inject::InjectMethod;
        let m = InjectMethod::Enigo;
        assert_eq!(m.clone(), InjectMethod::Enigo);
    }
}
