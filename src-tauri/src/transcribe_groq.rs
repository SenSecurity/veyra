use reqwest::multipart;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::pipeline::transcribe::TranscriptionResult;

/// Cloud transcription via Groq's whisper-large-v3-turbo endpoint.
///
/// Phase 2 keeps `response_format=json`, so the response only carries `text`
/// and we can't recover language/duration from upstream. Switching to
/// `verbose_json` is a Phase 3+ enhancement; for now `language` is `None`,
/// `model` hard-codes the literal we send in the multipart form, and
/// `duration_ms` is the wall-clock cost of the whole HTTP roundtrip.
pub async fn transcribe_groq(
    api_key: &str,
    audio_path: &PathBuf,
) -> Result<TranscriptionResult, String> {
    if api_key.is_empty() {
        return Err("Groq API key not set. Please enter your API key in settings.".to_string());
    }

    let started = Instant::now();

    let audio_bytes =
        std::fs::read(audio_path).map_err(|e| format!("Failed to read audio file: {}", e))?;

    let file_part = multipart::Part::bytes(audio_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| e.to_string())?;

    let form = multipart::Form::new()
        .text("model", "whisper-large-v3-turbo")
        .text("language", "en")
        .text("response_format", "json")
        .part("file", file_part);

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Groq API client failed: {}", e))?;
    let response = client
        .post("https://api.groq.com/openai/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Groq API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Groq API error ({}): {}", status, body));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Groq response: {}", e))?;

    let text = json["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No 'text' field in Groq response".to_string())?;

    let duration_ms = started.elapsed().as_millis() as u64;

    Ok(TranscriptionResult {
        text,
        language: None,
        duration_ms,
        model: "groq:whisper-large-v3-turbo".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_api_key() {
        let path = PathBuf::from("/tmp/test.wav");
        let result = transcribe_groq("", &path).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("API key not set"));
    }
}
