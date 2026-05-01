// IPC type skeleton. Expanded in subsequent Phase 3 tasks as routes
// gain real Tauri command bindings.

export type RecordingState = "Ready" | "Recording" | "Transcribing";

// `src-tauri/src/audio/recorder.rs::MicDevice` derives only Serialize with
// no `rename_all = "camelCase"`, so the JSON payload field is `is_default`.
// Match the wire shape exactly — a TS `isDefault` would silently always be
// `undefined` at runtime.
export interface MicDevice {
  name: string;
  is_default: boolean;
}

export type { Settings } from "./settings";
export type { Transcription } from "./transcription";
