---
type: spec
phase: 3
parent: 2026-04-23-wispr-flow-parity-design.md
created: 2026-05-01
---

# Phase 3 — UI Rewrite (Sub-spec)

Frontend cutover from vanilla TS (`src/main.ts` ~200 lines, single `index.html`) to React 19 + Vite 6 + Tailwind v4 (`@theme`) + shadcn + tanstack-router code-based routes. Wires through every existing Tauri command exposed by Phase 1+2; UI is presentation only — no new Rust commands.

Master spec sections this implements: §3 (UI + UX), §6 React tree, §7.7 (UI rewrite), §8 (frontend tests).

## Section 1 — Scope

### In

- Remove `index.html` standalone markup; replace with React entrypoint mounted at root.
- Keep `src/overlay.html` separate window (Tauri spawns it); rewrite as a tiny React app reusing the same `@theme` tokens.
- Build out every primary route from §3 (Home, History, Dictionary, Snippets, Scratchpad) + every settings tab (General, Transcription, Hotkeys, Overlay, Formatting, System, Stats, Data, About).
- First-run wizard route (`/wizard`).
- Layout: 220px sidebar collapsible, top-bar, command palette (Ctrl+K).
- shadcn ui components: Button, Input, Select, Switch, Tabs, Dialog, Dropdown, Toast, Form (zod + react-hook-form), Table, Popover, Combobox, Slider.
- zustand stores: `settings-store`, `session-store`, `overlay-store`.
- Live event subscriptions: `recording-state`, `transcription:new`, `settings:*` migration toasts.
- Tray icon Tauri 2 built-in (Rust side; UI just exposes label/icon swap on state change — Phase 4 if Rust tray not wired).
- Frontend tests (Vitest) per §8: stores, hooks, key components, `fts-highlight.ts`.

### Out (Phase 4+)

- Compose Mode (master spec §13).
- Command Mode UI affordances (overlay tint, denylist editor) — Phase 4.
- Stats milestone toasts — Phase 4 (settings.stats.milestone_notifications already gated).
- Tray menu items beyond status (Phase 4).
- Dark/light theme polish at design level — ship system default + manual switch only.
- Accessibility audit (a11y deferred to Phase 5 polish).
- Multi-monitor overlay positioning beyond what V0 had.

## Section 2 — Stack decisions

| Choice | Pick | Why |
|---|---|---|
| React | 19 (stable) | Master spec §1.3; Suspense `use()`, `<form action>`, useFormStatus |
| Build | Vite 6 (already installed) | Skip migration |
| Router | tanstack-router code-based | Master spec §3 routes; not file-based — no convention magic |
| Styling | Tailwind v4 + `@theme` | Master spec §3.1 tokens already drafted |
| Components | shadcn (canary post-v4) | Reusable, themable, owned-by-us pattern |
| State | zustand | Master spec §6 stores; tiny + Tauri sync friendly |
| Forms | react-hook-form + zod | Settings forms have lots of fields, validation matters |
| Virtualization | @tanstack/react-virtual | History list can be 500k rows post-cap |
| Icons | lucide-react | Tree-shakable |
| Date | date-fns | Streak calc, group-by-day |
| Animation | framer-motion | Overlay state transitions, list mount fades |
| Cmd palette | cmdk | Battle-tested, integrates with shadcn |
| Tests | Vitest + @testing-library/react | Vite native |

### Dependency lock targets (rough — implementer picks compatible)

- `react@19`, `react-dom@19`
- `@tanstack/react-router@1`, `@tanstack/react-virtual@3`
- `tailwindcss@4`, `@tailwindcss/vite@4`
- `zustand@5`
- `react-hook-form@7`, `zod@3`, `@hookform/resolvers@3`
- `framer-motion@11`
- `cmdk@1`
- `date-fns@4`
- `lucide-react@latest`
- `vitest@2`, `@testing-library/react@16`, `@testing-library/jest-dom@6`, `jsdom@latest`
- shadcn CLI installed once for component scaffolding; components copied into `src/components/ui/`

## Section 3 — IPC adapter

`src/lib/tauri.ts` exposes typed wrappers around `@tauri-apps/api` `invoke`. One function per Rust command. All inputs/outputs flow through TS interfaces in `src/types/`.

Existing Tauri commands inventory (from `main.rs` post-Phase 2):

