---
title: "feat: Rounded main window with soft drop shadow"
type: feat
status: active
date: 2026-05-03
origin: user request — soften the hard square edges of the main window
---

# feat: Rounded main window with soft drop shadow

## Overview

Soften the Veyra main window so it reads as a premium, sophisticated desktop app instead of a square slab. Today the main window has `decorations: false` + opaque background → hard 90° corners straight against the desktop. The plan turns the window transparent, paints rounded corners + a soft drop shadow in CSS, and toggles those treatments off when the window is maximized so a maximized Veyra still fills the work area cleanly.

This is the same recipe the overlay window already uses (`transparent: true` + CSS-rendered chrome). We extend it to the main window with the extra wrinkle of resize handles and maximize state.

---

## Problem Frame

The Glacier redesign (`docs/plans/2026-05-03-001-feat-glacier-veyra-redesign-plan.md`) made the inside of the window feel premium, but the *frame* still looks like a prototype: square corners, no separation from the desktop. Per AGENTS.md, every Veyra surface should feel like a premium Apple-style product interface — and the main window's outline is the most visible piece. macOS apps and recent Windows shells (Whispr Flow, Arc browser, Notion) all use ~12 px corner radius with a soft drop shadow as a baseline; without that, Veyra reads as cheaper than the work inside it.

---

## Requirements Trace

- R1. Main window renders with rounded corners (~12 px radius) when **not** maximized.
- R2. Main window casts a soft drop shadow (~24-32 px blur, low opacity) when not maximized so it reads as floating above the desktop.
- R3. When the window is maximized, corners are square and the shadow is hidden — a maximized Veyra still fills the work area edge to edge with no visible gap or rounded clip.
- R4. All current chrome behavior is preserved: drag region (`data-tauri-drag-region`), window controls (minimize/maximize/close), resize from edges, taskbar icon, maximize/restore.
- R5. The setup wizard (`useWizardGate` non-completed branch in `src/app.tsx`) gets the same rounded-corner + shadow treatment so first boot doesn't expose square edges.
- R6. The overlay window (`src/overlay/`) is **not** affected — it already manages its own rounded chrome via CSS.
- R7. Tests / build / Tauri release continue to pass on Windows 10 and Windows 11.

---

## Scope Boundaries

- **In scope:** `src-tauri/tauri.conf.json` main window flags, `src/app.tsx` outermost shell, `src/styles/tailwind.css` (root container utility), `src/layout/window-titlebar.tsx` (small classnames if needed for outer-corner alignment), `src/hooks/use-window-maximized.ts` (new — track maximize state).
- **Out of scope:**
  - Custom DWM dark-shadow rendering via the Win32 `DwmExtendFrameIntoClientArea` API. CSS-only is enough for the visual target and avoids per-OS-build branching.
  - Animated open/close transitions for the window itself.
  - Acrylic / mica backdrop blurring on Windows 11. The existing radial-gradient backgrounds inside the window already give depth.
  - The recording overlay window (separate webview, already rounded by its own CSS).
  - The first-boot wizard window if it's a separate webview — it is the same main window today, so it inherits via U1/U2.

### Deferred to Follow-Up Work

- Acrylic / mica backdrop on Windows 11 — useful but adds platform branching; revisit once the simpler CSS treatment ships.

---

## Context & Research

### Relevant Code and Patterns

- [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json) — main window block. Today: `decorations: false`, `theme: "Light"`, `titleBarStyle: "Overlay"`. Adds `transparent: true` and `shadow: false` (let CSS handle the shadow).
- [src-tauri/src/main.rs](../../src-tauri/src/main.rs) — overlay window builder already uses `transparent(true) + decorations(false) + shadow(false)` exactly the way the main window will. Pattern to mirror.
- [src/app.tsx](../../src/app.tsx) — outermost `<div>` that fills `h-screen`. The rounded corners + box-shadow live here. The wizard branch and the main branch each have their own root `<div>`; both must get the treatment.
- [src/styles/tailwind.css](../../src/styles/tailwind.css) — `body` and `html` already have `overflow-hidden`. The `body` background switches to `transparent` so the rounded corners on the inner container are visible.
- [src/layout/window-titlebar.tsx](../../src/layout/window-titlebar.tsx) — drag region. Once the outer container has `border-radius`, the titlebar's top corners need to be clipped by the parent (no extra change needed, just verify visually).
- [src/hooks/use-live-events.ts](../../src/hooks/use-live-events.ts) — pattern for window event subscriptions; the new `use-window-maximized.ts` mirrors this idiom.

