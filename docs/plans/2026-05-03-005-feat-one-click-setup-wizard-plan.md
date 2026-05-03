---
title: "feat: Sophisticated one-click first-boot setup (Whisper + Ollama + email model)"
type: feat
status: active
date: 2026-05-03
origin: user request — first-boot setup must reflect Glacier sophistication and install all requirements with one click
---

# feat: Sophisticated one-click first-boot setup (Whisper + Ollama + email model)

## Overview

Refresh the first-boot wizard so it (a) looks like the rest of the Glacier visual system and (b) installs every runtime dependency — Ollama, the Whisper model, and the local email-draft model — with as few clicks as possible. Today the wizard chrome predates the Glacier redesign (sky-blue chips, prototype-feel cards) and the Models step asks the user to *pick* a model and accept "Download later" status; nothing actually runs until the user opens Settings. The replacement orchestrates the installs end-to-end with live progress, restartable steps, and Glacier surfaces.

The Rust IPC plumbing for downloads already exists (`download_model` for Whisper emits `model:download:progress`; `check_email_draft_model` triggers Ollama pulls and emits `email-model:download:progress`; the NSIS installer auto-installs Ollama). This plan is **mostly a wizard UX rework**: one tiny new Rust command (Ollama presence check), then a fresh React install step that drives the existing pipes.

---

## Problem Frame

Per AGENTS.md: *"First boot must feel as polished as the main app: dedicated setup flow, clear choices, no regular menu access until completed."* Today's wizard meets the gating requirement but not the polish bar — it is the only Veyra surface that didn't get the Glacier treatment in the recent redesign passes. It also makes the user do the work the app could do for them: pick a Whisper model size, click into Settings, find Download model, find Install Ollama, find Download email model. Each step is a fork in the road where users get stuck.

Goal: pressing **Install everything** runs Whisper download + Ollama presence check + Ollama email-model pull as one orchestrated job, reports per-step progress, recovers from per-step failure with a single retry button, and never blocks the user from continuing through the rest of the wizard while installs run in the background.

---

## Requirements Trace

- R1. Wizard chrome adopts the Glacier visual system — light-glass shell, hairlines, `veyra-eyebrow` mono caption, italic Newsreader accents, the same `BrandMark` SVG used in titlebar/Home, ditching the legacy `bg-sky-50` chips.
- R2. New "Install" step orchestrates **three** installs in a single user-facing action: Whisper model download, Ollama runtime presence check (auto-install via NSIS hook on first install; fallback to a clear "Install Ollama" CTA if missing post-install), and email-draft model pull.
- R3. Each install reports live progress (percent + bytes-downloaded for downloads, status text for the runtime check) via the existing IPC events (`model:download:progress`, `email-model:download:progress`).
- R4. Per-step failure surfaces a clear error message inline with a single **Retry** button that re-runs only the failed step (does not require re-running successful steps).
- R5. Background install: the user can continue through Microphone and Hotkeys steps while installs are still running. The Ready step blocks completion until all three installs report Operational, OR offers an explicit "Skip and configure later" option that lands the user in the main app.
- R6. New Tauri command `is_ollama_installed()` returns `boolean` synchronously (file existence + PATH check) so the wizard can decide whether to render "Install Ollama" CTA before invoking the heavier `check_email_draft_model`.
- R7. Step order: Welcome → Install → Microphone → Hotkeys → Ready. Install is moved earlier than today's "Models" step (which was step 2) so the longest async task starts as early as possible while the user does the rest.
- R8. The wizard preserves all existing behavior: gating the main menu, `markWizardComplete` IPC call, the wizard-completed event dispatch, hotkey defaults (`F24`, `Pause`), microphone listing, settings persistence.
- R9. All existing tests (78/78) continue to pass; new tests cover the Install step state machine.

---

## Scope Boundaries

