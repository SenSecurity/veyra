//! Energy-based Voice Activity Detection used by push-to-talk auto-stop.

pub const SILENCE_RMS_THRESHOLD: f32 = 0.01;
pub const SILENCE_WINDOW_MS: u32 = 800;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VadDecision {
    Speech,
    Silence,
    AutoStop,
}

pub struct EnergyVad {
    threshold: f32,
    silence_window_ms: u32,
    accumulated_silence_ms: u32,
}

impl EnergyVad {
    pub fn new() -> Self {
        Self::with_threshold(SILENCE_RMS_THRESHOLD, SILENCE_WINDOW_MS)
    }

    pub fn with_threshold(threshold: f32, silence_window_ms: u32) -> Self {
        Self { threshold, silence_window_ms, accumulated_silence_ms: 0 }
    }

    pub fn tick(&mut self, samples: &[f32], sample_rate: u32) -> VadDecision {
        if samples.is_empty() {
            return VadDecision::Silence;
        }
        let sum_sq: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        let rms = (sum_sq / samples.len() as f64).sqrt() as f32;
        let elapsed_ms = (samples.len() as u64 * 1000 / sample_rate as u64) as u32;
        if rms >= self.threshold {
            self.accumulated_silence_ms = 0;
            VadDecision::Speech
        } else {
            self.accumulated_silence_ms = self.accumulated_silence_ms.saturating_add(elapsed_ms);
            if self.accumulated_silence_ms >= self.silence_window_ms {
                VadDecision::AutoStop
            } else {
                VadDecision::Silence
            }
        }
    }

    pub fn reset(&mut self) {
        self.accumulated_silence_ms = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(amp: f32, n: usize) -> Vec<f32> {
        (0..n).map(|_| amp).collect()
    }

    #[test]
    fn pure_silence_accumulates_then_auto_stops() {
        let mut vad = EnergyVad::new();
        // 16 kHz: 200ms = 3200 samples per tick.
        for _ in 0..3 { // 600ms total
            assert_eq!(vad.tick(&rms(0.0, 3200), 16_000), VadDecision::Silence);
        }
        // 4th tick brings accumulated >= 800ms → AutoStop.
        assert_eq!(vad.tick(&rms(0.0, 3200), 16_000), VadDecision::AutoStop);
    }

    #[test]
    fn loud_burst_returns_speech_and_resets_accumulator() {
        let mut vad = EnergyVad::new();
        vad.tick(&rms(0.0, 3200), 16_000); // 200ms silence
        let decision = vad.tick(&rms(0.5, 3200), 16_000);
        assert_eq!(decision, VadDecision::Speech);
        // After speech, silence accumulator must be 0 again — verify by
        // running 3x silence (600ms) and expecting Silence (not AutoStop).
        for _ in 0..3 {
            assert_eq!(vad.tick(&rms(0.0, 3200), 16_000), VadDecision::Silence);
        }
    }

    #[test]
    fn reset_clears_accumulator() {
        let mut vad = EnergyVad::new();
        for _ in 0..3 { vad.tick(&rms(0.0, 3200), 16_000); }
        vad.reset();
        assert_eq!(vad.tick(&rms(0.0, 3200), 16_000), VadDecision::Silence);
    }

    #[test]
    fn threshold_just_below_speech_counts_as_silence() {
        let mut vad = EnergyVad::with_threshold(0.05, 800);
        // RMS of constant 0.04 = 0.04 < 0.05.
        assert_eq!(vad.tick(&rms(0.04, 3200), 16_000), VadDecision::Silence);
    }
}
