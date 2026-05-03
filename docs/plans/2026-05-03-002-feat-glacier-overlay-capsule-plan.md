---
title: "feat: Glacier overlay capsule (light-glass STT/Drafter floating pill)"
type: feat
status: active
date: 2026-05-03
origin: docs/mockups/overlay-01-capsule.html
---

# feat: Glacier overlay capsule (light-glass STT/Drafter floating pill)

## Overview

Rebuild Veyra's recording overlay around the **Compact Capsule** mockup at `docs/mockups/overlay-01-capsule.html`. The current overlay is a 210×58 dark zinc-950 pill with two icon buttons and ten waveform bars. The replacement is a 520×80 light-glass capsule that surfaces a leading LED, a mode chip (`STT` / `Drafter`), a denser live waveform tinted by the active engine (cyan for STT, spark amber for Drafter), an elapsed timer in JetBrains Mono, and a circular ink-black stop button — with an ephemeral hotkey hint floating below the capsule for ~600 ms after activation. Tauri commands, voice-activity smoothing, and the `calculateWaveBarHeights` math are all preserved; this is a visual + dimensional rework, not a behavior change.

---

## Problem Frame

The Glacier redesign (`docs/plans/2026-05-03-001-feat-glacier-veyra-redesign-plan.md`, shipped) made the main window light, sophisticated, and dual-engine aware. The recording overlay still reads as a prototype: dark zinc background, generic sky/amber chips, no timer, no hotkey hint, and a width that can't host the canonical "STT · Whisper · Turbo" / "Drafter · Llama 3.2 · 1B" copy without truncating. Per AGENTS.md, "the floating overlay must appear while recording/transcribing and disappear when work finishes" and every Veyra surface should feel like "a premium Apple-style product interface". This plan brings the overlay up to the same bar.

---

## Requirements Trace

- R1. Replace the 210×58 dark zinc pill with a 520×80 light-glass capsule rendering the same lifecycle (`idle` → `recording` → `transcribing` → hidden). Glass treatment matches the mockup: white-ish backdrop blur + 1 px hairline + soft cyan/amber drop-shadow halo.
- R2. The capsule must visibly distinguish STT (cyan accent, halo, bar gradient) from Drafter (spark amber accent, glow, bar gradient). Tone is driven by `useOverlayStore.mode` (`dictation` → STT, `command` → Drafter) — no new Rust signal.
- R3. The capsule must show an elapsed timer in `mm:ss.t` while recording, frozen during `transcribing`, hidden in `idle`. Source of truth is a `recordingStartedAt` timestamp captured client-side on the `idle → recording` transition. No new IPC channel.
- R4. The capsule must show a leading LED that pulses in the active accent during `recording`, flips red during `transcribing`, and goes neutral grey during `idle/listening`.
- R5. A hotkey hint ("tap F24 to stop" / "tap Pause to draft") must appear briefly under the capsule when state transitions into `recording`, then fade out after ~600 ms.
- R6. `ipc.toggleRecording` (stop) and `ipc.cancelRecording` (cancel/X) must continue to work and remain keyboard-reachable.
- R7. `calculateWaveBarHeights` must remain exported with the existing signature so `src/overlay/pill.test.ts` passes unchanged.
- R8. The voice-activity smoothing pipeline (`nextVoiceActivity`, `INITIAL_VOICE_ACTIVITY`) and the `overlay:level` event subscription in `OverlayApp` must continue to drive the waveform amplitude.
- R9. The Tauri overlay window dimensions and bottom-center positioning math (`OVERLAY_WIDTH`, `OVERLAY_HEIGHT`, `OVERLAY_BOTTOM_MARGIN`) must be updated atomically with the new visual size; transparent + always-on-top + skip-taskbar + non-decorated invariants stay.
- R10. Visual fidelity vs `docs/mockups/overlay-01-capsule.html` must be verified by screenshot comparison before shipping (per AGENTS.md "for material UI redesigns, create or update mockup images first, then compare implementation screenshots against the mockups before shipping").

---

## Scope Boundaries

