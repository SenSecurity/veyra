import { cn } from "@/lib/utils";

export interface KpiCell {
  label: string;
  /** Primary number; pass `—` for unsourced cells (don't invent values). */
  value: string;
  /** Optional unit suffix, displayed smaller after the value. */
  unit?: string;
  /** Optional delta line under the value (e.g. "↑ 12.4%"). Render only when computable. */
  delta?: string;
  deltaTone?: "up" | "down" | "flat";
}

/**
 * Four-cell horizontal stat strip used on the Home hero band. Cells are
 * separated by a 1px hairline; the last cell drops the right border.
 * Numbers use tabular figures so the strip stays balanced as values
 * change.
 */
export function KpiStrip({ cells, className }: { cells: KpiCell[]; className?: string }) {
  return (
    <section
      className={cn(
        "grid overflow-hidden rounded-2xl border border-border/70 bg-white shadow-[0_1px_0_rgb(12_17_28_/_0.025)]",
        className,
      )}
      style={{ gridTemplateColumns: `repeat(${cells.length}, minmax(0, 1fr))` }}
    >
      {cells.map((cell, i) => (
        <div
          key={`${cell.label}-${i}`}
          className={cn(
            "px-5 py-3.5",
            i < cells.length - 1 ? "border-r border-border/70" : "",
          )}
        >
          <div className="font-mono text-[0.6rem] tracking-[0.22em] uppercase text-muted-foreground">
            {cell.label}
          </div>
          <div className="mt-1.5 text-[1.5rem] font-medium leading-none tracking-[-0.025em] text-foreground tabular-nums">
            {cell.value}
            {cell.unit ? (
              <span className="ml-1 text-[0.7rem] font-medium text-muted-foreground">
                {cell.unit}
              </span>
            ) : null}
          </div>
          {cell.delta ? (
            <div
              className={cn(
                "mt-1 font-mono text-[0.65rem] tracking-[0.1em]",
                cell.deltaTone === "down"
                  ? "text-amber-700"
                  : cell.deltaTone === "flat"
                    ? "text-muted-foreground"
                    : "text-[var(--cyan-deep)]",
              )}
            >
              {cell.delta}
            </div>
          ) : null}
        </div>
      ))}
    </section>
  );
}
