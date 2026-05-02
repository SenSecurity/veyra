import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { toast } from "sonner";

export function useLiveEvents() {
  useEffect(() => {
    const unsubs = [
      listen("settings:migrated", () => toast.info("Settings migrated")),
      listen<{ from: string; to: string }>("settings:model-remapped", (e) =>
        toast.info(`Model remapped: ${e.payload.from} -> ${e.payload.to}`),
      ),
      listen("settings:needs-groq-key", () =>
        toast.warning("Groq key needs to be re-entered"),
      ),
      listen<string>("settings:migration-failed", (e) =>
        toast.error(`Settings migration failed: ${e.payload}`),
      ),
      listen<{ rowId: number }>("transcription:new", (e) =>
        toast.success(`Transcription saved #${e.payload.rowId}`),
      ),
      listen<{ modelSize: string; downloaded: number; total: number }>(
        "model:download:progress",
        (e) => {
          const id = `model-${e.payload.modelSize}`;
          const pct =
            e.payload.total > 0
              ? Math.round((e.payload.downloaded / e.payload.total) * 100)
              : 0;
          if (e.payload.total > 0 && e.payload.downloaded >= e.payload.total) {
            toast.dismiss(id);
            return;
          }
          toast.loading(`Downloading ${e.payload.modelSize}: ${pct}%`, {
            id,
          });
        },
      ),
    ];

    return () => {
      for (const unsub of unsubs) {
        void unsub.then((fn) => fn()).catch(() => {});
      }
    };
  }, []);
}

