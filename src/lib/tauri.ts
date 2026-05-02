import { invoke } from "@tauri-apps/api/core";
import type {
  DailyStats,
  DictionaryTerm,
  MicDevice,
  NewDictionaryTermInput,
  NewScratchpadNoteInput,
  NewSnippetInput,
  RecordingState,
  ScratchpadNote,
  Settings,
  Snippet,
  StreakInfo,
  Totals,
  Transcription,
  WizardStatus,
} from "@/types/ipc";

// Typed wrapper over the Phase 1+2+3 Tauri command surface. UI code consumes
// `ipc` from this adapter; no raw `invoke` calls in components.
//
// Tauri 2 auto-converts snake_case command argument names to camelCase on the
// JS side, so `model_size: String` on the Rust handler becomes `modelSize`
// here. Confirmed against the legacy `src/main.ts` invocation style.
export const ipc = {
  // settings + recorder + models (Phase 1+2)
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) =>
    invoke<void>("save_settings", { settings }),
  listMicrophones: () => invoke<MicDevice[]>("list_microphones"),
  getRecordingState: () => invoke<RecordingState>("get_recording_state"),
  getRecordingLevel: () => invoke<number>("get_recording_level"),
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

  // snippets
  listSnippets: () => invoke<Snippet[]>("list_snippets"),
  upsertSnippet: (snippet: NewSnippetInput) =>
    invoke<number>("upsert_snippet", { snippet }),
  deleteSnippet: (id: number) => invoke<void>("delete_snippet", { id }),

  // scratchpad
  listScratchpadNotes: () =>
    invoke<ScratchpadNote[]>("list_scratchpad_notes"),
  upsertScratchpadNote: (note: NewScratchpadNoteInput) =>
    invoke<number>("upsert_scratchpad_note", { note }),
  deleteScratchpadNote: (id: number) =>
    invoke<void>("delete_scratchpad_note", { id }),
  pinScratchpadNote: (id: number, pinned: boolean) =>
    invoke<void>("pin_scratchpad_note", { id, pinned }),

  // stats
  getStatsTotals: () => invoke<Totals>("get_stats_totals"),
  getStatsStreak: () => invoke<StreakInfo>("get_stats_streak"),
  getStatsByDay: () => invoke<DailyStats[]>("get_stats_by_day"),

  // wizard
  wizardStatus: () => invoke<WizardStatus>("wizard_status"),
  markWizardComplete: () => invoke<void>("mark_wizard_complete"),

  // groq key test
  testGroqKey: (key: string) => invoke<void>("test_groq_key", { key }),
};
