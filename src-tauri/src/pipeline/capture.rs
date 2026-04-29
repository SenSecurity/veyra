//! Capture stage — stop the live cpal recorder, flush the WAV to disk, and
//! report the duration + byte size that downstream stages need.
//!
//! The capture stage is intentionally thin: the live `AudioRecorder` already
//! owns the ring buffer and `stop_and_save` routine. This module only adds
//! the per-session WAV path allocation and packages the result for the
//! orchestrator's logging + short-circuit checks.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::audio::AudioRecorder;
use crate::pipeline::tmp::session_wav_path;

/// Result of one successful capture: the WAV path on disk, the wall-clock
/// duration that was buffered, and the size of the resulting file.
///
/// `byte_size` is captured here (not in the orchestrator) because the
/// caller short-circuits when it is below the "real audio" threshold,
/// and we want all filesystem stat calls to live in this module.
#[derive(Debug)]
pub struct CaptureOutput {
    pub wav_path: PathBuf,
    pub duration_ms: u64,
    pub byte_size: u64,
}

/// Stop the active recording, write its samples to a fresh WAV in the
/// per-session tmp dir, and return a [`CaptureOutput`].
///
/// The `audio` mutex is locked for the duration of `stop_and_save`. We
/// drop the guard explicitly before the metadata stat so subsequent
/// pipeline stages can re-acquire the recorder if they need to (Phase 4
/// command mode does back-to-back captures).
pub fn stop_and_save(audio: &Mutex<AudioRecorder>) -> Result<CaptureOutput, String> {
    let wav_path = session_wav_path().ok_or_else(|| "tmp dir not available".to_string())?;

    let mut rec = audio
        .lock()
        .map_err(|e| format!("audio lock poisoned: {e}"))?;
    let duration_ms = rec.current_duration_ms();
    rec.stop_and_save(&wav_path)
        .map_err(|e| format!("stop_and_save: {e}"))?;
    drop(rec);

    let byte_size = std::fs::metadata(&wav_path).map(|m| m.len()).unwrap_or(0);
    Ok(CaptureOutput {
        wav_path,
        duration_ms,
        byte_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_output_carries_expected_fields() {
        // Smoke check on the data carrier — full capture round-trips need a
        // live cpal stream, which is platform-specific. The orchestrator's
        // integration test (Phase 5) covers the wired path.
        let cap = CaptureOutput {
            wav_path: PathBuf::from("Z:/tmp/x.wav"),
            duration_ms: 1234,
            byte_size: 4096,
        };
        assert_eq!(cap.duration_ms, 1234);
        assert_eq!(cap.byte_size, 4096);
        assert_eq!(cap.wav_path.extension().and_then(|s| s.to_str()), Some("wav"));
    }
}