- **In scope:** `src/routes/wizard.tsx` (full rewrite of the body), `src/hooks/` (new install-orchestration hook + `use-ollama-status`), `src-tauri/src/main.rs` (new `is_ollama_installed` command), Glacier-styled wizard utility classes if any are missing.
- **Out of scope:**
  - Modifying the NSIS installer hook (`src-tauri/windows/hooks.nsh`) — it already auto-installs Ollama. Improvements there are deferred.
  - Switching Whisper model defaults or adding new options. The wizard ships with the same `turbo` recommended choice; the model selector becomes "Recommended" + an "Advanced" expandable section.
  - Removing the Settings → Transcription manual install controls. They stay as the canonical recovery path.
  - Telemetry / metrics around install funnel — not requested, no privacy review yet.
  - macOS / Linux installer-side Ollama auto-install (NSIS is Windows-only). On non-Windows platforms the wizard still shows the manual install CTA.
  - Re-running the wizard after first boot. The wizard-gate logic is preserved verbatim.

### Deferred to Follow-Up Work

- Telemetry / install funnel metrics — separate plan, requires privacy review.
- A "Choose your size" modal for advanced Whisper variants (`base`, `large-v3`, `tiny`) — keep current `<select>` but tucked behind an Advanced disclosure.

---

## Context & Research

### Relevant Code and Patterns

- [src/routes/wizard.tsx](../../src/routes/wizard.tsx) — current 5-step wizard with prototype chrome. Full body rewrite.
- [src/components/page-shell.tsx](../../src/components/page-shell.tsx) — Glacier `Panel` + `eyebrow` props already exist (added in the Glacier redesign). Reuse for the wizard step containers.
- [src/components/brand-mark.tsx](../../src/components/brand-mark.tsx) — V SVG mark; replace the wizard's `BrandMark` reference at the same spec used in Home.
- [src/styles/tailwind.css](../../src/styles/tailwind.css) — `.veyra-window-shell`, `.veyra-glass`, `.veyra-eyebrow`, `.veyra-italic` utilities. Reuse rather than introduce new ones.
- [src-tauri/src/main.rs](../../src-tauri/src/main.rs) — `check_model_downloaded`, `download_model`, `check_email_draft_model`, `open_ollama_download` already wired. Add `is_ollama_installed`.
- [src-tauri/src/draft_email.rs](../../src-tauri/src/draft_email.rs) — `check_email_draft_model` already pulls Ollama models when missing and emits `email-model:download:progress`. No changes needed.
- [src-tauri/src/downloader.rs](../../src-tauri/src/downloader.rs) — Whisper model downloader, emits `model:download:progress` and `download-progress`. No changes needed.
- [src-tauri/windows/hooks.nsh](../../src-tauri/windows/hooks.nsh) — NSIS post-install hook silent-installs Ollama; remains unchanged.
- [src/hooks/use-live-events.ts](../../src/hooks/use-live-events.ts) — listen pattern for IPC events. Mirror for the install-orchestration hook.
- [src/routes/settings/transcription.tsx](../../src/routes/settings/transcription.tsx) — current consumer of `model:download:progress` and `email-model:download:progress`. Reference for event payload shape and progress UI.

### Institutional Learnings

- The completed Glacier redesign (`docs/plans/2026-05-03-001-feat-glacier-veyra-redesign-plan.md`) established the `Panel` + `eyebrow` + `veyra-italic` recipe and the cyan/spark dual-engine convention. The wizard reuses both — Whisper steps tint cyan, Drafter steps tint spark amber.
- AGENTS.md guardrails preserved: typr filenames untouched, premium Apple-style sophistication, do not reintroduce removed bloat.

### External References

- None required. Local Tauri command + event patterns are sufficient.

---

## Key Technical Decisions

