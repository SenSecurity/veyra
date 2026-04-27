//! Audio capture surface used by `pipeline::capture`.
//!
//! Phase 2 splits the previous monolithic `audio.rs` into:
//! - `recorder` — cpal stream + ring buffer + `save_wav`.
//! - `vad`      — energy-based push-to-talk auto-stop (added in Task 3).

pub mod recorder;

pub use recorder::{list_microphones, AudioRecorder, MicDevice};
