import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useSettings } from "@/hooks/use-settings";
import { cn } from "@/lib/utils";
import { ipc } from "@/lib/tauri";
import type { RecordingState } from "@/types/ipc";

/**
 * Twin-pill engine badge for the window titlebar.
 *
 * Surfaces both Veyra engines (STT + Drafter) as one rounded container
 * split by a hairline divider, each with a colored LED, role caption,
 * and active engine name. Names read from useSettings; falls back to
 * mockup defaults while settings are loading.
 *
 * The STT segment also reflects the live recording state — its LED
 * pulses cyan while idle and switches to a deeper hue while a session
 * is in flight.
 */
export function EngineBadge() {
  const { settings } = useSettings();
  const [recording, setRecording] = useState<RecordingState>("Ready");

  useEffect(() => {
    ipc.getRecordingState().then(setRecording).catch(() => {});
    const un = listen<RecordingState>("recording-state", (e) =>
      setRecording(e.payload),
    );
    return () => {
      un.then((fn) => fn()).catch(() => {});
    };
  }, []);

  const sttName = formatWhisperName(settings?.whisperModel);
  const drafterName = formatDrafterName(
    settings?.emailDraftEngine,
    settings?.emailDraftModel,
  );
  const live = recording !== "Ready";

  return (
    <div
      className={cn(
        "inline-flex h-6 items-center overflow-hidden rounded-lg border border-border/70 bg-white/85 text-[0.7rem] font-medium text-foreground/80",
        "shadow-[inset_0_1px_0_rgb(255_255_255_/_0.9),0_1px_2px_rgb(12_17_28_/_0.05)]",
      )}
    >
      <span className="inline-flex items-center gap-1.5 px-2.5">
        <span
          className={cn(
            "h-1.5 w-1.5 rounded-full",
            live
              ? "bg-cyan-500 shadow-[0_0_6px_rgb(43_199_255_/_0.65)]"
              : "bg-cyan-400 shadow-[0_0_6px_rgb(43_199_255_/_0.45)]",
          )}
          aria-hidden="true"
        />
        <span className="font-mono text-[0.6rem] tracking-[0.18em] uppercase text-muted-foreground">
          STT
        </span>
        <span className="text-foreground/85">{sttName}</span>
      </span>
      <span className="h-3 w-px bg-border/60" aria-hidden="true" />
      <span className="inline-flex items-center gap-1.5 px-2.5">
        <span
          className="h-1.5 w-1.5 rounded-full bg-amber-400 shadow-[0_0_6px_rgb(255_180_84_/_0.55)]"
          aria-hidden="true"
        />
        <span className="font-mono text-[0.6rem] tracking-[0.18em] uppercase text-muted-foreground">
          Drafter
        </span>
        <span className="text-foreground/85">{drafterName}</span>
      </span>
    </div>
  );
}

function formatWhisperName(raw: string | undefined): string {
  if (!raw) return "Whisper · Turbo";
  const lower = raw.toLowerCase();
  if (lower.includes("large-v3-turbo") || lower === "turbo") {
    return "Whisper · Turbo";
  }
  if (lower.includes("medium")) return "Whisper · Medium";
  if (lower.includes("small")) return "Whisper · Small";
  if (lower.includes("base")) return "Whisper · Base";
  return `Whisper · ${raw.replace(/^ggml-|\.bin$/g, "")}`;
}

function formatDrafterName(
  engine: "ollama" | "groq" | undefined,
  model: string | undefined,
): string {
  if (engine === "groq") {
    return model ? `Groq · ${prettyModel(model)}` : "Groq";
  }
  return model ? prettyModel(model) : "Llama · 3.2 · 1B";
}

function prettyModel(raw: string): string {
  // Split ollama identifiers like "llama3.2:1b" into segments, splitting
  // longer mixed-case segments at letter/digit boundaries while keeping
  // short tags ("1b") together so they uppercase cleanly into "1B".
  const segments = raw.split(/[:/\-_]/).filter(Boolean);
  const tokens = segments.flatMap((s) =>
    s.length >= 4 ? splitAtLetterDigitBoundary(s) : [s],
  );
  if (tokens.length === 0) return raw;
  return tokens
    .map((p) => (p.length <= 3 ? p.toUpperCase() : capitalize(p)))
    .join(" · ");
}

function splitAtLetterDigitBoundary(s: string): string[] {
  return s.split(/(?<=[a-zA-Z])(?=\d)|(?<=\d)(?=[a-zA-Z])/);
}

function capitalize(s: string): string {
  if (!s) return s;
  return s.charAt(0).toUpperCase() + s.slice(1);
}
