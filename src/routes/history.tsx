import { useEffect, useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Search } from "lucide-react";
import { EmptyState } from "@/components/empty-state";
import { PageShell, Panel, Toolbar } from "@/components/page-shell";
import { Input } from "@/components/ui/input";
import { TranscriptionRow } from "@/components/transcription-row";
import { ipc } from "@/lib/tauri";
import type { Transcription } from "@/types/transcription";

export function HistoryRoute() {
  const [rows, setRows] = useState<Transcription[]>([]);
  const [query, setQuery] = useState("");
  const [engine, setEngine] = useState("all");
  const parentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handle = window.setTimeout(() => {
      const load = query.trim()
        ? ipc.searchTranscriptions(query, 50)
        : ipc.listTranscriptions(100, 0);
      void load.then(setRows).catch(() => setRows([]));
    }, 250);
    return () => window.clearTimeout(handle);
  }, [query]);

  const filtered = useMemo(
    () => rows.filter((row) => engine === "all" || row.engine === engine),
    [rows, engine],
  );
  const virtualizer = useVirtualizer({
    count: filtered.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 124,
    overscan: 8,
  });

  async function deleteRow(id: number) {
    if (!window.confirm("Delete this transcription?")) return;
    await ipc.deleteTranscription(id);
    setRows((current) => current.filter((row) => row.id !== id));
  }

  return (
    <PageShell title="History" description="All your transcriptions and drafts.">
      <Toolbar>
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
      <Panel className="p-3 md:p-3">
      <div ref={parentRef} className="h-[calc(100vh-246px)] min-h-80 overflow-auto pr-1">
        {filtered.length === 0 ? (
          <EmptyState title="No matching transcriptions" />
        ) : (
          <div className="relative" style={{ height: virtualizer.getTotalSize() }}>
            {virtualizer.getVirtualItems().map((item) => {
              const row = filtered[item.index];
              return (
                <div
                  key={row.id}
                  className="absolute left-0 top-0 w-full pb-3"
                  style={{ transform: `translateY(${item.start}px)` }}
                >
                  <TranscriptionRow row={row} query={query} onDelete={deleteRow} />
                </div>
              );
            })}
          </div>
        )}
      </div>
      </Panel>
    </PageShell>
  );
}
