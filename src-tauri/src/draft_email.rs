use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time::timeout;

pub const DEFAULT_GROQ_DRAFT_MODEL: &str = "llama-3.3-70b-versatile";
pub const DEFAULT_OLLAMA_DRAFT_MODEL: &str = "llama3.2:1b";
const OLLAMA_DRAFT_TIMEOUT: Duration = Duration::from_secs(15);
const BONSAI_DRAFT_TIMEOUT: Duration = Duration::from_secs(90);
const BONSAI_MODEL_DIR: &str = "email-draft-models";
pub const ALLOWED_GROQ_DRAFT_MODELS: &[&str] = &[
    "llama-3.3-70b-versatile",
    "llama-3.1-8b-instant",
    "openai/gpt-oss-120b",
    "openai/gpt-oss-20b",
];
pub const ALLOWED_OLLAMA_DRAFT_MODELS: &[&str] = &[
    "llama3.2",
    "llama3.2:1b",
    "qwen3:1.7b",
    "qwen3:4b",
    "veyra-bonsai-1.7b",
];

#[derive(Clone, Copy)]
struct BonsaiModel {
    selected_id: &'static str,
    ollama_name: &'static str,
    repo: &'static str,
    file: &'static str,
    min_bytes: u64,
}

const BONSAI_MODELS: &[BonsaiModel] = &[
    BonsaiModel {
        selected_id: "veyra-bonsai-1.7b",
        ollama_name: "veyra-bonsai:f16-1.7b",
        repo: "prism-ml/Ternary-Bonsai-1.7B-gguf",
        file: "Ternary-Bonsai-1.7B-F16.gguf",
        min_bytes: 3_400_000_000,
    },
    BonsaiModel {
        selected_id: "veyra-bonsai-4b",
        ollama_name: "veyra-bonsai:4b",
        repo: "prism-ml/Ternary-Bonsai-4B-gguf",
        file: "Ternary-Bonsai-4B-Q2_0.gguf",
        min_bytes: 1_000_000_000,
    },
    BonsaiModel {
        selected_id: "veyra-bonsai-8b",
        ollama_name: "veyra-bonsai:8b",
        repo: "prism-ml/Ternary-Bonsai-8B-gguf",
        file: "Ternary-Bonsai-8B-Q2_0.gguf",
        min_bytes: 1_900_000_000,
    },
];

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
    repeat_penalty: f32,
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
    let result = match engine {
        "groq" => generate_groq_email_draft(api_key, model, instruction).await,
        "ollama" => generate_ollama_email_draft(model, instruction).await,
        other => Err(format!("Unsupported email draft engine `{other}`.")),
    };

    match result {
        Ok(draft) => Ok(draft),
        Err(error) if error.starts_with("Unsupported") => Err(error),
        Err(error) => {
            tracing::warn!(
                engine,
                model,
                error = %error,
                "email draft model failed; using local fallback"
            );
            Ok(local_email_fallback(instruction))
        }
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
    let selected_model = normalize_ollama_draft_model(model)?;
    let is_bonsai = bonsai_model(&selected_model).is_some();
    let model = ollama_runtime_model(&selected_model)?;
    ensure_ollama_running().await?;
    let timeout_duration = if is_bonsai {
        BONSAI_DRAFT_TIMEOUT
    } else {
        OLLAMA_DRAFT_TIMEOUT
    };
    let request = OllamaGenerateRequest {
        model,
        system: system_prompt(),
        prompt: ollama_prompt(instruction, is_bonsai),
        stream: false,
        options: OllamaOptions {
            temperature: if is_bonsai { 0.0 } else { 0.35 },
            num_predict: if is_bonsai { 180 } else { 320 },
            repeat_penalty: if is_bonsai { 1.35 } else { 1.1 },
        },
    };

    let client = ollama_client(timeout_duration)?;
    let response = timeout(
        timeout_duration,
        client
            .post("http://localhost:11434/api/generate")
            .json(&request)
            .send(),
    )
    .await
    .map_err(|_| {
        format!(
            "Ollama draft timed out after {} seconds.",
            timeout_duration.as_secs()
        )
    })?
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
    } else if is_bonsai && should_use_local_fallback(&draft) {
        tracing::warn!(
            selected_model,
            "email draft model returned low-quality text; using local fallback"
        );
        Ok(local_email_fallback(instruction))
    } else {
        Ok(clean_model_draft(&draft))
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
    let bonsai = bonsai_model(model);
    let model = ollama_runtime_model(model)?;
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

    let installed = parsed.models.iter().any(|candidate| {
        candidate
            .name
            .as_deref()
            .is_some_and(|name| ollama_model_matches(name, &model))
            || candidate
                .model
                .as_deref()
                .is_some_and(|name| ollama_model_matches(name, &model))
    });

    if !installed {
        return Err(format!("Ollama model `{model}` is not downloaded."));
    }

    smoke_test_ollama_model(&model, bonsai.is_some()).await.map_err(|e| {
        if let Some(bonsai) = bonsai {
            format!(
                "Bonsai model `{}` is installed but cannot run. Restart Ollama/Veyra and retry; Bonsai needs Ollama's new runner: {e}",
                bonsai.ollama_name
            )
        } else {
            format!("Ollama model `{model}` is installed but did not pass a generation test: {e}")
        }
    })?;

    Ok(())
}

