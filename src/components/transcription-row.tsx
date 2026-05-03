import { Copy, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { highlightParts } from "@/lib/fts-highlight";
import { formatDateTime, formatDuration } from "@/lib/format-date";
import type { Transcription } from "@/types/transcription";

export function TranscriptionRow({
  row,
  query = "",
  onDelete,
}: {
  row: Transcription;
  query?: string;
  onDelete?: (id: number) => void;
}) {
  const parts = highlightParts(row.finalText || row.rawText, query);

  async function copyText() {
    const text = row.finalText || row.rawText;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      toast.success("Copied");
    } catch (error) {
      toast.error(`Copy failed: ${String(error)}`);
    }
  }

  return (
    <article className="veyra-surface-hover rounded-2xl border border-border bg-white/78 p-4 shadow-[0_10px_28px_rgb(44_92_140_/_0.065)]">
      <div className="flex items-start gap-3">
        <div className="min-w-0 flex-1">
          <p className="whitespace-pre-wrap text-[0.93rem] font-medium leading-6 text-foreground">
            {parts.map((part, i) =>
              part.match ? (
                <mark key={`${part.text}-${i}`} className="rounded bg-accent px-0.5 text-accent-foreground">
                  {part.text}
                </mark>
              ) : (
                <span key={`${part.text}-${i}`}>{part.text}</span>
              ),
            )}
          </p>
          <div className="mt-3 flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
            <span>{formatDateTime(row.createdAt)}</span>
            <span className="h-1 w-1 rounded-full bg-muted-foreground/35" />
            <span>{row.wordCount} words</span>
            <span className="h-1 w-1 rounded-full bg-muted-foreground/35" />
            <span>{formatDuration(row.durationMs)}</span>
            <span className="rounded-full bg-accent/80 px-2 py-0.5 font-semibold text-accent-foreground">
              {row.mode === "command" ? "Email" : "Transcription"}
            </span>
            <span className="rounded-full bg-muted px-2 py-0.5">{row.engine}</span>
          </div>
        </div>
        <button
          type="button"
          className="veyra-icon-button"
          onClick={() => void copyText()}
          aria-label="Copy transcription"
          title="Copy"
        >
          <Copy className="h-4 w-4" />
        </button>
        {onDelete && (
          <button
            type="button"
            className="veyra-icon-button hover:text-destructive"
            onClick={() => onDelete(row.id)}
            aria-label="Delete transcription"
            title="Delete"
          >
            <Trash2 className="h-4 w-4" />
          </button>
        )}
      </div>
    </article>
  );
}
