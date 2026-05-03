import { cn } from "@/lib/utils";

export type EngineRole = "stt" | "drafter";

export interface EngineCardMetaItem {
  label?: string;
  value: string;
  bold?: boolean;
}

export interface EngineCardProps {
  role: EngineRole;
  /** Index shown in the role caption ("STT · 01", "Drafter · 02"). */
  index?: number;
  /** Engine display name; the part after the dot is rendered in italic Newsreader. */
  name: string;
  italic?: string;
  meta: EngineCardMetaItem[];
  /** Free-text status indicator (defaults to "Ready"). */
  status?: string;
  className?: string;
}

/**
 * Sidebar engine descriptor. Renders a single engine slice with its
 * accent rule (cyan for STT, spark amber for Drafter), a role caption,
 * the engine name (with optional italic accent), and a compact meta
 * row. Two cards are typically stacked inside one bordered panel —
 * see Sidebar.
 */
export function EngineCard({
  role,
  index,
  name,
  italic,
  meta,
  status = "Ready",
  className,
}: EngineCardProps) {
  const accent =
    role === "stt"
      ? "before:bg-[linear-gradient(180deg,var(--cyan),var(--cyan-deep))] before:shadow-[0_0_8px_var(--halo)]"
      : "before:bg-[linear-gradient(180deg,var(--spark),var(--spark-deep))] before:shadow-[0_0_8px_var(--spark-glow)]";

  const led =
    role === "stt"
      ? "bg-[var(--cyan)] shadow-[0_0_5px_var(--halo)]"
      : "bg-[var(--spark)] shadow-[0_0_5px_var(--spark-glow)]";

  const italicTone =
    role === "stt" ? "text-[var(--cyan-deep)]" : "text-[var(--spark-deep)]";

  return (
    <div
      className={cn(
        "relative px-3.5 py-3",
        "before:pointer-events-none before:absolute before:left-0 before:top-3 before:bottom-3 before:w-[2px] before:rounded-[2px]",
        accent,
        className,
      )}
    >
      <header className="mb-1 flex items-center justify-between">
        <span className="font-mono text-[0.6rem] tracking-[0.22em] uppercase text-muted-foreground">
          {role === "stt" ? "STT" : "Drafter"}
          {typeof index === "number" ? ` · ${index.toString().padStart(2, "0")}` : null}
        </span>
        <span className="inline-flex items-center gap-1.5 text-[0.65rem] font-medium text-foreground/85">
          <span className={cn("h-1.5 w-1.5 rounded-full", led)} aria-hidden="true" />
          {status}
        </span>
      </header>
      <div className="text-[0.82rem] font-semibold tracking-[-0.005em] text-foreground">
        {name}
        {italic ? (
          <span className={cn("veyra-italic ml-1 text-[0.78rem]", italicTone)}>
            {italic}
          </span>
        ) : null}
      </div>
      {meta.length > 0 ? (
        <div className="mt-1 flex flex-wrap gap-x-1.5 gap-y-0.5 font-mono text-[0.6rem] tracking-[0.16em] uppercase text-muted-foreground">
          {meta.map((item, i) => (
            <span key={`${item.value}-${i}`} className="inline-flex items-center gap-1.5">
              {i > 0 ? <span className="text-foreground/30">·</span> : null}
              {item.label ? <span className="text-muted-foreground">{item.label}</span> : null}
              <span className={cn(item.bold ? "font-semibold text-foreground" : "")}>
                {item.value}
              </span>
            </span>
          ))}
        </div>
      ) : null}
    </div>
  );
}