pub async fn download_email_draft_model(
    app: Option<AppHandle>,
    app_dir: Option<&Path>,
    engine: &str,
    model: &str,
) -> Result<(), String> {
    match engine {
        "groq" => Ok(()),
        "ollama" => download_ollama_email_draft_model(app, app_dir, model).await,
        other => Err(format!("Unsupported email draft engine `{other}`.")),
    }
}

async fn download_ollama_email_draft_model(
    app: Option<AppHandle>,
    app_dir: Option<&Path>,
    model: &str,
) -> Result<(), String> {
    let model = normalize_ollama_draft_model(model)?;
    if let Some(bonsai) = bonsai_model(&model) {
        return download_bonsai_email_draft_model(app, app_dir, bonsai).await;
    }
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

async fn download_bonsai_email_draft_model(
    app: Option<AppHandle>,
    app_dir: Option<&Path>,
    bonsai: BonsaiModel,
) -> Result<(), String> {
    ensure_ollama_running().await?;
    let app_dir = app_dir.ok_or_else(|| {
        "App data directory is unavailable, so Bonsai cannot be stored locally.".to_string()
    })?;
    let model_dir = app_dir.join(BONSAI_MODEL_DIR);
    std::fs::create_dir_all(&model_dir)
        .map_err(|e| format!("Failed to create Bonsai model directory: {e}"))?;

    let gguf_path = model_dir.join(bonsai.file);
    if !valid_bonsai_gguf(&gguf_path, bonsai) {
        let _ = std::fs::remove_file(&gguf_path);
        download_bonsai_gguf(&app, bonsai, &gguf_path).await?;
    } else {
        emit_email_download_progress(&app, bonsai.selected_id, 1, 1, "GGUF already downloaded");
    }

    let modelfile_path = model_dir.join(format!("{}.Modelfile", bonsai.selected_id));
    write_bonsai_modelfile(&modelfile_path, &gguf_path)?;
    create_ollama_model(bonsai, &modelfile_path)?;
    smoke_test_ollama_model(bonsai.ollama_name, true).await.map_err(|e| {
        format!(
            "Bonsai downloaded and registered as `{}`, but Ollama cannot load it: {e}. Restart Ollama/Veyra and retry; this Bonsai path uses the F16 GGUF and Ollama's new runner.",
            bonsai.ollama_name
        )
    })?;
    emit_email_download_progress(&app, bonsai.selected_id, 1, 1, "Bonsai ready");
    Ok(())
}

async fn download_bonsai_gguf(
    app: &Option<AppHandle>,
    bonsai: BonsaiModel,
    gguf_path: &Path,
) -> Result<(), String> {
    let url = bonsai_download_url(bonsai);
    let tmp_path = gguf_path.with_extension("gguf.download");
    let _ = std::fs::remove_file(&tmp_path);
    let client = ollama_client(Duration::from_secs(900))?;
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Bonsai GGUF download failed: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Bonsai GGUF download error ({status}): {body}"));
    }

    let total = response.content_length().unwrap_or(0);
    emit_email_download_progress(app, bonsai.selected_id, 0, total, "Downloading Bonsai GGUF");
    let mut file = std::fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed to create Bonsai temp file: {e}"))?;
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Bonsai GGUF stream failed: {e}"))?;
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write Bonsai GGUF: {e}"))?;
        downloaded += chunk.len() as u64;
        emit_email_download_progress(
            app,
            bonsai.selected_id,
            downloaded,
            total,
            "Downloading Bonsai GGUF",
        );
    }
    file.sync_all()
        .map_err(|e| format!("Failed to flush Bonsai GGUF: {e}"))?;
    drop(file);
    if downloaded < bonsai.min_bytes {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!(
            "Bonsai GGUF download was incomplete: got {} MB, expected at least {} MB.",
            downloaded / 1_000_000,
            bonsai.min_bytes / 1_000_000
        ));
    }
    match std::fs::rename(&tmp_path, gguf_path) {
        Ok(_) => {}
        Err(e) if valid_bonsai_gguf(gguf_path, bonsai) => {
            tracing::warn!(
                error = %e,
                model = bonsai.selected_id,
                "Bonsai temp rename failed, but final GGUF already exists"
            );
        }
        Err(e) => return Err(format!("Failed to finalise Bonsai GGUF: {e}")),
    }
    Ok(())
}

