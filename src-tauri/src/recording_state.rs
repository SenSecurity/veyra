//! Lightweight enum tracking the desktop UI's recording lifecycle.
//!
//! Lifted out of the deleted `recorder.rs` so the Tauri command layer in
//! `main.rs` can drive UI state directly while the heavy lifting lives in
//! `pipeline::run_session`.

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum RecordingState {
    Ready,
    Recording,
    Transcribing,
}
