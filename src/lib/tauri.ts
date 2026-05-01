import { invoke } from "@tauri-apps/api/core";
import type {
  MicDevice,
  RecordingState,
  Settings,
} from "@/types/ipc";

// Typed wrapper over the Phase 1+2 Tauri command surface. UI code consumes
// `ipc` from this adapter — no raw `invoke` calls in components.
//
// Tauri 2 auto-converts snake_case command argument names to camelCase on the
// JS side, so `model_size: String` on the Rust handler becomes `modelSize`
// here. Confirmed against the legacy `src/main.ts` invocation style.
export const ipc = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) =>
    invoke<void>("save_settings", { settings }),
  listMicrophones: () => invoke<MicDevice[]>("list_microphones"),
  getRecordingState: () => invoke<RecordingState>("get_recording_state"),
  checkModelDownloaded: (modelSize: string) =>
    invoke<boolean>("check_model_downloaded", { modelSize }),
  downloadModel: (modelSize: string) =>
    invoke<void>("download_model", { modelSize }),
  toggleRecording: () => invoke<string>("toggle_recording"),
};
