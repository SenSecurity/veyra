# Phase 3 — UI Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite frontend from vanilla TS (`src/main.ts` + `index.html`) to React 19 + Vite 6 + Tailwind v4 + shadcn + tanstack-router, covering every route in master spec §3 and §6.

**Architecture:** Single-shot rewrite. Each task migrates one screen or one infra slice; intermediate commits leave the app in a buildable, launchable state with the un-migrated parts of the old `main.ts` still active. Last task deletes `main.ts` + old `index.html` markup.

**Tech Stack:** React 19, Vite 6, Tailwind v4 + `@theme`, shadcn (canary v4-compatible), tanstack-router (code-based), tanstack-virtual, zustand 5, react-hook-form + zod, framer-motion, cmdk, date-fns, lucide-react, Vitest + @testing-library/react.

**Sub-spec:** `docs/superpowers/specs/2026-05-01-phase-3-ui-rewrite-design.md`

**Master spec:** `docs/superpowers/specs/2026-04-23-wispr-flow-parity-design.md` §3, §6.

---

## Task 0 — Scaffolding

**Goal:** Install all Phase 3 deps, configure Vite multi-entry, Tailwind v4 with `@theme` tokens from spec §3.1, shadcn init, base `<App />` shell rendering an empty layout. Old `main.ts` neutered (its DOM-mutating code-paths removed; the file deleted in T15).

**Files:**
- Modify: `package.json`
- Create: `vite.config.ts`, `tsconfig.json` (or modify), `postcss.config.js` (if needed by Tailwind v4 — v4 mostly via Vite plugin)
- Create: `src/main.tsx`, `src/app.tsx`, `src/router.tsx`
- Create: `src/styles/globals.css`, `src/styles/tailwind.css`
- Create: `src/types/ipc.ts` (initial empty)
- Modify: `index.html` — strip body markup, leave `<div id="root"></div>` + script tag pointing at `src/main.tsx`
- Delete (last step): old `<style>` and `<script>` tags inside `index.html` body

**Steps:**

- [ ] **0.1 — Install deps**

```bash
pnpm add react@19 react-dom@19 @tanstack/react-router@1 @tanstack/react-virtual@3 zustand@5 react-hook-form@7 zod@3 @hookform/resolvers@3 framer-motion@11 cmdk@1 date-fns@4 lucide-react
pnpm add -D @types/react@19 @types/react-dom@19 tailwindcss@4 @tailwindcss/vite@4 vitest@2 @testing-library/react@16 @testing-library/jest-dom@6 jsdom @vitejs/plugin-react @tanstack/router-devtools
```

(If `pnpm` errors, fall back to `npm install --save` with the same package list.)

- [ ] **0.2 — `vite.config.ts`**

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwind from "@tailwindcss/vite";
import { resolve } from "path";

export default defineConfig({
  plugins: [react(), tailwind()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "esnext",
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        overlay: resolve(__dirname, "overlay.html"),
      },
    },
  },
  resolve: { alias: { "@": resolve(__dirname, "src") } },
});
```

- [ ] **0.3 — `src/styles/tailwind.css`** with `@import "tailwindcss";`. **`src/styles/globals.css`** with the `@theme { ... }` block from sub-spec §3 / master spec §3.1 (oklch tokens). Import both in `main.tsx`.

- [ ] **0.4 — Strip `index.html`**: keep only `<!DOCTYPE html>`, `<head>` with title/meta, `<body><div id="root"></div><script type="module" src="/src/main.tsx"></script></body>`. Move existing `src/overlay.html` body to a backup snippet for T11 to copy from.

- [ ] **0.5 — `src/main.tsx`**:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider } from "@tanstack/react-router";
import { router } from "./router";
import "./styles/tailwind.css";
import "./styles/globals.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>,
);
```

- [ ] **0.6 — `src/router.tsx`** with a single root route + `/` index rendering a placeholder "Typr Phase 3 scaffolding" `<App />`.

- [ ] **0.7 — `src/app.tsx`** placeholder layout: `<div className="min-h-screen bg-bg text-fg">` + `<Outlet />`.

