import type { ScratchpadNote } from "@/types/scratchpad";

export function sortScratchpadNotes(notes: ScratchpadNote[]): ScratchpadNote[] {
  return [...notes].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
    if (a.updatedAt !== b.updatedAt) return b.updatedAt - a.updatedAt;
    return b.id - a.id;
  });
}