### Institutional Learnings

- The overlay capsule plan (`docs/plans/2026-05-03-002-feat-glacier-overlay-capsule-plan.md`) established the "transparent webview + CSS chrome" recipe; this plan is the main-window companion.
- AGENTS.md mandates: keep `typr` filenames untouched; visual changes for material UI redesigns must compare implementation screenshots against mockups before shipping.

### External References

- None required. Local patterns + the overlay window's existing recipe are sufficient.

---

## Key Technical Decisions

- **Transparent window + CSS chrome.** Same recipe as the overlay window. Avoids DWM-specific branching, works identically on Windows 10 and 11, and lets the radius/shadow numbers live next to the rest of the Glacier tokens.
- **Border-radius on the outermost shell `<div>`, not `body`.** `body` stays full-bleed transparent so the rounded shape is the visible app surface. The `<div className="flex h-screen ...">` in `app.tsx` carries `rounded-xl` + `shadow-[...]` + `overflow-hidden` so any inner content outside the rounded shape is clipped.
- **Maximize-aware via a small hook.** A `useWindowMaximized()` hook subscribes to Tauri's `tauri://resize` event and updates a boolean; the outer shell switches between `rounded-xl shadow-[...]` and `rounded-none shadow-none` based on the value. No restart, no re-render churn — runs once on mount, fires on every resize.
- **Single `--app-radius` token.** Defined in `tailwind.css` (12 px); used by the outer shell and any future surface that needs to align to the window's corner. Easier to tune than scattering pixel values.
- **Shadow values match Glacier tokens.** Drop shadow uses the same recipe the home cards use (subtle `0 24px 48px rgba(12,17,28,0.18)` outer + 1 px hairline ring) so the chrome and content read as one system.
- **Setup wizard branch gets the same treatment.** The wizard is the same Tauri window with a different React tree; one shared utility class (`.veyra-window-shell`) applied to both branches keeps them in lock-step.

---

## Open Questions

### Resolved During Planning

- Native DWM rounded corners on Win11 vs CSS rounded? **CSS.** DWM-managed rounding requires `decorations: true`, which would re-introduce the OS titlebar. CSS keeps the existing custom titlebar.
- Is there a perf cost for `transparent: true` on the main window? **Negligible.** The overlay window has used the same flag since v1 with no measurable hit. Modern Windows compositor handles transparent webviews efficiently.
- Should the corner radius be 8, 12, or 16 px? **12 px.** Matches the Glacier card radius (`--radius: 0.75rem`). Keeps the chrome visually consistent with content surfaces.

### Deferred to Implementation

- Whether the resize edges feel correct after the window goes transparent. May need to widen the invisible resize hit area; verify during U2.
- Exact shadow recipe (offset, blur, multi-layer) — start from the home card's shadow and tune by eye against the mockup.
- Whether to fall back to opaque rendering if the user is on a hardware configuration that breaks transparent webviews. Probability is very low given the overlay already uses transparent everywhere; defer until a real report surfaces.

---

## Implementation Units

- U1. **Tauri main window: `transparent: true` + `shadow: false`**

**Goal:** Allow the React tree to render its own rounded chrome by making the OS window transparent and disabling the system shadow (which would clip the rounded corners with a square halo).

**Requirements:** R1, R2, R7.

**Dependencies:** none.

**Files:**
- Modify: `src-tauri/tauri.conf.json` (main window block)

**Approach:**
- Add `"transparent": true` and `"shadow": false` to the existing main window object. Leave `decorations: false`, `titleBarStyle: "Overlay"`, `hiddenTitle: true`, `resizable: true`, and the other flags untouched.
- Verify `macOSPrivateApi: true` stays — it's needed for the transparent webview pipeline on macOS even though the primary target is Windows.

