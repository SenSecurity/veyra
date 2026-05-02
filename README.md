# Veyra

![Veyra interface preview](docs/assets/veyra-hero.svg)

**Veyra** is a local-first Windows dictation app built with Tauri, React, whisper.cpp, and Groq fallback. It is designed to feel fast, quiet, and close to the text field you are working in: press the hotkey, speak, stop, and Veyra transcribes into the active app.

## Highlights

- Floating recording overlay with animated waveform and transcribing state.
- Local whisper.cpp transcription with optional Groq fallback.
- Setup wizard for first-run configuration.
- History, dictionary, snippets, scratchpad, stats, and settings screens.
- Global hotkey support with push-to-talk and toggle-style recording flows.
- Model download controls with cancellation support.
- Local SQLite storage for transcription history and productivity data.

## Current Status

Veyra is in active development. The product branding is Veyra, while some internal paths and binary names still use `typr` for compatibility with existing local settings, models, and data.

## Development

```bash
npm install
npm run dev
```

Run the desktop app in development:

```bash
npm run tauri dev
```

Build and validate:

```bash
npm run build
npm test
cargo check --manifest-path src-tauri/Cargo.toml
npx tauri build --no-bundle
```

The release executable is currently produced at:

```text
src-tauri/target/release/typr.exe
```

## Tech Stack

- Tauri 2
- React 19
- TypeScript
- Tailwind CSS
- SQLite
- whisper.cpp
- Groq Whisper API fallback
- Rust audio and transcription pipeline

## Roadmap

- Full Veyra packaging migration, including executable and app identifier.
- Better onboarding around local model download and health status.
- More polished sound design and overlay states.
- Import/export for dictionary, snippets, and settings.
- Installer and signed release builds.