- **Wizard reuses existing IPC events.** No new download channels. The install step subscribes to `model:download:progress` (for Whisper), `email-model:download:progress` (for the Ollama email model), and a new `ollama:status` event emitted by `is_ollama_installed`. Net new IPC surface: one synchronous query command.
- **Single orchestrator hook.** `useInstallOrchestrator()` owns the state machine: per-step status (`idle | running | done | failed`), per-step progress (0–100), per-step error message, and a `runAll()` action that fires the three installs in parallel where independent (Whisper download and Ollama check are independent; email model pull depends on Ollama).
- **Per-step retry button.** Failure on any step renders an inline alert + Retry button that calls only that step's runner. Successful steps stay successful; the Retry doesn't restart upstream work.
- **Step ordering: Welcome → Install → Microphone → Hotkeys → Ready.** Install moves up so its async work starts immediately and finishes (or at least progresses meaningfully) while the user clicks through the lighter steps. Ready blocks `Start using Veyra` until all three installs report `done`, OR shows a clear "Continue without all components" secondary action that flips a settings flag the user can revisit.
- **Glacier visual contract.** Step shell is `Panel` from `page-shell.tsx` with the new wizard-specific eyebrow ("Step 02 · Install"). Each install card is its own `Panel` with the cyan / spark left rule used in `EngineCard`. Progress bars match the `KpiStrip` deltas style.
- **No NSIS-hook changes.** The Windows installer already silent-installs Ollama on first install. The wizard's "Install Ollama" CTA is the post-install recovery path, mirroring what Settings → Transcription already does.
- **Microphone step keeps `<select>`.** No need to make microphone selection a card grid; the current shadcn-styled `<select>` is fine — we only re-skin the surrounding shell.

---

## Open Questions

### Resolved During Planning

- Should the Install step block on Whisper before starting the Ollama check? **No** — they are independent. Run them in parallel; sequence the Ollama email-model pull after the runtime check passes.
- Should the wizard re-trigger Ollama install if the NSIS post-install hook failed? **No** — the wizard's Install button calls `open_ollama_download` (existing) when `is_ollama_installed` returns false. We do not re-run silent installers from inside the app.
- Is `is_ollama_installed` synchronous or async? **Synchronous Rust, async TS shell.** It does file-system existence checks and an optional `ollama --version` probe; cheap enough to run on every wizard mount.
- Should the user be able to swap models during the install step? **No** — model picker stays in the prior step (now folded into Install card details) but defaults to recommended; "Advanced" disclosure exposes alternates. Once install starts, the dropdown locks.

### Deferred to Implementation

- Whether `Skip and configure later` flips a settings flag (`setupSkipped: true`) that surfaces a banner on Home, or simply lands silently. Default: silent for v1; banner is a follow-up.
- Exact Ollama install detection logic — file paths from the NSIS hook are the seed (`$LOCALAPPDATA\Programs\Ollama\ollama.exe`, `$PROGRAMFILES\Ollama\ollama.exe`, `$PROGRAMFILES64\Ollama\ollama.exe`, PATH lookup). Mirror those in Rust.
- Whether to auto-advance from Install → Microphone the moment Whisper starts downloading, or wait for an explicit Next click. Default: explicit Next; auto-advance feels jumpy.

---

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce.*

```
Wizard step order (R7):
  Welcome → Install → Microphone → Hotkeys → Ready

Install step layout:
  ┌──────────────────────────────────────────┐
  │ STEP 02 · INSTALL                        │
  │ Install everything Veyra needs to run    │
  │                                          │
  │ ┌────────────┐ ┌────────────┐ ┌────────┐ │
  │ │ ① Whisper  │ │ ② Ollama   │ │ ③ Llama│ │
  │ │ Turbo · 1.5 GB │ │ runtime │ │ 3.2 1B  │ │
  │ │ ████░░ 64% │ │ ✓ Installed│ │ ░░░ pull│ │
  │ └────────────┘ └────────────┘ └────────┘ │
  │                                          │
  │  [ Install everything ]    [ Advanced ]  │
  └──────────────────────────────────────────┘

State machine (per card):
  idle → running → done
                ↘ failed → (Retry click) → running ...

Orchestrator parallelism:
  ① Whisper download              ─┐
  ② Ollama runtime presence check ─┤  parallel
                                    │
  ③ Llama 3.2 pull                 ─┘  starts only after ② done

Background continuation:
  User clicks Next → Microphone (cards keep ticking in the background)
  Ready step blocks "Start using Veyra" until all three are `done`.
  Secondary action: "Continue without all components" → finish anyway.
```

---

## Implementation Units

- U1. **Glacier visual refresh of the wizard shell**

