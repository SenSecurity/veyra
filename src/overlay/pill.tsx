import { motion } from "framer-motion";
import { Square, X } from "lucide-react";
import { ipc } from "@/lib/tauri";
import { useOverlayStore } from "@/stores/overlay-store";
import type { OverlayMode, OverlayState } from "@/stores/overlay-store";

export function OverlayPill({ state, mode }: { state: OverlayState; mode: OverlayMode }) {
  const busy = state === "transcribing";
  const commandMode = mode === "command";
  const modeLabel = commandMode ? "Email Drafter" : "Speech to Text";
  const level = useOverlayStore((s) => s.level);
  const bars = [6, 9, 12, 8, 14, 10, 7, 12, 9, 6];
  const voiceLevel = state === "recording"
    ? Math.min(1, Math.pow(Math.max(0, (level - 0.008) / 0.16), 0.72))
    : 0;

  return (
    <motion.div
      initial={{ opacity: 0, y: 6, scale: 0.96 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      transition={{ duration: 0.16 }}
      className="flex flex-col items-center gap-1 text-white"
    >
      <div
        className={
          commandMode
            ? "rounded-full border border-amber-200/40 bg-amber-300 px-2.5 py-0.5 text-[10px] font-semibold uppercase leading-none tracking-wide text-zinc-950 shadow-sm"
            : "rounded-full border border-sky-200/40 bg-sky-300 px-2.5 py-0.5 text-[10px] font-semibold uppercase leading-none tracking-wide text-zinc-950 shadow-sm"
        }
      >
        {modeLabel}
      </div>
      <div
        className="flex h-8 items-center gap-2 rounded-full border border-white/10 bg-zinc-950/95 px-1.5 text-white"
        style={{ boxShadow: "0 8px 20px rgba(0,0,0,0.28)" }}
      >
        <button
          type="button"
          disabled={busy}
          onClick={() => void ipc.cancelRecording().catch(() => {})}
          className="flex h-5 w-5 items-center justify-center rounded-full bg-zinc-700 text-zinc-200 hover:bg-zinc-600 disabled:opacity-50"
          aria-label="Cancel recording"
        >
          <X className="h-3.5 w-3.5" />
        </button>
        <div className="flex h-5 min-w-20 items-center justify-center gap-0.5 overflow-hidden" aria-hidden="true">
          {busy ? (
            <span className="typr-transcribing-label text-[11px] font-medium leading-none text-zinc-100">
              Transcribing
            </span>
          ) : (
            bars.map((height, index) => (
              <motion.span
                key={`${height}-${index}`}
                className="w-0.5 rounded-full bg-white"
                animate={{
                  height:
                    state === "recording"
                      ? Math.min(20, Math.max(4, Math.round(height * (0.42 + voiceLevel * 1.25))))
                      : height,
                  opacity: 1,
                }}
                transition={{
                  duration: 0.08,
                }}
              />
            ))
          )}
        </div>
        <button
          type="button"
          disabled={busy}
          onClick={() => void ipc.toggleRecording().catch(() => {})}
          className="flex h-5 w-5 items-center justify-center rounded-full bg-rose-500 text-white hover:bg-rose-400 disabled:bg-amber-500"
          aria-label="Stop recording"
        >
          <Square className="h-2.5 w-2.5 fill-current" />
        </button>
      </div>
    </motion.div>
  );
}
