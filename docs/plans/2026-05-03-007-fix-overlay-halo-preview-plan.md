---
title: "fix: Overlay Halo Orb switching and live preview"
type: fix
status: completed
date: 2026-05-03
origin: user request - Halo Orb breaks when selected; Overlay settings needs a way to preview/test appearance and position
---

# fix: Overlay Halo Orb switching and live preview

## Problem

Settings > Overlay lets the user switch between Capsule and Halo Orb, but switching to Halo Orb can leave the live overlay visually broken: the window appears in the right general area, but the content can render as a cropped/old capsule/transcribing layout instead of the orb. The same screen also has no reliable way to test what the overlay will look like or where it will appear without starting a real recording.

The feature needs two fixes:

1. Make style/size switching deterministic.
2. Add a real preview/test action from Settings > Overlay.

## Requirements

- R1. Selecting **Halo Orb** must immediately resize the transparent overlay window to the correct orb dimensions and render `HaloOrb`, not a cropped capsule.
- R2. Selecting **Capsule** must immediately resize back and render `OverlayPill`.
- R3. Switching style while the overlay is hidden, recording, or transcribing must not leave stale layout/state.
- R4. Settings > Overlay needs a **Preview** control that shows the overlay on the actual screen position it will use.
- R5. Preview must not start recording, touch microphone, transcribe, save history, play recording sounds, or affect stats.
- R6. Preview should support at least:
  - Speech to Text / recording state
  - Email Drafter / recording state
  - Transcribing state
- R7. Preview should animate fake audio levels so waveform/orb rings can be inspected without speaking.
- R8. Preview should auto-hide after a short duration and have a manual hide/cancel path.
- R9. Tests must prove style switching, preview command behavior, and no real recording side effects.

## Scope

In scope:

- `src/routes/settings/overlay.tsx`
- `src/overlay/overlay-app.tsx`
- `src/overlay/halo-orb.tsx`
- `src/overlay/pill.tsx`
- `src/stores/overlay-store.ts`
- `src-tauri/src/main.rs`
- `src/lib/tauri.ts`
- `src/routes/settings/overlay.test.tsx`
- `src/overlay/halo-orb.test.tsx`
- `src/overlay/capsule.test.tsx`

Out of scope:

- New overlay designs beyond Capsule and Halo Orb.
- Drag-to-position custom placement.
- Persisted per-monitor overlay placement.
- Microphone/audio pipeline changes.

## Current Code Notes

- `src-tauri/src/main.rs` has `overlay_dims(style, size)` with capsule and orb dimensions.
- `apply_overlay_layout(app, style, size)` resizes the overlay window and emits `overlay:layout`.
- `src/overlay/overlay-app.tsx` chooses `HaloOrb` when `overlayStyle === "orb"`, otherwise `OverlayPill`.
- Settings currently calls `update({ overlayStyle })`; `useSettings` triggers `ipc.setOverlayLayout(next.overlayStyle, next.overlaySize)` when settings change.
- There is no command to show overlay for preview. The overlay only shows from recording/transcribing flow.

## Root Cause Hypotheses To Verify

- H1. The overlay webview receives size changes before React has switched component style, causing a visible frame with stale capsule content in orb-sized window.
- H2. The current live overlay may be in `transcribing` state when style changes, so the old capsule transcribing UI remains visible until the next `overlay:layout`/state event.
- H3. Orb root dimensions and OS window dimensions do not include enough vertical space for chip/hint, causing crop or bottom clipping.
- H4. The settings page preview SVG is not tied to the real overlay implementation, so it can look fine while live overlay is broken.

## Design Decision

Add a Rust IPC preview command instead of faking preview inside the settings page only.

Reason: the user specifically needs to see where the overlay appears on the real desktop. Only the real Tauri overlay window can validate position, size, transparency, always-on-top behavior, and monitor/taskbar placement.

## Proposed IPC

Add command:

```ts
previewOverlay(style: OverlayStyle, size: OverlaySize, mode: "dictation" | "command", state: "Recording" | "Transcribing")
```

Rust behavior:

1. Call `apply_overlay_layout(app, style, size)`.
2. Position at `active_monitor_bottom_position(w, h)`.
3. Emit `overlay:preview` or reuse existing events in order:
   - `overlay:layout`
   - `overlay:mode`
   - `overlay:state`
   - simulated `overlay:level` ticks for 4-5 seconds
4. Show overlay.
5. Auto-hide after timeout if no real recording/transcribing is active.

Add command:

```ts
hideOverlayPreview()
```