fn valid_bonsai_gguf(path: &Path, bonsai: BonsaiModel) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.len() >= bonsai.min_bytes)
        .unwrap_or(false)
}

fn write_bonsai_modelfile(modelfile_path: &Path, gguf_path: &Path) -> Result<(), String> {
    let content = format!(
        "FROM {}\nPARAMETER temperature 0.35\nPARAMETER num_ctx 4096\nSYSTEM \"\"\"{}\"\"\"\n",
        gguf_path.display(),
        system_prompt()
    );
    std::fs::write(modelfile_path, content)
        .map_err(|e| format!("Failed to write Bonsai Modelfile: {e}"))
}

fn create_ollama_model(bonsai: BonsaiModel, modelfile_path: &Path) -> Result<(), String> {
    let executable = ollama_executable_candidates()
        .into_iter()
        .find(|candidate| !candidate.is_absolute() || candidate.exists())
        .ok_or_else(|| "Ollama executable not found.".to_string())?;
    let output = background_command(executable)
        .env("NO_COLOR", "1")
        .env("OLLAMA_NEW_ENGINE", "1")
        .arg("create")
        .arg(bonsai.ollama_name)
        .arg("-f")
        .arg(modelfile_path)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Failed to run `ollama create`: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "Bonsai downloaded, but Ollama could not create `{}`. stdout: {} stderr: {}",
            bonsai.ollama_name,
            compact_command_output(&stdout),
            compact_command_output(&stderr)
        ))
    }
}

