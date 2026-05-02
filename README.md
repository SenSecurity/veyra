# Veyra

![Veyra interface preview](docs/assets/veyra-hero.svg)

Veyra is a Windows speech-to-text app. Press a hotkey, speak, stop, and Veyra writes into the app you were using.

It runs dictation locally with whisper.cpp. Email/message drafts can run locally with Ollama.

## Install

Download the latest Windows installer from:

[github.com/SenSecurity/veyra/releases/latest](https://github.com/SenSecurity/veyra/releases/latest)

Run:

```text
Veyra_0.1.0_x64-setup.exe
```

The installer also installs Ollama if it is missing. Ollama is used for local email/message drafts.

## First Setup

1. Open Veyra.
2. Go to **Settings -> Transcription**.
3. Under **Whisper model**, keep `turbo - Recommended`.
4. Click **Download model**.
5. Under **Email draft model**, keep `Llama 3.2 3B - Recommended`.
6. Click **Download email model**.
7. Go to **Settings -> Hotkeys** and set the keys you want.

## Use

Default modes:

- Dictation: speak normal text and Veyra pastes the transcription.
- Email draft: speak an instruction like "reply to this email in English saying I can meet tomorrow" and Veyra writes the draft.

The floating overlay appears while recording and transcribing. The tray icon lets you show, hide, or exit Veyra.

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

Output:

```text
src-tauri/target/release/bundle/nsis/
```

Build only the executable:

```bash
npx tauri build --no-bundle
```

Note: the direct executable is still named `typr.exe` internally. Use the installer for the normal product install flow.

## Validate

```bash
npm run build
npm test -- --run
cargo test --manifest-path src-tauri/Cargo.toml
npx tauri build --bundles nsis
```

## Notes

- App data is currently stored under `%APPDATA%/com.typr.app/` for compatibility with earlier builds.
- Some internal filenames still use `typr`; the product name is Veyra.
