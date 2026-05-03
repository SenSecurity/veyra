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
  const live = state !== "Ready";
  return (
    <div className="inline-flex h-6 items-center gap-2 rounded-md border border-white/10 bg-white/8 px-2 text-[0.7rem] font-medium text-zinc-100 shadow-sm">
      <span
        className={
          live
            ? "h-1.5 w-1.5 rounded-full bg-sky-500 shadow-[0_0_0_3px_rgb(14_165_233_/_0.14)]"
            : "h-1.5 w-1.5 rounded-full bg-emerald-500 shadow-[0_0_0_3px_rgb(16_185_129_/_0.14)]"
        }
      />
      {label}
    </div>
  );
}
