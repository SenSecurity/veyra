---
title: Email Draft Command Mode
date: 2026-05-02
status: active
---

# Email Draft Command Mode

## Problem

Veyra already handles direct dictation with `F24`. The next useful capability is a second hotkey for command-style capture: the user speaks an instruction such as "reply to this email in English saying I can meet tomorrow", and Veyra inserts a polished draft instead of the raw transcript.

## Scope

In scope:
- Add a command hotkey path separate from dictation.
- Reuse the existing recording, transcription, overlay, sound, injection, and persistence pipeline.
- Generate a concise email/message draft from the transcribed instruction using Groq Chat Completions.
- Store command-mode history rows with `mode = "command"` and `enhanced = true`.
- Expose the command hotkey in Settings.

Out of scope for this first pass:
- Reading the current email thread from Outlook/Gmail/browser DOM.
- Selecting arbitrary app context automatically.
- Multi-step agents or tool use.

## Key Decisions

- Use `PipelineMode::Command` inside `src-tauri/src/pipeline/mod.rs` instead of creating a parallel pipeline.
- Use the same Groq API key already stored for cloud transcription. Command mode requires a key even when transcription itself is local.
- Use `llama-3.3-70b-versatile` as the default draft model, based on Groq's production model docs.
- Keep the generated text paste-only. No automatic send, no email app automation.
- Default new installs to `F12` for command mode, while existing settings can edit the field.

## Implementation Units

1. Command generation client
   - Files: `src-tauri/src/draft_email.rs`, `src-tauri/src/lib.rs`
   - Tests: unit tests for empty key/input behavior and prompt response parsing.

2. Pipeline branch
   - Files: `src-tauri/src/pipeline/mod.rs`
   - Tests: existing Rust tests plus command mode error/display coverage.

3. Hotkey registration
   - Files: `src-tauri/src/main.rs`
   - Tests: `cargo check`; behavior verified by app run.

4. Settings surface
   - Files: `src-tauri/src/settings/schema.rs`, `src-tauri/src/settings/legacy_v1.rs`, `src-tauri/src/settings/adapter.rs`, `src/types/settings.ts`, `src/routes/settings/hotkeys.tsx`
   - Tests: existing settings adapter/schema tests.

## Test Scenarios

- `F24` still records and pastes normal dictation.
- Command hotkey starts and stops recording without interfering with dictation hotkey.
- Command mode with local transcription and valid Groq key pastes a polished email draft.
- Command mode with missing Groq key fails cleanly and returns to Ready.
- Settings can save command hotkey without losing the dictation hotkey or Groq key.
- Rust tests, frontend tests, and Tauri build checks pass.