**Goal:** Replace the prototype chrome (sky-blue chips, `bg-sky-50`, custom rounded surfaces) with the Glacier `Panel` + `veyra-eyebrow` + `veyra-italic` recipe. Same visual contract as the rest of the app. No behavior change yet.

**Requirements:** R1.

**Dependencies:** none.

**Files:**
- Modify: `src/routes/wizard.tsx`

**Approach:**
- Replace the `OnboardingShell` → `veyra-glass` rounded panel with a single `Panel` from `page-shell.tsx` carrying an `eyebrow` ("Step NN · <name>") and an italic Newsreader accent in the title.
- Replace the `bg-sky-50` `SetupChoice` cards with the Glacier `Panel` recipe at smaller scale, hairlined.
- Replace the step-progress strip with five hairlined dots / segments tinted cyan when active, neutral when pending. Mirror the `KpiStrip` divider density.
- Remove the legacy "First boot" sky chip — replace with the `veyra-eyebrow` mono caption above the title.
- Keep `BrandMark` at 48 px in the hero, same component already used in Home hero.
- Pure presentational refactor; the step state machine and IPC calls are untouched in this unit.

**Patterns to follow:**
- `src/routes/index.tsx` for hero composition.
- `src/components/page-shell.tsx` `Panel` usage with eyebrow.

**Test scenarios:**
- Test expectation: none — pure styling. Visual verification via screenshot vs Glacier mockup style during U5.

**Verification:**
- `npm run build` succeeds. Manual: walk through the wizard from Welcome to Ready and confirm every step reads as Glacier (light-glass, hairlines, italic accent in titles).

---

- U2. **`is_ollama_installed` Tauri command**

**Goal:** Cheap synchronous probe so the wizard can render Ollama status without invoking the heavier `check_email_draft_model` (which also tries to pull and would queue work even when we just want a presence check).

**Requirements:** R6.

**Dependencies:** none.

**Files:**
- Modify: `src-tauri/src/main.rs` (new `#[tauri::command] fn is_ollama_installed() -> bool` + register in `invoke_handler!`)
- Modify: `src/lib/tauri.ts` (add `isOllamaInstalled` IPC wrapper)

**Approach:**
- Mirror the file-existence checks the NSIS hook does: `$LOCALAPPDATA\Programs\Ollama\ollama.exe`, `%ProgramFiles%\Ollama\ollama.exe`, `%ProgramFiles(x86)%\Ollama\ollama.exe`, plus a `which ollama` (`std::process::Command::new("where").arg("ollama")` on Windows; fall back to `Path::env_var` parsing if `where` is not available).
- Returns `bool` directly. No async, no events.
- Cross-platform: on macOS check `/Applications/Ollama.app/Contents/Resources/ollama` plus `/usr/local/bin/ollama` plus `which ollama`. On Linux `/usr/local/bin/ollama` plus `/usr/bin/ollama` plus `which ollama`.

**Patterns to follow:**
- Existing `open_ollama_download` for cross-platform branching.
- NSIS hook `IfFileExists` pattern for the path list.

**Test scenarios:**
- Test expectation: none — Rust unit test infeasible without filesystem fixtures; behavior is a small file-existence check. Verified by manual inspection during U3 and U4.

**Verification:**
- `cargo test --manifest-path src-tauri/Cargo.toml` passes (no new tests, existing must stay green).
- Manual: launch the wizard with Ollama installed → button reads "✓ Installed". Uninstall Ollama → button reads "Install Ollama".

---

- U3. **`useInstallOrchestrator` hook (state machine + IPC plumbing)**

**Goal:** Centralize the state machine for the three installs (Whisper, Ollama runtime, email model). Returns per-step status, progress, error, and `run` / `retry` callbacks. Subscribes to existing IPC events.

**Requirements:** R2, R3, R4, R5.

**Dependencies:** U2 (so the Ollama presence query is a single IPC call).

**Files:**
- Create: `src/hooks/use-install-orchestrator.ts`
- Test: `src/hooks/use-install-orchestrator.test.ts`