- [ ] **0.8 — Delete `src/main.ts`** if its references are only `index.html` script tag. Otherwise leave it un-imported. (Definitive deletion in T15.)

- [ ] **0.9 — `pnpm build` + `npx tauri build --no-bundle`** — both green.

- [ ] **0.10 — Launch typr.exe** → app shows the placeholder text. Overlay still uses old `overlay.html` (unchanged this task).

- [ ] **Commit:** `feat(ui): scaffold React 19 + Tailwind v4 + tanstack-router shell`

---

## Task 1 — Tauri IPC adapter + types

**Goal:** Typed wrappers around `@tauri-apps/api/invoke` for every existing Rust command. TypeScript interfaces for every payload.

**Files:**
- Create: `src/lib/tauri.ts`
- Create: `src/types/settings.ts`, `src/types/transcription.ts`
- Modify: `src/types/ipc.ts`

**Steps:**

- [ ] **1.1 — Mirror v1 `Settings` shape from `legacy_v1::Settings` (Rust)** into `src/types/settings.ts`:

```ts
export interface Settings {
  microphone: string;
  engine: "local" | "groq";
  whisperModel: "turbo" | "base" | "large-v3";
  groqApiKey: string;
  recordingMode: "toggle" | "push-to-talk";
  hotkey: string;
}
```

- [ ] **1.2 — `src/types/transcription.ts`**: mirror `storage::transcriptions::Transcription`. Same field names camelCased.

- [ ] **1.3 — `src/lib/tauri.ts`** with one function per command:

```ts
import { invoke } from "@tauri-apps/api/core";
import type { Settings } from "@/types/settings";
import type { MicDevice, RecordingState } from "@/types/ipc";

export const ipc = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  listMicrophones: () => invoke<MicDevice[]>("list_microphones"),
  getRecordingState: () => invoke<RecordingState>("get_recording_state"),
  checkModelDownloaded: (modelSize: string) => invoke<boolean>("check_model_downloaded", { modelSize }),
  downloadModel: (modelSize: string) => invoke<void>("download_model", { modelSize }),
  toggleRecording: () => invoke<string>("toggle_recording"),
};
```

- [ ] **1.4 — `npm run build`** green. No runtime use yet.

- [ ] **Commit:** `feat(ui): typed IPC adapter for Phase 1+2 Tauri commands`

---

## Task 2 — New Tauri commands (Rust side)

**Goal:** Wire commands the new UI needs that don't exist yet. All thin pass-throughs to existing storage repos.

**Files:**
- Modify: `src-tauri/src/main.rs`
- Optional: `src-tauri/src/commands/` module if main.rs gets too long.

**New commands:**
- `list_transcriptions(limit: u32, offset: u32) -> Vec<Transcription>`
- `search_transcriptions(query: String, limit: u32) -> Vec<Transcription>`
- `delete_transcription(id: i64) -> Result<(), String>`
- `list_dictionary_terms() -> Vec<DictionaryTerm>`
- `upsert_dictionary_term(term: NewDictionaryTermPayload) -> Result<i64, String>`
- `delete_dictionary_term(id: i64) -> Result<(), String>`
- `list_snippets() -> Vec<Snippet>`
- `upsert_snippet(snippet: NewSnippetPayload) -> Result<i64, String>`
- `delete_snippet(id: i64) -> Result<(), String>`
- `list_scratchpad_notes() -> Vec<ScratchpadNote>`
- `upsert_scratchpad_note(note: NewNotePayload) -> Result<i64, String>`
- `delete_scratchpad_note(id: i64) -> Result<(), String>`
- `pin_scratchpad_note(id: i64, pinned: bool) -> Result<(), String>`
- `get_stats_totals() -> Totals`
- `get_stats_streak() -> StreakInfo`
- `get_stats_by_day() -> Vec<DailyStats>`
- `wizard_status() -> WizardStatus { completed: bool }`
- `mark_wizard_complete() -> Result<(), String>`
- `test_groq_key(key: String) -> Result<(), String>`

**Steps:**

