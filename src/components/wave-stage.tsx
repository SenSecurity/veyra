import { cn } from "@/lib/utils";

export type WaveStageVariant = "stt" | "drafter";

export interface WaveStageProps {
  variant: WaveStageVariant;
  /** Reserved for future amplitude wiring; currently animates the idle pulse. */
  live?: boolean;
  /** Mono caption rendered bottom-left under the strip. */
  metaLeft?: string;
  /** Mono caption rendered bottom-right; bold-style highlight allowed via children. */
  metaRight?: React.ReactNode;
  className?: string;
}

const BAR_COUNT = 40;

/**
 * Light-glass horizontal waveform strip. Mirrors the visual language
 * of the recording overlay capsule (docs/mockups/overlay-01-capsule.html)
 * — subtle ice-tinted background, restrained cyan or spark bars, no
 * dark canvas, no reflection, no central spark. The meta line lives
 * directly below the strip rather than overlaid on it.
 */
export function WaveStage({
  variant,
  metaLeft,
  metaRight,
  className,
}: WaveStageProps) {
  const isSpark = variant === "drafter";
  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div
        className={cn(
          "veyra-wave relative h-16 px-3.5",
          isSpark && "veyra-wave-spark",
        )}
        data-variant={variant}
        role="presentation"
      >
        {Array.from({ length: BAR_COUNT }).map((_, i) => (
          <i
            key={i}
            style={{ animationDelay: `${(i % 13) * 0.06}s` }}
            className="veyra-wave-cell"
          />
        ))}
      </div>
      {(metaLeft || metaRight) && (
        <div className="flex justify-between font-mono text-[0.6rem] tracking-[0.18em] uppercase text-muted-foreground">
          <span>{metaLeft}</span>
          <span>{metaRight}</span>
        </div>
      )}
    </div>
  );
}
