import { motion } from "framer-motion";
import { useEffect, useMemo, useState } from "react";
import { Square, X } from "lucide-react";
import { ipc } from "@/lib/tauri";
import { useOverlayStore } from "@/stores/overlay-store";
import type { OverlayMode, OverlayState } from "@/stores/overlay-store";

const WAVE_BARS = [5, 8, 11, 7, 14, 9, 6, 12, 8, 5];
const SPEAKING_THRESHOLD = 0.02;

export function calculateWaveBarHeights({
  state,
  voiceLevel,
  phase,
}: {
  state: OverlayState;
  voiceLevel: number;
  phase: number;
}) {
  const speaking = voiceLevel > SPEAKING_THRESHOLD;
  return WAVE_BARS.map((height, index) => {
    if (state !== "recording") return height;
    if (!speaking) return Math.max(3, Math.round(height * 0.42));

    const wave = (Math.sin(phase + index * 0.88) + 1) / 2;
    const flutter = (Math.sin(phase * 1.7 + index * 0.46) + 1) / 2;
    const lift = voiceLevel * (0.9 + wave * 1.45 + flutter * 0.45);
    return Math.min(20, Math.max(3, Math.round(height * (0.35 + lift))));
  });
}

export function OverlayPill({ state, mode }: { state: OverlayState; mode: OverlayMode }) {
  const busy = state === "transcribing";
  const commandMode = mode === "command";
  const modeLabel = commandMode ? "Email Drafter" : "Speech to Text";
  const level = useOverlayStore((s) => s.level);
  const voiceLevel = state === "recording" ? Math.max(0, Math.min(1, level)) : 0;
  const speaking = voiceLevel > SPEAKING_THRESHOLD;
  const [phase, setPhase] = useState(0);

  useEffect(() => {
    if (state !== "recording" || !speaking) {
      setPhase(0);
      return;
    }

    let frame = 0;
    let previous = performance.now();
    const animate = (now: number) => {
      const delta = Math.min(48, now - previous);
      previous = now;
      setPhase((current) => current + delta * (0.018 + voiceLevel * 0.024));
      frame = window.requestAnimationFrame(animate);
    };

    frame = window.requestAnimationFrame(animate);
    return () => window.cancelAnimationFrame(frame);
  }, [speaking, state, voiceLevel]);

  const barHeights = useMemo(
    () => calculateWaveBarHeights({ state, voiceLevel, phase }),
    [phase, state, voiceLevel],
  );

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
        className="flex h-8 items-center gap-2 rounded-full border border-white/10 bg-zinc-950/95 px-1.5 text-white backdrop-blur"
        style={{ boxShadow: "0 10px 24px rgba(0,0,0,0.32)" }}
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
            WAVE_BARS.map((height, index) => (
              <motion.span
                key={`${height}-${index}`}
                className="block w-0.5 rounded-full bg-white"
                animate={{
                  height: barHeights[index],
                  opacity: state === "recording" ? 0.48 + voiceLevel * 0.52 : 1,
                }}
                transition={{
                  duration: 0.055,
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