**Approach:**
- Hook returns a single object:
  ```
  {
    whisper: { status, progress, error, retry() },
    ollama:  { status, error, retry() },
    drafter: { status, progress, error, retry() },
    runAll(),
    allDone: boolean,
    anyFailed: boolean,
  }
  ```
- `status` is `"idle" | "running" | "done" | "failed"`. `progress` is `0..100`.
- On mount: call `ipc.checkModelDownloaded(whisperModel)` to seed `whisper.status`, `ipc.isOllamaInstalled()` to seed `ollama.status`, and `ipc.checkEmailDraftModel(...)` to seed `drafter.status`. Initial probe runs in parallel.
- `runAll()`:
  - If `whisper.status !== "done"`, mark `running` and call `ipc.downloadModel(whisperModel)`. Subscribe to `model:download:progress` and update `progress`. Mark `done` when `model:download:done` fires (or when the IPC promise resolves). Mark `failed` with the error on rejection.
  - If `ollama.status !== "done"`, mark `running` and call `ipc.openOllamaDownload()`. Then poll `ipc.isOllamaInstalled()` every 2 s for up to 5 min. Mark `done` when it returns true. Mark `failed` on timeout with retry CTA.
  - When `ollama.status === "done"` AND `drafter.status !== "done"`: mark drafter `running` and call `ipc.checkEmailDraftModel(...)`. Subscribe to `email-model:download:progress`. Mark `done` on resolve, `failed` on reject.
- `retry()` per step re-runs only that step's runner.
- Cleanup: unsubscribe all listeners on unmount.

**Patterns to follow:**
- `src/routes/settings/transcription.tsx` for the `model:download:progress` and `email-model:download:progress` subscription patterns.
- `src/hooks/use-live-events.ts` for the listen/unlisten idiom.
- `src/hooks/use-window-maximized.ts` for the `let active = true` flag pattern that prevents stale state updates after unmount.

**Test scenarios:**
- Happy path: With all three IPC seeds resolving as `done`, `allDone` is `true` and `runAll()` is a no-op.
- Happy path: With `whisper` initially `idle`, calling `runAll()` calls `ipc.downloadModel()` exactly once and transitions through `running → done` when the IPC resolves.
- Edge case: A `model:download:progress` event fires while `whisper.status === "running"` — `whisper.progress` updates to the event's percent.
- Edge case: `runAll()` is invoked twice in quick succession — the second call does not start a duplicate download (the orchestrator guards against re-entry on `running` steps).
- Error path: `ipc.downloadModel()` rejects — `whisper.status` flips to `failed`, `whisper.error` carries the message. Calling `whisper.retry()` re-invokes the IPC.
- Error path: Ollama poll loop reaches the 5-minute timeout — `ollama.status` flips to `failed` with a timeout error message. Retry restarts the poll.
- Integration: Once `ollama.status` flips from `running → done`, the drafter step auto-starts (subscribes to `email-model:download:progress`).

**Verification:**
- `npm test -- --run src/hooks/use-install-orchestrator.test.ts` passes.

---

- U4. **Wizard Install step + step reordering**

**Goal:** Replace the existing `step === 2` "Models" branch with a new Install step that consumes `useInstallOrchestrator` and renders three Glacier engine-card-style mini panels with progress bars + retry buttons. Reorder steps to Welcome → Install → Microphone → Hotkeys → Ready (R7). Update Ready step to gate `Start using Veyra` on `allDone`.

**Requirements:** R2, R5, R7, R8.

**Dependencies:** U1, U3.

**Files:**
- Modify: `src/routes/wizard.tsx`
- Test: `src/routes/wizard.test.tsx` (new)

