import { cn } from "@/lib/utils";

export type WaveStageVariant = "stt" | "drafter";

export interface WaveStageProps {
  variant: WaveStageVariant;
  /** Reserved for future amplitude wiring; currently animates the idle pulse. */
  live?: boolean;
  /** Mono caption rendered bottom-left over the dark canvas. */
  metaLeft?: string;
  /** Mono caption rendered bottom-right; bold-style highlight allowed via children. */
  metaRight?: React.ReactNode;
  className?: string;
}

// V-shape bar height ladder — taller at edges, short in centre — matches
// docs/mockups/08-glacier-veyra.html .vsg-bars geometry.
const BAR_HEIGHTS = [
  95, 88, 78, 70, 62, 54, 46, 38, 30, 22, 16, 12,
  12, 16, 22, 30, 38, 46, 54, 62, 70, 78, 88, 95,
];

const BAR_DELAYS = [
  0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.35, 0.40, 0.45, 0.50, 0.55, 0.60,
  0.62, 0.55, 0.50, 0.45, 0.40, 0.35, 0.30, 0.25, 0.20, 0.15, 0.10, 0.05,
];

/**
 * Dark "wave-stage" canvas hosting the V-shaped signal bars + a warm
 * spark below the horizon line. Used inside Glacier engine cards on
 * the Home route. Bar heights are baked in to match the brand mark.
 */
export function WaveStage({
  variant,
  metaLeft,
  metaRight,
  className,
}: WaveStageProps) {
  return (
    <div
      className={cn("veyra-wave-stage", className)}
      data-variant={variant}
      role="presentation"
    >
      <div className="vsg-bars">
        {BAR_HEIGHTS.map((h, i) => (
          <i
            key={`b-${i}`}
            style={
              {
                "--h": `${h}%`,
                animationDelay: `${BAR_DELAYS[i]}s`,
              } as React.CSSProperties
            }
          />
        ))}
      </div>
      <div className="vsg-bars vsg-refl" aria-hidden="true">
        {BAR_HEIGHTS.map((h, i) => (
          <i
            key={`r-${i}`}
            style={
              {
                "--h": `${h * 0.45}%`,
                animationDelay: `${BAR_DELAYS[i]}s`,
              } as React.CSSProperties
            }
          />
        ))}
      </div>
      <span className="vsg-spark" aria-hidden="true" />
      {(metaLeft || metaRight) && (
        <div className="vsg-meta">
          <span>{metaLeft}</span>
          <span>{metaRight}</span>
        </div>
      )}
    </div>
  );
}
