import { AnimatePresence, motion } from "framer-motion";
import { useSettings } from "@/hooks/use-settings";
import { ipc } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { useOverlayStore } from "@/stores/overlay-store";
import type { OverlayMode, OverlayState } from "@/stores/overlay-store";
import type { OverlaySize } from "@/types/settings";
import { BrandMark } from "@/components/brand-mark";
import { useElapsedLabel } from "./use-elapsed-label";
import { useHintVisibility } from "./pill";

interface SizeSpec {
  orb: number;
  ring1: number;
  ring2: number;
  ring3: number;
}

const SIZE_SPECS: Record<OverlaySize, SizeSpec> = {
  smaller: { orb: 34, ring1: 44, ring2: 54, ring3: 64 },
  small: { orb: 42, ring1: 58, ring2: 72, ring3: 86 },
  medium: { orb: 52, ring1: 74, ring2: 92, ring3: 110 },
  large: { orb: 64, ring1: 92, ring2: 116, ring3: 140 },
};

/**
 * Halo Orb overlay (mockup overlay-03-halo-orb.html). 96 px squircle with
 * the V brand mark inside, three concentric rings that ripple outward in
 * sequence while recording, a single dashed shimmer ring while
 * transcribing, no rings when idle. Tone follows the active engine
 * (cyan for STT, spark amber for Drafter). A small light-glass timer
 * chip floats below the orb showing the elapsed time. The optional
 * hover-bubble from the mockup is not rendered in v1 — there is no
 * streaming transcript IPC channel yet.
 */
export function HaloOrb({
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
  const dataMode = mode === "command" ? "drafter" : "stt";
  const recordingStartedAt = useOverlayStore((s) => s.recordingStartedAt);
  const elapsedLabel = useElapsedLabel(recordingStartedAt, state);
  const { settings } = useSettings();
  const dictationHotkey = settings?.hotkey || "F24";
  const commandHotkey = settings?.commandHotkey || "Pause";
  const hotkey = mode === "command" ? commandHotkey : dictationHotkey;
  const hintVerb = mode === "command" ? "draft" : "stop";
  const showHint = useHintVisibility(recordingStartedAt, state);

  const spec = SIZE_SPECS[size];
  const showChip = size !== "smaller";

  const primaryAction = busy
    ? () => void ipc.cancelRecording().catch(() => {})
    : () => void ipc.toggleRecording().catch(() => {});

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.96 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.16 }}
      className="flex flex-col items-center gap-1.5 overflow-visible"
    >
      <div
        className="veyra-orb-wrap relative grid place-items-center"
        data-mode={dataMode}
        data-state={state}
        data-size={size}
        style={{ width: spec.ring3, height: spec.ring3 }}
      >
        {recording ? (
          <>
            <span
              className="veyra-orb-ring"
              style={
                {
                  width: spec.ring1,
                  height: spec.ring1,
                  animationDelay: "0s",
                } as React.CSSProperties
              }
              aria-hidden="true"
            />
            <span
              className="veyra-orb-ring"
              style={
                {
                  width: spec.ring2,
                  height: spec.ring2,
                  animationDelay: "0.6s",
                } as React.CSSProperties
              }
              aria-hidden="true"
            />
            <span
              className="veyra-orb-ring"
              style={
                {
                  width: spec.ring3,
                  height: spec.ring3,
                  animationDelay: "1.2s",
                } as React.CSSProperties
              }
              aria-hidden="true"
            />
          </>
        ) : null}
        {busy ? (
          <span
            className="veyra-orb-shimmer"
            style={{ width: spec.ring1, height: spec.ring1 }}
            aria-hidden="true"
          />
        ) : null}
        <button
          type="button"
          onClick={primaryAction}
          className={cn(
            "veyra-orb relative z-10 grid place-items-center rounded-[22%]",
            "transition-transform hover:scale-[1.03] focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-4",
          )}
          style={{ width: spec.orb, height: spec.orb }}
          aria-label={
            busy ? "Cancel transcription" : recording ? "Stop recording" : "Veyra overlay"
          }
        >
          <BrandMark className="h-full w-full rounded-[22%]" />
        </button>
      </div>

      {showChip ? (
      <div
        className="veyra-orb-chip"
        role="status"
        aria-live="polite"
      >
        <span className={cn("veyra-orb-led", busy && "veyra-orb-led-busy")} aria-hidden="true" />
        <span className="font-mono text-[8px] tracking-[0.16em] uppercase text-muted-foreground">
          {busy ? "working" : recording ? "rec" : "ready"}
        </span>
        <strong className="font-mono text-[10px] tabular-nums text-foreground">
          {busy ? "transcribing…" : elapsedLabel}
        </strong>
      </div>
      ) : null}

      <AnimatePresence>
        {showHint ? (
          <motion.div
            key="orb-hint"
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
