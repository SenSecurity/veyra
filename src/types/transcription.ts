// Mirror of `src-tauri/src/storage/transcriptions.rs::Transcription`.
//
// NOTE: at the time of T1 the Rust struct does NOT derive `serde::Serialize`
// and is not exposed by any Tauri command yet. T2 will add the list/search
// commands and is expected to add `#[serde(rename_all = "camelCase")]` so
// this camelCase shape matches the wire payload. If T2 chooses snake_case
// instead, this file must be updated to match.
export interface Transcription {
  id: number;
  createdAt: number;
  rawText: string;
  finalText: string;
  wordCount: number;
  durationMs: number;
  language: string;
  engine: string;
  model: string | null;
  appContext: string | null;
  mode: string;
  enhanced: boolean;
}
