use serde::{Deserialize, Serialize};
use std::time::Duration;

pub const DRAFT_MODEL: &str = "llama-3.3-70b-versatile";

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: &'static str,
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

pub async fn generate_email_draft(api_key: &str, instruction: &str) -> Result<String, String> {
    if api_key.trim().is_empty() {
        return Err("Groq API key not set. Add it in Settings > Transcription.".to_string());
    }

    let instruction = instruction.trim();
    if instruction.is_empty() {
        return Err("No command instruction was transcribed.".to_string());
    }

    let request = ChatRequest {
        model: DRAFT_MODEL,
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
        let result = generate_email_draft("", "reply in English").await;
        assert!(result.unwrap_err().contains("API key"));
    }

    #[tokio::test]
    async fn rejects_empty_instruction() {
        let result = generate_email_draft("gsk_test", "   ").await;
        assert!(result.unwrap_err().contains("No command instruction"));
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
