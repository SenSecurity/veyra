use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub const DEFAULT_GROQ_DRAFT_MODEL: &str = "llama-3.3-70b-versatile";
pub const DEFAULT_OLLAMA_DRAFT_MODEL: &str = "llama3.2";
pub const ALLOWED_GROQ_DRAFT_MODELS: &[&str] = &[
    "llama-3.3-70b-versatile",
    "llama-3.1-8b-instant",
    "openai/gpt-oss-120b",
    "openai/gpt-oss-20b",
];
pub const ALLOWED_OLLAMA_DRAFT_MODELS: &[&str] =
    &["llama3.2", "llama3.2:1b", "qwen3:1.7b", "qwen3:4b"];

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    id: String,
}

#[derive(Debug, Serialize)]
struct OllamaGenerateRequest {
    model: String,
    system: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaPullRequest {
    model: String,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Debug, Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelInfo>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelInfo {
    name: Option<String>,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaPullChunk {
    status: Option<String>,
    completed: Option<u64>,
    total: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmailModelDownloadProgress {
    model: String,
    downloaded: u64,
    total: u64,
    percent: f64,
    status: String,
}

pub async fn generate_email_draft(
    api_key: &str,
    engine: &str,
    model: &str,
    instruction: &str,
) -> Result<String, String> {
    let instruction = instruction.trim();
    if instruction.is_empty() {
        return Err("No command instruction was transcribed.".to_string());
    }
    match engine {
        "groq" => generate_groq_email_draft(api_key, model, instruction).await,
        "ollama" => generate_ollama_email_draft(model, instruction).await,
        other => Err(format!("Unsupported email draft engine `{other}`.")),
    }
}

async fn generate_groq_email_draft(
    api_key: &str,
    model: &str,
    instruction: &str,
) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("Groq API key not set. Add it in Settings > Transcription.".to_string());
    }
    let model = normalize_groq_draft_model(model)?;

    let request = ChatRequest {
        model,
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: instruction.to_string(),
            },
        ],
        temperature: 0.35,
        max_tokens: 700,
    };

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Groq draft client failed: {e}"))?;

    let response = client
        .post("https://api.groq.com/openai/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Groq draft request failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Groq draft error ({status}): {body}"));
    }

    let parsed: ChatResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Groq draft response: {e}"))?;

    extract_draft(parsed)
}

async fn generate_ollama_email_draft(model: &str, instruction: &str) -> Result<String, String> {
    let model = normalize_ollama_draft_model(model)?;
    ensure_ollama_running().await?;
    let request = OllamaGenerateRequest {
        model,
        system: system_prompt(),
        prompt: instruction.to_string(),
        stream: false,
        options: OllamaOptions {
            temperature: 0.35,
            num_predict: 700,
        },
    };

    let client = ollama_client(Duration::from_secs(180))?;
    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Ollama request failed. Is Ollama running? {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Ollama draft error ({status}): {body}"));
    }

    let parsed: OllamaGenerateResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {e}"))?;
    let draft = parsed.response.trim().to_string();
    if draft.is_empty() {
        Err("Ollama response did not include text.".to_string())
    } else {
        Ok(draft)
    }
}

pub async fn check_email_draft_model(
    api_key: &str,
    engine: &str,
    model: &str,
) -> Result<(), String> {
    match engine {
        "groq" => check_groq_email_draft_model(api_key, model).await,
        "ollama" => check_ollama_email_draft_model(model).await,
        other => Err(format!("Unsupported email draft engine `{other}`.")),
    }
}

async fn check_groq_email_draft_model(api_key: &str, model: &str) -> Result<(), String> {
    if api_key.trim().is_empty() {
        return Err("Groq API key not set. Add it in Settings > Transcription.".to_string());
    }
    let model = normalize_groq_draft_model(model)?;

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Groq model check client failed: {e}"))?;

    let response = client
        .get("https://api.groq.com/openai/v1/models")
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| format!("Groq model check failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Groq model check error ({status}): {body}"));
    }

    let parsed: ModelsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Groq models response: {e}"))?;

    if parsed.data.iter().any(|candidate| candidate.id == model) {
        Ok(())
    } else {
        Err(format!(
            "Groq model `{model}` is not available for this API key."
        ))
    }
}