- [ ] **2.1 — Add serde structs** for payloads (camelCase via `#[serde(rename_all = "camelCase")]`).
- [ ] **2.2 — Implement each command** as a `#[tauri::command]` calling the existing `*Repo::<method>`. Most are 3–5 lines.
- [ ] **2.3 — `wizard_status` / `mark_wizard_complete`** read/write `app_meta` row `wizard_completed`.
- [ ] **2.4 — `test_groq_key`** does a single Groq `GET /openai/v1/models` with the supplied key; OK on 200, error otherwise.
- [ ] **2.5 — Register all in `tauri::generate_handler!`**.
- [ ] **2.6 — `cargo test --lib`** + `cargo build` green.
- [ ] **2.7 — Extend `src/lib/tauri.ts`** to expose them.
- [ ] **Commit:** `feat(commands): add CRUD + stats + wizard commands for Phase 3 UI`

---

## Task 3 — App shell + layout

**Goal:** Sidebar + topbar + outlet. Routes empty placeholders — proper content lands per-task.

**Files:**
- Create: `src/layout/sidebar.tsx`, `src/layout/topbar.tsx`
- Modify: `src/app.tsx`, `src/router.tsx`

**Steps:**

- [ ] **3.1 — Init shadcn** (`pnpm dlx shadcn@canary init`). Configure for Tailwind v4. Generate `Button`, `Input`, `Card`, `Tabs`, `Switch`, `Toast`.
- [ ] **3.2 — Sidebar**: 220px collapsible, links to `/`, `/history`, `/dictionary`, `/snippets`, `/scratchpad`, `/settings/general`.
- [ ] **3.3 — Topbar**: app title left, recording-state pill right (subscribes to `recording-state` event — placeholder for now).
- [ ] **3.4 — `router.tsx`**: nested routes per spec §3.2. Each leaf returns `<Placeholder name="route-name" />` for now.
- [ ] **3.5 — `pnpm build`** + `npx tauri build --no-bundle` green; launch shows sidebar + outlet placeholder for `/`.
- [ ] **Commit:** `feat(ui): app shell with sidebar + topbar + nested routes`

---

## Task 4 — Settings routes (all tabs)

**Goal:** Every settings tab functional: read settings on mount, edit fields with react-hook-form + zod, save through `saveSettings`.

**Files:**
- Create: `src/routes/settings/layout.tsx`, `general.tsx`, `transcription.tsx`, `hotkeys.tsx`, `overlay.tsx`, `formatting.tsx`, `system.tsx`, `stats.tsx`, `data.tsx`, `about.tsx`
- Create: `src/stores/settings-store.ts`
- Create: `src/hooks/use-settings.ts`

**Steps:**

- [ ] **4.1 — `settings-store.ts`**: zustand store with `settings: Settings | null`, `load()`, `save(patch)`, optimistic update.
- [ ] **4.2 — `use-settings.ts`** hook returning current `settings` and `update(field, value)`.
- [ ] **4.3 — `settings/layout.tsx`**: `<Tabs>` shell with each tab routing.
- [ ] **4.4 — Implement each tab** mapping the v1 fields:
  - General: microphone (Select), recordingMode (toggle/PTT)
  - Transcription: engine, whisperModel + Download button (calls `downloadModel`), groqApiKey (masked Input)
  - Hotkeys: hotkey (capture component) — note `hotkey-input.tsx` reused
  - Overlay: not in v1 schema — Phase 4 (placeholder)
  - Formatting: not in v1 (Phase 4 placeholder), keep blank
  - System: not in v1 (Phase 4 placeholder)
  - Stats: not in v1 (Phase 4 placeholder)
  - Data: word_count_cap, purge_on_exceed — these AREN'T in v1 either; show "Phase 4" until adapter exposes them.
  - About: app version, "Open logs folder" button (calls Tauri shell open).
- [ ] **4.5 — Note the v1 limitation**: most settings UI tabs will land mostly empty in Phase 3 because the adapter still flattens to v1. Phase 4 task: extend adapter or add `get_settings_v3` for nested access. Document in plan.
- [ ] **4.6 — `Vitest`** test for `settings-store` load/save.
- [ ] **Commit:** `feat(ui): settings routes with v1-shape persistence`

