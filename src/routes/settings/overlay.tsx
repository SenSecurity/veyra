import { Check, Eye, EyeOff } from "lucide-react";
import { useState } from "react";
import { useSettings } from "@/hooks/use-settings";
import { ipc } from "@/lib/tauri";
import { cn } from "@/lib/utils";
import { SettingsPanel } from "./general";
import type { OverlaySize, OverlayStyle } from "@/types/settings";

const STYLE_OPTIONS: {
  value: OverlayStyle;
  title: string;
  desc: string;
  preview: React.ReactNode;
}[] = [
  {
    value: "capsule",
    title: "Capsule",
    desc: "Wide light-glass pill anchored bottom-center. Live waveform + timer at a glance.",
    preview: <CapsulePreview />,
  },
  {
    value: "orb",
    title: "Halo Orb",
    desc: "Compact floating orb with concentric rings rippling out as you speak.",
    preview: <OrbPreview />,
  },
];

const SIZES: { value: OverlaySize; label: string; capsule: string; orb: string }[] = [
  { value: "small", label: "Small", capsule: "460 × 92", orb: "200 × 168" },
  { value: "medium", label: "Medium", capsule: "560 × 96", orb: "240 × 200" },
  { value: "large", label: "Large", capsule: "680 × 104", orb: "300 × 248" },
];

export function SettingsOverlayRoute() {
  const { settings, update, error, reload } = useSettings();
  const [previewError, setPreviewError] = useState<string | null>(null);

  if (!settings) {
    return (
      <SettingsPanel title="Overlay" muted={error ?? "Loading settings."}>
        {error ? (
          <button type="button" className="veyra-select" onClick={() => void reload()}>
            Retry
          </button>
        ) : null}
      </SettingsPanel>
    );
  }

  const currentStyle = settings.overlayStyle;
  const currentSize = settings.overlaySize;

  async function preview(
    mode: "dictation" | "command",
    recordingState: "Recording" | "Transcribing",
  ) {
    setPreviewError(null);
    try {
      await ipc.previewOverlay(currentStyle, currentSize, mode, recordingState);
    } catch (error) {
      setPreviewError(String(error));
    }
  }

  async function hidePreview() {
    setPreviewError(null);
    try {
      await ipc.hideOverlayPreview();
    } catch (error) {
      setPreviewError(String(error));
    }
  }

  return (
    <SettingsPanel
      title="Overlay"
      muted={error ?? "Pick the floating recording overlay's shape and size."}
    >
      {/* Style cards */}
      <div className="grid gap-3 sm:grid-cols-2">
        {STYLE_OPTIONS.map((opt) => {
          const active = currentStyle === opt.value;
          return (
            <button
              key={opt.value}
              type="button"
              onClick={() => void update({ overlayStyle: opt.value })}
              className={cn(
                "group relative flex flex-col gap-3 rounded-xl border bg-white p-4 text-left shadow-sm transition-all",
                "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2",
                active
                  ? "border-[var(--cyan-deep)] shadow-[0_0_0_1px_var(--cyan-deep),0_8px_22px_-12px_var(--halo)]"
                  : "border-border/70 hover:border-border",
              )}
              aria-pressed={active}
            >
              <span
                className={cn(
                  "absolute right-3 top-3 grid h-5 w-5 place-items-center rounded-full border text-[var(--cyan-deep)] transition-opacity",
                  active
                    ? "border-[var(--cyan-deep)] bg-[var(--ice-50)] opacity-100"
                    : "border-border opacity-0",
                )}
                aria-hidden="true"
              >
                <Check className="h-3 w-3" strokeWidth={2} />
              </span>
              <div
                className="grid h-20 place-items-center rounded-lg bg-[linear-gradient(180deg,#1c2535_0%,#0a0e15_70%,#06080d_100%)] p-3"
                aria-hidden="true"
              >
                {opt.preview}
              </div>
              <div className="flex flex-col gap-1">
                <span className="text-sm font-semibold tracking-[-0.005em] text-foreground">
                  {opt.title}
                </span>
                <span className="text-xs text-muted-foreground">{opt.desc}</span>
              </div>
            </button>
          );
        })}
      </div>

      {/* Size selector */}
      <div className="flex flex-col gap-2">
        <span className="text-sm font-medium text-foreground">Size</span>
        <div
          className="inline-flex gap-1 self-start rounded-xl border border-border bg-white p-1"
          role="radiogroup"
          aria-label="Overlay size"
        >
          {SIZES.map((s) => {
            const active = currentSize === s.value;
            const dims = currentStyle === "orb" ? s.orb : s.capsule;
            return (
              <button
                key={s.value}
                type="button"
                role="radio"
                aria-checked={active}
                onClick={() => void update({ overlaySize: s.value })}
                className={cn(
                  "flex flex-col gap-0.5 rounded-lg px-4 py-2 text-center transition-colors",
                  "focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2",
                  active
                    ? "bg-[var(--ice-50)] text-foreground shadow-[inset_0_0_0_1px_var(--cyan-deep)]"
                    : "text-muted-foreground hover:bg-frost hover:text-foreground",
                )}
              >
                <span className="text-sm font-medium">{s.label}</span>
                <span className="font-mono text-[0.6rem] tracking-[0.06em] text-muted-foreground">
                  {dims}
                </span>
              </button>
            );
          })}
        </div>
        <p className="text-xs text-muted-foreground">
          Changes apply to the next dictation; the live overlay window resizes immediately.
        </p>
      </div>

      <div className="flex flex-col gap-3 rounded-xl border border-border/70 bg-white/70 p-3 shadow-sm">
        <div className="flex flex-col gap-1">
          <span className="text-sm font-medium text-foreground">Preview</span>
          <span className="text-xs text-muted-foreground">
            Uses the real desktop overlay window. No audio is recorded.
          </span>
        </div>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="veyra-select inline-flex w-auto items-center gap-2 px-3"
            onClick={() => void preview("dictation", "Recording")}
          >
            <Eye className="h-3.5 w-3.5" />
            Preview STT
          </button>
          <button
            type="button"
            className="veyra-select inline-flex w-auto items-center gap-2 px-3"
            onClick={() => void preview("command", "Recording")}
          >
            <Eye className="h-3.5 w-3.5" />
            Preview Drafter
          </button>
          <button
            type="button"
            className="veyra-select inline-flex w-auto items-center gap-2 px-3"
            onClick={() => void preview("dictation", "Transcribing")}
          >
            <Eye className="h-3.5 w-3.5" />
            Preview Transcribing
          </button>
          <button
            type="button"
            className="veyra-select inline-flex w-auto items-center gap-2 px-3"
            onClick={() => void hidePreview()}
          >
            <EyeOff className="h-3.5 w-3.5" />
            Hide preview
          </button>
        </div>
        {previewError ? (
          <p className="text-xs font-medium text-destructive">{previewError}</p>
        ) : null}
      </div>
    </SettingsPanel>
  );
}

