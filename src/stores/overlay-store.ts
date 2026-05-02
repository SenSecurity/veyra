import { create } from "zustand";

export type OverlayState = "idle" | "recording" | "transcribing" | "success" | "error" | "cancelled";
export type OverlayMode = "dictation" | "command";

interface OverlayStore {
  state: OverlayState;
  mode: OverlayMode;
  level: number;
  setState: (state: OverlayState) => void;
  setMode: (mode: OverlayMode) => void;
  setLevel: (level: number) => void;
}

export const useOverlayStore = create<OverlayStore>((set) => ({
  state: "idle",
  mode: "dictation",
  level: 0,
  setState: (state) => set({ state }),
  setMode: (mode) => set({ mode }),
  setLevel: (level) => set({ level }),
}));
