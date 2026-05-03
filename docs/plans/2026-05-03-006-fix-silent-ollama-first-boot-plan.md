---
title: "fix: Silent Ollama install from first boot"
type: fix
status: implemented
date: 2026-05-03
origin: user request - first boot should install Ollama without showing the Ollama setup window when possible
---

# fix: Silent Ollama install from first boot

## Problem

Veyra 0.1.18 correctly moved Ollama installation out of the Windows installer and into first boot. However, `Install everything` currently launches `OllamaSetup.exe` interactively, so the user sees the Ollama installer window.

Target behavior: first boot should keep the install experience inside Veyra. The Ollama setup should run silently when supported, while Veyra shows status/progress in its own install cards.

## Requirements

- R1. The Veyra NSIS installer must keep doing nothing with Ollama.
- R2. First boot `Install everything` downloads the official Ollama setup if Ollama is missing.
- R3. On Windows, Veyra first tries a silent Ollama install using Inno Setup flags: `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-`.
- R4. Veyra waits for the silent installer to finish, then re-checks installed paths and PATH.
- R5. If silent install fails or times out, Veyra falls back to launching the same installer visibly and tells the user to complete it.
- R6. UI copy must say `Installing Ollama` during silent mode, not `Complete the Ollama installer`.
- R7. Existing manual recovery from Settings > Transcription must remain available.

## Files

- `src-tauri/src/main.rs`
- `src/hooks/use-install-orchestrator.ts`
- `src/hooks/use-install-orchestrator.test.ts`
- `src/lib/tauri.ts` if the IPC contract changes
- `README.md`

## Technical Approach

`install_ollama_runtime_windows()` should:

1. Return immediately if `is_ollama_installed()` is already true.
2. Download `https://ollama.com/download/OllamaSetup.exe` to `%TEMP%\Veyra-OllamaSetup.exe`.
3. Run the installer with:
   - `/VERYSILENT`
   - `/SUPPRESSMSGBOXES`
   - `/NORESTART`
   - `/SP-`
4. Poll `is_ollama_installed()` while the installer runs. Do not rely on installer process exit, because the Ollama setup can install successfully while leaving the app process alive.
5. If installed, return `Ok`.
6. If the silent process is still alive after timeout, return `Ok` and let the wizard keep polling.
7. If the silent process exits without installing, launch the installer visibly as fallback.

The current failure came from using NSIS-style `/S` earlier. The Ollama setup window identifies as Inno Setup, so `/VERYSILENT` is the correct silent mode.

## Tests

- `src/hooks/use-install-orchestrator.test.ts`
  - Existing missing-Ollama test still verifies `installOllamaRuntime()` is called.
  - Added chain-order coverage: Ollama first, email model pull second, Whisper third.

- Rust validation:
  - `cargo test --manifest-path src-tauri/Cargo.toml`

- Frontend validation:
  - `npm test -- --run`
  - `npm run build`

## Manual Stress Test

1. Stop Veyra/Ollama.
2. Remove:
   - `%APPDATA%\com.typr.app`
   - `%LOCALAPPDATA%\com.typr.app`
   - `%LOCALAPPDATA%\Programs\Ollama`
   - `%LOCALAPPDATA%\Ollama`
   - `%USERPROFILE%\.ollama`
3. Install Veyra.
4. Open first boot.
5. Click `Install everything`.
6. Confirm no Ollama setup window appears during the normal path.
7. Confirm Veyra detects Ollama as installed.
8. Confirm email model pull starts after Ollama is available.

## Implementation Notes

- Direct machine test showed the Ollama installer creates `ollama.exe` successfully with `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-`, but the installer process does not exit promptly. The implementation therefore polls installed state instead of waiting for process completion.

## Release

If implemented, bump to `0.1.19`, build `Veyra_0.1.19_x64-setup.exe`, push `main`, and publish a GitHub release with the installer.
