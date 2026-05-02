import { useEffect, useMemo, useState } from "react";
import { marked } from "marked";
import { Pin, Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { ipc } from "@/lib/tauri";
import { sortScratchpadNotes } from "@/lib/scratchpad-order";
import type { ScratchpadNote } from "@/types/scratchpad";

export function ScratchpadRoute() {
  const [rows, setRows] = useState<ScratchpadNote[]>([]);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [pinned, setPinned] = useState(false);
  const sorted = useMemo(() => sortScratchpadNotes(rows), [rows]);
  const reload = () => ipc.listScratchpadNotes().then(setRows).catch(() => setRows([]));
  useEffect(() => void reload(), []);

  async function save() {
    if (!body.trim()) return;
    await ipc.upsertScratchpadNote({ title: title.trim() || null, body, pinned });
    setTitle("");
    setBody("");
    setPinned(false);
    await reload();
  }

  return (
    <section className="space-y-4 p-6">
      <h1 className="text-2xl font-semibold">Scratchpad</h1>
      <div className="grid gap-2 rounded-lg border border-border bg-card p-4">
        <Input value={title} onChange={(e) => setTitle(e.target.value)} placeholder="Title" />
        <textarea className="min-h-28 rounded-md border border-border bg-background p-3 text-sm" value={body} onChange={(e) => setBody(e.target.value)} placeholder="Markdown note" />
        <div className="flex items-center justify-between">
          <label className="flex items-center gap-2 text-sm"><Switch checked={pinned} onCheckedChange={setPinned} /> Pin</label>
          <Button type="button" onClick={save}><Plus className="h-4 w-4" /> Add note</Button>
        </div>
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        {sorted.map((note) => (
          <article key={note.id} className="rounded-lg border border-border bg-card p-4">
            <div className="flex items-start justify-between gap-3">
              <h2 className="font-semibold">{note.title || "Untitled"}</h2>
              <div className="flex gap-1">
                <button type="button" className="p-1 text-muted-foreground hover:text-foreground" onClick={() => void ipc.pinScratchpadNote(note.id, !note.pinned).then(reload)} aria-label="Pin">
                  <Pin className={note.pinned ? "h-4 w-4 fill-current" : "h-4 w-4"} />
                </button>
                <button type="button" className="p-1 text-muted-foreground hover:text-danger" onClick={() => void ipc.deleteScratchpadNote(note.id).then(reload)} aria-label="Delete">
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>
            </div>
            <div className="prose prose-sm mt-3 max-w-none text-sm" dangerouslySetInnerHTML={{ __html: marked.parse(note.body, { async: false }) }} />
          </article>
        ))}
      </div>
    </section>
  );
}
