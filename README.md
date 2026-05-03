# Veyra

![Veyra interface preview](docs/assets/veyra-hero.svg)

Veyra is a Windows speech-to-text tool with a second hotkey for email drafts.

## Install

1. Download the latest installer from [Releases](https://github.com/SenSecurity/veyra/releases/latest).
2. Run `Veyra_*_x64-setup.exe`.
3. Open Veyra.
4. Go to **Settings > Transcription**.
5. Click **Download model** for Whisper.
6. Click **Download email model** for the local email drafter.

The installer attempts to install Ollama when missing. If that fails, Veyra shows **Install Ollama** in **Settings > Transcription**.

## Default Hotkeys

- **F24**: dictation
- **Pause**: email draft
- **Ctrl+K**: command palette
- **Ctrl+\\**: collapse sidebar

Change hotkeys in **Settings > Hotkeys**, then restart Veyra.

## Use

Dictation:

1. Press **F24**.
2. Speak.
3. Press **F24** again.
4. Veyra transcribes and pastes the text.

Email draft:

1. Press **Pause**.
2. Say the instruction, for example: `faz-me um email a dizer que hoje vou la as 5 da tarde para o senhor Bruno Rodrigues`.
3. Press **Pause** again.
4. Veyra writes the draft and saves a copy under **Email Drafter**.

## Models

Speech-to-text uses local `whisper.cpp`.

Recommended:

- `turbo`

Email drafts use local Ollama by default.

Recommended:

- `Llama 3.2 1B`

Other local options:

- `Llama 3.2 3B`
- `Qwen3 1.7B`
- `Qwen3 4B`
- `Ternary Bonsai 1.7B F16` (experimental)

Bonsai note: Veyra uses Prism's `Ternary-Bonsai-1.7B-F16.gguf` through Ollama. The smaller Prism `Q2_0` Bonsai GGUF files are not exposed because this Ollama build fails to load them reliably.

## Data

Veyra stores settings, database, logs, and downloaded models here:

```text
%APPDATA%\com.typr.app\
```

## Build

Requirements:

- Windows
- Node.js
- Rust

```bash
npm install
npm run build
npm test -- --run
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri build
```

Installer output:

```text
src-tauri/target/release/bundle/nsis/
```

## Release Rule

Every push to `main` must include a new GitHub release with a fresh Windows installer.