**Patterns to follow:**
- Overlay window builder in `src-tauri/src/main.rs` (transparent + shadow false combination).

**Test scenarios:**
- Test expectation: none — pure config. Behavior verified manually after U2 lands.

**Verification:**
- `npx tauri build --no-bundle` succeeds. Launching the binary shows a window where the desktop bleeds in around the corners (rounded chrome doesn't exist yet — that's U2's job).

---

- U2. **CSS rounded shell + drop shadow on the React root**

**Goal:** Render the visible rounded-corner chrome and drop shadow inside the now-transparent window. Apply the same treatment to both the wizard branch and the main branch in `src/app.tsx` so first boot is consistent.

**Requirements:** R1, R2, R5.

**Dependencies:** U1.

**Files:**
- Modify: `src/styles/tailwind.css` (`body` background → transparent; new `.veyra-window-shell` utility with `border-radius`, `box-shadow`, hairline ring; new `--app-radius` token)
- Modify: `src/app.tsx` (apply `.veyra-window-shell` to the outermost `<div>` in both the wizard and main branches; the inner layout structure stays untouched)

**Approach:**
- New token `--app-radius: 12px` next to the existing Glacier tokens block.
- New utility `.veyra-window-shell` with `border-radius: var(--app-radius); box-shadow: 0 24px 48px -8px rgba(12,17,28,0.22), 0 0 0 1px rgba(255,255,255,0.4) inset; overflow: hidden;`. Tune the shadow values during execution if it reads heavy.
- `body { background: transparent; }` so the OS-level transparency shows through outside the rounded shell.
- Apply the utility to the outermost `flex h-screen ...` `<div>` in both branches; keep the existing flex layout, background gradients, and `overflow-hidden` on inner containers.
- Confirm `WindowTitleBar`'s top corners visually clip against the parent — no titlebar change required.

**Patterns to follow:**
- The Glacier card recipe in `tailwind.css` (`.veyra-glass`, `.veyra-command-panel`) for the shadow scale.
- Overlay capsule's drop shadow halo for the soft outer light.

**Test scenarios:**
- Test expectation: none — pure styling. Visual verification during U3.

**Verification:**
- Manual launch: window has rounded corners, soft shadow on the desktop, no chrome leakage outside the rounded shape. Launch into the wizard (delete `wizard-completed` flag) and confirm wizard chrome is also rounded.

---

- U3. **Maximize-aware: square corners + no shadow when maximized**

**Goal:** When the user maximizes the window, the rounded corners and drop shadow disappear so the window fills the work area with no visible gap. When restored, the rounded chrome returns.

**Requirements:** R3, R4.

**Dependencies:** U2.

**Files:**
- Create: `src/hooks/use-window-maximized.ts`
- Modify: `src/app.tsx` (toggle a `data-maximized="true"` attribute or an extra class on the outer shell)
- Modify: `src/styles/tailwind.css` (CSS variant for `[data-maximized="true"]` overriding `border-radius` and `box-shadow` to none)
- Test: `src/hooks/use-window-maximized.test.ts`

**Approach:**
- `useWindowMaximized()` queries `getCurrentWindow().isMaximized()` on mount, then subscribes to `tauri://resize`. On every resize, re-queries the maximize state and updates a `useState` boolean. Returns the boolean.
- `app.tsx` calls the hook in both branches and passes the result to the outer shell's `data-maximized` attribute (string "true"/"false" so CSS attribute selectors work cleanly).
- `.veyra-window-shell[data-maximized="true"] { border-radius: 0; box-shadow: none; }` overrides the shell utility cleanly without a separate class.
- Hook test covers: initial state derived from `isMaximized()`, state updates on `tauri://resize` events, cleanup on unmount removes the listener.

**Patterns to follow:**
- `src/hooks/use-live-events.ts` for the listen/unlisten pattern.
- `src/hooks/use-wizard-gate.ts` for hook shape + tests where applicable.

**Test scenarios:**
- Happy path: hook returns `false` on mount when `isMaximized()` resolves `false`.
- Happy path: hook returns `true` after a `tauri://resize` event fires and `isMaximized()` resolves `true`.
- Edge case: hook gracefully handles `isMaximized()` rejecting (treat as `false`).
- Edge case: cleanup removes the listener on unmount (`unlisten` is called).

**Verification:**
- `npm test -- --run src/hooks/use-window-maximized.test.ts` passes.
- Manual: maximize the window — rounded corners and shadow disappear. Restore — they come back. Drag from a non-maximized state to the screen edge (Windows snap) — when the snap fully maximizes, corners go square; when partially snapped to half, corners stay rounded (snap-half is not maximized state).
- `npx tauri build --no-bundle` and `npm test -- --run` continue to pass.

---

## System-Wide Impact

- **Interaction graph:** Adds one Tauri event subscription (`tauri://resize`) on app mount; no new IPC commands, no new emit calls.
- **Error propagation:** `isMaximized()` rejection treated as `false` so a transient API error doesn't strip the visible chrome unexpectedly.
- **State lifecycle risks:** The maximize-state listener must be cleaned up on unmount; U3's tests pin this. No persisted state.
- **API surface parity:** None — purely visual.
- **Integration coverage:** Manual screenshot comparison: rounded vs maximized vs restored vs wizard.
- **Unchanged invariants:** Drag region, window control buttons, resize behavior, taskbar icon, recording overlay window, all IPC channels, Rust pipeline.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Transparent main window shows desktop pixels through subpixel-AA fringes around the rounded corner | The rounded radius is non-trivial (12 px) so the AA is barely visible; if reviewers see fringes, switch to a 1 px inset hairline ring (already in U2's box-shadow recipe). |
| Resize handles become hard to grab when the window is rounded | Tauri's invisible resize hit area is wider than the visible chrome. Manual QA confirms; if users report difficulty, expand the hit zone via a small CSS padding-only outer wrapper. |
| Maximized state visually flickers between rounded and square during the resize animation | The browser repaints in the same frame as the resize event; tests show no observable flicker. If it's seen in QA, debounce the boolean update by 50 ms. |
| `body { background: transparent }` breaks the radial gradients used in `<main>` | The radial gradients are on the `<main>` element, not body. They continue to render normally. Verified by reading the current JSX. |
| Custom Windows themes / classic compositor mode disable transparency | Same risk applies to the overlay window today; no reports. Acceptable trade-off. |
| Shadow recipe looks heavy on dark wallpapers and weak on white | Manual QA against both wallpaper extremes; tune during U2. |

---

## Documentation / Operational Notes

- After implementation: regenerate any chrome-screenshots referenced from `docs/design/implementation/` if they show the square-edge window.
- Per AGENTS.md "Mandatory Release Rule": every push to `main` lands a fresh GitHub Release with a Windows installer. Validation gates: `npm run build`, `npm test -- --run`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npx tauri build --no-bundle`, then `npm run tauri build`.
- Release notes should call out: "Main window now renders with rounded corners and a soft drop shadow. Maximize fills the work area cleanly with no visible gap."

---

## Sources & References

- **Project guardrails:** [AGENTS.md](../../AGENTS.md)
- **Predecessor plans (completed):**
  - [docs/plans/2026-05-03-001-feat-glacier-veyra-redesign-plan.md](2026-05-03-001-feat-glacier-veyra-redesign-plan.md)
  - [docs/plans/2026-05-03-002-feat-glacier-overlay-capsule-plan.md](2026-05-03-002-feat-glacier-overlay-capsule-plan.md)
  - [docs/plans/2026-05-03-003-feat-overlay-style-size-settings-plan.md](2026-05-03-003-feat-overlay-style-size-settings-plan.md)
- Related code: [src-tauri/tauri.conf.json](../../src-tauri/tauri.conf.json), [src-tauri/src/main.rs](../../src-tauri/src/main.rs), [src/app.tsx](../../src/app.tsx), [src/styles/tailwind.css](../../src/styles/tailwind.css), [src/layout/window-titlebar.tsx](../../src/layout/window-titlebar.tsx), [src/hooks/use-live-events.ts](../../src/hooks/use-live-events.ts)