// ---------- preview sketches (inline SVG, intentionally tiny) ----------

function CapsulePreview() {
  return (
    <svg viewBox="0 0 220 60" className="h-full w-full" aria-hidden="true">
      <defs>
        <linearGradient id="cap-prev-cyan" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#a4e6ff" />
          <stop offset="50%" stopColor="#2bc7ff" />
          <stop offset="100%" stopColor="#0a8bc4" />
        </linearGradient>
      </defs>
      <rect
        x="6"
        y="14"
        width="208"
        height="32"
        rx="16"
        fill="rgba(255,255,255,0.92)"
        stroke="rgba(255,255,255,0.85)"
      />
      <circle cx="20" cy="30" r="3" fill="#2bc7ff" />
      {Array.from({ length: 26 }).map((_, i) => (
        <rect
          key={i}
          x={36 + i * 5}
          y={22 + (i % 4)}
          width={2}
          height={16 - (i % 4) * 3}
          rx={1}
          fill="url(#cap-prev-cyan)"
        />
      ))}
      <text x="180" y="33" fontSize="7" fontFamily="monospace" fill="#0c111c">
        00:14
      </text>
      <circle cx="206" cy="30" r="6" fill="#0c111c" />
    </svg>
  );
}

function OrbPreview() {
  return (
    <svg viewBox="0 0 80 80" className="h-full w-full" aria-hidden="true">
      <defs>
        <linearGradient id="orb-prev-vbar" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#a4e6ff" />
          <stop offset="50%" stopColor="#2bc7ff" />
          <stop offset="100%" stopColor="#0a8bc4" />
        </linearGradient>
      </defs>
      <circle cx="40" cy="40" r="36" fill="none" stroke="#2bc7ff" strokeOpacity="0.25" />
      <circle cx="40" cy="40" r="28" fill="none" stroke="#2bc7ff" strokeOpacity="0.4" />
      <rect
        x="20"
        y="20"
        width="40"
        height="40"
        rx="9"
        fill="url(#orb-bg)"
      />
      <defs>
        <radialGradient id="orb-bg" cx="30%" cy="20%" r="80%">
          <stop offset="0%" stopColor="#3a4658" />
          <stop offset="60%" stopColor="#1a212e" />
          <stop offset="100%" stopColor="#07090d" />
        </radialGradient>
      </defs>
      <g fill="url(#orb-prev-vbar)">
        <rect x="25" y="28" width="1.6" height="18" rx="0.8" />
        <rect x="28" y="30" width="1.6" height="16" rx="0.8" />
        <rect x="31" y="32" width="1.6" height="14" rx="0.8" />
        <rect x="34" y="34" width="1.6" height="12" rx="0.8" />
        <rect x="37" y="36" width="1.6" height="10" rx="0.8" />
        <rect x="40" y="36" width="1.6" height="10" rx="0.8" />
        <rect x="43" y="34" width="1.6" height="12" rx="0.8" />
        <rect x="46" y="32" width="1.6" height="14" rx="0.8" />
        <rect x="49" y="30" width="1.6" height="16" rx="0.8" />
        <rect x="52" y="28" width="1.6" height="18" rx="0.8" />
      </g>
    </svg>
  );
}
