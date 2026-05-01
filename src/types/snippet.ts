// Mirror of `src-tauri/src/storage/snippets.rs::Snippet`.
// Camel-cased on the wire via `serde(rename_all = "camelCase")`.
export interface Snippet {
  id: number;
  createdAt: number;
  updatedAt: number;
  trigger: string;
  expansion: string;
  description: string | null;
  enabled: boolean;
  useCount: number;
}

// Payload for `upsert_snippet`. Matches Rust `NewSnippetPayload`.
export interface NewSnippetInput {
  trigger: string;
  expansion: string;
  description: string | null;
  enabled: boolean;
}
