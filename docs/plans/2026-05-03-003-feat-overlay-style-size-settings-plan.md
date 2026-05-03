---
title: "feat: Overlay style + size settings (Capsule / Halo Orb, S/M/L) and Home wave shrink"
type: feat
status: active
date: 2026-05-03
origin: docs/mockups/overlay-01-capsule.html, docs/mockups/overlay-03-halo-orb.html
---

# feat: Overlay style + size settings (Capsule / Halo Orb, S/M/L) and Home wave shrink

## Overview

Two related changes that ride together so the Home and overlay surfaces stay in sync:

1. **Shrink the Home engine-card waveforms.** The current 64 px tall ice-tinted strip reads as a panel rather than a discreet signal hint. Drop to ~32–36 px with tighter bar density so the strip becomes a quiet decoration under the action button rather than the dominant element of each card.
2. **Let users pick the recording overlay's visual style and size in Settings.** Today the overlay is hard-wired to the 560×96 light-glass capsule from `docs/plans/2026-05-03-002-feat-glacier-overlay-capsule-plan.md`. Add a new `Overlay` tab in Settings with two style options — **Capsule** (current capsule) and **Halo Orb** (small floating orb with concentric rings, derived from `docs/mockups/overlay-03-halo-orb.html`) — plus a Small / Medium / Large size selector that scales the chosen style. Settings persist via the existing v1 settings JSON, the Tauri overlay window resizes itself when the choice changes, and the React overlay app routes between the two components.

---

## Problem Frame

The Glacier redesign shipped a single overlay shape (capsule). On wider monitors or for users who prefer a smaller footprint, the capsule can feel either oversized or unnecessarily horizontal. The orb mockup (`overlay-03-halo-orb.html`) was approved as a viable alternate aesthetic during the overlay design pass; surfacing it as a user choice closes the loop. The Home wave-strip shrink is a visual nit observed live in the running app: with the spec row directly below it, the 64 px wave-stage carries too much weight relative to the rest of the engine card.

---

## Requirements Trace

