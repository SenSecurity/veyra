import { useEffect, useState } from "react";
import { Plus, Search, Trash2 } from "lucide-react";
import { EmptyState } from "@/components/empty-state";
import { PageShell, Panel, Toolbar } from "@/components/page-shell";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/tauri";
import type { DictionaryTerm } from "@/types/dictionary";

export function DictionaryRoute() {
  const [rows, setRows] = useState<DictionaryTerm[]>([]);
  const [term, setTerm] = useState("");
  const [replacement, setReplacement] = useState("");
  const [query, setQuery] = useState("");
  const [abbr, setAbbr] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reload = () =>
    ipc
      .listDictionaryTerms()
      .then((items) => {
        setError(null);
        setRows(items);
      })
      .catch((loadError) => {
        setError(String(loadError));
        setRows([]);
      });
  useEffect(() => void reload(), []);

  async function save() {
    if (!term.trim()) return;
    await ipc.upsertDictionaryTerm({
      term: term.trim(),
      replacement: replacement.trim() || null,
      isAbbreviation: abbr,
      autoAdded: false,
      enabled: true,
    });
    setTerm("");
    setReplacement("");
    setAbbr(false);
    await reload();
  }

  const filteredRows = rows.filter((row) => {
    const needle = query.trim().toLowerCase();
    if (!needle) return true;
    return [row.term, row.replacement ?? ""].join(" ").toLowerCase().includes(needle);
  });

  return (
    <PageShell title="Dictionary" description="Custom words and replacements.">
      <Panel title="Add entry" description="Keep names, product terms, and abbreviations consistent.">
        <div className="grid gap-3 md:grid-cols-[1fr_1fr_auto_auto]">
          <Input value={term} onChange={(e) => setTerm(e.target.value)} placeholder="Word or phrase" />
          <Input value={replacement} onChange={(e) => setReplacement(e.target.value)} placeholder="Replacement" />
          <label className="flex h-9 items-center gap-2 rounded-lg border border-border bg-white/64 px-3 text-sm text-muted-foreground">
            <Switch checked={abbr} onCheckedChange={setAbbr} />
            Abbrev
          </label>
          <Button type="button" onClick={save}>
            <Plus className="h-4 w-4" />
            Add entry
          </Button>
        </div>
      </Panel>
      <Toolbar>
        <div className="relative flex-1">
          <Search className="pointer-events-none absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input className="pl-9" value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search dictionary" />
        </div>
      </Toolbar>
      <Panel className="p-0 md:p-0">
        {error ? (
          <div className="p-4">
            <EmptyState title="Could not load dictionary">{error}</EmptyState>
          </div>
        ) : filteredRows.length === 0 ? (
          <div className="p-4">
            <EmptyState title="No dictionary entries" />
          </div>
        ) : (
          <DataTable
            rows={filteredRows.map((row) => ({
              id: row.id,
              cells: [row.term, row.replacement ?? "", row.isAbbreviation ? "Yes" : "No", row.enabled ? "Enabled" : "Off"],
            }))}
            headers={["Word/Phrase", "Replacement", "Abbrev", "State"]}
            onDelete={async (id) => {
              await ipc.deleteDictionaryTerm(id);
              await reload();
            }}
          />
        )}
      </Panel>
    </PageShell>
  );
}

export function DataTable({
  headers,
  rows,
  onDelete,
}: {
  headers: string[];
  rows: { id: number; cells: string[] }[];
  onDelete: (id: number) => Promise<void>;
}) {
  return (
    <div className="overflow-hidden rounded-xl bg-white/58">
      <table className="w-full text-left text-sm">
        <thead className="border-b border-border bg-accent/45 text-[0.68rem] uppercase tracking-[0.08em] text-muted-foreground">
          <tr>
            {headers.map((h) => <th key={h} className="px-4 py-3 font-semibold">{h}</th>)}
            <th className="w-12 px-4 py-3" />
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.id} className="border-b border-border/70 last:border-b-0 hover:bg-white/62">
              {row.cells.map((cell, i) => <td key={`${row.id}-${i}`} className="px-4 py-3">{cell}</td>)}
              <td className="px-4 py-3">
                <button type="button" onClick={() => void onDelete(row.id)} className="veyra-icon-button hover:text-destructive" aria-label="Delete" title="Delete">
                  <Trash2 className="h-4 w-4" />
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