async fn check_ollama_email_draft_model(model: &str) -> Result<(), String> {
    let model = normalize_ollama_draft_model(model)?;
    ensure_ollama_running().await?;
    let client = ollama_client(Duration::from_secs(15))?;
    let response = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map_err(|e| {
            format!("Ollama is not reachable at localhost:11434. Is it installed and running? {e}")
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Ollama model check error ({status}): {body}"));
    }

    let parsed: OllamaTagsResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama model list: {e}"))?;

    if parsed.models.iter().any(|candidate| {
        candidate
            .name
            .as_deref()
            .is_some_and(|name| ollama_model_matches(name, &model))
            || candidate
                .model
                .as_deref()
                .is_some_and(|name| ollama_model_matches(name, &model))
    }) {
        Ok(())
    } else {
        Err(format!("Ollama model `{model}` is not downloaded."))
    }
}

pub async fn download_email_draft_model(
    app: Option<AppHandle>,
    engine: &str,
    model: &str,
) -> Result<(), String> {
    match engine {
        "groq" => Ok(()),
        "ollama" => download_ollama_email_draft_model(app, model).await,
        other => Err(format!("Unsupported email draft engine `{other}`.")),
    }
}

async fn download_ollama_email_draft_model(
    app: Option<AppHandle>,
    model: &str,
) -> Result<(), String> {
    let model = normalize_ollama_draft_model(model)?;
    ensure_ollama_running().await?;
    let client = ollama_client(Duration::from_secs(900))?;
    let response = client
        .post("http://localhost:11434/api/pull")
        .json(&OllamaPullRequest {
            model: model.clone(),
            stream: true,
        })
        .send()
        .await
        .map_err(|e| format!("Ollama pull failed. Is Ollama running? {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Ollama pull error ({status}): {body}"));
    }

    emit_email_download_progress(&app, &model, 0, 0, "Starting download");
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut last_status = String::new();
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Ollama pull stream failed: {e}"))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(newline) = buffer.find('\n') {
            let line = buffer[..newline].trim().to_string();
            buffer = buffer[newline + 1..].to_string();
            if line.is_empty() {
                continue;
            }
            let parsed: OllamaPullChunk = serde_json::from_str(&line)
                .map_err(|e| format!("Failed to parse Ollama pull progress: {e}"))?;
            let status = parsed.status.unwrap_or_else(|| last_status.clone());
            if !status.is_empty() {
                last_status = status.clone();
            }
            emit_email_download_progress(
                &app,
                &model,
                parsed.completed.unwrap_or(0),
                parsed.total.unwrap_or(0),
                if status.is_empty() {
                    "Downloading"
                } else {
                    &status
                },
            );
        }
    }

    let rest = buffer.trim();
    if !rest.is_empty() {
        let parsed: OllamaPullChunk = serde_json::from_str(rest)
            .map_err(|e| format!("Failed to parse Ollama pull progress: {e}"))?;
        let status = parsed.status.unwrap_or_else(|| "success".to_string());
        emit_email_download_progress(
            &app,
            &model,
            parsed.completed.unwrap_or(0),
            parsed.total.unwrap_or(0),
            &status,
        );
    }

    Ok(())
}

pub fn normalize_groq_draft_model(model: &str) -> Result<String, String> {
    let trimmed = model.trim();
    if ALLOWED_GROQ_DRAFT_MODELS.contains(&trimmed) {
        Ok(trimmed.to_string())
    } else {
        Err(format!(
            "Unsupported Groq email draft model `{trimmed}`. Pick one from Settings > Transcription."
        ))
    }
}

pub fn normalize_ollama_draft_model(model: &str) -> Result<String, String> {
    let trimmed = model.trim();
    if ALLOWED_OLLAMA_DRAFT_MODELS.contains(&trimmed) {
        Ok(trimmed.to_string())
    } else {
        Err(format!(
            "Unsupported Ollama email draft model `{trimmed}`. Pick one from Settings > Transcription."
        ))
    }
}

fn ollama_client(timeout: Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(timeout)
        .build()
        .map_err(|e| format!("Ollama client failed: {e}"))
}