---

## Task 5 — Home route

**Goal:** Stats cards + recent transcriptions list. Reads `getStatsTotals`, `getStatsStreak`, `listTranscriptions(5, 0)`.

**Files:**
- Create: `src/routes/index.tsx`
- Create: `src/components/stat-card.tsx`, `src/components/empty-state.tsx`

**Steps:**

- [ ] **5.1 — Cards**: Total words, Sessions today, Current streak, Longest streak.
- [ ] **5.2 — Recent list**: `transcription-row` placeholder (full impl in T6); show last 5.
- [ ] **5.3 — Empty state**: when totals are 0, show empty-state component.
- [ ] **5.4 — Tests**: stat-card snapshot, empty-state visibility.
- [ ] **Commit:** `feat(ui): home route with stats cards and recent transcriptions`

---

## Task 6 — History route

**Goal:** Virtualized list, FTS search, filter chips (date / engine / language / mode).

**Files:**
- Create: `src/routes/history.tsx`
- Create: `src/hooks/use-transcriptions.ts`
- Create: `src/components/transcription-row.tsx`
- Create: `src/lib/fts-highlight.ts`

**Steps:**

- [ ] **6.1 — `use-transcriptions`** with paginated infinite scroll via `@tanstack/react-virtual`.
- [ ] **6.2 — Search input**: debounced 250ms, calls `searchTranscriptions(query, 50)`.
- [ ] **6.3 — `fts-highlight.ts`**: tokenize query, render `<mark>` spans on matched substrings in the row's `final_text`.
- [ ] **6.4 — Filter chips**: stub UI; wire date and engine filters via in-memory client-side filter on the loaded page (full DB-side filtering in Phase 4).
- [ ] **6.5 — Delete row**: confirm dialog → `deleteTranscription(id)` → optimistic remove.
- [ ] **6.6 — Tests**: `fts-highlight` token + span generation; transcription-row variants snapshot.
- [ ] **Commit:** `feat(ui): history route with virtualized list + FTS search`

---

## Task 7 — Dictionary route

**Goal:** Table of dictionary terms. Add/edit/delete. Toggle abbreviation flag, replacement field.

**Files:**
- Create: `src/routes/dictionary.tsx`
- Create: `src/types/dictionary.ts`

**Steps:**

- [ ] **7.1 — Table**: shadcn Table; columns `term`, `replacement`, `is_abbreviation`, `auto_added`, `enabled`, actions.
- [ ] **7.2 — Add modal**: form with term + replacement + isAbbreviation switch.
- [ ] **7.3 — Inline-edit replacement** by clicking a row.
- [ ] **7.4 — Delete with confirm**.
- [ ] **7.5 — Tests**: form validation (term required, ≥1 char).
- [ ] **Commit:** `feat(ui): dictionary CRUD route`

---

## Task 8 — Snippets route

**Goal:** Same shape as dictionary but for trigger→expansion pairs.

**Files:**
- Create: `src/routes/snippets.tsx`
- Create: `src/types/snippet.ts`

**Steps:**

- [ ] **8.1 — Table**: columns trigger, expansion, description, enabled, use_count, actions.
- [ ] **8.2 — Add/edit modal**.
- [ ] **8.3 — Tests**: trigger uniqueness check (frontend hint; backend already enforces).
- [ ] **Commit:** `feat(ui): snippets CRUD route`

---

## Task 9 — Scratchpad route

**Goal:** Notes list. Pin / unpin. Inline markdown render (use `marked` or `remark`-lite).

**Files:**
- Create: `src/routes/scratchpad.tsx`

**Steps:**

- [ ] **9.1 — List with pinned-first ordering**.
- [ ] **9.2 — Add note (textarea + title)**.
- [ ] **9.3 — Render markdown read-only**.
- [ ] **9.4 — Pin toggle**.
- [ ] **9.5 — Tests**: ordering by pinned + updated_at desc.
- [ ] **Commit:** `feat(ui): scratchpad route with markdown render`