- R1. Home `WaveStage` strip renders at ~32–36 px height with tighter bar density; the rest of the Home composition is unchanged.
- R2. A new TypeScript `Settings` field `overlayStyle: "capsule" | "orb"` is persisted via the existing v1 settings JSON, defaulting to `"capsule"`.
- R3. A new TypeScript `Settings` field `overlaySize: "small" | "medium" | "large"` is persisted alongside `overlayStyle`, defaulting to `"medium"` (the current capsule's 560×96 dimensions).
- R4. A new `HaloOrb` overlay component renders the orb mockup (V brand mark inside a 96 px squircle, concentric pulsing rings tinted by mode, timer chip below, hover-expand transcript bubble) with the same lifecycle hooks the capsule uses (`state`, `mode`, `recordingStartedAt`, voice level).
- R5. `OverlayApp` reads `overlayStyle` from settings and renders either the capsule or the orb. The existing IPC plumbing (`overlay:state`, `overlay:mode`, `overlay:level`) drives both equally.
- R6. The Tauri overlay window resizes itself whenever `overlayStyle` or `overlaySize` changes, snapping to the new size and re-centering against the work-area bottom margin. No window flicker on settings save.
- R7. A new Settings tab `Overlay` renders a 2-up radio card group (Capsule vs Halo Orb) with a thumbnail/preview of each option, plus a 3-segment Small / Medium / Large size selector. Both controls are immediately persisted via the existing `ipc.saveSettings` flow (no separate "Apply" button).
- R8. The recording lifecycle remains unchanged: overlay appears while recording/transcribing and disappears when work finishes (per AGENTS.md).
- R9. Existing Capsule-related tests (`pill.test.ts`, `capsule.test.tsx`, `overlay-store.test.ts`) continue to pass without modification — the orb is additive.

---

## Scope Boundaries

- **In scope:** Home `WaveStage` height/density refresh, `Settings` type extension (TS + Rust), new Settings UI tab, new `HaloOrb` React component + tests, `OverlayApp` router, Tauri overlay window resize plumbing.
- **Out of scope:**
  - Custom user position (drag-to-reposition) for the overlay — bottom-center anchoring stays.
  - Per-engine style/size (e.g., capsule for STT, orb for Drafter) — global setting only.
  - Animated transitions between styles when the user changes the setting mid-recording — settings change fully takes effect on the next `idle → recording` cycle if the user is mid-session; the live overlay snaps without animation.
  - Onboarding wizard exposure — the overlay tab is reachable only via the regular Settings shell.
  - Per-monitor preferences when users plug in a second display.
  - Live transcript bubble inside the orb beyond the static mockup hover state — would need a new IPC channel (deferred per the prior overlay capsule plan).

### Deferred to Follow-Up Work

- Drag-to-reposition the overlay window — separate UX exploration; not blocked by this plan.
- Stage-Card style (`docs/mockups/overlay-02-stage-card.html`) — not requested; deliberately not introduced as a third option.

---

## Context & Research

### Relevant Code and Patterns

- [src/components/wave-stage.tsx](../../src/components/wave-stage.tsx) — current 64 px ice-tinted strip; refresh in U1.
- [src/styles/tailwind.css](../../src/styles/tailwind.css) — `.veyra-wave` + `.veyra-wave-spark` utilities own the height/gap/bar styling; tweak in U1 alongside `.veyra-wave-amp` keyframe.
- [src/types/settings.ts](../../src/types/settings.ts) — `Settings` interface; current 9 fields. Plan adds 2.
- [src-tauri/src/settings/legacy_v1.rs](../../src-tauri/src/settings/legacy_v1.rs) — Rust mirror of the settings struct with `serde(rename = "...")` directives. Adds 2 new fields with `#[serde(default = "...")]` so existing `config.json` files keep loading.
- [src-tauri/src/main.rs](../../src-tauri/src/main.rs) — `OVERLAY_WIDTH` / `OVERLAY_HEIGHT` / `OVERLAY_BOTTOM_MARGIN` constants (today: 560 / 96 / 12); `update_overlay` helper that calls `overlay.set_position`. Plan converts the dimensions into a lookup table indexed by `(style, size)` and adds a new Tauri command `set_overlay_layout` that reads the current settings and resizes the window.
- [src/overlay/pill.tsx](../../src/overlay/pill.tsx) — capsule shell. Already accepts `state` and `mode`; will additionally accept `size` to scale the existing 520-wide capsule into 420 / 520 / 640.
- [src/overlay/overlay-app.tsx](../../src/overlay/overlay-app.tsx) — current renderer; switches to a router branch after U4.
- [src/routes/settings/layout.tsx](../../src/routes/settings/layout.tsx) — settings tab strip; add the `Overlay` tab here.
- [src/router.tsx](../../src/router.tsx) — registers the settings sub-routes; add the new route file.
- [docs/mockups/overlay-01-capsule.html](../mockups/overlay-01-capsule.html) — capsule visual contract.
- [docs/mockups/overlay-03-halo-orb.html](../mockups/overlay-03-halo-orb.html) — orb visual contract; 96 px squircle, three concentric rings, transcript bubble on hover, timer chip below.

### Institutional Learnings

- The completed Glacier redesign (`docs/plans/2026-05-03-001-feat-glacier-veyra-redesign-plan.md`) and overlay capsule plan (`docs/plans/2026-05-03-002-feat-glacier-overlay-capsule-plan.md`) established the cyan/spark engine duality, the light-glass surface recipe, and the `recordingStartedAt` store slice. The orb component reuses those primitives wholesale — no new tokens, no new state.
- AGENTS.md mandates that `typr` filenames stay untouched. The new orb component lives at `src/overlay/halo-orb.tsx` (a new file under the existing `src/overlay/` directory) so no rename is required.

### External References

- None required. Local design system + the two approved mockups is the contract.

---

## Key Technical Decisions

- **Single overlay window, swappable React tree.** Keep the existing `overlay` Tauri webview and route between Capsule/Orb in React rather than spawning a second window. Simpler IPC plumbing, no double-bind on `overlay:state`.
- **Window dimensions from a 6-cell table.** A `(style, size) → (width, height)` lookup lives once in Rust and once mirrored in TS so the React side can size its outer container correctly while Rust authoritatively resizes the OS window. Both sides default to Capsule × Medium = 560×96 to preserve current behavior.
- **Rust-driven resize via a new IPC command.** Add `set_overlay_layout(style, size)` invoked from `saveSettings` whenever those fields change. Rust looks up dimensions, calls `overlay.set_size` + recomputes `set_position` against the work area. No new event channel needed — the React side already knows the values; Rust just enforces them on the OS window.
- **`overlaySize` as a coarse 3-step enum, not a numeric slider.** Discrete sizes keep the UI simple, lock the dimensions to ones we've actually tested, and avoid the case where a user types `2000×4000` and breaks anchoring.
- **Capsule scaling is purely CSS.** The capsule already centers its content via grid; widening from 420 to 640 only changes `width`, the inner column ratios stay. Bar count stays at 40 across all sizes — denser at Small, looser at Large — which keeps the visual rhythm consistent without per-size code.
- **Orb scaling is the squircle diameter.** Small = 72 px, Medium = 96 px, Large = 128 px. Ring sizes scale with the squircle; the timer chip + hover bubble keep their minimum legible sizes regardless of orb size.
- **Default = `capsule + medium`.** Existing users keep the exact same visual on first launch after upgrade. New `serde(default = "...")` attributes on the Rust struct ensure stale `config.json` files keep loading.
- **Home wave shrink is decoupled.** `WaveStage` height moves from `h-16` (64 px) to `h-9` (36 px); bar gap drops from 2 px to 1 px; bar count stays at 40. Independent of the overlay work; lands as its own commit so it can ship on its own if the overlay surface needs more iteration.

---

## Open Questions

### Resolved During Planning

- Should the new fields live in the Rust v1 struct (`legacy_v1.rs`) or the v2 schema? **v1.** That is what the React app reads via `to_v1_view` and writes via `apply_v1_v2_fields`; the v2 schema mirrors v1 and is plumbed through the same adapter. Adding to v1 is the minimum that reaches the user.
- Should we expose the orb's hover-bubble in v1? **No.** The hover bubble in `overlay-03-halo-orb.html` is mockup-only; without a streaming transcript IPC channel the bubble has no real content to show. Render the orb without the bubble for v1; the timer chip below the orb covers the "what's happening" question.
- Does Tauri 2 support live `set_size` on a transparent always-on-top window? **Yes.** The existing code already calls `overlay.set_position(...)` while the window is visible; `set_size` follows the same `WebviewWindow` API.

### Deferred to Implementation

- Whether the size selector reads as `Small / Medium / Large` or as the literal pixel values (e.g., `420 / 520 / 640`). Default: human labels, with the dimension shown as small caption text under each segment.
- Exact CSS rules for the orb's hover-bubble (opacity, transition timing) — drive from the mockup but tune during execution.
- Whether `set_overlay_layout` should be called eagerly on every save or only when the layout fields actually change. Default: only when changed (cheap diff in Rust).

---

## Implementation Units

- U1. **Shrink Home `WaveStage` to a discreet strip**

**Goal:** Drop the Home engine-card waveform from 64 px to ~36 px, halve the bar gap, and keep the existing cyan/spark tone differentiation. The wave reads as quiet decoration; the spec row below it becomes the dominant numeric content.

**Requirements:** R1.

**Dependencies:** none.

**Files:**
- Modify: `src/components/wave-stage.tsx`
- Modify: `src/styles/tailwind.css` (`.veyra-wave` height/gap; review `.veyra-wave-amp` keyframe so the smaller bars still read animated, not static)

**Approach:**
- Drop the `h-16` className from the inner wave element to `h-9` (36 px) and remove the bottom meta-row remnant if any leaked back during U2's testing.
- Reduce `.veyra-wave` `gap` from 2 px to 1 px and tighten `padding-inline` so the wave still spans the card width edge-to-edge.
- Verify the bars still read at 36 px on Windows DPI-scaled displays (100 % / 125 % / 150 %).

**Patterns to follow:**
- Existing `.veyra-wave` token recipe; don't introduce a parallel utility.

**Test scenarios:**
- Test expectation: none — pure styling. Visual verification: launch the app, compare the Home cards against the mockup ratio and confirm the wave reads as a strip rather than a panel.

**Verification:**
- Manual: Home cards render with a discreet wave under the action button, spec row remains visually dominant.

---

- U2. **Extend `Settings` with `overlayStyle` + `overlaySize` (TS + Rust + adapter)**

**Goal:** Introduce the two new persisted fields end-to-end so the React app, the Rust settings store, the legacy v1 JSON file, and the v2 schema all agree.

**Requirements:** R2, R3.

**Dependencies:** none.

**Files:**
- Modify: `src/types/settings.ts`
- Modify: `src-tauri/src/settings/legacy_v1.rs`
- Modify: `src-tauri/src/settings/schema.rs` (mirror in v2 if v2 stores these; otherwise leave to adapter)
- Modify: `src-tauri/src/settings/adapter.rs` (`to_v1_view` + `apply_v1_v2_fields` carry the two new fields through)
- Test: `src-tauri/src/settings/legacy_v1.rs` (existing test module — extend the round-trip case to cover the new defaults)

**Approach:**
- TS: add `overlayStyle: "capsule" | "orb"` and `overlaySize: "small" | "medium" | "large"` to the `Settings` interface. Both narrow unions so misuse is caught at the type level.
- Rust v1: add `overlay_style: String` (`#[serde(rename = "overlayStyle", default = "default_overlay_style")]`) and `overlay_size: String` (`#[serde(rename = "overlaySize", default = "default_overlay_size")]`) with default helpers returning `"capsule"` / `"medium"`.
- Adapter: extend `to_v1_view` and `apply_v1_v2_fields` so changes flow both ways. If v2 doesn't have a matching field, store the value on a v2 sub-struct (`appearance` or similar) created during this unit.
- Settings store + `useSettings` hook do not need changes — they read/write whatever the v1 view exposes.

**Patterns to follow:**
- Existing `commandHotkey` field (added similarly with `default_command_hotkey`) is the canonical example of a `#[serde(default)]`-backed v1 extension.

**Test scenarios:**
- Happy path: Loading a `config.json` that includes `overlayStyle: "orb"` produces a `Settings` with that value.
- Edge case: Loading a `config.json` that omits both fields (legacy file) produces `overlayStyle: "capsule"` and `overlaySize: "medium"`.
- Edge case: Saving and re-loading a `Settings` with `overlayStyle: "orb", overlaySize: "large"` round-trips identically.
- Integration: `to_v1_view(apply_v1_v2_fields(v2_default, v1_view))` is idempotent for any combination of the two fields.

**Verification:**
- `cargo test --manifest-path src-tauri/Cargo.toml` passes; new defaults do not break the existing `legacy_v1` round-trip test.
- TypeScript build (`npm run build`) compiles with no widening errors anywhere `Settings` is consumed.

---

- U3. **`HaloOrb` overlay component**

**Goal:** New React component `src/overlay/halo-orb.tsx` that renders the orb shape from `docs/mockups/overlay-03-halo-orb.html` against the same store slices the capsule uses. Three concentric rings pulsing in sequence while recording, single dashed shimmer ring while transcribing, no rings while idle. Cyan tone for STT, spark for Drafter. Timer chip below the orb. Hover bubble deferred per Open Questions.

**Requirements:** R4.

**Dependencies:** U2 (so `overlaySize` is available on the settings type when sizing).

**Files:**
- Create: `src/overlay/halo-orb.tsx`
- Modify: `src/styles/tailwind.css` (add `.veyra-orb`, `.veyra-orb-ring`, `.veyra-orb-chip` utilities + keyframes for ring pulse and orb breathe)
- Test: `src/overlay/halo-orb.test.tsx` (new)

**Approach:**
- Component signature mirrors `OverlayPill`: `({ state, mode, size })` where `size: "small" | "medium" | "large"`. Reads `recordingStartedAt` and `level` from `useOverlayStore` and renders the elapsed timer via the same `useElapsedLabel` hook (extract it into `src/overlay/use-elapsed-label.ts` so both components share the source).
- Visual: a squircle (V brand mark inline SVG, reused from `src/components/brand-mark.tsx`) with three absolutely-positioned `.veyra-orb-ring` elements ticking out via `animation-delay: 0s / 0.6s / 1.2s` while recording. Tone follows `data-mode="stt"` / `data-mode="drafter"`.
- Transcribing: collapse to a single dashed ring rotating around the squircle (`animation: shimmerSpin 1.6s linear infinite`).
- Idle: no rings, slower breathe.
- Timer chip is a small light-glass pill anchored to the orb's bottom; the keycap-style hotkey ID inside the chip uses the same `useSettings()` lookup the capsule uses.
- Sizes:
  - `small` → 72 px squircle, ring radii 100 / 130 / 160.
  - `medium` → 96 px squircle, ring radii 130 / 170 / 210 (matches the mockup).
  - `large` → 128 px squircle, ring radii 170 / 220 / 270.

**Patterns to follow:**
- `src/overlay/pill.tsx` for the React shape (memoized values, `useElapsedLabel`, `useEffect` cleanup).
- `src/components/brand-mark.tsx` for the V SVG (reuse the existing component; don't duplicate the gradients).
- `.veyra-capsule` utility recipe in `tailwind.css` for the light-glass aesthetic of the timer chip.

**Test scenarios:**
- Happy path: Rendering with `state="recording"` + `mode="dictation"` shows three concentric rings (`role="presentation"` elements) and a cyan-tinted squircle.
- Happy path: Rendering with `state="recording"` + `mode="command"` shows the same rings but with the spark-amber tint hooks (`data-mode="drafter"`).
- Happy path: Timer chip text is the `mm:ss.t` formatter output sourced from `recordingStartedAt`.
- Edge case: Rendering with `state="transcribing"` collapses the three rings into a single dashed ring.
- Edge case: Rendering with `state="idle"` removes the rings entirely.
- Edge case: Each `size` value renders the squircle at its expected diameter (assert `data-size` attribute or computed inline style).

**Verification:**
- `npm test -- --run src/overlay/halo-orb.test.tsx` passes.
- Visual diff against `docs/mockups/overlay-03-halo-orb.html` confirms ring spacing and tones.

---

- U4. **`OverlayApp` router + Tauri `set_overlay_layout` command**

**Goal:** Pick the right component (capsule or orb) based on settings, and resize the OS window when the user changes the choice. Existing IPC events drive both components; the user never sees the window flicker on save.

**Requirements:** R5, R6, R8.

**Dependencies:** U2, U3.

**Files:**
- Modify: `src/overlay/overlay-app.tsx` (router; subscribe to settings changes via the store)
- Modify: `src/lib/tauri.ts` (add `setOverlayLayout(style, size)` IPC wrapper)
- Modify: `src-tauri/src/main.rs` (replace the three `OVERLAY_WIDTH/HEIGHT/BOTTOM_MARGIN` constants with a `(style, size) → (width, height)` lookup; add the `set_overlay_layout` Tauri command; call it from `save_settings` when those fields change)
- Modify: `src/stores/settings-store.ts` (after a successful `save`, call `ipc.setOverlayLayout` if the relevant fields changed)
- Test: `src-tauri/src/main.rs` (Rust unit test for the lookup table — every (style, size) yields a positive (w, h) and unique-per-style values)

**Approach:**
- TS-side: `OverlayApp` subscribes to `useSettings()` and passes `style + size` down. Renders `<OverlayPill size={size} />` or `<HaloOrb size={size} />`. Capsule accepts a new `size` prop in U4 (no behavior change beyond a width class).
- Rust-side: a single `fn overlay_dims(style: &str, size: &str) -> (i32, i32)` returning the lookup. The bottom-margin stays a single constant (does not vary per style/size).
- New Tauri command `set_overlay_layout(style: String, size: String)` looks up the dimensions, calls `overlay.set_size(LogicalSize::new(w, h))` and `overlay.set_position(...)` against the work area. Skips no-op transitions if the window is already at the target size.
- `save_settings` invokes `set_overlay_layout(...)` whenever `overlay_style` or `overlay_size` differs between previous and incoming v2 snapshots. Avoids resizing on every unrelated save.
- The settings store pings the same command on the React side so a manual UI change propagates without waiting for a subsequent save round-trip — belt-and-braces.

**Patterns to follow:**
- Existing Tauri command idiom in `src-tauri/src/main.rs` (`#[tauri::command] fn save_settings(...)`)
- Existing `WebviewWindow::set_position` call in `update_overlay`.

**Test scenarios:**
- Happy path: Calling `set_overlay_layout("capsule", "medium")` resizes the overlay to 560×96 and re-centers against the work-area bottom margin. (Manual; cargo cannot drive a webview.)
- Happy path: Switching from `capsule + medium` to `orb + small` resizes to 72×112 (squircle + chip allowance) and re-centers.
- Edge case: Calling `set_overlay_layout` with unknown values returns `Err(...)` and leaves the window untouched.
- Edge case: Saving settings with no change in `overlayStyle`/`overlaySize` does NOT trigger a resize.
- Integration: With the overlay window currently visible, switching style mid-session changes the rendered React tree on the next state event without losing the recording session (manual).
- Unit (Rust): `overlay_dims` returns a positive `(w, h)` for every valid `(style, size)` combination and `(0, 0)` (or panic) for invalid input. Pin the table.

**Verification:**
- `npm test -- --run` and `cargo test --manifest-path src-tauri/Cargo.toml` pass.
- `npx tauri build --no-bundle` succeeds.
- Manual: Settings → Overlay → toggle Capsule/Orb and S/M/L. The overlay window snaps to the new size and re-centers without flicker; the next dictation session shows the chosen style.

---

- U5. **Settings UI: new `Overlay` tab with style cards + size selector**

**Goal:** Surface the two new settings via an `Overlay` tab in the existing Settings shell. 2-up radio cards for style with a small preview thumbnail of each option; a 3-segment Small / Medium / Large selector for size. Changes persist immediately via the existing `ipc.saveSettings` pipeline.

**Requirements:** R7.

**Dependencies:** U2, U4.

**Files:**
- Create: `src/routes/settings/overlay.tsx`
- Modify: `src/routes/settings/layout.tsx` (add the new tab to the strip)
- Modify: `src/router.tsx` (register the new sub-route)
- Test: `src/routes/settings/overlay.test.tsx` (new)

**Approach:**
- Tab strip ordering: `General · Transcription · Hotkeys · Overlay`.
- Style card: rounded panel containing the option name, a one-line description, and a small inline-SVG preview (~120×60 capsule SVG; ~80×80 orb SVG with a single ring). Selected card shows a 2 px cyan left rule + ring.
- Size selector: a 3-segment ghost button group (Small / Medium / Large) with the literal pixel size shown as a small caption under the active label (e.g. `560 × 96` for Capsule × Medium).
- On change → call the existing `useSettings().update({ overlayStyle, overlaySize })` mutation; the settings store's `save` flow already triggers `ipc.saveSettings`. After the save resolves, the store calls `ipc.setOverlayLayout` (added in U4) to enforce the OS-window resize.
- A small "Preview" inline panel under the selectors renders the chosen overlay in a frozen `state="idle"`, so the user can see the picked style without having to start a dictation.

**Patterns to follow:**
- `src/routes/settings/general.tsx` and `src/routes/settings/transcription.tsx` for the overall route layout, control patterns, and copy density.
- `src/components/page-shell.tsx` `Panel` for grouped sections.
- `src/components/ui/button.tsx` for the segmented size selector.

**Test scenarios:**
- Happy path: Mounting the route renders both style cards and three size segments, with the current `overlayStyle` and `overlaySize` from `useSettings` reflected as selected.
- Happy path: Clicking the Halo Orb card calls `useSettings().update({ overlayStyle: "orb" })` exactly once.
- Happy path: Clicking `Large` calls `useSettings().update({ overlaySize: "large" })` exactly once.
- Edge case: While `useSettings` is still loading (`settings === null`), the route renders skeleton placeholders rather than throwing.
- Integration: After the user selects a new style, the `Preview` panel re-renders with the new component (capsule ↔ orb).

**Verification:**
- `npm test -- --run src/routes/settings/overlay.test.tsx` passes.
- Manual: open Settings → Overlay; toggle the four combinations and confirm the live overlay reflects the choice on the next dictation.

---

## System-Wide Impact

- **Interaction graph:** Adds one new Tauri command (`set_overlay_layout`) and one new IPC wrapper (`ipc.setOverlayLayout`). The settings save pipeline gains a side effect that resizes the overlay window — only when the relevant fields change.
- **Error propagation:** `set_overlay_layout` errors (unknown enum values, missing webview) bubble up as `Result<_, String>` via the existing Tauri command boilerplate; the React caller swallows them with `.catch(() => {})` mirroring the existing pattern in `OverlayApp`.
- **State lifecycle risks:** Switching style mid-session must not corrupt the React store's `recordingStartedAt`. Since both components read the same store, the swap is safe — but verify manually that the timer keeps running across the swap.
- **API surface parity:** None — the new fields are purely client-side configuration. No external API contracts touched.
- **Integration coverage:** The Rust round-trip test covers the persistence path; the manual smoke test covers the live OS-window resize.
- **Unchanged invariants:** Tauri overlay window flags (`transparent`, `decorations(false)`, `always_on_top(true)`, `skip_taskbar(true)`, `focused(false)`, `shadow(false)`); the `RecordingState` Rust enum; the audio capture pipeline; the existing `overlay:state` / `overlay:mode` / `overlay:level` event channels; the Home composition outside `WaveStage`.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Resizing the overlay mid-recording causes a visual stutter | The `set_size` + `set_position` calls are batched; the always-on-top transparent webview redraws within one frame. Manual QA covers this; if reviewers see flicker, fall back to "apply on next dictation" as a stop-gap. |
| Stale `config.json` files without the new fields fail to load | New fields use `#[serde(default = "...")]` returning `"capsule"` / `"medium"`. The Rust `legacy_v1` round-trip test pins this. |
| `overlayStyle` / `overlaySize` strings drift between TS and Rust enums | Both sides parse the same narrow set of strings and reject anything outside it. Plan adds a Rust unit test for `overlay_dims` that fails if the table loses any combination. |
| Halo Orb hover-bubble looks dead without a transcript | Bubble is intentionally deferred (Open Questions). The orb without bubble is a complete v1 — the timer chip below the orb covers the "what's happening" question. |
| Home `WaveStage` shrink hides the cyan/spark distinction at 36 px | Bars still render with the same gradient; only height changes. Manual visual check confirms differentiation reads. If not, revisit gap and bar width before bumping height back. |
| Settings UI add introduces a fourth tab that overflows on small windows | The tab strip already uses `overflow-x-auto`. Manual QA at the minimum window size (700×500 from `tauri.conf.json`) confirms the new tab fits; if not, drop the labels to icons-only at narrow widths. |

---

## Documentation / Operational Notes

- After implementation: regenerate any Settings screenshots referenced from `docs/design/implementation/`; the existing references show only General/Transcription/Hotkeys.
- Per AGENTS.md "Mandatory Release Rule": every push to `main` lands a fresh GitHub Release with a Windows installer. Validation gates before release: `npm run build`, `npm test -- --run`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npx tauri build --no-bundle`, then `npm run tauri build`.
- Release notes should call out: "Overlay style and size are now configurable in Settings. Choose between the Glacier Capsule (default) and the Halo Orb, and pick Small / Medium / Large to fit your monitor. Home engine-card waveforms are now a more discreet strip."

---

## Sources & References

- **Origin mockups:** [docs/mockups/overlay-01-capsule.html](../mockups/overlay-01-capsule.html), [docs/mockups/overlay-03-halo-orb.html](../mockups/overlay-03-halo-orb.html)
- **Project guardrails:** [AGENTS.md](../../AGENTS.md)
- **Predecessor plans (completed):**
  - [docs/plans/2026-05-03-001-feat-glacier-veyra-redesign-plan.md](2026-05-03-001-feat-glacier-veyra-redesign-plan.md)
  - [docs/plans/2026-05-03-002-feat-glacier-overlay-capsule-plan.md](2026-05-03-002-feat-glacier-overlay-capsule-plan.md)
- Related code: [src/types/settings.ts](../../src/types/settings.ts), [src-tauri/src/settings/legacy_v1.rs](../../src-tauri/src/settings/legacy_v1.rs), [src-tauri/src/main.rs](../../src-tauri/src/main.rs), [src/overlay/pill.tsx](../../src/overlay/pill.tsx), [src/overlay/overlay-app.tsx](../../src/overlay/overlay-app.tsx), [src/routes/settings/layout.tsx](../../src/routes/settings/layout.tsx), [src/components/wave-stage.tsx](../../src/components/wave-stage.tsx)
