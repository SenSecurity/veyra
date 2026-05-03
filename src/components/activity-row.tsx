import { Copy, Mail, Mic } from "lucide-react";
import { cn } from "@/lib/utils";
import type { Transcription } from "@/types/ipc";

export interface ActivityRowProps {
  row: Transcription;
  onCopy?: (row: Transcription) => void;
  className?: string;
}

/**
 * Dense Glacier activity row. 6-column grid: icon · text · tag · word
 * count · time · copy. The tag follows the engine duality (cyan
 * Dictation / spark Draft).
 */
export function ActivityRow({ row, onCopy, className }: ActivityRowProps) {
  const isDraft = row.mode === "command";
  const Icon = isDraft ? Mail : Mic;
  const text = (row.finalText || row.rawText || "").trim();

  return (
    <div
      className={cn(
        "grid items-center gap-4 px-5 py-3 text-[0.85rem] text-foreground/85 transition-colors hover:bg-frost",
        className,
      )}
      style={{ gridTemplateColumns: "26px 1fr 92px 76px 60px 30px" }}
    >
      <span
        className={cn(
          "inline-grid h-6 w-6 place-items-center rounded-md border border-border/70",
          isDraft
            ? "bg-[#fff5e6] text-[var(--spark-deep)]"
            : "bg-[var(--ice-50)] text-[var(--cyan-deep)]",
        )}
        aria-hidden="true"
      >
        <Icon className="h-3 w-3" strokeWidth={1.6} />
      </span>
      <p className="truncate text-foreground/95">{text}</p>
      <span
        className={cn(
          "justify-self-start rounded-md border px-2 py-[2px] font-mono text-[0.6rem] tracking-[0.18em] uppercase",
          isDraft
            ? "border-[rgba(255,138,31,0.22)] bg-[#fff5e6] text-[var(--spark-deep)]"
            : "border-[rgba(43,199,255,0.25)] bg-[var(--ice-50)] text-[var(--cyan-deep)]",
        )}
      >
        {isDraft ? "Draft" : "Dictation"}
      </span>
      <span className="font-mono text-[0.65rem] tracking-[0.14em] uppercase text-muted-foreground">
        {row.wordCount} words
      </span>
      <span className="text-right font-mono text-[0.65rem] tracking-[0.14em] uppercase text-muted-foreground">
        {formatRelativeTime(row.createdAt)}
      </span>
      <button
        type="button"
        className="grid h-7 w-7 place-items-center justify-self-end rounded-md border border-border/70 bg-white text-muted-foreground transition-colors hover:border-border hover:text-foreground"
        onClick={() => onCopy?.(row)}
        aria-label="Copy"
        title="Copy"
      >
        <Copy className="h-3 w-3" strokeWidth={1.6} />
      </button>
    </div>
  );
}

function formatRelativeTime(epochMs: number | undefined): string {
  if (!epochMs) return "—";
  const ms = epochMs > 1e12 ? epochMs : epochMs * 1000;
  const date = new Date(ms);
  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();
  if (sameDay) {
    return date.toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
  }
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  const isYesterday =
    date.getFullYear() === yesterday.getFullYear() &&
    date.getMonth() === yesterday.getMonth() &&
    date.getDate() === yesterday.getDate();
  if (isYesterday) return "yesterday";
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}
