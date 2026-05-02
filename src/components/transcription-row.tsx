import { Trash2 } from "lucide-react";
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
  return (
    <article className="rounded-lg border border-border bg-card p-4 shadow-sm">
      <div className="flex items-start gap-3">
        <div className="min-w-0 flex-1">
          <p className="text-sm leading-6 text-foreground">
            {parts.map((part, i) =>
              part.match ? (
                <mark key={`${part.text}-${i}`} className="rounded bg-muted px-0.5">
                  {part.text}
                </mark>
              ) : (
                <span key={`${part.text}-${i}`}>{part.text}</span>
              ),
            )}
          </p>
          <div className="mt-3 flex flex-wrap gap-2 text-xs text-muted-foreground">
            <span>{formatDateTime(row.createdAt)}</span>
            <span>{row.wordCount} words</span>
            <span>{formatDuration(row.durationMs)}</span>
            <span>{row.engine}</span>
            <span>{row.mode}</span>
          </div>
        </div>
        {onDelete && (
          <button
            type="button"
            className="rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-danger"
            onClick={() => onDelete(row.id)}
            aria-label="Delete transcription"
          >
            <Trash2 className="h-4 w-4" />
          </button>
        )}
      </div>
    </article>
  );
}