fn emit_email_download_progress(
    app: &Option<AppHandle>,
    model: &str,
    downloaded: u64,
    total: u64,
    status: &str,
) {
    let percent = if total > 0 {
        (downloaded as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    let payload = EmailModelDownloadProgress {
        model: model.to_string(),
        downloaded,
        total,
        percent,
        status: status.to_string(),
    };
    if let Some(app) = app {
        let _ = app.emit("email-model:download:progress", payload);
    }
}

async fn ensure_ollama_running() -> Result<(), String> {
    if ollama_is_running().await {
        return Ok(());
    }

    let mut last_error = String::new();
    let mut spawned = false;
    for candidate in ollama_executable_candidates() {
        if candidate.is_absolute() && !candidate.exists() {
            continue;
        }
        match Command::new(&candidate)
            .arg("serve")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(_) => {
                spawned = true;
                break;
            }
            Err(error) => {
                last_error = format!("{error}");
            }
        }
    }

    for _ in 0..20 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        if ollama_is_running().await {
            return Ok(());
        }
    }

    if spawned {
        Err(
            "Ollama was found, but it did not start listening on localhost:11434. Open Ollama manually and retry."
                .to_string(),
        )
    } else {
        let detail = if last_error.is_empty() {
            String::new()
        } else {
            format!(" Last error: {last_error}")
        };
        Err(format!(
            "Ollama is required for local email drafts but is not installed or not on PATH. Install it from https://ollama.com/download, open it once, then retry.{detail}"
        ))
    }
}

async fn ollama_is_running() -> bool {
    let Ok(client) = ollama_client(Duration::from_secs(2)) else {
        return false;
    };
    client
        .get("http://localhost:11434/api/version")
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

fn ollama_executable_candidates() -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from("ollama")];
    if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(local_appdata).join("Programs\\Ollama\\ollama.exe"));
    }
    if let Ok(program_files) = std::env::var("ProgramFiles") {
        candidates.push(PathBuf::from(program_files).join("Ollama\\ollama.exe"));
    }
    candidates
}

fn ollama_model_matches(installed: &str, selected: &str) -> bool {
    installed == selected
        || (selected.find(':').is_none() && installed == format!("{selected}:latest"))
}

fn system_prompt() -> String {
    [
        "You write polished drafts for direct insertion into the user's active text field.",
        "The user speaks an instruction, often in Portuguese, for an email or message reply.",
        "Return only the finished draft text. No explanations, no markdown fences, no subject line unless explicitly requested.",
        "Respect the language requested by the user. If no language is requested, use the language that best fits the instruction.",
        "Keep the tone professional, warm, concise, and natural. Do not invent facts not present in the instruction.",
    ]
    .join(" ")
}

fn extract_draft(response: ChatResponse) -> Result<String, String> {
    let draft = response
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content.trim().to_string())
        .filter(|text| !text.is_empty())
        .ok_or_else(|| "Groq draft response did not include text.".to_string())?;

    Ok(draft)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_empty_api_key() {
        let result =
            generate_email_draft("", "groq", DEFAULT_GROQ_DRAFT_MODEL, "reply in English").await;
        assert!(result.unwrap_err().contains("API key"));
    }

    #[tokio::test]
    async fn rejects_empty_instruction() {
        let result =
            generate_email_draft("gsk_test", "groq", DEFAULT_GROQ_DRAFT_MODEL, "   ").await;
        assert!(result.unwrap_err().contains("No command instruction"));
    }

    #[test]
    fn accepts_only_known_groq_draft_models() {
        assert!(normalize_groq_draft_model("llama-3.3-70b-versatile").is_ok());
        assert!(normalize_groq_draft_model("../not-a-model").is_err());
    }

    #[test]
    fn accepts_only_known_ollama_draft_models() {
        assert!(normalize_ollama_draft_model("llama3.2").is_ok());
        assert!(normalize_ollama_draft_model("llama3.2:1b").is_ok());
        assert!(normalize_ollama_draft_model("../not-a-model").is_err());
    }

    #[test]
    fn ollama_latest_tag_matches_short_model_name() {
        assert!(ollama_model_matches("llama3.2:latest", "llama3.2"));
        assert!(ollama_model_matches("llama3.2:1b", "llama3.2:1b"));
        assert!(!ollama_model_matches("llama3.2:1b", "llama3.2"));
    }

    #[test]
    fn ollama_candidates_include_path_name() {
        let candidates = ollama_executable_candidates();
        assert_eq!(candidates.first().unwrap(), &PathBuf::from("ollama"));
    }

    #[test]
    fn extracts_first_choice_text() {
        let response = ChatResponse {
            choices: vec![ChatChoice {
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: "  Hello there.  ".to_string(),
                },
            }],
        };

        assert_eq!(extract_draft(response).unwrap(), "Hello there.");
    }
}
