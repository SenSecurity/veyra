use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub microphone: String,
    pub engine: String,
    #[serde(rename = "whisperModel")]
    pub whisper_model: String,
    #[serde(rename = "emailDraftEngine", default = "default_email_draft_engine")]
    pub email_draft_engine: String,
    #[serde(rename = "emailDraftModel", default = "default_email_draft_model")]
    pub email_draft_model: String,
    #[serde(rename = "groqApiKey")]
    pub groq_api_key: String,
    #[serde(rename = "recordingMode")]
    pub recording_mode: String,
    pub hotkey: String,
    #[serde(rename = "commandHotkey", default = "default_command_hotkey")]
    pub command_hotkey: String,
    #[serde(rename = "overlayStyle", default = "default_overlay_style")]
    pub overlay_style: String,
    #[serde(rename = "overlaySize", default = "default_overlay_size")]
    pub overlay_size: String,
}

fn default_command_hotkey() -> String {
    "Pause".to_string()
}

fn default_email_draft_model() -> String {
    "llama3.2:1b".to_string()
}

fn default_email_draft_engine() -> String {
    "ollama".to_string()
}

fn default_overlay_style() -> String {
    "capsule".to_string()
}

fn default_overlay_size() -> String {
    "medium".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            microphone: "default".to_string(),
            engine: "local".to_string(),
            whisper_model: "small".to_string(),
            email_draft_engine: default_email_draft_engine(),
            email_draft_model: default_email_draft_model(),
            groq_api_key: String::new(),
            recording_mode: "toggle".to_string(),
            hotkey: "F24".to_string(),
            command_hotkey: default_command_hotkey(),
            overlay_style: default_overlay_style(),
            overlay_size: default_overlay_size(),
        }
    }
}

impl Settings {
    pub fn config_path(app_dir: &PathBuf) -> PathBuf {
        app_dir.join("config.json")
    }

    pub fn load(app_dir: &PathBuf) -> Self {
        let path = Self::config_path(app_dir);
        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self, app_dir: &PathBuf) -> Result<(), String> {
        let path = Self::config_path(app_dir);
        fs::create_dir_all(app_dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.microphone, "default");
        assert_eq!(settings.engine, "local");
        assert_eq!(settings.whisper_model, "small");
        assert_eq!(settings.email_draft_engine, "ollama");
        assert_eq!(settings.email_draft_model, "llama3.2:1b");
        assert_eq!(settings.groq_api_key, "");
        assert_eq!(settings.recording_mode, "toggle");
        assert_eq!(settings.hotkey, "F24");
        assert_eq!(settings.command_hotkey, "Pause");
        assert_eq!(settings.overlay_style, "capsule");
        assert_eq!(settings.overlay_size, "medium");
    }

    #[test]
    fn test_legacy_config_without_overlay_fields_loads_with_defaults() {
        // Legacy config.json files written before the overlay fields existed
        // must keep loading; missing fields fall back to capsule + medium.
        let legacy_json = r#"{
            "microphone": "default",
            "engine": "local",
            "whisperModel": "turbo",
            "emailDraftEngine": "ollama",
            "emailDraftModel": "llama3.2:1b",
            "groqApiKey": "",
            "recordingMode": "toggle",
            "hotkey": "F24",
            "commandHotkey": "Pause"
        }"#;
        let parsed: Settings = serde_json::from_str(legacy_json).expect("legacy parse");
        assert_eq!(parsed.overlay_style, "capsule");
        assert_eq!(parsed.overlay_size, "medium");
    }

    #[test]
    fn test_overlay_fields_round_trip() {
        let mut settings = Settings::default();
        settings.overlay_style = "orb".to_string();
        settings.overlay_size = "large".to_string();
        let json = serde_json::to_string(&settings).expect("serialize");
        let restored: Settings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.overlay_style, "orb");
        assert_eq!(restored.overlay_size, "large");
    }

    #[test]
    fn test_save_and_load() {
        let dir = temp_dir().join("typr_test_settings");
        let _ = fs::remove_dir_all(&dir);

        let mut settings = Settings::default();
        settings.engine = "cloud".to_string();
        settings.groq_api_key = "test-key-123".to_string();

        settings.save(&dir).unwrap();
        let loaded = Settings::load(&dir);

        assert_eq!(loaded.engine, "cloud");
        assert_eq!(loaded.groq_api_key, "test-key-123");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let dir = temp_dir().join("typr_test_missing");
        let _ = fs::remove_dir_all(&dir);
        let settings = Settings::load(&dir);
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn test_load_corrupt_json_returns_default() {
        let dir = temp_dir().join("typr_test_corrupt");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("config.json"), "not json").unwrap();

        let settings = Settings::load(&dir);
        assert_eq!(settings, Settings::default());

        let _ = fs::remove_dir_all(&dir);
    }
}