- **In scope:** `src/overlay/` React surface, `src/stores/overlay-store.ts` (additive — new `recordingStartedAt`), Rust constants `OVERLAY_WIDTH` / `OVERLAY_HEIGHT` / `OVERLAY_BOTTOM_MARGIN`, the related Tauri positioning math, and `pill.test.ts` test coverage.
- **Out of scope:**
  - Audio capture pipeline (`src-tauri/src/audio/recorder.rs`), VAD threshold, or anything touching `whisper.cpp` / Ollama.
  - The `RecordingState` enum in Rust or the IPC event shape — capsule consumes the existing `overlay:state` / `overlay:mode` / `overlay:level` events verbatim.
  - The two other overlay mockups under `docs/mockups/` (`overlay-02-stage-card.html`, `overlay-03-halo-orb.html`) — kept as future-design exploration; the plan picks the **capsule** unambiguously.
  - Settings UI for customizing capsule appearance (size, opacity, position) — no current product driver.
  - First-boot wizard, main-window chrome, or Home — already covered by the prior Glacier redesign plan.

### Deferred to Follow-Up Work

- Live transcript preview inside the capsule (currently shown only inside the `Stage Card` mockup) — would need a new `overlay:partial-transcript` IPC channel and a streaming Whisper hook that does not exist today.

---

## Context & Research

### Relevant Code and Patterns

- [src/overlay/pill.tsx](../../src/overlay/pill.tsx) — current implementation. `OverlayPill({ state, mode })` builds the UI; `calculateWaveBarHeights` is a pure function exported for unit tests. Replace the visual shell while preserving the export and call sites.
- [src/overlay/overlay-app.tsx](../../src/overlay/overlay-app.tsx) — owns the IPC listeners (`overlay:state`, `overlay:mode`, `overlay:level`), polls `ipc.getRecordingState` / `ipc.getRecordingMode` every 180 ms, and feeds the store. Untouched except for one additive subscription to capture `recordingStartedAt`.
- [src/overlay/voice-activity.ts](../../src/overlay/voice-activity.ts) + [src/overlay/voice-activity.test.ts](../../src/overlay/voice-activity.test.ts) — smoothing pipeline. Stays as is.
- [src/overlay/pill.test.ts](../../src/overlay/pill.test.ts) — pinned contract on `calculateWaveBarHeights`. Must continue to pass.
- [src/stores/overlay-store.ts](../../src/stores/overlay-store.ts) — three slices today (`state`, `mode`, `level`). Plan adds `recordingStartedAt: number | null` and `setRecordingStartedAt`.
- [src-tauri/src/main.rs](../../src-tauri/src/main.rs) — owns `OVERLAY_WIDTH = 210`, `OVERLAY_HEIGHT = 58`, `OVERLAY_BOTTOM_MARGIN = 8`, the `WebviewWindowBuilder::new(app, "overlay", …)` invocation, and the `update_overlay` / `emit_overlay_mode` helpers. Plan changes only the three constants and any hard-coded layout math that depends on them.
- [src/components/brand-mark.tsx](../../src/components/brand-mark.tsx) and [src/styles/tailwind.css](../../src/styles/tailwind.css) — Glacier tokens and idiom (cyan/spark accents, hairlines, light-glass shadow recipes) shipped in the prior redesign. The capsule reuses these tokens; it does not introduce new ones.
- [docs/plans/2026-05-03-001-feat-glacier-veyra-redesign-plan.md](2026-05-03-001-feat-glacier-veyra-redesign-plan.md) — prior plan (status: completed) defining the Glacier visual contract and engine duality (cyan ↔ spark amber). The capsule is its overlay-side companion.
- [docs/mockups/overlay-01-capsule.html](../mockups/overlay-01-capsule.html) — visual contract. Animations (`amp`, `shimmer`, `ledPulse`) and bar count (~40 cells across the wave area) come from this file.

### Institutional Learnings

- The completed Glacier redesign established the two-tone engine pattern (cyan / spark amber) and the light-glass surface recipe. Reusing the same tokens prevents another layer of visual drift.
- Per the overlay's existing comments, the wave-bar animation is intentionally pure-CSS-style geometry on a `framer-motion` shell — the React tree stays small to avoid stalling the always-on-top webview during dictation. Keep that constraint.

### External References

- None required. Local design system + the approved capsule mockup is the contract.

---

## Key Technical Decisions