---

## Task 10 — First-run wizard

**Goal:** Modal route blocking the app until completed; 6 steps per master spec §3.1 wizard subsection.

**Files:**
- Create: `src/routes/wizard.tsx`
- Modify: `src/app.tsx` (gate: if `wizard_status().completed === false`, redirect to `/wizard`)

**Steps:**

- [ ] **10.1 — Steps**: Welcome → Mic picker (level meter via Web Audio API on the test record) → Engine picker (radio, Groq key paste, test button) → Hotkey test → Language → Done.
- [ ] **10.2 — Skip path**: writes `=0` and routes to `/`.
- [ ] **10.3 — Re-runnable from `/settings/about`**.
- [ ] **10.4 — Tests**: step transitions, validation.
- [ ] **Commit:** `feat(ui): first-run wizard with mic/engine/hotkey/language gates`

---

## Task 11 — Overlay rewrite

**Goal:** `overlay.html` + `src/overlay-main.tsx` mount a tiny React app showing the pill that mirrors `recording-state` events.

**Files:**
- Modify: `overlay.html`
- Create: `src/overlay-main.tsx`, `src/overlay/overlay-app.tsx`, `src/overlay/pill.tsx`
- Modify: `src-tauri/src/main.rs::update_overlay` — replace `eval(js)` with `app.emit_to("overlay", "overlay:state", state)` so the React overlay listens via `@tauri-apps/api/event`.
- Modify: `src-tauri/tauri.conf.json` — overlay window URL points at the built `overlay.html`.

**Steps:**

- [ ] **11.1 — `overlay-main.tsx`**: mount `<OverlayApp />`, listen for `overlay:state`, dispatch into a tiny zustand `overlay-store`.
- [ ] **11.2 — Pill component**: animated state colors (idle / recording / transcribing / success / error), tween via framer-motion.
- [ ] **11.3 — Drop the JS-eval branch in Rust `update_overlay`**; keep the function name + signature, but body becomes `app.emit_to("overlay", "overlay:state", state.clone())`.
- [ ] **11.4 — Manual smoke**: launch typr, press F24, see pill animate.
- [ ] **Commit:** `feat(overlay): rewrite overlay window as React + state event listener`

---

## Task 12 — Command palette

**Goal:** Ctrl+K opens cmdk dialog; FTS search + page jumps.

**Files:**
- Create: `src/layout/command-palette.tsx`
- Modify: `src/app.tsx` (mount palette, register Ctrl+K shortcut globally)

**Steps:**

- [ ] **12.1 — cmdk Dialog**: tabs: pages, search history, quick-add (snippet/dict/note).
- [ ] **12.2 — Tests**: keyboard navigation, dispatch.
- [ ] **Commit:** `feat(ui): command palette Ctrl+K`

---

## Task 13 — Live events + toasts

**Goal:** Centralised subscription to all `settings:*` and `transcription:new` and `model:download:progress`. Toaster for migration events.

**Files:**
- Create: `src/hooks/use-live-events.ts`
- Modify: `src/app.tsx` (mount `<Toaster />` and call `useLiveEvents()` once)

**Steps:**

- [ ] **13.1 — Subscriptions** registered in `useEffect` on `<App />` mount; cleanup on unmount.
- [ ] **13.2 — Toast for each migration event**; success toast for `transcription:new` (with row preview).
- [ ] **13.3 — Tests**: mock `@tauri-apps/api/event`, verify dispatch.
- [ ] **Commit:** `feat(ui): live event hook + migration toasts`

---

## Task 14 — Vitest + smoke

**Goal:** All component/hook/store tests green. Vitest config covers `src/**/*.{test,spec}.ts(x)`.

**Files:**
- Modify: `package.json` scripts (`"test": "vitest run"`, `"test:watch": "vitest"`)
- Create: `vitest.config.ts`, `src/test/setup.ts`

**Steps:**

- [ ] **14.1 — Vitest config**: jsdom env, alias `@`, setup file with `@testing-library/jest-dom` registration.
- [ ] **14.2 — Run all tests**: `pnpm test` green.
- [ ] **Commit:** `test(ui): vitest setup with jsdom + testing-library`

