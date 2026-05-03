import { create } from "zustand";
import { ipc } from "@/lib/tauri";
import type { Settings } from "@/types/settings";

interface SettingsStore {
  settings: Settings | null;
  loading: boolean;
  error: string | null;
  load: () => Promise<void>;
  save: (patch: Partial<Settings>) => Promise<void>;
}

export const useSettingsStore = create<SettingsStore>((set, get) => ({
  settings: null,
  loading: false,
  error: null,
  load: async () => {
    set({ loading: true, error: null });
    try {
      const settings = await ipc.getSettings();
      set({ settings, loading: false });
    } catch (error) {
      set({ error: String(error), loading: false });
    }
  },
  save: async (patch) => {
    const current = get().settings;
    if (!current) return;
    const next = { ...current, ...patch };
    set({ settings: next, error: null });
    try {
      await ipc.saveSettings(next);
      // If overlay layout fields changed, ping the OS window to resize
      // immediately rather than waiting for the next dictation cycle.
      const layoutChanged =
        current.overlayStyle !== next.overlayStyle ||
        current.overlaySize !== next.overlaySize;
      if (layoutChanged) {
        ipc
          .setOverlayLayout(next.overlayStyle, next.overlaySize)
          .catch(() => {});
      }
    } catch (error) {
      set({ settings: current, error: String(error) });
      throw error;
    }
  },
}));