- **Single React component, two style modes.** Continue to express the capsule as one `OverlayPill` (or renamed `OverlayCapsule`) component that takes `state` and `mode` and switches CSS variables, rather than two parallel components per engine. Keeps the diff small and locks tone parity at the source.
- **Timer is client-derived.** Capture `recordingStartedAt = Date.now()` in the store when state transitions `idle → recording`; clear it on `idle`. Avoids a new IPC round-trip and avoids forcing the audio pipeline to publish wall-clock timing.
- **Hotkey hint is one-shot, animated-out.** Render only when `state === "recording"` and `Date.now() - recordingStartedAt < 600`. Animate opacity to zero via framer-motion `AnimatePresence`. No timers, no manual cleanup.
- **Resize the Tauri window once, update the math everywhere.** `OVERLAY_WIDTH`, `OVERLAY_HEIGHT`, and `OVERLAY_BOTTOM_MARGIN` are the single source of truth; the centering formula already references them, so changing the constants is enough. Bumping height to ~80 px (capsule body 56 + 24 px headroom for the hotkey hint) keeps the hint inside the window so it doesn't get clipped.
- **Light-glass over a transparent webview.** The Tauri overlay window already has `transparent: true` + `decorations: false` + `shadow: false`; the visible glass effect lives entirely in CSS (`backdrop-filter: blur(28px) saturate(170%)` + 1 px white inner hairline + soft drop shadow). No native chrome change.
- **Bar count and animation timing match the mockup.** ~40 cells across the wave area to fill the wider capsule; `vsg-amp` / `shimmer` / `ledPulse` keyframes ported into `tailwind.css` as `.veyra-overlay-*` utilities so the capsule can be reskinned without touching the React tree.
- **Cancel button is preserved as the secondary action.** The current pill exposes both Cancel (X) and Stop (square). The mockup shows only Stop, but the cancel intent has product value (rough drafts, accidental triggers). Plan keeps the cancel button reachable via keyboard (`Esc`) but does not draw it on the capsule unless the user is in `transcribing` state — matching the mockup's "transcribing" frame which renders an X.

---

## Open Questions

### Resolved During Planning

- Should the capsule be wider (520 px in the mockup) than the existing 210 px window? **Yes** — the Tauri constants are bumped in U4. Without a wider window the mockup copy ("STT · Whisper · Turbo") cannot fit.
- Does the existing voice-activity smoothing need rework for a 40-cell waveform? **No** — `calculateWaveBarHeights` already accepts an arbitrary bar count via the `WAVE_BARS` array; the array gets re-tuned in U2 alongside the visual.
- Should there be a `Drafter` accent for `transcribing`? **No** — once recording stops, the engine identity is no longer signaled by colour; the LED flips red and the bars dim into a neutral shimmer regardless of mode. Consistent with the mockup.

### Deferred to Implementation

- Whether the elapsed timer should reset to `0.0` instantly or animate from `recordingStartedAt` on first paint after a state transition. Default: render the live computed value; the first frame after the transition naturally lands at sub-100 ms which reads as `00:00.1`.
- Exact `framer-motion` exit duration for the hotkey hint (target ~180 ms; tunable during execution if it feels jumpy at 60 fps).
- Whether `OverlayPill` should be renamed `OverlayCapsule`. Default: keep `OverlayPill` so the import surface and `pill.test.ts` filename stay stable; rename only if reviewers prefer it during U2.

---

## Implementation Units

- U1. **Overlay store: `recordingStartedAt` + reset on idle**

**Goal:** Add a derived timestamp slot that the capsule timer can read without a new IPC channel. Reset deterministically on `idle` so a new dictation always starts at `00:00`.

**Requirements:** R3.

**Dependencies:** none.

**Files:**
- Modify: `src/stores/overlay-store.ts`
- Modify: `src/overlay/overlay-app.tsx` (call `setRecordingStartedAt` on the `idle → recording` and `→ idle` transitions)
- Test: `src/stores/overlay-store.test.ts` (new)

**Approach:**
- Add `recordingStartedAt: number | null` and `setRecordingStartedAt(value)` to the zustand store.
- In `OverlayApp`, derive the transition from the existing state subscription: when the new state is `recording` and the previous was not, set `recordingStartedAt = Date.now()`. When the new state is `idle`, set it to `null`. Do not clobber it on the `recording → transcribing` transition (timer freezes at the recording-end value).
- Keep the rest of the IPC plumbing untouched.

**Patterns to follow:**
- Existing store slice + setter idiom in `src/stores/overlay-store.ts`.
- Existing `useEffect` + `listen("overlay:state", …)` pattern in `OverlayApp`.

**Test scenarios:**
- Happy path: transitioning the store from `idle` to `recording` sets `recordingStartedAt` to a non-null number close to `Date.now()`.
- Happy path: transitioning `recording → transcribing` leaves `recordingStartedAt` unchanged.
- Edge case: transitioning `transcribing → idle` resets `recordingStartedAt` to `null`.
- Edge case: a redundant `idle → idle` transition does not change the value.