**Approach:**
- Update the `steps` array to `["Welcome", "Install", "Microphone", "Hotkeys", "Ready"]`.
- The Install step body renders three cards in a 3-up grid (1-up on narrow widths). Each card shows: brand chip (Whisper / Ollama / Llama), recommended-default subtitle (e.g. "Turbo · 1.5 GB"), inline progress bar driven by `useInstallOrchestrator()`, status pill (Idle / Downloading X% / Ready / Failed), and a Retry button when status is `failed`.
- A primary action button at the bottom: `Install everything` (calls `runAll()`); changes label to `Continue` when `allDone`.
- An `Advanced` disclosure under the cards exposes the existing model `<select>` controls (Whisper variant + email model variant) — locked once `runAll()` is invoked.
- Microphone step body unchanged except for Glacier shell.
- Hotkeys step body unchanged except for Glacier shell.
- Ready step shows three Operational pills (one per install) and a primary `Start using Veyra` that is **disabled** when `!allDone`. Secondary `Continue without all components` button surfaces only when `anyFailed` AND the user has clicked Retry at least once on the failed step (avoids one-click skip without trying).
- The orchestrator's state persists across step navigation because the hook is hosted at the wizard route level.

**Patterns to follow:**
- `src/components/page-shell.tsx` `Panel` for each install card.
- `src/components/engine-card.tsx` left-rule accent for the cyan / spark tint per card.
- `src/components/kpi-strip.tsx` for the per-cell delta typography idiom.
- Existing `Button` + `select` + `HotkeyInput` components stay.

**Test scenarios:**
- Happy path: Mounting the wizard at Install step renders three install cards and a primary button labeled "Install everything".
- Happy path: With `useInstallOrchestrator` mocked into `allDone === true`, the Ready step renders three "Operational" pills and the primary button is enabled.
- Edge case: With `whisper.status === "failed"`, the Whisper card shows the error message and a Retry button. Clicking Retry calls the orchestrator's `whisper.retry()` exactly once.
- Edge case: While `whisper.status === "running"` with `progress: 50`, the card shows "Downloading 50%".
- Integration: Walking from Install → Microphone → Hotkeys → Ready with the orchestrator initially `running` shows progress in Ready cards (background continuation per R5).
- Integration: `markWizardComplete` is called exactly once when the user clicks `Start using Veyra` and `allDone` is true.
- Edge case: `Start using Veyra` is disabled while any step is `idle` or `running`.
- Regression: Microphone selector still surfaces system mics from `ipc.listMicrophones()`.
- Regression: Hotkey defaults are `F24` and `Pause`.

**Verification:**
- `npm test -- --run src/routes/wizard.test.tsx` passes.
- Manual: delete the wizard-completed flag, launch the app, walk through every step, observe live progress, simulate a failure (kill download mid-flight in dev tools), retry, complete.

---

- U5. **Validation, screenshots, release notes**

**Goal:** Confirm the new wizard against the Glacier visual contract and AGENTS.md release rule, regenerate the wizard screenshot in `docs/design/implementation/`, and write release-note copy.

**Requirements:** R1, R9.

**Dependencies:** U1, U2, U3, U4.

**Files:**
- Modify (or create): `docs/design/implementation/veyra-wizard-install.png`
- Modify: `docs/plans/2026-05-03-005-feat-one-click-setup-wizard-plan.md` (the plan itself; mark complete in execution)