async fn smoke_test_ollama_model(model: &str, is_bonsai: bool) -> Result<(), String> {
    let timeout_duration = if is_bonsai {
        BONSAI_DRAFT_TIMEOUT
    } else {
        OLLAMA_DRAFT_TIMEOUT
    };
    let request = OllamaGenerateRequest {
        model: model.to_string(),
        system: "Return only the word ok.".to_string(),
        prompt: "ok".to_string(),
        stream: false,
        options: OllamaOptions {
            temperature: 0.0,
            num_predict: 4,
            repeat_penalty: 1.1,
        },
    };
    let client = ollama_client(timeout_duration)?;
    let response = timeout(
        timeout_duration,
        client
            .post("http://localhost:11434/api/generate")
            .json(&request)
            .send(),
    )
    .await
    .map_err(|_| {
        format!(
            "load test timed out after {} seconds",
            timeout_duration.as_secs()
        )
    })?
    .map_err(|e| format!("load test request failed: {e}"))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("load test error ({status}): {}", body.trim()))
    }
}

fn compact_command_output(output: &str) -> String {
    let cleaned = strip_ansi(output).replace('\r', "\n");
    let mut lines = cleaned
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    lines.dedup();
    let text = lines.join(" | ");
    if text.len() > 900 {
        format!("{}...", &text[..900])
    } else {
        text
    }
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            while let Some(next) = chars.next() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
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

fn ollama_runtime_model(model: &str) -> Result<String, String> {
    let model = normalize_ollama_draft_model(model)?;
    Ok(bonsai_model(&model)
        .map(|bonsai| bonsai.ollama_name.to_string())
        .unwrap_or(model))
}

fn bonsai_model(model: &str) -> Option<BonsaiModel> {
    BONSAI_MODELS
        .iter()
        .copied()
        .find(|candidate| candidate.selected_id == model || candidate.ollama_name == model)
}

fn bonsai_download_url(bonsai: BonsaiModel) -> String {
    format!(
        "https://huggingface.co/{}/resolve/main/{}?download=true",
        bonsai.repo, bonsai.file
    )
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
        match background_command(&candidate)
            .env("OLLAMA_NEW_ENGINE", "1")
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

fn background_command<S: AsRef<OsStr>>(program: S) -> Command {
    let mut command = Command::new(program);
    configure_background_command(&mut command);
    command
}

#[cfg(windows)]
fn configure_background_command(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn configure_background_command(_command: &mut Command) {}

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

fn ollama_prompt(instruction: &str, is_bonsai: bool) -> String {
    if !is_bonsai {
        return instruction.to_string();
    }

    [
        "Task: write one finished email draft from the instruction.",
        "Return only the email text.",
        "No labels, no explanation, no repeated alternatives.",
        "Use Portuguese unless the instruction asks for another language.",
        "",
        "Instruction:",
        instruction.trim(),
        "",
        "Email draft:",
    ]
    .join("\n")
}

fn clean_model_draft(draft: &str) -> String {
    draft
        .trim()
        .trim_matches('`')
        .lines()
        .map(str::trim_end)
        .filter(|line| {
            let lower = line.trim().to_lowercase();
            !matches!(
                lower.as_str(),
                "email draft:" | "draft:" | "resposta:" | "greeting:" | "body:" | "closing:"
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn should_use_local_fallback(draft: &str) -> bool {
    let cleaned = clean_model_draft(draft);
    let lower = cleaned.to_lowercase();
    if cleaned.len() > 1_200 || cleaned.lines().count() > 12 {
        return true;
    }
    let bad_markers = [
        "return only",
        "just the",
        "here is",
        "aqui est",
        "posso ajudar",
        "o que voc",
        "greeting",
        "body:",
        "closing:",
        "subject:",
        "instruction:",
        "email draft:",
    ];
    if bad_markers.iter().any(|marker| lower.contains(marker)) {
        return true;
    }
    let mut seen_lines = std::collections::HashSet::new();
    for line in lower.lines().map(str::trim).filter(|line| line.len() > 12) {
        if !seen_lines.insert(line) {
            return true;
        }
    }

    has_repeated_ngram(&lower, 3) || has_repeated_ngram(&lower, 4)
}

fn has_repeated_ngram(text: &str, size: usize) -> bool {
    let words = text
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if words.len() < size * 2 {
        return false;
    }

    let mut previous = String::new();
    let mut repeats = 0;
    for window in words.windows(size) {
        let current = window.join(" ");
        if current == previous {
            repeats += 1;
            if repeats >= 1 {
                return true;
            }
        } else {
            repeats = 0;
            previous = current;
        }
    }

    false
}

fn local_email_fallback(instruction: &str) -> String {
    let instruction = clean_email_instruction(instruction);
    let recipient = extract_portuguese_recipient(&instruction);
    let body = clean_body_sentence(&instruction);

    let greeting = recipient
        .map(|name| format!("Olá Sr. {name},"))
        .unwrap_or_else(|| "Olá,".to_string());

    format!("{greeting}\n\nEscrevo para informar que {body}.\n\nCumprimentos,")
}

fn clean_email_instruction(instruction: &str) -> String {
    let mut text = instruction.trim().trim_matches('"').trim().to_string();
    let prefixes = [
        "queria-me um email que diga que",
        "queria me um email que diga que",
        "queria-me um email que diga",
        "queria me um email que diga",
        "queria um email que diga que",
        "queria um email que diga",
        "quero um email que diga que",
        "quero um email que diga",
        "faz-me um email a dizer que",
        "faz me um email a dizer que",
        "faz-me um email a dizer, que",
        "faz me um email a dizer, que",
        "faz-me um email a dizer",
        "faz me um email a dizer",
        "cria um email a dizer que",
        "cria um email a dizer",
        "escreve um email a dizer que",
        "escreve um email a dizer",
    ];

    loop {
        let lower = text.to_lowercase();
        let Some(prefix) = prefixes.iter().find(|prefix| lower.starts_with(**prefix)) else {
            break;
        };
        text = text[prefix.len()..]
            .trim_start_matches(|c: char| c == ',' || c == ':' || c.is_whitespace())
            .to_string();
    }

    loop {
        let lower = text.to_lowercase();
        if lower.starts_with("que ") {
            text = text[4..]
                .trim_start_matches(|c: char| c == ',' || c == ':' || c.is_whitespace())
                .to_string();
        } else {
            break;
        }
    }

    text
}

fn extract_portuguese_recipient(instruction: &str) -> Option<String> {
    let lower = instruction.to_lowercase();
    let marker = "para o senhor ";
    let start = lower.find(marker)? + marker.len();
    let tail = &instruction[start..];
    let stop_words = [" que ", " hoje ", " amanhã ", " as ", " às ", ".", ","];
    let mut end = tail.len();
    let tail_lower = tail.to_lowercase();
    for stop in stop_words {
        if let Some(pos) = tail_lower.find(stop) {
            end = end.min(pos);
        }
    }
    let name = tail[..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn clean_body_sentence(instruction: &str) -> String {
    let mut body = instruction.trim().trim_end_matches('.').to_string();
    body = normalize_portuguese_times(&body);
    body = body.replace("eu vou aí estar", "vou aí estar");
    body = body.replace("eu vou la estar", "vou aí estar");
    body = body.replace("eu vou lá estar", "vou aí estar");
    body = body.replace("vou la", "vou aí");
    body = body.replace("vou lá", "vou aí");
    if let Some(name) = extract_portuguese_recipient(&body) {
        body = body.replace(&format!(" para o senhor {name}"), "");
    }
    body.trim()
        .trim_start_matches(|c: char| c == ',' || c.is_whitespace())
        .to_string()
}

fn normalize_portuguese_times(input: &str) -> String {
    let mut text = input.to_string();
    for hour in 1..=12 {
        text = text.replace(
            &format!(" às {hour} da tarde"),
            &format!(" às {}h", if hour == 12 { 12 } else { hour + 12 }),
        );
        text = text.replace(
            &format!(" as {hour} da tarde"),
            &format!(" às {}h", if hour == 12 { 12 } else { hour + 12 }),
        );
        text = text.replace(
            &format!(" às {hour} da noite"),
            &format!(" às {}h", if hour == 12 { 0 } else { hour + 12 }),
        );
        text = text.replace(
            &format!(" as {hour} da noite"),
            &format!(" às {}h", if hour == 12 { 0 } else { hour + 12 }),
        );
        text = text.replace(&format!(" as {hour} da manhã"), &format!(" às {hour}h"));
        text = text.replace(&format!(" às {hour} da manhã"), &format!(" às {hour}h"));
    }
    text
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
    async fn falls_back_when_groq_key_is_missing() {
        let result =
            generate_email_draft("", "groq", DEFAULT_GROQ_DRAFT_MODEL, "reply in English").await;
        let draft = result.unwrap();
        assert!(draft.contains("Olá,"));
        assert!(draft.contains("reply in English"));
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
        assert!(normalize_ollama_draft_model("veyra-bonsai-1.7b").is_ok());
        assert!(normalize_ollama_draft_model("veyra-bonsai-4b").is_err());
        assert!(normalize_ollama_draft_model("../not-a-model").is_err());
    }

    #[test]
    fn bonsai_model_metadata_maps_to_ollama_runtime_name() {
        assert_eq!(
            bonsai_model("veyra-bonsai-1.7b").unwrap().ollama_name,
            "veyra-bonsai:f16-1.7b"
        );
        assert_eq!(
            ollama_runtime_model("veyra-bonsai-1.7b").unwrap(),
            "veyra-bonsai:f16-1.7b"
        );
    }

    #[test]
    fn bonsai_download_url_points_to_hugging_face_gguf() {
        let bonsai = bonsai_model("veyra-bonsai-1.7b").unwrap();
        assert_eq!(
            bonsai_download_url(bonsai),
            "https://huggingface.co/prism-ml/Ternary-Bonsai-1.7B-gguf/resolve/main/Ternary-Bonsai-1.7B-F16.gguf?download=true"
        );
    }

    #[test]
    fn detects_low_quality_model_output() {
        assert!(should_use_local_fallback(
            "Greeting:\nOla,\nBody:\nO que voce pode fazer hoje? O que voce pode fazer hoje?"
        ));
        assert!(should_use_local_fallback(
            ", que esta em casa.\n\nObrigado, e espero que ele me ajude.\n\nP.S. Se voce nao estiver aqui hoje, por favor, me avise.\n\nP.S. Se voce nao estiver aqui hoje, por favor, me avise."
        ));
        assert!(!should_use_local_fallback(
            "Ola Sr. Bruno Rodrigues,\n\nEscrevo para informar que hoje vou ai estar as 17h.\n\nCumprimentos,"
        ));
    }

    #[test]
    fn compact_command_output_strips_ansi_and_repeated_lines() {
        let output = "\u{1b}[?2026hgathering model components\r\n\u{1b}[?25lgathering model components\r\nerror";
        assert_eq!(
            compact_command_output(output),
            "gathering model components | error"
        );
    }

    #[test]
    fn local_fallback_creates_basic_portuguese_email() {
        let draft = local_email_fallback(
            "faz me um email a dizer, que hoje vou la as 5 da tarde para o senhor Bruno Rodrigues",
        );

        assert!(draft.contains("Olá Sr. Bruno Rodrigues,"));
        assert!(draft.contains("hoje vou aí às 17h"));
        assert!(draft.contains("Cumprimentos,"));
    }

    #[test]
    fn local_fallback_strips_transcribed_request_wording() {
        let draft = local_email_fallback(
            "Queria-me um email que diga que eu vou aí estar às 10 da noite para o senhor Luís Rodrigues",
        );

        assert!(draft.contains("Olá Sr. Luís Rodrigues,"));
        assert!(draft.contains("vou aí estar às 22h"));
        assert!(!draft.contains("Queria-me"));
        assert!(!draft.contains("email que diga"));
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