**Verification:**
- Store unit tests pass.
- `OverlayApp` re-renders correctly when the state stream toggles.

---

- U2. **Capsule shell: light-glass surface, mode chip, LED, waveform, timer, stop**

**Goal:** Replace the dark zinc pill body with the Glacier capsule from the mockup. Tone follows `mode`; LED pulses match `state`; waveform reuses `calculateWaveBarHeights` with a re-tuned `WAVE_BARS` array; timer reads from the store via a small formatter; stop button stays wired to `ipc.toggleRecording`.

**Requirements:** R1, R2, R4, R6, R7.

**Dependencies:** U1.

**Files:**
- Modify: `src/overlay/pill.tsx`
- Modify: `src/styles/tailwind.css` (add `.veyra-overlay-*` utilities for the glass shell, keyframes for `amp` / `shimmer` / `ledPulse`)
- Test: `src/overlay/pill.test.ts` (existing — must continue to pass; extend with capsule-specific cases)
- Test: `src/overlay/capsule.test.tsx` (new — DOM-level tests; uses `@testing-library/react`)

**Approach:**
- Extract the existing `OverlayPill` body into a single capsule layout matching the mockup: `[ LED · mode chip · waveform · timer · stop ]` on a 520×56 row inside an 80 px frame, with the hotkey hint reserved for U3.
- Drive accent colours from a CSS variable on the capsule root (`--accent`, `--accent-deep`, `--accent-glow`) so `data-mode="stt"` vs `data-mode="drafter"` switches tone without branching JSX.
- Render the leading LED with three CSS classes (`led-recording`, `led-transcribing`, `led-idle`) selected by `data-state`.
- Re-tune `WAVE_BARS` to match the mockup density (about 40 cells). Keep the same `(state, voiceLevel, phase)` signature so existing tests continue to pass; the array length change is internal.
- Replace the dark backdrop + hover colors with Glacier hairlines and the light-glass shadow recipe.
- Mode chip uses JetBrains Mono uppercase caption + Inter Tight bold name (e.g. `STT` / `Whisper · Turbo`). Source the visible name from the same `formatWhisperName` / `formatDrafterName` logic already implemented in `src/components/engine-badge.tsx` — extract those into a shared util so the overlay and the titlebar share one source of truth.
- Stop button is a 36 px circular ink-black control with white square icon (mockup-faithful), wired to `ipc.toggleRecording`. The `transcribing` frame swaps the icon to an X bound to `ipc.cancelRecording`.
- Provide an `aria-live="polite"` region with the textual mode name so screen readers continue to announce mode changes.

**Patterns to follow:**
- Glacier light-glass recipe in `src/styles/tailwind.css` (`.veyra-glass`, `.veyra-command-panel`).
- `engine-badge.tsx` for the engine-name formatter helpers.

**Test scenarios:**
- Happy path: rendering the capsule in `recording` + `dictation` shows the `STT` chip, the cyan-tinted LED, and the wave-bar grid.
- Happy path: rendering in `recording` + `command` shows the `Drafter` chip and the spark amber LED.
- Happy path: clicking the stop button calls `ipc.toggleRecording` exactly once.
- Edge case: rendering in `transcribing` swaps the icon to X and calls `ipc.cancelRecording` on click.
- Edge case: rendering in `idle` shows the floor-height bars (mockup specifies a single 4 px floor) and a neutral grey LED with no animation.
- Edge case: when the keyboard `Esc` key is pressed while the capsule is mounted, `ipc.cancelRecording` is invoked once. (Implementation may live in `OverlayApp`; assert via the same IPC mock.)
- Regression: existing `calculateWaveBarHeights` tests continue to pass with the re-tuned `WAVE_BARS` array.

**Verification:**
- `npm test -- --run src/overlay` passes (existing + new tests).
- Manual visual diff against `docs/mockups/overlay-01-capsule.html` shows: matching capsule width/height, hairlined glass surface, correct LED tone per state, identical bar gradient per mode, matching stop-button geometry.

---

- U3. **Hotkey hint: ephemeral fade under the capsule**

**Goal:** Render the "tap F24 to stop" / "tap Pause to draft" caption below the capsule for ~600 ms after the `idle → recording` transition, then animate it out. Caption text follows the active mode and the user's configured hotkey from `useSettings`.