---

## Task 15 — Cleanup

**Goal:** Delete `src/main.ts` and old `index.html` markup. Drop unused vanilla CSS.

**Files:**
- Delete: `src/main.ts`
- Modify: `index.html` (final form: just root + script tag)

**Steps:**

- [ ] **15.1 — Verify nothing imports `main.ts`** (`grep -r "main.ts" src/`).
- [ ] **15.2 — Delete file** + remove any `<script>` tag in `index.html` that pointed at it.
- [ ] **15.3 — Final `pnpm build` + `npx tauri build --no-bundle`** + manual smoke (launch app, navigate every route).
- [ ] **Commit:** `refactor(ui): delete vanilla TS entrypoint after React cutover`

---

## Task 16 — Manual smoke + completion docs

**Goal:** Walk Bruno through the route matrix; on green, append completion section.

**Smoke matrix:**
- [ ] Wizard runs end-to-end on a fresh config (delete `app_meta.wizard_completed` row to test).
- [ ] Sidebar links navigate every primary route.
- [ ] Settings tabs navigate; one round-trip save+reload persists.
- [ ] Recording overlay animates on F24 dictation.
- [ ] History route shows dictation rows; FTS search returns hits.
- [ ] Dictionary CRUD round-trips.
- [ ] Snippets CRUD round-trips.
- [ ] Scratchpad add + pin round-trips.
- [ ] Ctrl+K opens command palette.
- [ ] Migration toasts fire on a fresh v1 config.

- [ ] **16.1 — Append completion section** like Phase 2 did.
- [ ] **16.2 — Push** to `wispr-parity`.
- [ ] **Commit:** `docs(phase-3): append completion status`

---

## Cutover summary

```
Layer 0 — Task 0          (scaffolding; old UI still works in parallel)
Layer 1 — Tasks 1, 2      (typed adapters + new Rust commands)
Layer 2 — Task 3          (app shell)
Layer 3 — Tasks 4..10     (routes — each is independent of the others)
Layer 4 — Task 11         (overlay rewrite)
Layer 5 — Task 12, 13     (palette + live events; can interleave with route tasks)
Layer 6 — Task 14         (vitest infra, can run early — preferred T0+1)
Layer 7 — Task 15, 16     (cleanup + smoke + push)
```

Tasks within Layer 3 are parallel-safe in principle, but the controller dispatches one at a time per the subagent-driven-development skill (single in-progress).

---

## Self-review notes

- **Spec coverage**: every primary nav route in master spec §3 → tasks 5..10. Settings tabs → T4. Overlay → T11. Tray menu items deferred to Phase 4 per sub-spec §1.2.
- **Non-trivial dep risk**: Tailwind v4 + shadcn canary. T0 step 0.10 forces a build, so any incompat surfaces immediately.
- **Settings v1 vs v3 nesting**: noted as a Phase 4 follow-up in T4.5; Phase 3 lives within v1 shape boundaries.
- **Type safety**: every Tauri command goes through `src/lib/tauri.ts` typed wrapper. No raw `invoke` calls in components.
- **Frontend test gate**: T14 runs Vitest; subsequent route tasks add their tests inline so the suite grows naturally.

## Completion Status - 2026-05-01

Implemented Phase 3 UI rewrite through the React shell, route matrix, v1 settings persistence UI, history search with virtualized rows, dictionary/snippet/scratchpad CRUD, first-run wizard gate, React overlay event listener, command palette, live event toasts, Vitest setup, and removal of the legacy vanilla entrypoint/CSS.

Verification:
- `npm run build` passed.
- `npm test` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml` passed.
- `npx tauri build --no-bundle` passed and produced `src-tauri/target/release/typr.exe`.

Known Phase 4 carry-forward:
- Settings tabs backed by fields not present in the v1 adapter remain informational until a nested v3 settings command is exposed.
- Command palette ships page navigation; deeper quick-add actions can be expanded once route-level dialogs are centralized.
