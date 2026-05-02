//! whisper-cli `--output-file <stem>` writes the transcription sidecar at
//! `<stem>.txt` (probed 2026-04-26 against bundled `whisper-cli.exe` with
//! `--output-txt`). Task 13 of Phase 2 reads that sidecar first, falling back
//! to stdout-scrape only if the sidecar is missing.

use std::path::Path;
use std::time::{Duration, Instant};
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use tokio::time::timeout;

use crate::pipeline::transcribe::TranscriptionResult;

/// Run the bundled `whisper-cpp` sidecar against `wav_path` using the model
/// at `model_path`. Returns a typed [`TranscriptionResult`].
///
/// Primary parse path reads `<wav_stem>.txt` written by `--output-txt
/// --output-file`; if the sidecar is missing for any reason, falls back to
/// scraping stdout. Language is detected from the combined stdout+stderr
/// capture (whisper-cli prints "auto-detected language: …" to stderr when
/// `-l` is `auto`; otherwise this returns `None`).
pub async fn transcribe_local(
    app: &AppHandle,
    model_path: &Path,
    wav_path: &Path,
) -> Result<TranscriptionResult, String> {
    if !model_path.exists() {
        return Err("Whisper model not found. Please download a model first.".to_string());
    }

    let stem = wav_path
        .file_stem()
        .ok_or_else(|| "wav path has no stem".to_string())?
        .to_string_lossy()
        .into_owned();
    let parent = wav_path.parent().unwrap_or_else(|| Path::new("."));
    let stem_for_cli = parent.join(&stem); // path WITHOUT extension
    let txt_path = parent.join(format!("{stem}.txt"));

    let started = Instant::now();
    let stdout_plus_stderr = run_whisper_cli(app, model_path, wav_path, &stem_for_cli).await?;
    let elapsed_ms = started.elapsed().as_millis() as u64;

    let raw = if txt_path.exists() {
        let content =
            std::fs::read_to_string(&txt_path).map_err(|e| format!("read sidecar: {e}"))?;
        let _ = std::fs::remove_file(&txt_path);
        content
    } else {
        tracing::warn!("whisper-cli -otxt sidecar missing; falling back to stdout scrape");
        scrape_stdout(&stdout_plus_stderr)
    };

    let cleaned = raw.trim().to_string();

    Ok(TranscriptionResult {
        text: cleaned,
        language: detect_language(&stdout_plus_stderr),
        duration_ms: elapsed_ms,
        model: model_filename_to_label(model_path),
    })
}

