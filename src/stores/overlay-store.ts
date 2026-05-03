import { create } from "zustand";

export type OverlayState = "idle" | "recording" | "transcribing" | "success" | "error" | "cancelled";
export type OverlayMode = "dictation" | "command";

interface OverlayStore {
  state: OverlayState;
  mode: OverlayMode;
  level: number;
  /**
   * Wall-clock timestamp (Date.now()) of the most recent idle -> recording
   * transition. Frozen across recording -> transcribing so the elapsed
   * timer holds at the recording-end value while the engine is finalizing.
   * Reset to null when the overlay returns to idle.
   */
  recordingStartedAt: number | null;
  setState: (state: OverlayState) => void;
  setMode: (mode: OverlayMode) => void;
  setLevel: (level: number) => void;
  setRecordingStartedAt: (value: number | null) => void;
}

export const useOverlayStore = create<OverlayStore>((set) => ({
  state: "idle",
  mode: "dictation",
  level: 0,
  recordingStartedAt: null,
  setState: (next) =>
    set((prev) => {
      if (prev.state === next) return {};
      // Stamp the start of a fresh recording session.
      if (next === "recording" && prev.state !== "recording") {
        return { state: next, recordingStartedAt: Date.now() };
      }
      // Returning to idle clears the stamp so the next session starts at 0.
      if (next === "idle") {
        return { state: next, recordingStartedAt: null };
      }
      return { state: next };
    }),
  setMode: (mode) => set({ mode }),
  setLevel: (level) => set({ level }),
  setRecordingStartedAt: (recordingStartedAt) => set({ recordingStartedAt }),
}));
