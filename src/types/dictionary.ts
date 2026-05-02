// Mirror of `src-tauri/src/storage/dictionary.rs::DictionaryTerm`.
// The Rust struct derives `serde(rename_all = "camelCase")`, so wire fields
// arrive camelCased.
export interface DictionaryTerm {
  id: number;
  createdAt: number;
  updatedAt: number;
  term: string;
  replacement: string | null;
  isAbbreviation: boolean;
  autoAdded: boolean;
  enabled: boolean;
}

// Payload for `upsert_dictionary_term`. Matches the Rust
// `NewDictionaryTermPayload` struct (camelCase via serde).
export interface NewDictionaryTermInput {
  term: string;
  replacement: string | null;
  isAbbreviation: boolean;
  autoAdded: boolean;
  enabled: boolean;
}
