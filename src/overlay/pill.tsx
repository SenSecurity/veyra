import { AnimatePresence, motion } from "framer-motion";
import { useEffect, useState } from "react";
import { Square, X } from "lucide-react";
import { useSettings } from "@/hooks/use-settings";
import { formatDrafterName, formatWhisperName } from "@/lib/engine-format";
import { ipc } from "@/lib/tauri";
import { useOverlayStore } from "@/stores/overlay-store";
import type { OverlayMode, OverlayState } from "@/stores/overlay-store";
import type { OverlaySize } from "@/types/settings";
import { formatElapsed, useElapsedLabel } from "./use-elapsed-label";

export { formatElapsed };

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

const CAPSULE_WIDTHS: Record<OverlaySize, number> = {
  small: 420,
  medium: 520,
  large: 640,
};

export function OverlayPill({
  state,
  mode,
  size = "medium",
}: {
  state: OverlayState;
  mode: OverlayMode;
  size?: OverlaySize;
}) {
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

  const dictationHotkey = settings?.hotkey || "F24";
  const commandHotkey = settings?.commandHotkey || "Pause";
  const hotkey = commandMode ? commandHotkey : dictationHotkey;
  const hintVerb = commandMode ? "draft" : "stop";
  const showHint = useHintVisibility(recordingStartedAt, state);

  const primaryAction = busy
    ? () => void ipc.cancelRecording().catch(() => {})
    : () => void ipc.toggleRecording().catch(() => {});

  return (
    <motion.div
      initial={{ opacity: 0, y: 6, scale: 0.96 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      transition={{ duration: 0.16 }}
      className="flex flex-col items-center gap-1.5"
      style={{ width: CAPSULE_WIDTHS[size] }}
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

      <AnimatePresence>
        {showHint ? (
          <motion.div
            key="hotkey-hint"
            initial={{ opacity: 0, y: -2 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.18 }}
            className="veyra-capsule-hint"
          >
            tap <kbd>{hotkey}</kbd> to {hintVerb}
          </motion.div>
        ) : null}
      </AnimatePresence>
    </motion.div>
  );
}

export const HINT_DURATION_MS = 600;

export function useHintVisibility(
  startedAt: number | null,
  state: OverlayState,
): boolean {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (state !== "recording" || startedAt == null) {
      setVisible(false);
      return;
    }
    const elapsed = Date.now() - startedAt;
    if (elapsed >= HINT_DURATION_MS) {
      setVisible(false);
      return;
    }
    setVisible(true);
    const id = window.setTimeout(
      () => setVisible(false),
      HINT_DURATION_MS - elapsed,
    );
    return () => window.clearTimeout(id);
  }, [startedAt, state]);

  return visible;
}

