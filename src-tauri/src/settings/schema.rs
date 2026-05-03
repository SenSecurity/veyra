//! v2 Settings — nested shape persisted as JSON (`config.json`).
//! Secrets (`groqApiKey`) live in Windows Credential Manager, not here.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    pub microphone: String,
    pub transcription: Transcription,
    pub hotkeys: Hotkeys,
    pub overlay: Overlay,
    pub formatting: Formatting,
    pub dictionary: Dictionary,
    pub stats: Stats,
    pub data: DataPolicy,
    pub system: System,
    pub ui: Ui,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcription {
    pub engine: String,
    pub whisper_model: String,
    #[serde(default = "default_email_draft_engine")]
    pub email_draft_engine: String,
    #[serde(default = "default_email_draft_model")]
    pub email_draft_model: String,
    pub languages: Vec<String>,
    pub auto_detect: bool,
    pub gpu_acceleration: String,
    pub vad_enabled: bool,
    pub no_speech_threshold: f64,
}

pub fn default_email_draft_engine() -> String {
    "ollama".to_string()
}

pub fn default_email_draft_model() -> String {
    "llama3.2:1b".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hotkeys {
    pub dictation: String,
    pub command_mode: String,
    pub recording_mode: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Overlay {
    pub style: String,
    #[serde(default = "default_overlay_size")]
    pub size: String,
    pub position: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_pos: Option<OverlayPos>,
}

pub fn default_overlay_size() -> String {
    "medium".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlayPos {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Formatting {
    pub enhance_enabled: bool,
    pub remove_fillers: bool,
    pub filler_words: Vec<String>,
    pub explicit_commands: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dictionary {
    pub auto_add: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub enabled: bool,
    pub milestone_notifications: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataPolicy {
    pub word_count_cap: u64,
    pub purge_on_exceed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct System {
    pub launch_at_login: bool,
    pub close_to_tray: bool,
    pub dictation_sounds: bool,
    pub mute_music_on_dictate: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ui {
    pub language: String,
    pub theme: String,
    pub accent: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 3,
            microphone: "default".to_string(),
            transcription: Transcription {
                engine: "local".to_string(),
                whisper_model: "turbo".to_string(),
                email_draft_engine: default_email_draft_engine(),
                email_draft_model: default_email_draft_model(),
                languages: vec!["pt".to_string(), "en".to_string()],
                auto_detect: true,
                gpu_acceleration: "auto".to_string(),
                vad_enabled: true,
                no_speech_threshold: 0.6,
            },
            hotkeys: Hotkeys {
                dictation: "F24".to_string(),
                command_mode: "Pause".to_string(),
                recording_mode: "push-to-talk".to_string(),
            },
            overlay: Overlay {
                style: "capsule".to_string(),
                size: default_overlay_size(),
                position: "near-cursor".to_string(),
                custom_pos: None,
            },
            formatting: Formatting {
                enhance_enabled: false,
                remove_fillers: true,
                filler_words: vec![
                    "uh".to_string(),
                    "um".to_string(),
                    "né".to_string(),
                    "tipo".to_string(),
                ],
                explicit_commands: true,
            },
            dictionary: Dictionary { auto_add: false },
            stats: Stats {
                enabled: true,
                milestone_notifications: true,
            },
            data: DataPolicy {
                word_count_cap: 500_000,
                purge_on_exceed: true,
            },
            system: System {
                launch_at_login: false,
                close_to_tray: true,
                dictation_sounds: true,
                mute_music_on_dictate: false,
            },
            ui: Ui {
                language: "en".to_string(),
                theme: "system".to_string(),
                accent: "indigo".to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_matches_spec_defaults() {
        let s = Settings::default();
        assert_eq!(s.schema_version, 3);
        assert_eq!(s.microphone, "default");
        assert_eq!(s.transcription.engine, "local");
        assert_eq!(s.transcription.whisper_model, "turbo");
        assert_eq!(s.transcription.email_draft_engine, "ollama");
        assert_eq!(s.transcription.email_draft_model, "llama3.2:1b");
        assert_eq!(s.transcription.languages, vec!["pt", "en"]);
        assert!(s.transcription.auto_detect);
        assert_eq!(s.transcription.gpu_acceleration, "auto");
        assert!(s.transcription.vad_enabled);
        assert!((s.transcription.no_speech_threshold - 0.6).abs() < 1e-9);
        assert_eq!(s.hotkeys.dictation, "F24");
        assert_eq!(s.hotkeys.command_mode, "Pause");
        assert_eq!(s.hotkeys.recording_mode, "push-to-talk");
        assert_eq!(s.overlay.style, "capsule");
        assert_eq!(s.overlay.size, "medium");
        assert_eq!(s.data.word_count_cap, 500_000);
        assert!(s.data.purge_on_exceed);
        assert_eq!(s.ui.theme, "system");
    }

    #[test]
    fn serializes_with_camelcase_keys() {
        let s = Settings::default();
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["schemaVersion"], 3);
        assert_eq!(json["transcription"]["whisperModel"], "turbo");
        assert_eq!(json["transcription"]["emailDraftEngine"], "ollama");
        assert_eq!(json["transcription"]["emailDraftModel"], "llama3.2:1b");
        assert_eq!(json["hotkeys"]["recordingMode"], "push-to-talk");
        assert_eq!(json["data"]["wordCountCap"], 500_000);
        assert_eq!(json["system"]["muteMusicOnDictate"], false);
        assert!(json["transcription"].get("groqApiKey").is_none());
    }

    #[test]
    fn round_trip_preserves_fields() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