**Approach:**
- Run the full validation gate: `npm run build`, `npm test -- --run`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npx tauri build --no-bundle`.
- Capture screenshots of: Welcome → Install (idle / running / done / failed states) → Microphone → Hotkeys → Ready. Compare against the Glacier mockup style.
- Update release notes draft for the next version bump (call out: one-click install of Whisper + Ollama + email model; live progress bars per step; per-step retry; background continuation while stepping through Microphone / Hotkeys).

**Test scenarios:**
- Test expectation: none — validation + manual screenshot pass.

**Verification:**
- All four validation commands pass.
- Screenshot diff against the prior wizard shows the Glacier chrome adopted end to end.

---

## System-Wide Impact

- **Interaction graph:** Adds one new Tauri command (`is_ollama_installed`) and one new IPC wrapper. Subscribes to two existing event channels (`model:download:progress`, `email-model:download:progress`). No new emit calls.
- **Error propagation:** Each install step has its own `failed` state with an inline error message + retry button. Failures do not cascade — Whisper failing does not block Ollama or the email model from progressing.
- **State lifecycle risks:** The orchestrator hook lives at the wizard route level. Navigating between steps does NOT unmount it (the wizard route keeps the hook alive); only completing the wizard or closing the window does. Cleanup must unsubscribe all listeners + clear the Ollama poll interval.
- **API surface parity:** The Settings → Transcription manual install controls remain functional and unchanged. They are the canonical recovery path post-wizard.
- **Integration coverage:** Wizard test covers the state-machine transitions; manual QA covers the live IPC subscriptions during a real download.
- **Unchanged invariants:** `markWizardComplete` is called exactly once, exactly when the user explicitly clicks `Start using Veyra` (or the secondary skip path). Wizard gating (`useWizardGate`) behavior is unchanged. Hotkey defaults stay `F24` / `Pause`. NSIS installer hook is untouched.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Ollama silent install via NSIS hook fails on locked-down machines | The wizard's Install button calls `open_ollama_download` as a fallback, which opens the Ollama download page in the default browser. Same recovery path Settings already uses. |
| Whisper download is slow on poor networks; user navigates away mid-download | The orchestrator hook lives at the wizard route level so cross-step navigation preserves progress. Cleanup runs only when the wizard route unmounts (i.e., on completion or app close). |
| `email-model:download:progress` event name varies if the user swaps email engine to Groq mid-flight | Engine swap is gated by the Advanced disclosure. Once `runAll()` starts, the disclosure is locked. If the user retries with a different engine, the orchestrator re-subscribes. |
| `is_ollama_installed` returns false-positives via PATH but missing executable bit on Linux/macOS | The probe runs `ollama --version` once when the file exists; only true on success. Cheap (~50 ms). |
| Background install continues while user finishes wizard, then user closes the window mid-pull | Existing IPC continues to run in Rust regardless of webview state. Re-entering the wizard gate post-restart re-probes status and resumes from `done` if completed. |
| Visual diff vs Glacier mockup drifts during execution | AGENTS.md mandates screenshot comparison for material UI redesigns; U5 captures it explicitly. |
| Poll loop on `is_ollama_installed` (2 s × 150 = 5 min) leaks if the user closes the wizard mid-poll | Orchestrator cleanup clears the interval. Test scenario covers unmount cleanup. |

---

## Documentation / Operational Notes

- After implementation: regenerate `docs/design/implementation/veyra-wizard-install.png` (and any other wizard screenshots referenced from the README) so they show Glacier chrome.
- Per AGENTS.md "Mandatory Release Rule": every push to `main` lands a fresh GitHub Release with a Windows installer. Validation gates: `npm run build`, `npm test -- --run`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npx tauri build --no-bundle`, then `npm run tauri build`.
- Release notes should call out: "First-boot setup is now one click. Install Whisper, Ollama, and the local email-draft model in parallel with live progress bars and per-step retry. Glacier chrome end to end."

---

## Sources & References

- **Project guardrails:** [AGENTS.md](../../AGENTS.md)
- **Predecessor plans (completed):**
  - [docs/plans/2026-05-03-001-feat-glacier-veyra-redesign-plan.md](2026-05-03-001-feat-glacier-veyra-redesign-plan.md)
  - [docs/plans/2026-05-03-002-feat-glacier-overlay-capsule-plan.md](2026-05-03-002-feat-glacier-overlay-capsule-plan.md)
  - [docs/plans/2026-05-03-003-feat-overlay-style-size-settings-plan.md](2026-05-03-003-feat-overlay-style-size-settings-plan.md)
  - [docs/plans/2026-05-03-004-feat-rounded-window-shadow-plan.md](2026-05-03-004-feat-rounded-window-shadow-plan.md)
- Related code: [src/routes/wizard.tsx](../../src/routes/wizard.tsx), [src/components/page-shell.tsx](../../src/components/page-shell.tsx), [src/components/brand-mark.tsx](../../src/components/brand-mark.tsx), [src-tauri/src/main.rs](../../src-tauri/src/main.rs), [src-tauri/src/draft_email.rs](../../src-tauri/src/draft_email.rs), [src-tauri/src/downloader.rs](../../src-tauri/src/downloader.rs), [src-tauri/windows/hooks.nsh](../../src-tauri/windows/hooks.nsh), [src/routes/settings/transcription.tsx](../../src/routes/settings/transcription.tsx)
