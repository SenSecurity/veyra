import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { ipc } from "@/lib/tauri";
import type { RecordingState } from "@/types/ipc";

// Subscribes to the `recording-state` Tauri event and renders a small text
// pill. Phase 4 / Task 11 will swap this for animated tinted variants.
export function RecordingPill() {
  const [state, setState] = useState<RecordingState>("Ready");

  useEffect(() => {
    ipc.getRecordingState().then(setState).catch(() => {});
    const un = listen<RecordingState>("recording-state", (e) =>
      setState(e.payload),
    );
    return () => {
      un.then((fn) => fn()).catch(() => {});
    };
  }, []);

  const label = state.toLowerCase();
  return (
    <div className="rounded-md border border-border bg-card px-2 py-1 text-xs font-medium text-foreground">
      {label}
    </div>
  );
}
