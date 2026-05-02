# Veyra

![Veyra interface preview](docs/assets/veyra-hero.svg)

Veyra is a Windows dictation tool. Press a hotkey, speak, stop, and Veyra writes into the app you were already using.

It uses local `whisper.cpp` for speech-to-text. Email drafts can run locally with Ollama, with a built-in fallback so a draft is still saved if the local model is unavailable.

## Install

Download the latest Windows installer:

[github.com/SenSecurity/veyra/releases/latest](https://github.com/SenSecurity/veyra/releases/latest)

Run the installer:

```text
Veyra_0.1.0_x64-setup.exe
```

The installer tries to install Ollama automatically if it is missing. If that fails, open **Settings > Transcription** in Veyra and click **Install Ollama**.

## First Run

1. Open Veyra.
2. Go to **Settings > Transcription**.
3. Keep **Whisper model** as `turbo - Recommended`.
4. Click **Download model**.
5. Keep **Email draft engine** as `Local Ollama`.
6. Keep **Email draft model** as `Llama 3.2 3B - Recommended`, or choose a lighter/experimental model.
7. Click **Download email model**.

## Default Hotkeys

- **F24**: dictation.
- **Pause**: email draft.
- **Ctrl+K**: command palette.
- **Ctrl+\\**: collapse or expand the sidebar.

You can change the dictation and email draft hotkeys in **Settings > Hotkeys**. Restart Veyra after changing global shortcuts.

## Use

Dictation:

1. Press **F24**.
2. Speak normally.
3. Press **F24** again.
4. Veyra transcribes and pastes the text.

Email draft:

1. Press **Pause**.
2. Say an instruction, for example: `faz-me um email a dizer que hoje vou la as 5 da tarde para o senhor Bruno Rodrigues`.
3. Press **Pause** again.
4. Veyra writes the draft and saves it under **Email Drafter**.

The floating overlay shows recording and transcribing state. The Windows tray icon lets you show, hide, or exit Veyra.

## Email Draft Models

Stable local option:

- `Llama 3.2 3B - Recommended`

Lighter local options:

- `Llama 3.2 1B`
- `Qwen3 1.7B`
- `Qwen3 4B`

Experimental Bonsai options:

- `Ternary Bonsai 1.7B`
- `Ternary Bonsai 4B`
- `Ternary Bonsai 8B`

Bonsai downloads GGUF files from Hugging Face and creates an Ollama model. These models use PrismML `Q2_0` GGUF; if your Ollama build does not support that format yet, Veyra will show an error and continue using the local fallback.

## Data

App data is stored under:

```text
%APPDATA%\com.typr.app\
```

This includes settings, the local database, logs, and downloaded email draft GGUF files.

## Build From Source

Requirements:

- Windows
- Node.js
- Rust

Install dependencies:

```bash
npm install
```

Run in development:

```bash
npm run tauri dev
```

Build the Windows installer:

```bash
npm run tauri build
```

Installer output:

```text
src-tauri/target/release/bundle/nsis/
```

Build only the executable:

```bash
npx tauri build --no-bundle
```

The direct executable is still named `typr.exe` internally. Use the installer for the normal product install flow.

## Validate

```bash
npm run build
npm test -- --run
cargo test --manifest-path src-tauri/Cargo.toml
npx tauri build --no-bundle
```