async fn run_whisper_cli(
    app: &AppHandle,
    model_path: &Path,
    wav_path: &Path,
    output_stem: &Path,
) -> Result<String, String> {
    let output = timeout(
        Duration::from_secs(180),
        app.shell()
            .sidecar("whisper-cpp")
            .map_err(|e| format!("Failed to create sidecar command: {}", e))?
            .args([
                "-m",
                model_path.to_str().ok_or("model path not utf-8")?,
                "-f",
                wav_path.to_str().ok_or("wav path not utf-8")?,
                "--no-timestamps",
                "-l",
                "pt",
                "--prompt",
                "Transcricao em portugues europeu de Portugal.",
                "--output-txt",
                "--output-file",
                output_stem.to_str().ok_or("output stem not utf-8")?,
            ])
            .output(),
    )
    .await
    .map_err(|_| "whisper.cpp timed out after 180 seconds".to_string())?
    .map_err(|e| format!("Failed to run whisper.cpp: {}", e))?;

    if output.status.code() != Some(0) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("whisper.cpp failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(format!("{stdout}\n{stderr}"))
}

fn scrape_stdout(stdout: &str) -> String {
    let re = regex::Regex::new(r"\[\d{2}:\d{2}\.\d{3}\s*-->\s*\d{2}:\d{2}\.\d{3}\]").unwrap();
    re.replace_all(stdout, "")
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn detect_language(stderr_or_stdout: &str) -> Option<String> {
    let re = regex::Regex::new(r"auto-detected language:\s*([a-z]{2})").unwrap();
    re.captures(stderr_or_stdout)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn model_filename_to_label(model_path: &Path) -> String {
    model_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.trim_start_matches("ggml-").to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// Map a settings-level model key (e.g. "turbo") to the on-disk filename used
/// by whisper.cpp / Hugging Face. "turbo" is a UX nickname; the full
/// `ggml-large-v3-turbo.bin` artifact is no longer present on the repository's
/// `main` branch, but the quantized `q5_0` artifact is available and supported
/// by whisper.cpp.
fn model_stem(model_size: &str) -> Result<&'static str, String> {
    match model_size {
        "turbo" | "large-v3-turbo" | "ggml-large-v3-turbo" | "ggml-large-v3-turbo.bin" => {
            Ok("large-v3-turbo-q5_0")
        }
        "large-v3-turbo-q5_0" | "ggml-large-v3-turbo-q5_0" | "ggml-large-v3-turbo-q5_0.bin" => {
            Ok("large-v3-turbo-q5_0")
        }
        "base" | "ggml-base" | "ggml-base.bin" => Ok("base"),
        "large-v3" | "ggml-large-v3" | "ggml-large-v3.bin" => Ok("large-v3"),
        other => Err(format!("unsupported whisper model: {other}")),
    }
}

pub fn model_filename(model_size: &str) -> Result<String, String> {
    Ok(format!("ggml-{}.bin", model_stem(model_size)?))
}

pub fn model_download_url(model_size: &str) -> Result<String, String> {
    Ok(format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
        model_stem(model_size)?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_filename() {
        assert_eq!(model_filename("base").unwrap(), "ggml-base.bin");
        assert_eq!(model_filename("large-v3").unwrap(), "ggml-large-v3.bin");
        // Turbo is a UX label; on-disk file is the available q5_0 variant.
        assert_eq!(
            model_filename("turbo").unwrap(),
            "ggml-large-v3-turbo-q5_0.bin"
        );
        assert_eq!(
            model_filename("large-v3-turbo").unwrap(),
            "ggml-large-v3-turbo-q5_0.bin"
        );
        assert_eq!(
            model_filename("ggml-large-v3-turbo.bin").unwrap(),
            "ggml-large-v3-turbo-q5_0.bin"
        );
        assert!(model_filename("../config").is_err());
        assert!(model_filename("base/../../x").is_err());
    }

    #[test]
    fn test_model_download_url() {
        assert_eq!(
            model_download_url("base").unwrap(),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin"
        );
        assert_eq!(
            model_download_url("turbo").unwrap(),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin"
        );
        assert_eq!(
            model_download_url("large-v3-turbo").unwrap(),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin"
        );
    }

    #[test]
    fn scrape_stdout_strips_timestamps() {
        let raw = "[00:00.000 --> 00:02.500]  hello world\n[00:02.500 --> 00:04.000]  goodbye\n";
        assert_eq!(scrape_stdout(raw), "hello world goodbye");
    }

    #[test]
    fn scrape_stdout_drops_empty_lines() {
        let raw = "\n\n  hello  \n\n";
        assert_eq!(scrape_stdout(raw), "hello");
    }

    #[test]
    fn detect_language_extracts_pt() {
        let stderr = "auto-detected language: pt (probability 0.97)";
        assert_eq!(detect_language(stderr), Some("pt".to_string()));
    }

    #[test]
    fn detect_language_returns_none_when_absent() {
        assert_eq!(detect_language("nothing here"), None);
    }

    #[test]
    fn model_filename_to_label_strips_prefix() {
        let p = Path::new("C:\\foo\\ggml-turbo.bin");
        assert_eq!(model_filename_to_label(p), "turbo");
    }

    #[test]
    fn model_filename_to_label_handles_bare_name() {
        let p = Path::new("C:\\foo\\custom.bin");
        assert_eq!(model_filename_to_label(p), "custom");
    }
}
