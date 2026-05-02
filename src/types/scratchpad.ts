// Mirror of `src-tauri/src/storage/scratchpad.rs::ScratchpadNote`.
export interface ScratchpadNote {
  id: number;
  createdAt: number;
  updatedAt: number;
  title: string | null;
  body: string;
  pinned: boolean;
}

// Payload for `upsert_scratchpad_note`. `id` undefined creates; otherwise
// updates in place.
export interface NewScratchpadNoteInput {
  id?: number | null;
  title: string | null;
  body: string;
  pinned: boolean;
}
