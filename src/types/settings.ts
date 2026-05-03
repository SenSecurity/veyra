// Mirror of `src-tauri/src/settings/legacy_v1.rs::Settings`.
//
// The Rust side serialises with explicit serde renames for the camelCase
// fields below, and leaves `microphone`, `engine`, `hotkey` unchanged. All
// fields are plain `String` on the Rust side; the narrow unions here reflect
// the values the legacy frontend actually writes (see `src/main.ts`). If the
// pipeline ever introduces a new value, widen the union here first.
export type OverlayStyle = "capsule" | "orb";
export type OverlaySize = "small" | "medium" | "large";

export interface Settings {
  microphone: string;
  // Legacy frontend uses "local" | "cloud"; "cloud" routes to Groq.
  engine: "local" | "cloud";
  whisperModel: string;
  emailDraftEngine: "ollama" | "groq";
  emailDraftModel: string;
  groqApiKey: string;
  recordingMode: "toggle" | "push-to-talk";
  hotkey: string;
  commandHotkey: string;
  /** Visual style of the floating recording overlay. */
  overlayStyle: OverlayStyle;
  /** Three-step size for the chosen overlay style. */
  overlaySize: OverlaySize;
}
