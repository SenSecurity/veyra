import { useEffect, useState } from "react";
import { Plus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ipc } from "@/lib/tauri";
import type { Snippet } from "@/types/snippet";
import { DataTable } from "./dictionary";

export function SnippetsRoute() {
  const [rows, setRows] = useState<Snippet[]>([]);
  const [trigger, setTrigger] = useState("");
  const [expansion, setExpansion] = useState("");
  const [description, setDescription] = useState("");
  const reload = () => ipc.listSnippets().then(setRows).catch(() => setRows([]));
  useEffect(() => void reload(), []);

  async function save() {
    if (!trigger.trim() || !expansion.trim()) return;
    await ipc.upsertSnippet({
      trigger: trigger.trim(),
      expansion: expansion.trim(),
      description: description.trim() || null,
      enabled: true,
    });
    setTrigger("");
    setExpansion("");
    setDescription("");
    await reload();
  }

  return (
    <section className="space-y-4 p-6">
      <h1 className="text-2xl font-semibold">Snippets</h1>
      <div className="grid gap-2 rounded-lg border border-border bg-card p-4 md:grid-cols-[1fr_1fr_1fr_auto]">
        <Input value={trigger} onChange={(e) => setTrigger(e.target.value)} placeholder=":trigger" />
        <Input value={expansion} onChange={(e) => setExpansion(e.target.value)} placeholder="expansion" />
        <Input value={description} onChange={(e) => setDescription(e.target.value)} placeholder="description" />
        <Button type="button" onClick={save}><Plus className="h-4 w-4" /> Add</Button>
      </div>
      <DataTable
        headers={["Trigger", "Expansion", "Description", "Uses"]}
        rows={rows.map((row) => ({
          id: row.id,
          cells: [row.trigger, row.expansion, row.description ?? "", String(row.useCount)],
        }))}
        onDelete={async (id) => {
          await ipc.deleteSnippet(id);
          await reload();
        }}
      />
    </section>
  );
}
