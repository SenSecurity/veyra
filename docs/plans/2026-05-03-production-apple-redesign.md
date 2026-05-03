---
title: Production Apple-Style Redesign
status: completed
created: 2026-05-03
origin: user request in Codex session
---

# Production Apple-Style Redesign

## Problem Frame

Veyra works, but the application UI still feels like a functional prototype in several places. The target is a production-quality Windows utility with Apple-style restraint: white-blue surfaces, graphite chrome, precise spacing, rich but quiet controls, and polished first-run setup. The app must feel more refined than Whispr Flow while staying focused on the actual product: speech-to-text, email drafting, history, dictionary, and settings.

## Scope

In scope:

- Create durable mockup images for the overall app and each primary menu.
- Update app shell, titlebar, sidebar, panels, rows, forms, and first boot wizard.
- Use the new Veyra icon consistently in app chrome and Windows assets where possible.
- Lock first boot to onboarding until setup is complete.
- Add product guidance to `AGENTS.md`: Apple-style sophistication and release discipline.
- Remove visible traces of model refusal output from history/email-draft surfaces where feasible without destructive data mutation.
- Validate with screenshots, unit tests, build, installer generation, local install, push, and GitHub release.

Out of scope:

- Changing the Rust transcription pipeline unless needed for UI/onboarding correctness.
- Reintroducing removed bloat such as Snippets, Scratchpad, placeholder settings sections.
- Replacing the core icon concept unless current assets fail to load.

## Design Direction

Visual thesis: a calm, glassy white-blue desktop utility with graphite window chrome, cyan audio signal, compact high-confidence typography, and restrained Apple-like depth.

Content plan:

- App shell: graphite titlebar, compact command button, recording status, narrow sidebar with product-ready navigation.
- Home: two primary work modules, live readiness, recent activity, compact stats.
- History: searchable transcript timeline with quality status, copy/delete, mode badges.
- Email Drafter: draft inbox with email-focused cards, copy/delete, saved fallback visibility.
- Dictionary: compact correction table and add-entry composer.
- Settings: three strong sections only: General, Transcription, Hotkeys.
- First boot: full-screen setup flow, no access to other menus until complete, clear options for mic, dictation model, email engine/model, hotkeys.

Interaction plan:

- Soft focus/hover transitions on controls.
- Progress/ready states that read as operational, not decorative.
- First boot step transitions and completion summary.

## Requirements Trace

- R1: Provide mockup images for app overview and each menu.
- R2: Improve visual quality objectively: typography, spacing, palette, shadows, component consistency.
- R3: Replace titlebar placeholder icon with Veyra icon treatment.
- R4: Add AGENTS guidance requiring Apple-style sophistication.
- R5: First boot must be a dedicated setup experience with no regular menu access before completion.
- R6: Existing bad email refusal text must not keep looking like a valid email draft in product surfaces.
- R7: Keep app focused on Home, History, Email Drafter, Dictionary, Settings.
- R8: Build installer and release after pushing to `main`.

## Existing Patterns

- React + Vite + Tauri frontend in `src/`.
- Tailwind v4 tokens and utility classes in `src/styles/tailwind.css`.
- Shared shell in `src/app.tsx`, `src/layout/sidebar.tsx`, `src/layout/window-titlebar.tsx`.
- Shared panels in `src/components/page-shell.tsx`.
- First boot gate in `src/hooks/use-wizard-gate.ts` and `src/routes/wizard.tsx`.
- Settings store in `src/stores/settings-store.ts`, typed IPC wrapper in `src/lib/tauri.ts`.

## Implementation Units

### U1: Design Mockup Board

Files:

- Create `docs/design/veyra-production-mockups.html`
- Create `docs/design/mockups/*.png`

Approach:

- Build a static HTML design board using the target visual system.
- Render one overview mockup plus Home, History, Email Drafter, Dictionary, Settings, First Boot.
- Keep text realistic and code-native, not marketing copy.

Tests / Verification:

- Render screenshots at desktop app size.
- Inspect mockups with `view_image`.

### U2: Design System Tokens and Shared Components

Files:

- Modify `src/styles/tailwind.css`
- Modify `src/components/page-shell.tsx`
- Modify `src/components/transcription-row.tsx`
- Add shared helpers/components if needed under `src/components/`

Approach:

- Tighten page spacing, panel depth, buttons, toolbar, rows.
- Introduce reusable premium primitives rather than one-off styling.
- Ensure copy/delete controls remain visible and accessible.

Tests / Verification:

- `npm run build`
- Visual screenshots for Home, History, Email Drafter, Dictionary, Settings.

### U3: App Shell and Icon Treatment

Files:

- Modify `src/layout/window-titlebar.tsx`
- Modify `src/layout/sidebar.tsx`
- Inspect/possibly regenerate `src-tauri/icons/*`

Approach:

- Use a code-native Veyra icon mark in titlebar/sidebar that matches existing icon assets.
- Keep graphite titlebar compact but more refined.
- Avoid placeholder lucide icon as brand icon.

Tests / Verification:

- Screenshot titlebar and sidebar.
- Confirm Windows installer still uses current icon assets.

### U4: First Boot Setup

Files:

- Modify `src/app.tsx`
- Modify `src/hooks/use-wizard-gate.ts`
- Modify `src/routes/wizard.tsx`
- Possibly modify `src/router.tsx`

Approach:

- If wizard incomplete, show onboarding as a dedicated app surface without sidebar, command palette, or regular menu access.
- Steps: Welcome, Microphone, Dictation model, Email draft model, Hotkeys, Ready.
- Save options directly through existing settings store/IPC.
- Provide operational checks/download actions where existing IPC supports them.

Tests / Verification:

- Unit/build coverage where available.
- Manual screenshot of first boot route.
- Confirm completed wizard returns to normal shell.

### U5: Product Surface Cleanup

Files:

- Modify `src/routes/history.tsx`
- Modify `src/routes/email-drafts.tsx`
- Possibly modify `src/components/transcription-row.tsx`

Approach:

- Detect refusal/meta outputs at render time and label them as recovered/unusable rather than presenting them as a valid draft.
- Prefer non-destructive cleanup: no automatic deletion of user history.
- Keep copy/delete working.

Tests / Verification:

- Add or update frontend tests if logic is extracted.
- Manual screenshot with current stored refusal item.

### U6: Agent Instructions and Release

Files:

- Modify `AGENTS.md`
- Version bump in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`

Approach:

- Add explicit Apple-style UI bar and mockup/screenshot discipline.
- Build, install, push, create GitHub release with installer.

Tests / Verification:

- `npm run build`
- `npm test -- --run`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `npx tauri build --no-bundle`
- `npm run tauri build`
- Install NSIS locally and launch.

## Risks and Edge Cases

- Wizard gate can race and briefly show normal menus. Fix by rendering shell conditionally, not just navigating.
- Current refusal text exists in stored history. Avoid destructive migration; mark/recover visually.
- Tauri app icon can differ between titlebar, taskbar, Windows Search, and installer assets. Validate generated assets and titlebar separately.
- Hotkey capture must stay unchanged.
- Settings model checks/download states must remain usable after visual changes.
- Text must not overflow in 900px-wide app windows.

## Completion Criteria

- Mockup images exist and are viewable.
- App implementation matches mockup direction across every primary menu.
- First boot is a refined locked setup flow.
- New icon treatment appears in Veyra titlebar/sidebar.
- Existing bad refusal draft no longer appears as a normal valid draft.
- Validation passes.
- New installer release is published and installed locally.
