import { useEffect, useMemo, useState } from "react";
import { Command } from "cmdk";
import { useNavigate } from "@tanstack/react-router";

const pages = [
  { label: "Home", to: "/" },
  { label: "History", to: "/history" },
  { label: "Email Drafter", to: "/email-drafts" },
  { label: "Dictionary", to: "/dictionary" },
  { label: "Settings", to: "/settings/general" },
] as const;

export function CommandPalette() {
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const filtered = useMemo(
    () => pages.filter((p) => p.label.toLowerCase().includes(search.toLowerCase())),
    [search],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen((o) => !o);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/20 p-6" onClick={() => setOpen(false)}>
      <Command
        className="mx-auto max-w-lg overflow-hidden rounded-md border border-border bg-popover text-popover-foreground shadow-3"
        onClick={(e) => e.stopPropagation()}
      >
        <Command.Input
          value={search}
          onValueChange={setSearch}
          className="h-11 w-full border-b border-border bg-transparent px-3 text-sm outline-none"
          placeholder="Jump to page"
        />
        <Command.List className="max-h-80 overflow-auto p-2">
          <Command.Empty className="px-3 py-6 text-center text-sm text-muted-foreground">No results</Command.Empty>
          <Command.Group heading="Pages" className="text-xs text-muted-foreground">
            {filtered.map((page) => (
              <Command.Item
                key={page.to}
                value={page.label}
                className="cursor-pointer rounded-md px-3 py-2 text-sm text-foreground aria-selected:bg-muted"
                onSelect={() => {
                  setOpen(false);
                  void navigate({ to: page.to });
                }}
              >
                {page.label}
              </Command.Item>
            ))}
          </Command.Group>
        </Command.List>
      </Command>
    </div>
  );
}
