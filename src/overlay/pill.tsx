import { motion } from "framer-motion";
import { useEffect, useState } from "react";
import { Square, X } from "lucide-react";
import { useSettings } from "@/hooks/use-settings";
import { formatDrafterName, formatWhisperName } from "@/lib/engine-format";
import { ipc } from "@/lib/tauri";
import { useOverlayStore } from "@/stores/overlay-store";
import type { OverlayMode, OverlayState } from "@/stores/overlay-store";

const WAVE_BARS = [5, 8, 11, 7, 14, 9, 6, 12, 8, 5];
const SPEAKING_THRESHOLD = 0.02;
const CAPSULE_BAR_COUNT = 40;

/**
 * Legacy bar-height calculator. Kept verbatim so the pre-Glacier
 * `pill.test.ts` contract continues to hold. The capsule no longer
 * consumes its output directly — it renders 40 bars driven by the
 * smoothed `voiceLevel` via a single CSS variable — but the math
 * is preserved as the canonical reference for amplitude shaping.
 */
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

export function formatElapsed(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) ms = 0;
  const total = Math.floor(ms);
  const minutes = Math.floor(total / 60_000);
  const seconds = Math.floor((total % 60_000) / 1000);
  const tenths = Math.floor((total % 1000) / 100);
  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}.${tenths}`;
}

export function OverlayPill({ state, mode }: { state: OverlayState; mode: OverlayMode }) {
  const busy = state === "transcribing";
  const recording = state === "recording";
  const commandMode = mode === "command";
  const dataMode = commandMode ? "drafter" : "stt";
  const { settings } = useSettings();

  const engineName = commandMode
    ? formatDrafterName(settings?.emailDraftEngine, settings?.emailDraftModel)
    : formatWhisperName(settings?.whisperModel);

  const level = useOverlayStore((s) => s.level);
  const recordingStartedAt = useOverlayStore((s) => s.recordingStartedAt);
  const voiceLevel = recording ? Math.max(0, Math.min(1, level)) : 0;

  // Drive the wave amplitude via a single CSS variable; the per-bar
  // stagger lives in tailwind.css keyframes (no per-frame React work).
  const capAmp = recording
    ? voiceLevel > SPEAKING_THRESHOLD
      ? 0.4 + voiceLevel * 0.9
      : 0.32
    : busy
      ? 0.6
      : 0.3;

  const elapsedLabel = useElapsedLabel(recordingStartedAt, state);

  const primaryAction = busy
    ? () => void ipc.cancelRecording().catch(() => {})
    : () => void ipc.toggleRecording().catch(() => {});

  return (
    <motion.div
      initial={{ opacity: 0, y: 6, scale: 0.96 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      transition={{ duration: 0.16 }}
      className="flex w-[520px] flex-col items-center gap-1.5"
    >
      <div
        className="veyra-capsule grid w-full items-center gap-3.5 px-2.5 pl-3.5"
        style={{
          gridTemplateColumns: "12px auto 1fr 56px 36px",
          ["--cap-amp" as string]: capAmp.toFixed(3),
        }}
        data-mode={dataMode}
        data-state={state}
        role="status"
        aria-live="polite"
        aria-label={
          busy
            ? `${commandMode ? "Email Drafter" : "Speech to Text"} transcribing`
            : recording
              ? `${commandMode ? "Email Drafter" : "Speech to Text"} recording`
              : `${commandMode ? "Email Drafter" : "Speech to Text"} idle`
        }
      >
        <span className="veyra-capsule-led" aria-hidden="true" />
        <span className="flex flex-col leading-tight">
          <span
            className="font-mono text-[10px] uppercase tracking-[0.2em] font-medium"
            style={{ color: "var(--accent-deep)" }}
          >
            {commandMode ? "Drafter" : "STT"}
          </span>
          <span className="text-[12.5px] font-semibold tracking-[-0.005em] text-[var(--ink,#0c111c)]">
            {busy ? "Transcribing…" : recording ? engineName : "Listening…"}
          </span>
        </span>
        <div className="veyra-capsule-wave" aria-hidden="true">
          {Array.from({ length: CAPSULE_BAR_COUNT }).map((_, i) => (
            <i key={i} />
          ))}
        </div>
        <span
          className="text-right font-mono text-[12px] tabular-nums tracking-[0.04em]"
          style={{ color: busy ? "var(--slate-400, #8b95a6)" : "var(--ink, #0c111c)" }}
        >
          {elapsedLabel}
        </span>
        <button
          type="button"
          onClick={primaryAction}
          className="grid h-9 w-9 place-items-center rounded-full bg-[#0c111c] text-white shadow-[inset_0_1px_0_rgb(255_255_255_/_0.15),0_4px_10px_-2px_rgb(12_17_28_/_0.45)] transition-colors hover:bg-[#1a212e] focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-400"
          aria-label={busy ? "Cancel transcription" : "Stop recording"}
        >
          {busy ? (
            <X className="h-3 w-3" strokeWidth={2} />
          ) : (
            <Square className="h-3 w-3 fill-current" strokeWidth={0} />
          )}
        </button>
      </div>
    </motion.div>
  );
}

function useElapsedLabel(
  startedAt: number | null,
  state: OverlayState,
): string {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    if (state !== "recording" || startedAt == null) return;
    const id = window.setInterval(() => setNow(Date.now()), 100);
    return () => window.clearInterval(id);
  }, [startedAt, state]);

  if (startedAt == null) return "00:00.0";
  if (state === "transcribing") return formatElapsed(Math.max(0, now - startedAt));
  if (state !== "recording") return "00:00.0";
  return formatElapsed(now - startedAt);
}
