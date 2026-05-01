// IPC type skeleton. Expanded in subsequent Phase 3 tasks as routes
// gain real Tauri command bindings.

export type RecordingState = "Ready" | "Recording" | "Transcribing";

export interface MicDevice {
  name: string;
  isDefault: boolean;
}
