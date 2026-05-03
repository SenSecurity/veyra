import { cn } from "@/lib/utils";

/**
 * Veyra brand mark.
 *
 * Inline SVG implementation of the Glacier logo (V-shaped cyan waveform
 * over a dark squircle, with a warm spark at the base). Use only in the
 * window titlebar (~22px) and the Home hero (~88px) — the rest of the
 * app uses neutral lucide icons. Visual contract:
 * docs/mockups/08-glacier-veyra.html (#veyra-v symbol).
 *
 * The original PNG asset (src/assets/veyra-icon.png) is intentionally
 * preserved on disk for the Windows installer / .ico pipeline; this
 * component no longer imports it.
 */
export function BrandMark({
  className,
  svgClassName,
}: {
  className?: string;
  svgClassName?: string;
}) {
  return (
    <span
      className={cn(
        "veyra-brand-mark relative inline-flex shrink-0 items-center justify-center overflow-hidden rounded-[22%]",
        "bg-[radial-gradient(120%_120%_at_30%_18%,#3a4658_0%,#1a212e_40%,#07090d_90%)]",
        "shadow-[inset_0_1.5px_0_rgb(255_255_255_/_0.18),inset_0_-2px_0_rgb(255_255_255_/_0.04),inset_0_0_0_1px_rgb(255_255_255_/_0.08),0_6px_18px_-6px_rgb(0_0_0_/_0.55)]",
        "before:pointer-events-none before:absolute before:inset-[4%_4%_60%_4%]",
        "before:rounded-[22%_22%_50%_50%/22%_22%_100%_100%]",
        "before:bg-[linear-gradient(180deg,rgb(255_255_255_/_0.32)_0%,rgb(255_255_255_/_0.04)_70%,transparent_100%)]",
        className,
      )}
      aria-hidden="true"
    >
      <svg
        viewBox="0 0 100 100"
        role="presentation"
        focusable="false"
        className={cn("absolute inset-0 h-full w-full", svgClassName)}
      >
        <defs>
          <linearGradient id="veyra-vbar" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#a4e6ff" />
            <stop offset="50%" stopColor="#2bc7ff" />
            <stop offset="100%" stopColor="#0a8bc4" />
          </linearGradient>
          <radialGradient id="veyra-spark" cx="50%" cy="100%" r="60%">
            <stop offset="0%" stopColor="#fff1d0" />
            <stop offset="40%" stopColor="#ffb454" />
            <stop offset="100%" stopColor="#ff8a1f" stopOpacity="0" />
          </radialGradient>
        </defs>

        {/* left wing (descending V) */}
        <g fill="url(#veyra-vbar)">
          <rect x="14" y="20" width="3" height="42" rx="1.5" />
          <rect x="20" y="24" width="3" height="38" rx="1.5" />
          <rect x="26" y="30" width="3" height="32" rx="1.5" />
          <rect x="32" y="36" width="3" height="26" rx="1.5" />
          <rect x="38" y="42" width="3" height="20" rx="1.5" />
          <rect x="44" y="48" width="3" height="14" rx="1.5" />
        </g>

        {/* right wing (ascending V) */}
        <g fill="url(#veyra-vbar)">
          <rect x="53" y="48" width="3" height="14" rx="1.5" />
          <rect x="59" y="42" width="3" height="20" rx="1.5" />
          <rect x="65" y="36" width="3" height="26" rx="1.5" />
          <rect x="71" y="30" width="3" height="32" rx="1.5" />
          <rect x="77" y="24" width="3" height="38" rx="1.5" />
          <rect x="83" y="20" width="3" height="42" rx="1.5" />
        </g>

        {/* horizon line */}
        <rect x="14" y="64" width="72" height="0.6" fill="rgba(164,230,255,0.35)" />

        {/* reflection */}
        <g
          fill="url(#veyra-vbar)"
          opacity="0.25"
          transform="translate(0,128) scale(1,-1)"
        >
          <rect x="14" y="56" width="3" height="10" rx="1.5" />
          <rect x="20" y="56" width="3" height="10" rx="1.5" />
          <rect x="26" y="56" width="3" height="10" rx="1.5" />
          <rect x="32" y="56" width="3" height="10" rx="1.5" />
          <rect x="38" y="56" width="3" height="10" rx="1.5" />
          <rect x="44" y="56" width="3" height="10" rx="1.5" />
          <rect x="53" y="56" width="3" height="10" rx="1.5" />
          <rect x="59" y="56" width="3" height="10" rx="1.5" />
          <rect x="65" y="56" width="3" height="10" rx="1.5" />
          <rect x="71" y="56" width="3" height="10" rx="1.5" />
          <rect x="77" y="56" width="3" height="10" rx="1.5" />
          <rect x="83" y="56" width="3" height="10" rx="1.5" />
        </g>

        {/* spark (candle) */}
        <ellipse cx="50" cy="78" rx="9" ry="14" fill="url(#veyra-spark)" />
        <rect x="49" y="64" width="2" height="14" rx="1" fill="#ffb454" />
      </svg>
    </span>
  );
}
