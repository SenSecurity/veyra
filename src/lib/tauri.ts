import { invoke } from "@tauri-apps/api/core";
import type {
  DailyStats,
  DictionaryTerm,
  MicDevice,
  NewDictionaryTermInput,
  RecordingState,
  Settings,
  StreakInfo,
  Totals,
  Transcription,
  WizardStatus,
} from "@/types/ipc";

// Typed wrapper over the Tauri command surface used by the current UI.
// UI code consumes `ipc` from this adapter; no raw `invoke` calls in components.
//
// Tauri 2 auto-converts snake_case command argument names to camelCase on the
// JS side, so `model_size: String` on the Rust handler becomes `modelSize`
// here. Confirmed against the legacy `src/main.ts` invocation style.
export const ipc = {
  // settings + recorder + models (Phase 1+2)
  windowMinimize: () => invoke<void>("window_minimize"),
  windowToggleMaximize: () => invoke<void>("window_toggle_maximize"),
  windowClose: () => invoke<void>("window_close"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) =>
    invoke<void>("save_settings", { settings }),
  listMicrophones: () => invoke<MicDevice[]>("list_microphones"),
  getRecordingState: () => invoke<RecordingState>("get_recording_state"),
  getRecordingLevel: () => invoke<number>("get_recording_level"),
  getRecordingMode: () => invoke<"dictation" | "command">("get_recording_mode"),
  checkModelDownloaded: (modelSize: string) =>
    invoke<boolean>("check_model_downloaded", { modelSize }),
  downloadModel: (modelSize: string) =>
    invoke<void>("download_model", { modelSize }),
  cancelModelDownload: () => invoke<void>("cancel_model_download"),
  cancelRecording: () => invoke<void>("cancel_recording"),
  toggleRecording: () => invoke<string>("toggle_recording"),

  // transcriptions
  listTranscriptions: (limit: number, offset: number) =>
    invoke<Transcription[]>("list_transcriptions", { limit, offset }),
  listEmailDrafts: (limit: number, offset: number) =>
    invoke<Transcription[]>("list_email_drafts", { limit, offset }),
  searchTranscriptions: (query: string, limit: number) =>
    invoke<Transcription[]>("search_transcriptions", { query, limit }),
  deleteTranscription: (id: number) =>
    invoke<void>("delete_transcription", { id }),

  // dictionary
  listDictionaryTerms: () =>
    invoke<DictionaryTerm[]>("list_dictionary_terms"),
  upsertDictionaryTerm: (term: NewDictionaryTermInput) =>
    invoke<number>("upsert_dictionary_term", { term }),
  deleteDictionaryTerm: (id: number) =>
    invoke<void>("delete_dictionary_term", { id }),

  // stats
  getStatsTotals: () => invoke<Totals>("get_stats_totals"),
  getStatsStreak: () => invoke<StreakInfo>("get_stats_streak"),
  getStatsByDay: () => invoke<DailyStats[]>("get_stats_by_day"),

  // wizard
  wizardStatus: () => invoke<WizardStatus>("wizard_status"),
  markWizardComplete: () => invoke<void>("mark_wizard_complete"),

  // groq key test
  testGroqKey: (key: string) => invoke<void>("test_groq_key", { key }),
  checkEmailDraftModel: (key: string, engine: string, model: string) =>
    invoke<void>("check_email_draft_model", { key, engine, model }),
  downloadEmailDraftModel: (engine: string, model: string) =>
    invoke<void>("download_email_draft_model", { engine, model }),
  openOllamaDownload: () => invoke<void>("open_ollama_download"),
};