- `get_settings(): V1Settings` — adapter still returns v1 shape for backward compat. UI consumes v1; the adapter does v1↔v3 in Rust. Phase 4 may add a parallel `get_settings_v3()` if UI needs nested shape.
- `save_settings(settings: V1Settings): Result<()>` — same.
- `list_microphones(): MicDevice[]`
- `get_recording_state(): RecordingState` — Ready / Recording / Transcribing.
- `check_model_downloaded(modelSize: string): boolean`
- `download_model(modelSize: string): Result<()>` — emits progress events on `model:download:progress`.
- `toggle_recording(): Result<string>` — for clickable mic in UI.

**New commands needed (additive — written as part of Phase 3, not Phase 4):**

- `list_transcriptions(limit, offset): Transcription[]`
- `search_transcriptions(query, limit): Transcription[]`
- `delete_transcription(id): Result<()>`
- `list_dictionary(): DictionaryTerm[]`, `upsert_dictionary_term(term): Result<()>`, `delete_dictionary_term(id): Result<()>`
- `list_snippets(): Snippet[]`, `upsert_snippet`, `delete_snippet`
- `list_scratchpad_notes(): Note[]`, `upsert_scratchpad_note`, `delete_scratchpad_note`, `pin_scratchpad_note(id, pinned)`
- `get_stats_totals(): Totals`, `get_stats_streak(): StreakInfo`, `get_stats_by_day(): DailyStats[]`
- `wizard_status(): { completed: bool }`, `mark_wizard_complete(): Result<()>` — backed by `app_meta.wizard_completed`.
- `test_groq_key(key): Result<()>` — used by wizard step 3.

**Events** (subscribe via `@tauri-apps/api/event listen`):

- `recording-state` — payload: `RecordingState`
- `transcription:new` — payload: `{ rowId: number }`
- `settings:migrated` / `settings:model-remapped` / `settings:needs-groq-key` / `settings:migration-failed` — toasts from boot.
- `model:download:progress` — `{ modelSize, downloaded, total }`

## Section 4 — Layout

- `src/main.tsx` — mounts `<App />` into `#root`. RouterProvider, QueryClientProvider (if we use tanstack-query — yes, per master spec §6 hooks `use-transcriptions` is paginated).
- `src/app.tsx` — root layout: `<Sidebar />` `<TopBar />` `<Outlet />` plus `<Toaster />` and `<CommandPalette />`.
- `src/overlay/main.tsx` — separate Vite entry; Tauri config `frontendDist` already points `../dist` so we'll need two HTML entries (`index.html` + `overlay.html`) both pointing at React mounts.

### Vite multi-entry

`vite.config.ts` `build.rollupOptions.input = { main: "index.html", overlay: "overlay.html" }`. Each HTML loads a different `main.tsx` / `overlay-main.tsx`. Tauri's `tauri.conf.json` already references `src/overlay.html` for the overlay window — adjust to point at the built `overlay.html` in dist.

### Overlay state machine

Implement per master spec §3.5 — not as full FSM library, just a zustand store with `state: 'idle' | 'recording' | 'transcribing' | 'success' | 'error' | 'cancelled'` plus timers for the success/error flash auto-clear. Subscribed to `recording-state` event for upstream sync.

## Section 5 — Module tree (Phase 3 target)

```
src/
├── main.tsx
├── overlay-main.tsx
├── router.tsx
├── app.tsx
├── layout/
│   ├── sidebar.tsx
│   ├── topbar.tsx
│   └── command-palette.tsx
├── routes/
│   ├── index.tsx                  -- Home
│   ├── history.tsx
│   ├── dictionary.tsx
│   ├── snippets.tsx
│   ├── scratchpad.tsx
│   ├── wizard.tsx
│   └── settings/
│       ├── layout.tsx
│       ├── general.tsx
│       ├── transcription.tsx
│       ├── hotkeys.tsx
│       ├── overlay.tsx
│       ├── formatting.tsx
│       ├── system.tsx
│       ├── stats.tsx
│       ├── data.tsx
│       └── about.tsx
├── overlay/
│   ├── overlay-app.tsx
│   ├── pill.tsx
│   ├── bar.tsx
│   └── waveform.tsx
├── components/
│   ├── ui/                        -- shadcn
│   ├── stat-card.tsx
│   ├── streak-calendar.tsx
│   ├── transcription-row.tsx
│   ├── hotkey-input.tsx
│   ├── engine-picker.tsx
│   ├── model-picker.tsx
│   └── empty-state.tsx
├── stores/
│   ├── settings-store.ts
│   ├── session-store.ts
│   └── overlay-store.ts
├── lib/
│   ├── tauri.ts
│   ├── format-date.ts
│   ├── fts-highlight.ts
│   └── hotkey-utils.ts
├── hooks/
│   ├── use-settings.ts
│   ├── use-transcriptions.ts
│   ├── use-live-events.ts
│   └── use-shortcut.ts
├── styles/
│   ├── globals.css                -- @theme tokens
│   └── tailwind.css
└── types/
    ├── settings.ts
    ├── transcription.ts
    └── ipc.ts
```

