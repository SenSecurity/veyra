# Veyra Agent Instructions

## Project

Veyra is a Windows Tauri app for local speech-to-text and email/message draft generation.

Core product behavior:

- Dictation uses local `whisper.cpp`.
- Email Draft uses Ollama or Groq, with a deterministic local fallback so drafts are still saved if the LLM fails.
- The floating overlay must appear while recording/transcribing and disappear when work finishes.
- User-facing product name is **Veyra**. Some internal filenames still use `typr`; do not rename those casually.

## Working Rules

- Prefer small, direct changes that preserve existing app behavior.
- Do not reintroduce removed product bloat such as Snippets, Scratchpad, placeholder settings pages, or unused navigation unless explicitly requested.
- Do not revert user changes or unrelated dirty work.
- Keep UI text concise and product-focused.
- For frontend edits, verify text fits and controls remain usable at the current app size.
- For global hotkeys, remember existing defaults:
  - Dictation: `F24`
  - Email Draft: `Pause`

## Validation Before Shipping

Run these before a push that changes product behavior:

```bash
npm run build
npm test -- --run
cargo test --manifest-path src-tauri/Cargo.toml
npx tauri build --no-bundle
```

For installer releases, also run:

```bash
npm run tauri build
```

If `npx tauri build --no-bundle` or `npm run tauri build` fails on Windows with access denied for `typr.exe`, close the running Veyra process and retry:

```powershell
Get-Process | Where-Object { $_.ProcessName -match 'typr|veyra' } | Stop-Process -Force -ErrorAction SilentlyContinue
```

## Mandatory Release Rule

Every time changes are pushed to `main`, create a new Windows installer release with the pushed changes.

Required release flow:

1. Ensure all validation commands pass.
2. Build the installer with `npm run tauri build`.
3. Find the NSIS installer under `src-tauri/target/release/bundle/nsis/`.
4. Create a new GitHub Release with:
   - a new tag;
   - concise release notes summarizing the pushed changes;
   - the Windows installer attached.
5. Confirm the release URL in the final response.

Do not tell the user "push is done" after a product push unless the matching installer release is also created or a blocker is clearly reported.

## Current Release Notes Checklist

When drafting release notes, include user-visible changes first:

- Dictation and overlay behavior.
- Email Drafter and saved drafts.
- Model/download changes, including Ollama and experimental Bonsai support.
- Hotkey changes.
- Install/setup changes.
- Bug fixes and validation.
