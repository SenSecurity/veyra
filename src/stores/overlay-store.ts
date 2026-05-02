import { create } from "zustand";

export type OverlayState = "idle" | "recording" | "transcribing" | "success" | "error" | "cancelled";

interface OverlayStore {
  state: OverlayState;
  level: number;
  setState: (state: OverlayState) => void;
  setLevel: (level: number) => void;
}

export const useOverlayStore = create<OverlayStore>((set) => ({
  state: "idle",
  level: 0,
  setState: (state) => set({ state }),
  setLevel: (level) => set({ level }),
}));
