import { useEffect } from "react";
import { useSettingsStore } from "@/stores/settings-store";

export function useSettings() {
  const settings = useSettingsStore((s) => s.settings);
  const loading = useSettingsStore((s) => s.loading);
  const error = useSettingsStore((s) => s.error);
  const load = useSettingsStore((s) => s.load);
  const save = useSettingsStore((s) => s.save);

  useEffect(() => {
    if (!settings && !loading && !error) {
      void load();
    }
  }, [settings, loading, error, load]);

  return {
    settings,
    loading,
    error,
    reload: load,
    update: save,
  };
}
