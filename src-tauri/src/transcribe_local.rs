//! whisper-cli `--output-file <stem>` writes the transcription sidecar at
//! `<stem>.txt` (probed 2026-04-26 against bundled `whisper-cli.exe` with
//! `--output-txt`). Task 13 of Phase 2 relies on this exact filename.

use std::path::PathBuf;
use std::time::Instant;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

use crate::pipeline::transcribe::TranscriptionResult;

/// Run the bundled `whisper-cpp` sidecar against `audio_path` using the model
/// at `model_path`. Returns a typed [`TranscriptionResult`] with `language`
/// `None` (current `-otxt`-only flow doesn't expose a language tag) and
/// `duration_ms` set to the wall-clock cost of the shell-out.
///
/// `model` is derived from `model_path.file_stem()` with the `ggml-` prefix
/// stripped (e.g. `ggml-turbo.bin` → `"turbo"`).
pub async fn transcribe_local(
    app: &AppHandle,
    model_path: &PathBuf,
    audio_path: &PathBuf,
) -> Result<TranscriptionResult, String> {
    if !model_path.exists() {
        return Err("Whisper model not found. Please download a model first.".to_string());
    }

    let model_name = model_path
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.strip_prefix("ggml-").unwrap_or(s).to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("[Typr] Running whisper.cpp sidecar with model {:?}", model_path);

    let started = Instant::now();

    let output = app
        .shell()
        .sidecar("whisper-cpp")
        .map_err(|e| format!("Failed to create sidecar command: {}", e))?
        .args([
            "-m",
            model_path.to_str().unwrap(),
            "-f",
            audio_path.to_str().unwrap(),
            "--no-timestamps",
            "-l",
            "pt",
            "--prompt",
            "Transcricao em portugues europeu de Portugal.",
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run whisper.cpp: {}", e))?;

    if output.status.code() != Some(0) {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("whisper.cpp failed: {}", stderr));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let duration_ms = started.elapsed().as_millis() as u64;
    println!("[Typr] Whisper output: {}", text);

    Ok(TranscriptionResult {
        text,
        language: None,
        duration_ms,
        model: model_name,
    })
}

pub fn model_filename(model_size: &str) -> String {
    format!("ggml-{}.bin", model_size)
}

pub fn model_download_url(model_size: &str) -> String {
    format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{}.bin",
        model_size
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_filename() {
        assert_eq!(model_filename("small"), "ggml-small.bin");
        assert_eq!(model_filename("medium"), "ggml-medium.bin");
    }

    #[test]
    fn test_model_download_url() {
        assert_eq!(
            model_download_url("small"),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
        );
    }
}
