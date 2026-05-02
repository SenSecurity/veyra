import { describe, expect, it } from "vitest";
import { sortScratchpadNotes } from "./scratchpad-order";
import type { ScratchpadNote } from "@/types/scratchpad";

const note = (id: number, updatedAt: number, pinned: boolean): ScratchpadNote => ({
  id,
  createdAt: updatedAt,
  updatedAt,
  title: null,
  body: String(id),
  pinned,
});

describe("sortScratchpadNotes", () => {
  it("orders pinned first, then newest update", () => {
    expect(sortScratchpadNotes([note(1, 10, false), note(2, 5, true), note(3, 20, false)]).map((n) => n.id)).toEqual([2, 3, 1]);
  });
});

