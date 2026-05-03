import { useEffect, useMemo, useState } from "react";
import { Search } from "lucide-react";
import { EmptyState } from "@/components/empty-state";
import { PageShell, Panel, Toolbar } from "@/components/page-shell";
import { TranscriptionRow } from "@/components/transcription-row";
import { Input } from "@/components/ui/input";
import { ipc } from "@/lib/tauri";
import type { Transcription } from "@/types/transcription";

export function EmailDraftsRoute() {
  const [rows, setRows] = useState<Transcription[]>([]);
  const [query, setQuery] = useState("");

  useEffect(() => {
    void ipc.listEmailDrafts(100, 0).then(setRows).catch(() => setRows([]));
  }, []);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return rows;
    return rows.filter((row) =>
      [row.rawText, row.finalText, row.model ?? "", row.appContext ?? ""]
        .join(" ")
        .toLowerCase()
        .includes(needle),
    );
  }, [rows, query]);

  async function deleteRow(id: number) {
    if (!window.confirm("Delete this email draft?")) return;
    await ipc.deleteTranscription(id);
    setRows((current) => current.filter((row) => row.id !== id));
  }

  return (
    <PageShell
      title="Email Drafter"
      description="Draft emails generated from your voice."
    >
      <Toolbar>
      <div className="relative flex-1">
        <Search className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
        <Input
          className="pl-9"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Search email drafts"
        />
      </div>
        <select className="veyra-select md:w-36" defaultValue="all" aria-label="Draft filter">
          <option value="all">All drafts</option>
        </select>
      </Toolbar>
      <Panel className="p-3 md:p-3">
        {filtered.length === 0 ? (
          <EmptyState title="No email drafts yet">
            Use the Email Draft hotkey once and generated drafts appear here.
          </EmptyState>
        ) : (
          <div className="space-y-3">
            {filtered.map((row) => (
              <TranscriptionRow key={row.id} row={row} query={query} onDelete={deleteRow} />
            ))}
          </div>
        )}
      </Panel>
    </PageShell>
  );
}
