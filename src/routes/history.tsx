import { useEffect, useMemo, useState } from "react";
import { Search } from "lucide-react";
import { EmptyState } from "@/components/empty-state";
import { PageShell, Panel, Toolbar } from "@/components/page-shell";
import { Input } from "@/components/ui/input";
import { TranscriptionRow } from "@/components/transcription-row";
import { isUsableTranscription } from "@/lib/email-output-quality";
import { ipc } from "@/lib/tauri";
import type { Transcription } from "@/types/transcription";

export function HistoryRoute() {
  const [rows, setRows] = useState<Transcription[]>([]);
  const [query, setQuery] = useState("");
  const [engine, setEngine] = useState("all");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const handle = window.setTimeout(() => {
      const load = query.trim()
        ? ipc.searchTranscriptions(query, 50)
        : ipc.listTranscriptions(100, 0);
      void load
        .then((items) => {
          setError(null);
          setRows(items.filter(isUsableTranscription));
        })
        .catch((loadError) => {
          setError(String(loadError));
          setRows([]);
        });
    }, 250);
    return () => window.clearTimeout(handle);
  }, [query]);

  const filtered = useMemo(
    () => rows.filter((row) => engine === "all" || row.engine === engine),
    [rows, engine],
  );

  async function deleteRow(id: number) {
    if (!window.confirm("Delete this transcription?")) return;
    await ipc.deleteTranscription(id);
    setRows((current) => current.filter((row) => row.id !== id));
  }

  return (
    <PageShell title="History" description="All your transcriptions and drafts." className="max-w-[1080px]">
      <Toolbar className="shrink-0">
        <div className="relative flex-1">
          <Search className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input className="pl-9" value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search transcriptions" />
        </div>
        <select className="veyra-select md:w-40" value={engine} onChange={(e) => setEngine(e.target.value)}>
          <option value="all">All engines</option>
          <option value="local">Local</option>
          <option value="groq">Groq</option>
          <option value="cloud">Cloud</option>
        </select>
      </Toolbar>
      <Panel className="min-h-0 flex-1 p-3 md:p-3">
      <div className="h-full min-h-0 space-y-3 overflow-auto pr-1">
        {error ? (
          <EmptyState title="Could not load history">{error}</EmptyState>
        ) : filtered.length === 0 ? (
          <EmptyState title="No matching transcriptions" />
        ) : (
          filtered.map((row) => (
            <TranscriptionRow key={row.id} row={row} query={query} onDelete={deleteRow} />
          ))
        )}
      </div>
      </Panel>
    </PageShell>
  );
}