Rust behavior:

1. Stop preview task/ticks.
2. Hide overlay only if real `recording_state` is `Ready`.

## UI Plan

In `src/routes/settings/overlay.tsx`:

- Keep style cards.
- Add a **Preview** row under size selector:
  - `Preview STT`
  - `Preview Drafter`
  - `Preview Transcribing`
  - `Hide preview`
- Show short copy:
  - “Preview uses the real overlay window. It does not record audio.”
- Disable preview buttons while settings are loading.

Optional later:

- Add mini segmented control for preview mode/state instead of multiple buttons.

## Implementation Units

### U1. Make overlay layout switching deterministic

Files:

- `src-tauri/src/main.rs`
- `src/overlay/overlay-app.tsx`

Approach:

- Ensure `apply_overlay_layout` emits layout after resizing and positioning.
- Include a monotonically increasing layout revision in `overlay:layout`, so the overlay app can force a render key change when style changes.
- In `OverlayApp`, render keyed root:
  - key = `${overlayStyle}:${overlaySize}:${layoutRevision}`
- Ensure the root wrapper has explicit width/height matching current overlay style.

Tests:

- Add/extend `src/overlay/halo-orb.test.tsx` to assert orb root uses `data-size`.
- Add `overlay-app` test if existing test harness supports event mocks.

### U2. Add preview IPC commands

Files:

- `src-tauri/src/main.rs`
- `src/lib/tauri.ts`

Approach:

- Add `preview_overlay(style, size, mode, state)`.
- Add `hide_overlay_preview()`.
- Preview must not mutate `recording_state`.
- Preview must not call audio recorder, pipeline, storage, or chimes.
- Preview emits fake levels on a timed task.
- Store cancellation token/flag in `AppState` or a small preview controller.

Tests:

- Rust unit-testable pieces:
  - `overlay_dims("orb", "small|medium|large")` unchanged.
  - preview state mapping helper if extracted.
- Manual Tauri behavior tested via local build.

### U3. Add Settings preview controls

Files:

- `src/routes/settings/overlay.tsx`
- `src/routes/settings/overlay.test.tsx`

Approach:

- Add preview button group.
- On click, call `ipc.previewOverlay(currentStyle, currentSize, mode, state)`.
- Add `Hide preview` button.
- Show toast/error if preview command fails.

Tests:

- Clicking `Preview STT` calls `previewOverlay(currentStyle, currentSize, "dictation", "Recording")`.
- Clicking `Preview Drafter` calls `previewOverlay(currentStyle, currentSize, "command", "Recording")`.
- Clicking `Preview Transcribing` calls `previewOverlay(currentStyle, currentSize, "dictation", "Transcribing")`.
- Clicking `Hide preview` calls `hideOverlayPreview()`.

### U4. Fix Halo Orb clipping/stale content

Files:

- `src/overlay/halo-orb.tsx`
- `src/styles/tailwind.css`
- `src-tauri/src/main.rs`

Approach:

- Validate orb dimensions:
  - small: 200 x 168
  - medium: 240 x 200
  - large: 300 x 248
- If chip/hint clips, increase height in `overlay_dims`.
- Ensure `HaloOrb` wrapper uses `overflow-visible` internally but root overlay window does not require body scroll.
- Ensure `body`, `#root`, overlay app root use transparent background and exact viewport sizing.

Tests:

- `src/overlay/halo-orb.test.tsx`: transcribing state renders orb shimmer + chip, not capsule text layout.
- Screenshot/manual validation: Halo Orb small/medium/large does not clip on desktop.

### U5. Manual validation / release

Commands:

- `npm test -- --run`
- `npm run build`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `npx tauri build`

Manual:

1. Open Settings > Overlay.
2. Select Capsule / Small; click Preview STT.
3. Select Halo Orb / Small; click Preview STT.
4. Select Halo Orb / Medium; click Preview Drafter.
5. Click Preview Transcribing.
6. Verify overlay appears bottom-center above taskbar, not cropped.
7. Verify no recording starts and no history row is created.
8. Start real dictation with Halo Orb selected; verify live overlay matches preview.

Release:

- Bump to `0.1.20`.
- Build `Veyra_0.1.20_x64-setup.exe`.
- Push `main`.
- Publish GitHub release with installer.

## Success Criteria

- Halo Orb selection no longer renders cropped capsule/transcribing UI.
- User can preview actual overlay position and style without recording.
- Preview covers both STT and Email Drafter modes.
- All automated tests pass.
- New Windows installer release exists.