`index.html` and `overlay.html` stay at repo root (Vite convention). Rust `main.rs::update_overlay` JS injection (`document.getElementById('mic').className = ...`) must keep working OR be replaced by an event the overlay subscribes to. Pick the latter — emits `overlay:state` from Rust, overlay listens.

## Section 6 — Migration strategy

Single-shot rewrite. The current `src/main.ts` is small (~200 lines) and tightly coupled to `index.html` IDs. Trying to keep it limping while React grows in parallel produces churn. Plan does:

1. Land scaffolding (deps, vite config, theme, base layout) on top of existing `index.html` entry, but neutered — just renders an empty React tree. Build still produces something Tauri can launch.
2. Migrate routes one-by-one, with each PR/task replacing one screen. Old `main.ts` shrinks per task; deleted at the last route's task.
3. Overlay rewrite is its own task — needs separate `overlay-main.tsx`, vite multi-entry, Tauri overlay window pointed at new build.

Each task ends with: `cargo build` green, `pnpm build` green, `npx tauri build --no-bundle` green, app launches, the migrated route works end-to-end.

## Section 7 — Test strategy

- **Vitest**: store unit tests, hook tests with `@tanstack/react-router` test utils, key component tests (transcription-row variants, hotkey-input keystroke capture, streak-calendar date math).
- **Mock Tauri**: `vi.mock('@tauri-apps/api/core')` returning fixture data so component tests don't need a real backend.
- **Manual smoke per task**: Bruno launches typr.exe, opens the migrated route, checks the user flow described in §3 of master spec.
- **No Playwright/E2E** for V1 (master spec §8 defers).

## Section 8 — Exit criteria

- Every route in §3 navigable (sidebar links work, settings tabs work).
- Settings sync: edit a field → save → reload → field persisted.
- Wizard: first-run flow runs end-to-end, can be re-opened from About.
- Dark/light parity: switching theme toggles all primary surfaces.
- Overlay: pill renders, recording → transcribing → idle transitions visible, mic-disconnect error shows toast and resets.
- Command palette: Ctrl+K opens, FTS history search works, page jumps work.
- All Vitest tests green.
- `cargo test` still green (Rust unaffected).
- `npx tauri build --no-bundle` produces a working exe.
- Bruno's manual run-through across at least: Home, History (with rows), Dictionary (CRUD), one settings tab, wizard.

## Section 9 — Risks

| Risk | Mitigation |
|---|---|
| Tailwind v4 + shadcn compat | shadcn-ui 2.x supports v4; pin to known-good. If breakage, downgrade to v3 + revisit late Phase 3. |
| tanstack-router learning curve | Code-based routes are simpler than file-based; plan tasks small enough to absorb friction. |
| Multi-entry Vite + Tauri overlay path | Audit dist/ output paths early in scaffolding task; tauri.conf.json `frontendDist` and overlay window URL must agree. |
| Existing v1 settings shape vs v3 nested in UI | Phase 3 keeps v1 shape via adapter. UI consumes flat fields. Plan does NOT remap UI to nested shape — that's Phase 4 cleanup. |
| Frontend test infra | First task scaffolds Vitest; if green, all later tasks add tests as they go. |

## Section 10 — Carry-forward into Phase 4+

- Master spec §12 (Command Mode), §13 (Compose Mode) both need overlay + history UI extensions.
- Tray menu items beyond status — needs Rust side too.
- LLM Enhance (`enhanceEnabled=true`) needs settings UI + pipeline integration.
- App context capture (Phase 4 fills `app_context` field) — UI displays in History row.
- Milestone toasts (`stats.milestone_notifications`) — Phase 4 wires consumer.
- Frontend a11y audit, dark mode design polish, motion-reduce respect — Phase 5.