**Requirements:** R5.

**Dependencies:** U1, U2.

**Files:**
- Modify: `src/overlay/pill.tsx` (or a new `src/overlay/hotkey-hint.tsx` co-located component)
- Test: `src/overlay/capsule.test.tsx` (extend)

**Approach:**
- Use `framer-motion`'s `AnimatePresence` with a child whose presence is gated by `state === "recording"` and `Date.now() - (recordingStartedAt ?? 0) < HINT_DURATION_MS` (default 600). The hint mounts on first paint after recording starts, then unmounts when the predicate flips false.
- Re-evaluate the predicate via a single `requestAnimationFrame`-driven re-render, or simpler: a `setTimeout` armed once when `recordingStartedAt` becomes non-null. Either is acceptable; the implementer picks during U3 with a unit test that proves the unmount fires.
- Read the hotkey label from `useSettings()` (`settings?.hotkey ?? "F24"` for `dictation`, `settings?.commandHotkey ?? "Pause"` for `command`). Falls back to mockup defaults during initial load.
- Style: 10 px JetBrains Mono caption, white-on-transparent, with a small `<kbd>`-style key glyph. Sits in the 24 px area below the capsule body so the existing always-on-top window can host it without clipping (made possible by U4's `OVERLAY_HEIGHT` bump).

**Patterns to follow:**
- `useSettings()` consumption in `src/components/engine-badge.tsx`.
- `AnimatePresence` usage for entering/exiting toasts in `src/components/`.

**Test scenarios:**
- Happy path: with `state` mocked into `recording` and `recordingStartedAt = Date.now()`, the hotkey hint appears and contains "F24" for `dictation` mode.
- Happy path: with `mode = "command"` the hint contains "Pause".
- Edge case: with `state = "transcribing"` the hint is not in the DOM (regardless of `recordingStartedAt`).
- Edge case: advancing the clock past the 600 ms window unmounts the hint.

**Verification:**
- DOM tests demonstrate the mount/unmount lifecycle.
- Manual: pressing the hotkey shows the capsule + hint; the hint fades after ~0.6 s while the capsule stays.

---

- U4. **Tauri overlay window: resize and reposition**

**Goal:** Resize the Tauri overlay window so the wider capsule + hotkey hint render without clipping; keep the bottom-center positioning math, transparency, always-on-top, and skip-taskbar invariants intact.

**Requirements:** R1, R9.

**Dependencies:** U2 (so the new shell exists when the wider window opens).

**Files:**
- Modify: `src-tauri/src/main.rs` (constants `OVERLAY_WIDTH`, `OVERLAY_HEIGHT`, `OVERLAY_BOTTOM_MARGIN` and any nearby positioning math that hard-codes them)

**Approach:**
- Bump `OVERLAY_WIDTH` from 210 to **560** (520 capsule + 20 px slack on each side for the soft drop shadow + hotkey hint headroom).
- Bump `OVERLAY_HEIGHT` from 58 to **96** (capsule body 56 + ~24 px hotkey hint area + ~16 px shadow halo).
- Leave `OVERLAY_BOTTOM_MARGIN` at 8 unless visual review requests adjustment; the existing centering formula uses these constants directly, so no math change is required.
- Verify the `WebviewWindowBuilder` flags (`transparent`, `decorations(false)`, `always_on_top(true)`, `skip_taskbar(true)`, `focused(false)`, `shadow(false)`) remain unchanged — the new shadow is rendered by CSS, not the OS chrome.
- Verify `update_overlay` still hides / shows the window based on `RecordingState` without other side effects.

**Patterns to follow:**
- The existing constant + position-arithmetic block in `src-tauri/src/main.rs`.

**Test scenarios:**
- Test expectation: none — pure dimensional configuration. Behavior verified manually by running the app and confirming the capsule (and its drop shadow + hotkey hint) is fully visible without clipping or repositioning artifacts.

**Verification:**
- `cargo test --manifest-path src-tauri/Cargo.toml` passes (no new tests; existing suite must stay green).
- `npx tauri build --no-bundle` succeeds.
- Manual: launching `typr.exe`, pressing F24, confirming the capsule appears centered horizontally, anchored bottom-margin 8 from the work-area bottom, with no clipping of the shadow/hint and no taskbar entry.

---

## System-Wide Impact

- **Interaction graph:** The capsule consumes the same `overlay:state`, `overlay:mode`, `overlay:level` IPC events. The new `recordingStartedAt` slice is purely client-side and not observed by any other surface.
- **Error propagation:** `ipc.toggleRecording` and `ipc.cancelRecording` errors are already swallowed in the current pill (`.catch(() => {})`); the new shell preserves the same error treatment.
- **State lifecycle risks:** `recordingStartedAt` must reset on every transition into `idle` — failing to do so leaves the timer "stuck" on the next dictation. U1's test scenarios pin this.
- **API surface parity:** None — the overlay surface is the only consumer.
- **Integration coverage:** `voice-activity.test.ts` continues to prove the smoothing chain; new `capsule.test.tsx` proves the DOM-level rendering across modes and states; `overlay-store.test.ts` proves the timer reset.
- **Unchanged invariants:** Tauri window flags (`transparent`, `decorations(false)`, `always_on_top(true)`, `skip_taskbar(true)`, `focused(false)`, `shadow(false)`); the `RecordingState` Rust enum; the audio capture pipeline; the `calculateWaveBarHeights(state, voiceLevel, phase)` signature; the click-through behavior of the overlay during transcribing.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Wider window steals click area on small monitors | New width is 560 px — well below the 800 px width of the smallest commonly-used Windows display. The bottom-center positioning math automatically clamps to the work area; if reviewers report clipping during manual QA, fall back to 480 px. |
| `framer-motion` exit animation on the hotkey hint stutters when the always-on-top webview is unfocused | The capsule webview already runs `framer-motion` (initial mount transition). The hint reuses the same motion engine; if jank is observed, replace the exit animation with a CSS opacity transition (no behavior change). |
| Timer drifts because `recordingStartedAt` is captured client-side rather than from the audio pipeline | The drift bound is the IPC latency between `RecordingState::Recording` being emitted and the React listener firing — empirically <50 ms. Acceptable for a `mm:ss.t` display. If a tighter binding is later needed, the pipeline can publish a `started_at` epoch on the `overlay:state` payload. |
| Rebuild of `WAVE_BARS` breaks the `pill.test.ts` snapshot for `idle` | The idle-frame test pins the literal array `[5, 8, 11, 7, 14, 9, 6, 12, 8, 5]`. U2 must update the test to reflect the new array, or keep the legacy array as the idle-floor seed. The plan picks the latter — keep idle floor identical, change only the recording-active bar generation if needed. |
| Hotkey hint label reads stale when the user reassigns hotkeys mid-session | `useSettings()` is reactive; the hint re-renders on the next paint after the settings store updates. No additional plumbing needed. |
| Visual fidelity drifts vs the mockup as React + Tailwind diverge from raw HTML/CSS | Per AGENTS.md, screenshot comparison gating is mandatory before the release rule fires. U2 verification calls this out explicitly. |

---

## Documentation / Operational Notes

- After implementation, regenerate any overlay-related screenshots referenced from `docs/design/implementation/` or the README; the existing references show the dark zinc pill.
- Per AGENTS.md "Mandatory Release Rule": every push to `main` lands a fresh GitHub Release with a Windows installer. Validation gates before release: `npm run build`, `npm test -- --run`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npx tauri build --no-bundle`, then `npm run tauri build`. If `tauri build` fails on Windows with access denied for `typr.exe`, run the documented PowerShell `Stop-Process` recovery before retrying.
- Release notes should call out: "Recording overlay redesigned as a Glacier light-glass capsule. Two engines (Whisper STT / Llama Drafter) are now visually distinct in the floating pill, with elapsed timer and ephemeral hotkey hint. No behavior changes."

---

## Sources & References

- **Origin mockup:** [docs/mockups/overlay-01-capsule.html](../mockups/overlay-01-capsule.html)
- **Project guardrails:** [AGENTS.md](../../AGENTS.md)
- **Predecessor plan (completed):** [docs/plans/2026-05-03-001-feat-glacier-veyra-redesign-plan.md](2026-05-03-001-feat-glacier-veyra-redesign-plan.md)
- Related code: [src/overlay/pill.tsx](../../src/overlay/pill.tsx), [src/overlay/overlay-app.tsx](../../src/overlay/overlay-app.tsx), [src/stores/overlay-store.ts](../../src/stores/overlay-store.ts), [src-tauri/src/main.rs](../../src-tauri/src/main.rs), [src/components/engine-badge.tsx](../../src/components/engine-badge.tsx)
