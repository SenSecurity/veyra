import { useEffect, useState } from "react";
import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/tauri";
import type { DictionaryTerm } from "@/types/dictionary";

export function DictionaryRoute() {
  const [rows, setRows] = useState<DictionaryTerm[]>([]);
  const [term, setTerm] = useState("");
  const [replacement, setReplacement] = useState("");
  const [abbr, setAbbr] = useState(false);

  const reload = () => ipc.listDictionaryTerms().then(setRows).catch(() => setRows([]));
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

  return (
    <section className="space-y-4 p-6">
      <h1 className="text-2xl font-semibold">Dictionary</h1>
      <div className="grid gap-2 rounded-lg border border-border bg-card p-4 md:grid-cols-[1fr_1fr_auto_auto]">
        <Input value={term} onChange={(e) => setTerm(e.target.value)} placeholder="term" />
        <Input value={replacement} onChange={(e) => setReplacement(e.target.value)} placeholder="replacement" />
        <label className="flex items-center gap-2 text-sm"><Switch checked={abbr} onCheckedChange={setAbbr} /> Abbrev</label>
        <Button type="button" onClick={save}><Plus className="h-4 w-4" /> Add</Button>
      </div>
      <DataTable
        rows={rows.map((row) => ({
          id: row.id,
          cells: [row.term, row.replacement ?? "", row.isAbbreviation ? "yes" : "no", row.enabled ? "enabled" : "off"],
        }))}
        headers={["Term", "Replacement", "Abbrev", "State"]}
        onDelete={async (id) => {
          await ipc.deleteDictionaryTerm(id);
          await reload();
        }}
      />
    </section>
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
    <div className="overflow-hidden rounded-lg border border-border bg-card">
      <table className="w-full text-left text-sm">
        <thead className="bg-muted text-xs uppercase text-muted-foreground">
          <tr>
            {headers.map((h) => <th key={h} className="px-3 py-2">{h}</th>)}
            <th className="w-10 px-3 py-2" />
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.id} className="border-t border-border">
              {row.cells.map((cell, i) => <td key={`${row.id}-${i}`} className="px-3 py-2">{cell}</td>)}
              <td className="px-3 py-2">
                <button type="button" onClick={() => void onDelete(row.id)} className="text-muted-foreground hover:text-danger" aria-label="Delete">
                  <Trash2 className="h-4 w-4" />
                </button>
              </td>
            </tr>
          ))}
          {rows.length === 0 && (
            <tr><td className="px-3 py-8 text-center text-muted-foreground" colSpan={headers.length + 1}>No rows</td></tr>
          )}
        </tbody>
      </table>
    </div>
  );
}
