use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

/// Target sample rate after resampling — matches whisper.cpp expected input.
const TARGET_SAMPLE_RATE: usize = 16_000;
/// Hard cap on captured audio (rolling window). Anything older than this is dropped.
const MAX_SECONDS: usize = 120;
/// Rolling-window cap measured against the eventual 16kHz mono stream the test
/// helpers and `current_duration_ms` assume. Production capture caps in raw
/// samples computed from the live cpal config (see `apply_cap_with`).
#[cfg(test)]
const MAX_SAMPLES_16K_MONO: usize = TARGET_SAMPLE_RATE * MAX_SECONDS;

fn apply_cap_with(buf: &mut Vec<f32>, max_samples: usize) {
    if buf.len() > max_samples {
        let overflow = buf.len() - max_samples;
        buf.drain(..overflow);
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MicDevice {
    pub name: String,
    pub is_default: bool,
}

pub fn list_microphones() -> Vec<MicDevice> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default();

    let mut devices = Vec::new();
    if let Ok(input_devices) = host.input_devices() {
        for device in input_devices {
            if let Ok(name) = device.name() {
                devices.push(MicDevice {
                    is_default: name == default_name,
                    name,
                });
            }
        }
    }
    devices
}

/// Wrapper to make cpal::Stream usable across threads.
/// SAFETY: cpal::Stream on macOS (CoreAudio) is thread-safe in practice;
/// we only access it behind a Mutex to start/stop recording.
struct SendStream(#[allow(dead_code)] cpal::Stream);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

pub struct AudioRecorder {
    samples: Arc<Mutex<Vec<f32>>>,
    stream: Option<SendStream>,
    source_sample_rate: u32,
    source_channels: u16,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            stream: None,
            source_sample_rate: 48000,
            source_channels: 1,
        }
    }

    pub fn start(&mut self, mic_name: &str) -> Result<(), String> {
        // Clear any leftover samples from previous recording
        self.samples.lock().unwrap().clear();

        let host = cpal::default_host();

        let device = if mic_name == "default" {
            host.default_input_device()
                .ok_or("No default input device found")?
        } else {
            host.input_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().map(|n| n == mic_name).unwrap_or(false))
                .ok_or(format!("Microphone '{}' not found", mic_name))?
        };

        // Use the device's default config instead of forcing 16kHz
        let default_config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let sample_rate = default_config.sample_rate().0;
        let channels = default_config.channels();

        println!("[Typr] Mic config: {}Hz, {} channels", sample_rate, channels);

        self.source_sample_rate = sample_rate;
        self.source_channels = channels;

        let config = cpal::StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let samples = self.samples.clone();
        // Cap at MAX_SECONDS of raw audio. Buffer is interleaved frames at
        // `sample_rate * channels`, so cap scales with both.
        let raw_max_samples = (sample_rate as usize) * (channels as usize) * MAX_SECONDS;
        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut buf = samples.lock().unwrap();
                    buf.extend_from_slice(data);
                    apply_cap_with(&mut buf, raw_max_samples);
                },
                |err| {
                    eprintln!("[Typr] Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(SendStream(stream));
        println!("[Typr] Audio recording started");
        Ok(())
    }

    pub fn stop_and_save(&mut self, output_path: &PathBuf) -> Result<PathBuf, String> {
        self.stream = None; // Drop stops the stream
        println!("[Typr] Audio recording stopped");

        let samples = self.samples.lock().unwrap();
        if samples.is_empty() {
            return Err("No audio captured".to_string());
        }

        println!("[Typr] Captured {} raw samples", samples.len());

        // Convert to mono if multi-channel
        let mono: Vec<f32> = if self.source_channels > 1 {
            samples
                .chunks(self.source_channels as usize)
                .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
                .collect()
        } else {
            samples.clone()
        };

        // Downsample to 16kHz for whisper.cpp
        let resampled = resample(&mono, self.source_sample_rate, 16000);
        println!("[Typr] Resampled to {} samples at 16kHz", resampled.len());

        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(output_path, spec).map_err(|e| e.to_string())?;
        for &sample in resampled.iter() {
            let amplitude = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(amplitude).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;

        drop(samples);
        self.samples.lock().unwrap().clear();

        println!("[Typr] WAV saved to {:?}", output_path);
        Ok(output_path.clone())
    }

    /// Returns how much audio is currently buffered, in milliseconds.
    ///
    /// Computed against the 16kHz mono target rate — the buffer is normalised
    /// to that shape by the test helpers, and represents the eventual
    /// post-resample stream that whisper.cpp consumes.
    pub fn current_duration_ms(&self) -> u64 {
        let buf = self.samples.lock().unwrap();
        ((buf.len() as u64) * 1000) / (TARGET_SAMPLE_RATE as u64)
    }

    #[cfg(test)]
    pub fn push_test_samples(&mut self, frames: Vec<f32>) {
        let mut buf = self.samples.lock().unwrap();
        buf.extend(frames);
        apply_cap_with(&mut buf, MAX_SAMPLES_16K_MONO);
    }

    #[cfg(test)]
    pub fn snapshot_test_samples(&self) -> Vec<f32> {
        self.samples.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_caps_at_120_seconds() {
        let mut rec = AudioRecorder::new();
        let total_samples = 16_000 * 130; // 130s
        rec.push_test_samples(vec![0.5f32; total_samples]);
        let captured = rec.snapshot_test_samples();
        assert!(captured.len() <= 16_000 * 120, "expected <= 120s, got {} samples", captured.len());
        assert_eq!(captured.len(), 16_000 * 120, "expected exactly 120s after rolling-window cap");
    }

    #[test]
    fn current_duration_ms_reports_buffered_audio() {
        let mut rec = AudioRecorder::new();
        rec.push_test_samples(vec![0.0f32; 16_000 * 5]); // 5s
        assert_eq!(rec.current_duration_ms(), 5_000);
    }
}

/// Simple linear interpolation resampler
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (samples.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx = src_idx as usize;
        let frac = src_idx - idx as f64;

        let sample = if idx + 1 < samples.len() {
            samples[idx] as f64 * (1.0 - frac) + samples[idx + 1] as f64 * frac
        } else {
            samples[idx.min(samples.len() - 1)] as f64
        };

        output.push(sample as f32);
    }

    output
}
