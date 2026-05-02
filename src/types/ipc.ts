// IPC types used by the current Veyra UI.

export type RecordingState = "Ready" | "Recording" | "Transcribing";

// `src-tauri/src/audio/recorder.rs::MicDevice` derives only Serialize with
// no `rename_all = "camelCase"`, so the JSON payload field is `is_default`.
// Match the wire shape exactly; a TS `isDefault` would silently always be
// `undefined` at runtime.
export interface MicDevice {
  name: string;
  is_default: boolean;
}

export type { Settings } from "./settings";
export type { Transcription } from "./transcription";
export type { DictionaryTerm, NewDictionaryTermInput } from "./dictionary";
export type { Totals, StreakInfo, DailyStats } from "./stats";
export type { WizardStatus } from "./wizard";
