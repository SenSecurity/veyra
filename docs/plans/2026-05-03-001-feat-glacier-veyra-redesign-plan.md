---
title: "feat: Glacier Veyra redesign (sophisticated ice/cyan + spark)"
type: feat
status: active
date: 2026-05-03
origin: docs/mockups/08-glacier-veyra.html
---

# feat: Glacier Veyra redesign (sophisticated ice/cyan + spark)

## Overview

Apply the **Glacier** visual system from `docs/mockups/08-glacier-veyra.html` to the live Tauri+React app, replacing the current prototype-feel chrome (graphite titlebar, monochrome sidebar with `Ctrl+\` collapse, sky-blue/orange Home cards) with a sophisticated ice/cyan + spark amber palette over a light glass surface. The redesign covers the shared shell (titlebar, sidebar, page shell, panel, brand mark) and the Home route, preserves all Tauri IPC, settings, hotkeys, and routing, and presents the two engines (Whisper STT, Llama Drafter) as a first-class concept across the chrome.

The mockup is the visual contract. Code does not duplicate mockup HTML one-to-one — it adopts the system as Tailwind tokens + reusable React components.

---

## Problem Frame

Today's Home and shell do not match the Apple-style sophistication mandated in [AGENTS.md](../../AGENTS.md): the titlebar is dark graphite, the sidebar status panel exposes raw paths and "Local services running" copy, the engine concept is implicit (only one mic icon and an envelope), keyboard hints lean on `⌘` symbols on a Windows app, and the typography reads as generic dashboard. The user explicitly rejected: too dark, too childish, ⌘ everywhere, "%appdata%" exposed, logo repeated in too many surfaces. Mockup `08-glacier-veyra.html` was approved as the look-and-feel target after several iterations.

---

## Requirements Trace

- R1. Light surface across shell — titlebar, sidebar, main — driven by an ice/cyan palette with spark amber accent. No dark sidebar.
- R2. Two distinct engines surfaced everywhere: STT (Whisper · Turbo) + Drafter (Llama 3.2 · 1B), each with its own status LED, latency/spec, and accent color (cyan vs amber).
- R3. New Veyra logo (V waveform + spark) rendered as a stable component, used **only** in titlebar (small, ~22px) and the Home hero (~88px). Removed from action cards and primary buttons.
- R4. No `⌘` symbols in nav badges or button hints on the desktop app. The single literal `Ctrl K` palette hint stays because it is the actual binding.
- R5. Sidebar footer becomes a status block (Storage / Models / Status — "All systems nominal"). No `%appdata%`, no "100% local" copy, no raw filesystem paths.
- R6. Home page surfaces a 4-cell KPI strip (Sessions today, Words transcribed, Drafts composed, STT latency) backed by existing `getStatsTotals` / `getStatsStreak` IPC, with per-cell deltas where computable.
- R7. Engine cards on Home use a dark glossy "wave-stage" with the V-shape animated bars + spark — STT in cyan, Drafter in amber — plus a 3-cell `spec` row (Model / Latency-or-Quant / Language-or-Runtime).
- R8. Recent activity becomes a denser table-like list with tag (Dictation / Draft), word count, time, copy button. Same data source (`listTranscriptions`).
- R9. Typography pairs **Inter Tight** (display + body) with **JetBrains Mono Variable** (kbd / eyebrows) and **Newsreader** italic 300 used sparingly only for accent words inside larger headings.
- R10. Tauri IPC, hotkey behavior (`F24`, `Pause`, `Ctrl+K`, `Ctrl+\`), settings store, and routing are preserved. No backend changes.
- R11. Per AGENTS.md: validation includes `npm run build`, `npm test -- --run`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npx tauri build --no-bundle`, plus screenshot comparison against the approved mockup before shipping.

---

## Scope Boundaries

- **In scope:** Visual system tokens, brand-mark component, titlebar, sidebar, page shell + panel, Home route. Light visual touch-ups in shared primitives if their existing styles fight the new palette.
- **Out of scope:**
  - Rust / `whisper.cpp` / Ollama / overlay code.
  - Backend or schema changes; no new IPC surfaces.
  - Other routes (`history`, `email-drafts`, `dictionary`, `settings/*`, `wizard`) — they will inherit shell + token changes automatically; route-level redesign is deferred.
  - First-boot wizard chrome — already polished, only adopts new tokens passively.
  - Renaming `typr` internal filenames or any Cargo-side identifiers (AGENTS.md guardrail).

### Deferred to Follow-Up Work

- Per-route redesign for History, Email Drafter detail, Dictionary, Settings — separate plan once the shell lands and patterns are exercised.

---

## Context & Research

### Relevant Code and Patterns

- [src/routes/index.tsx](../../src/routes/index.tsx) — current Home implementation; uses `PageShell`, `Panel`, `veyra-glass`, `veyra-wave`, `veyra-wave-orange`, calls `ipc.getStatsTotals()`, `ipc.getStatsStreak()`, `ipc.listTranscriptions(12, 0)`. Replace inline JSX with token-driven components but keep the same IPC and copy semantics.
- [src/layout/window-titlebar.tsx](../../src/layout/window-titlebar.tsx) — current dark graphite titlebar. Becomes light glass with twin engines pill instead of generic recording pill in the secondary slot.
- [src/layout/sidebar.tsx](../../src/layout/sidebar.tsx) — already a light gradient sidebar with `Ctrl+\` collapse. Keep collapse logic and `useSettings` hook, replace `StatusLine` block with the dual-engine panel + storage status footer.
- [src/components/page-shell.tsx](../../src/components/page-shell.tsx) — `PageShell` and `Panel` shape stays; classes refreshed to match new tokens. Add an `eyebrow` prop on `PageShell` for the small mono caption (`Workspace · Home · …`) and an italic `accent` slot on titles.
- [src/components/brand-mark.tsx](../../src/components/brand-mark.tsx) — currently a `<img src=…veyra-icon.png>`. Reimplement as inline SVG matching the mockup's `#veyra-v` symbol so it scales crisply at 22px and 88px and respects theme.
- [src/components/recording-pill.tsx](../../src/components/recording-pill.tsx) — keep, but recolor LED to cyan; will be embedded inside the new engines pill on the titlebar in some states.
- [src/styles/tailwind.css](../../src/styles/tailwind.css) — single source of truth for `--primary`, `--sidebar`, `--accent`, `veyra-*` utility classes; extend rather than replace so other routes still render until they migrate.
- [src/components/stat-card.tsx](../../src/components/stat-card.tsx) — preferred base for the KPI strip; if its current shape is too constrained, the strip can render its own inline cells.

### Institutional Learnings

- `docs/plans/2026-05-03-production-apple-redesign.md` (status: completed) established the "Apple-style restraint, white-blue surface, graphite chrome" mandate. This plan extends that thesis: keep the restraint and depth, drop the graphite chrome from the main window in favor of light glass, formalize the engine duality.
- AGENTS.md guardrails carried forward verbatim: do not reintroduce removed bloat, do not rename `typr` internals, validate with screenshot diffing on material UI redesigns, every push to `main` must produce a fresh installer release.

### External References

- None required. Local design system + approved mockup is the contract.

---

## Key Technical Decisions

- **Tokens over hardcoded colors.** Add a Glacier theme layer to `src/styles/tailwind.css` (`--ice-50..400`, `--cyan / cyan-deep / cyan-soft`, `--spark / spark-deep`, `--halo`, `--hairline`, etc.) and re-point existing CSS variables (`--primary`, `--sidebar`, `--accent`) onto the new palette. Routes that don't migrate this pass still render coherently.
- **Brand mark as inline SVG, not PNG.** Removes the heavyweight glossy raster and gives crisp rendering at 22px and 88px. The PNG asset stays on disk for installer/Windows asset use (AGENTS.md "use the new Veyra icon … in app chrome and Windows assets") but the in-app component does not consume it.
- **Engine concept as a first-class component.** `EngineBadge` (titlebar twin-pill row) and `EngineCard` (sidebar dual-card panel) live next to each other so STT vs Drafter accent rules are encoded in one place.
- **No `⌘` on Windows desktop chrome.** Nav badges deleted; button hint pills only show literal Windows-side bindings (`F24`, `Pause`, `Ctrl K`).
- **Spark amber stays scoped to Drafter.** Keeps the cool/warm balance of the logo; orange does not leak into general chrome (no generic "warning" semantics overlap).
- **Wave-stage component is dark.** It is the only deliberately dark surface in the redesign — a "stage" for the live signal — and its darkness is contained to the card body.
- **No new IPC.** All KPIs are derived from `getStatsTotals`, `getStatsStreak`, and `listTranscriptions`. Latency and "drafts composed today" use existing fields where present; if a delta isn't computable yet, render the value without a delta rather than inventing one.

---

## Open Questions

### Resolved During Planning

- Is the dark Glacier titlebar staying? **No** — user explicitly rejected dark in this iteration. Light glass titlebar.
- Does `Ctrl+\` sidebar collapse survive? **Yes** — existing keybinding preserved verbatim.
- Where does the V waveform logo appear? **Titlebar (22px) and Home hero (88px) only.** Action cards switch to flat lucide-style icons (mic for STT chip, envelope for Drafter chip).

### Deferred to Implementation

- Whether `EngineBadge` should call live IPC (e.g., observed STT engine, last-known latency) or read from `useSettings` only. Default is settings-only; latency value can be wired later if a hook exists.
- Final Newsreader font ingestion: either `@fontsource/newsreader` package (preferred, matches existing `@fontsource-variable/geist` pattern) or Google Fonts CDN. Decide during U1.
- Whether the wave-stage animation runs always-on or only when `RecordingPill` is in `recording` state. Default: always-on idle bars when card is mounted; promotes to "live" amplitude when `ipc.recording` event is active.

---

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce.*

```
src/styles/tailwind.css
  └─ adds Glacier theme layer (ice + cyan + spark + hairline tokens)
       │
       ▼
src/components/brand-mark.tsx ── (rewritten as inline SVG <use #veyra-v>)
       │
       ├─ used in ──► src/layout/window-titlebar.tsx (22px, left)
       │                 │
       │                 ├─ light glass bg, EngineBadge twin-pill (center)
       │                 └─ window controls (right) — light variant
       │
       ├─ used in ──► src/routes/index.tsx <hero> (88px)
       │
src/components/engine-badge.tsx (new)        — twin-pill for titlebar
src/components/engine-card.tsx (new)         — sidebar dual-card panel
src/components/wave-stage.tsx (new)          — dark V-bars + spark canvas
src/components/kpi-strip.tsx (new)           — 4-cell KPI row
src/components/activity-row.tsx (new)        — denser recent-activity row

src/layout/sidebar.tsx
  ├─ replaces StatusLine block with <EngineCard /> stack
  └─ replaces footer "Local services running" with Storage/Models/Status

src/components/page-shell.tsx
  ├─ PageShell gains optional `eyebrow` prop ("Workspace · Home · …")
  └─ Panel gains optional `eyebrow` + accepts mock-style title with em-italic accent

src/routes/index.tsx
  └─ recomposes existing IPC data into:
        Hero(BrandMark + eyebrow + h1 + sub + actions)
     +  KpiStrip(totals, streak)
     +  EngineCard.STT(WaveStage cyan)
     +  EngineCard.Drafter(WaveStage spark)
     +  Panel "Recent activity"
          └─ ActivityRow[] mapped from listTranscriptions
```

---

## Implementation Units

- U1. **Glacier design tokens + typography**

**Goal:** Land the Glacier palette, hairlines, fonts, and `veyra-*` utility refresh in a single token pass so subsequent units can mount components without redefining colors.

**Requirements:** R1, R2, R7, R9.

**Dependencies:** none.

**Files:**
- Modify: `src/styles/tailwind.css`
- Modify: `src/styles/globals.css` (font-family stack)
- Modify: `package.json` (add `@fontsource-variable/inter-tight` + `@fontsource/newsreader` if Google Fonts route is rejected)

**Approach:**
- Add a Glacier theme layer below the existing `:root` block: `--ice-50..400`, `--cyan`, `--cyan-soft`, `--cyan-deep`, `--spark`, `--spark-deep`, `--spark-glow`, `--halo`, `--hairline`, `--hairline-2`, `--paper`, `--frost`. Mirror naming from the mockup.
- Re-point `--primary` → cyan, `--accent` → ice-50, `--sidebar` → ice-50→white gradient endpoint, `--ring` → cyan, keeping shadcn primitives consistent.
- Refresh `.veyra-glass`, `.veyra-command-panel`, `.veyra-wave` (cyan instead of sky-blue), `.veyra-wave-orange` → rename intent to `.veyra-wave-spark` while keeping the old class as a thin alias to avoid breaking `email-drafts`/`history` until they migrate.
- Add `--font-sans: 'Inter Tight Variable', …` and a `--font-display-italic: 'Newsreader', serif` token. Keep `--font-mono` on JetBrains Mono Variable as today.
- Document in a comment block which tokens are Glacier-system so future routes know what to pull.

**Patterns to follow:**
- Existing `@theme inline` block in `tailwind.css` already exposes shadcn vars; mirror that style.
- Existing `@fontsource-variable/geist` import pattern.

**Test scenarios:**
- Test expectation: none — pure styling and token plumbing. Visual verification covered in U6.

**Verification:**
- `npm run build` succeeds.
- `npm run dev` boots and the existing Home renders with shifted (cyan/ice) palette without layout breakage. No console warnings about missing fonts.

---

- U2. **Veyra V-mark SVG component**

**Goal:** Replace the PNG-backed `BrandMark` with a crisp inline SVG matching the mockup's `#veyra-v` symbol so the logo scales from 22px (titlebar) to 88px (hero) without aliasing and respects the theme.

**Requirements:** R3.

**Dependencies:** U1 (uses cyan / spark tokens).

**Files:**
- Modify: `src/components/brand-mark.tsx`
- Reference: `docs/mockups/08-glacier-veyra.html` (`<symbol id="veyra-v">` block)

**Approach:**
- Rewrite `BrandMark` to render an inline `<svg>` with two `<linearGradient>`s (`vbar` cyan stack, `spark` radial) and the V-bar geometry pulled verbatim from the mockup symbol.
- Keep the squircle shell as a CSS-painted parent (`bg`, `box-shadow` glossy highlights) so the asset works on top of any background.
- Keep the same prop signature (`className`, optional `imageClassName` becomes `svgClassName` or similar — only used internally).
- Keep `src/assets/veyra-icon.png` on disk for Windows installer assets; do not import it from this component anymore.
- Add `aria-hidden="true"` and `role="presentation"` on the SVG.

**Patterns to follow:**
- Other inline SVG icon usages already in the codebase (lucide-react).

**Test scenarios:**
- Happy path: BrandMark renders with role=presentation and contains a single `<svg>` element.
- Edge case: Renders at default size when no `className` is passed.
- Edge case: Accepts `className` and merges via `cn()` without losing default classes.

**Verification:**
- Visual diff at 22px (titlebar) and 88px (hero) shows V-shape + spark identical to mockup; no blurring.
- `npm test -- --run brand-mark` passes.

---

- U3. **Light glass titlebar + EngineBadge twin-pill**

**Goal:** Convert the graphite titlebar to the Glacier light-glass look and replace the bare command-palette button with an `EngineBadge` twin-pill that surfaces both engines (STT + Drafter), keeping every existing IPC binding (`windowMinimize`, `windowToggleMaximize`, `windowClose`, command-palette dispatcher).

**Requirements:** R1, R2, R3, R4.

**Dependencies:** U1, U2.

**Files:**
- Modify: `src/layout/window-titlebar.tsx`
- Create: `src/components/engine-badge.tsx`

**Approach:**
- Titlebar: swap dark gradient + zinc text for light-glass background (`--paper` + 20px backdrop blur), `--hairline` bottom border, dark ink text. Keep `data-tauri-drag-region` on the brand area. Window controls stay as lucide icons; tints become slate, close hover stays red.
- `BrandMark` rendered at `h-[22px] w-[22px]`.
- New `EngineBadge` component renders a single rounded container with two segments split by a hairline: each segment shows a colored LED + role label (`STT` / `Drafter` in mono uppercase) + engine name. Segment 1 = cyan LED + `Whisper · Turbo`; segment 2 = spark amber LED + `Llama 3.2 · 1B`. Names read from `useSettings` (`emailDraftEngine`, `whisperModel`); fall back to mockup defaults when settings are still loading.
- Keep the command-palette button to the right of the EngineBadge but render it in the light variant (white surface, slate text, mono `Ctrl K` keycap). Hide on `setupMode={true}` exactly like today.
- Preserve all `ipc.*` calls and the synthetic `Ctrl+K` keydown dispatch.

**Patterns to follow:**
- Existing `RecordingPill` for the LED + label idiom.
- `cn()` + tailwind class composition pattern used across `layout/`.

**Test scenarios:**
- Happy path: WindowTitleBar mounts, calls `ipc.windowMinimize` once when minimize clicked.
- Happy path: EngineBadge renders both segments with role labels "STT" and "Drafter".
- Edge case: When `useSettings` is still loading, EngineBadge falls back to default engine names without throwing.
- Edge case: `setupMode={true}` hides EngineBadge and command-palette button (parity with current behavior).
- Integration: Pressing the command-palette button dispatches a `keydown` Ctrl+K event on `window`.

**Verification:**
- Visual diff against mockup titlebar: light surface, twin-pill, mono keycap. Window-control hover states still functional on Windows.
- All IPC calls preserved; command palette opens via the button.

---

- U4. **Light sidebar with dual EngineCard + clean status footer**

**Goal:** Replace the current `StatusLine` strip and `Local services running` block in the sidebar with a Glacier-style nav (no `⌘` badges) + a dual `EngineCard` panel + a 3-line status footer (Storage / Models / Status). Preserve `Ctrl+\` collapse and version display behavior.

**Requirements:** R1, R2, R4, R5.

**Dependencies:** U1, U2.

**Files:**
- Modify: `src/layout/sidebar.tsx`
- Create: `src/components/engine-card.tsx`

**Approach:**
- Sidebar bg: `linear-gradient(180deg, var(--ice-50), white)` instead of the current sidebar mix; right border becomes a single `--hairline`. Width unchanged (182px expanded, 56px collapsed).
- Nav items: keep the same five entries and same `Link` + `activeProps` pattern, but drop any `⌘`/keyboard hint badges. Active state uses a 2px cyan left rule + white pill background, matching the mockup.
- Replace the current "Veyra ready / Local services running / StatusLine x3" block with `<EngineCard role="stt" …/>` + `<EngineCard role="drafter" …/>` stacked inside one bordered panel.
- `EngineCard` props: `role: "stt" | "drafter"`, `name`, `meta` (array of `{ label, value }`), `latencyMs?`. It owns the colored left rule (cyan or spark) and the LED. Reads sensible defaults from `useSettings` if not passed.
- Below the EngineCard panel: a three-row status block — `Storage` (placeholder static value pending an IPC; default "—" if no source), `Models` ("4 installed" derived from settings count if available, else "—"), `Status` ("All systems nominal" with a cyan LED). No `%appdata%`, no "Local services running".
- Collapsed mode hides the EngineCard panel and status block (same as today's status panel).
- Footer collapse button stays exactly as is.

**Patterns to follow:**
- Existing `useSettings` consumption.
- Existing `Ctrl+\` keydown handler (untouched).
- `cn()` composition style.

**Test scenarios:**
- Happy path: Sidebar renders five nav items with no `⌘` characters in the DOM.
- Happy path: Two `EngineCard`s render with role labels "STT" and "Drafter".
- Happy path: Status footer renders three rows: Storage, Models, Status. The "Status" line contains the literal "All systems nominal".
- Edge case: When collapsed, EngineCard panel and status footer are hidden; collapse button still visible.
- Edge case: Sidebar contains no `%appdata%`, no `Local services running`, no `Ctrl ⌘` glyphs.
- Integration: Pressing `Ctrl+\` toggles collapsed state (regression guard for existing behavior).

**Verification:**
- Visual diff against mockup sidebar.
- `localStorage` `typr.sidebar.collapsed` still updates (regression).

---

- U5. **PageShell + Panel: eyebrow + italic accent**

**Goal:** Extend `PageShell` and `Panel` so the new Home hero can render a mono eyebrow caption above the title and an italic Newsreader accent inside the title (e.g., `Boa tarde, Bruno. — quiet desk, ready when you are.` where the second clause is italic). All other routes that use these components keep working unchanged.

**Requirements:** R7, R9.

**Dependencies:** U1.

**Files:**
- Modify: `src/components/page-shell.tsx`

**Approach:**
- Add optional `eyebrow?: ReactNode` prop on both `PageShell` and `Panel`. Renders above the heading in `font-mono`, uppercase, tracking-wide, cyan. No-op when omitted.
- Allow the existing `title` prop to accept either a string (current behavior) or a `ReactNode`, so Home can pass a fragment with an italic `<em>` child without losing typing.
- Update class lists to align with the new tokens (hairline border, white surface, ice-tinted ambient gradient on `Panel`).
- Verify all current call sites compile — no breaking change.

**Patterns to follow:**
- Existing `cn()` and `ReactNode`-typed slot pattern in this file.

**Test scenarios:**
- Happy path: `PageShell title="Home"` renders today's exact DOM (regression).
- Happy path: `PageShell eyebrow={<>Workspace · Home · Saturday 03 May</>}` renders the eyebrow above the title.
- Happy path: `PageShell title={<>Boa tarde. <em>quiet desk.</em></>}` renders the `<em>` element inside the heading.
- Edge case: Existing routes (history, email-drafts, dictionary, settings) compile and render unchanged.

**Verification:**
- `npm test -- --run page-shell` passes.
- Type-check passes (`tsc -b` via `npm run build`).

---

- U6. **Home route: hero + KPI strip + dual EngineCard wave-stage + activity table**

**Goal:** Recompose `HomeRoute` to mount the Glacier hero, KPI strip, two engine cards with the dark V wave-stage (cyan / spark), and a denser recent-activity list — all reading from the existing IPC surface.

**Requirements:** R2, R3, R6, R7, R8, R9.

**Dependencies:** U1, U2, U3, U4, U5.

**Files:**
- Modify: `src/routes/index.tsx`
- Create: `src/components/wave-stage.tsx`
- Create: `src/components/kpi-strip.tsx`
- Create: `src/components/activity-row.tsx`
- Test: `src/routes/__tests__/index.test.tsx`

**Approach:**
- Hero block: `BrandMark` at 88px on the left, eyebrow `Workspace · Home · <date>` (locale-aware via `Intl.DateTimeFormat`), h1 with first sentence regular + second sentence italic Newsreader, sub-paragraph (one line, mono-kbd-friendly), and the existing `Start dictation` action wired to `ipc.toggleRecording()`. Drop `⌘N` keycap; secondary button reads `New session`.
- KPI strip: new `KpiStrip` component renders four cells:
  1. Sessions today — derived from `totals.sessionCount` (best available) and `streak.current`.
  2. Words transcribed — `totals.wordCount`.
  3. Drafts composed — count today's `Transcription` entries with `mode === "command"` from the slice already fetched (or call `listTranscriptions(50, 0)` and count today; cheaper than a new IPC).
  4. STT latency · p50 — placeholder rendered as `—` until a live source exists. Render the cell shape regardless so the strip stays balanced.
  Each cell can optionally show a delta line; only render the delta when it is computable, otherwise omit. No fake data.
- EngineCard.STT (cyan): card-head with eyebrow `01 · Capture · STT`, title `Speech to Text — Whisper` (italic accent), status pill, `action` row with mic icon (lucide `Mic`) + `Hold {dictationHotkey} to dictate` + keycap, then `WaveStage` (cyan), then `spec` row with three cells (Model / Latency / Language).
- EngineCard.Drafter (spark): card-head with eyebrow `02 · Compose · LLM`, title `Email Drafter — Llama` (italic accent), status pill, `action` row with envelope icon (lucide `Mail`) + `Hold {emailHotkey} to draft` + keycap, then `WaveStage` (spark variant), then `spec` row (Model / Quant / Runtime).
- `WaveStage`: dark squircle (`background: radial-gradient(...)` + inset hairlines) hosting:
  - 24 V-shape bars with predetermined heights matching the mockup (taller at edges, short in center) and `animation-delay` per index for the "amp" pulse.
  - A reflection layer (`scaleY(-1)`, opacity 0.32, blur 1px) below a thin cyan horizon line.
  - A central spark element (`::before` or absolute child) animated with `sparkFlicker`. Spark renders only when `variant="stt"` plus the DRafter variant; both keep parity with mockup.
  - `variant: "stt" | "drafter"` switches gradient stops cyan vs spark amber.
  - `live?: boolean` prop reserved for future amplitude wiring; for now, defaults to idle pulse.
- `ActivityRow`: 6-column grid (icon · text · tag · word-count · time · copy). Tag text is `Dictation` (cyan tint) when `row.mode !== "command"` else `Draft` (spark tint). Icon is `Mic` for dictations, `Mail` for drafts. Copy button preserves the existing `navigator.clipboard.writeText` + `toast` flow from the current Home.
- Continue calling `ipc.getStatsTotals()`, `ipc.getStatsStreak()`, `ipc.listTranscriptions(12, 0)` exactly as today; only the rendering changes. Keep `isUsableTranscription` filter.

**Patterns to follow:**
- Existing `useEffect` + `Promise.all` data-fetch pattern in current `HomeRoute`.
- `toast` + clipboard pattern in `copyRecent`.

**Test scenarios:**
- Happy path: `HomeRoute` mounts and calls all three IPC methods exactly once.
- Happy path: KPI strip renders four cells with labels "Sessions today", "Words transcribed", "Drafts composed", "STT latency".
- Happy path: With `recent` containing one `mode: "command"` row and one dictation row, two `ActivityRow`s render with tags "Draft" and "Dictation" respectively.
- Edge case: When IPC returns `null` totals/streak, KPI cells render `0` (or `—` for latency) without throwing.
- Edge case: When `recent` is empty, the activity panel renders the existing `EmptyState` ("No transcriptions yet"). Regression with current behavior.
- Edge case: Hero accent italic renders inside the h1, not as a separate paragraph.
- Edge case: No `⌘` glyph appears in the rendered Home DOM (regression guard for R4).
- Integration: Clicking the copy button on an `ActivityRow` writes the row's `finalText || rawText` to the clipboard and triggers a `toast.success("Copied")`. Mock `navigator.clipboard.writeText`.
- Integration: Clicking the hero "Start dictation" button invokes `ipc.toggleRecording()` once.

**Verification:**
- `npm test -- --run` passes.
- `npm run build` succeeds.
- `npx tauri build --no-bundle` succeeds on Windows.
- Screenshot of running app (Home route) compared side-by-side with `docs/mockups/08-glacier-veyra.html` rendered at the same window size shows: same V hero logo, same KPI strip composition, same engine card layout (cyan + spark wave-stages), same recent activity row shape. Per AGENTS.md "compare implementation screenshots against the mockups before shipping".
- Manual: `F24` still toggles dictation; `Pause` still triggers email draft; `Ctrl+K` still opens the command palette; `Ctrl+\` still collapses the sidebar.

---

## System-Wide Impact

- **Interaction graph:** No IPC channel added or removed. `ipc.toggleRecording`, `ipc.windowMinimize`, `ipc.windowToggleMaximize`, `ipc.windowClose`, `ipc.getStatsTotals`, `ipc.getStatsStreak`, `ipc.listTranscriptions` continue to be called from the same call sites with the same signatures.
- **Error propagation:** Existing `.catch(() => null)` / `.catch(() => [])` swallowing patterns in `HomeRoute` are preserved; KPI cells render `0` / `—` on failure rather than crashing.
- **State lifecycle risks:** `localStorage` key `typr.sidebar.collapsed` and the `Ctrl+\` keydown listener in the sidebar are preserved verbatim. No new persisted state.
- **API surface parity:** Other routes (`history`, `email-drafts`, `dictionary`, `settings/*`) inherit the new tokens via re-pointed CSS variables and refreshed `veyra-*` utility classes. They are expected to render coherently because only colors/borders shift; layouts are untouched.
- **Integration coverage:** The clipboard + toast flow on `ActivityRow` is exercised by an integration test mocking `navigator.clipboard.writeText`.
- **Unchanged invariants:** Hotkeys (`F24`, `Pause`, `Ctrl+K`, `Ctrl+\`), routing tree, settings store, internal `typr.*` filenames, Rust crate, and overlay window are explicitly not changed by this plan.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Re-pointing `--primary` and `--sidebar` tokens visually breaks unmigrated routes | U1 keeps backward-compatible `veyra-wave-orange` alias; spot-check `history`, `email-drafts`, `dictionary`, and `settings/general` in `npm run dev` before closing U1. |
| Newsreader font fails to load on Windows installer build | U1 prefers `@fontsource/newsreader` package over Google Fonts CDN; offline installs already do this for Geist. Fallback stack ends in `serif`. |
| Inline SVG `BrandMark` regresses places that import the PNG asset URL | Search for `veyra-icon.png` consumers before U2 lands; PNG asset is **kept** on disk and only the React component stops importing it. |
| Wave-stage animation drives CPU on idle window | Bars use pure CSS keyframes (no JS), 24 elements per card, 6 simultaneous animations max. Browser-side compositing on a Tauri webview is acceptable; revisit only if profiling shows >1% main-thread cost. |
| KPI cell latency value shipped as `—` looks unfinished | Acceptable per Open Question — it is the only cell without a real source. Do not invent latency values. Cell visually balanced via `—` placeholder. |
| Visual drift between mockup and real app due to React/shadcn primitives | AGENTS.md mandates screenshot comparison; U6's verification step explicitly requires it before shipping. |
| `npx tauri build` Windows access-denied while a Veyra process holds `typr.exe` | Documented in AGENTS.md; the PowerShell stop-process snippet is the recovery path. Not a code risk. |

---

## Documentation / Operational Notes

- After implementation: regenerate `docs/assets/veyra-hero.svg` (or equivalent) with a Glacier-styled hero shot if README image diverges.
- Per AGENTS.md "Mandatory Release Rule": every push to `main` that lands this redesign must be followed by a fresh GitHub Release with a Windows installer attached. Validation gate before release: `npm run build`, `npm test -- --run`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npx tauri build --no-bundle`, then `npm run tauri build`.
- Release notes should call out: "Home and shell redesigned with the new Glacier visual system. Two engines (Whisper + Llama Drafter) are now first-class in the chrome. No behavior changes."

---

## Sources & References

- **Origin mockup:** [docs/mockups/08-glacier-veyra.html](../mockups/08-glacier-veyra.html)
- **Project guardrails:** [AGENTS.md](../../AGENTS.md)
- **Prior redesign plan (completed):** [docs/plans/2026-05-03-production-apple-redesign.md](2026-05-03-production-apple-redesign.md)
- Related code: [src/routes/index.tsx](../../src/routes/index.tsx), [src/layout/window-titlebar.tsx](../../src/layout/window-titlebar.tsx), [src/layout/sidebar.tsx](../../src/layout/sidebar.tsx), [src/components/page-shell.tsx](../../src/components/page-shell.tsx), [src/components/brand-mark.tsx](../../src/components/brand-mark.tsx), [src/styles/tailwind.css](../../src/styles/tailwind.css)
