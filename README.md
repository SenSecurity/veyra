<div align="center">

![Veyra — Glacier desktop dictation](docs/assets/veyra-hero.svg)

<h3>
  Speak. Watch words appear.
  <br>
  <em>Quiet desk, ready when you are.</em>
</h3>

<p>
  Veyra is a Windows speech-to-text companion with a second hotkey for email drafts.
  Local Whisper, local Llama, two hotkeys. Nothing leaves your machine.
</p>

<p>
  <a href="https://github.com/SenSecurity/veyra/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/SenSecurity/veyra?style=for-the-badge&color=2bc7ff&labelColor=0d1320"></a>
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-0d1320?style=for-the-badge&color=4d6480&labelColor=0d1320">
  <img alt="100% local" src="https://img.shields.io/badge/100%25-local-16a34a?style=for-the-badge&labelColor=0d1320">
  <img alt="One-click setup" src="https://img.shields.io/badge/setup-one--click-ffb454?style=for-the-badge&labelColor=0d1320">
</p>

<p>
  <a href="https://github.com/SenSecurity/veyra/releases/latest">↓ Download installer</a>
  &nbsp;·&nbsp;
  <a href="#install">Install</a>
  &nbsp;·&nbsp;
  <a href="#use">Use</a>
  &nbsp;·&nbsp;
  <a href="#models">Models</a>
  &nbsp;·&nbsp;
  <a href="#build-from-source">Build</a>
</p>

</div>

---

## Why Veyra

|  | Veyra |
|---|---|
| 🎙️ **Dictation** | Local `whisper.cpp`, sub-second latency, paste straight at the cursor |
| 📧 **Email Drafter** | Speak the intent, local Llama writes the draft, saved automatically |
| 🛡️ **Private** | Audio never leaves the machine. No cloud, no telemetry |
| ⚡ **One-click setup** | First boot installs Whisper, Ollama, and the email model in parallel |
| 🌊 **Glacier UI** | Light-glass shell, rounded window, two-tone engine duality (cyan + spark) |
| 🪟 **Floating overlay** | Pick **Capsule** or **Halo Orb**. Three sizes. Lives only while recording |

---

## Install

1. Download the latest installer from [Releases](https://github.com/SenSecurity/veyra/releases/latest).
2. Run `Veyra_*_x64-setup.exe`.
3. Open Veyra. The **first-boot wizard** runs automatically.

The wizard installs everything in **one click**:

| Step | What it does |
|------|--------------|
| ① **Whisper** | Downloads the speech model (~1.5 GB, default `turbo`) |
| ② **Ollama** | Detects the runtime; if missing, the first-boot wizard downloads and starts the official Ollama installer |
| ③ **Email model** | Pulls the local LLM (`Llama 3.2 1B` by default) via Ollama |

All three run in parallel where independent. Per-step retry. You can keep going through Microphone + Hotkeys while installs finish in the background.

---

## Hotkeys

| Action | Default | Configurable in |
|--------|---------|-----------------|
| **Dictation** | `F24` | Settings → Hotkeys |
| **Email draft** | `Pause` | Settings → Hotkeys |
| **Command palette** | `Ctrl + K` | — |
| **Collapse sidebar** | `Ctrl + \` | — |

---

## Use

### Dictation

1. Press `F24`.
2. Speak.
3. Press `F24` again.
4. Veyra transcribes the audio and pastes it at the cursor.

### Email draft

1. Press `Pause`.
2. Speak the instruction. Example:
   > *faz-me um email a dizer que hoje vou lá às cinco da tarde para o senhor Bruno Rodrigues*
3. Press `Pause` again.
4. Veyra writes the draft and saves a copy under **Email Drafter**.

Drafts default to the language you spoke. Switch explicitly with `diz em inglês` / `diz em francês`. Drafts run a touch fuller on purpose — easier to delete than to ask for more.

---

## The recording overlay

While you record, a floating overlay appears above all windows. Pick the look in **Settings → Overlay**.

|  | Capsule | Halo Orb |
|--|---------|----------|
| **Shape** | Horizontal light-glass pill | Compact dark squircle with concentric rings |
| **Footprint** | 420 / 520 / 640 px wide | 72 / 96 / 128 px diameter |
| **Best for** | Wide monitors, transcript glances | Minimal footprint, ambient feel |

Tone follows the active engine: **cyan** for STT (Whisper), **spark amber** for the Drafter (Llama).

---

## Models

### Speech-to-text (`whisper.cpp`)

| Model | Size | Notes |
|-------|------|-------|
| **`turbo`** ⭐ | ~1.5 GB | Recommended default |
| `base` | ~150 MB | Fastest, lightest |
| `large-v3` | ~3 GB | Highest accuracy |

### Email drafter (Ollama, local)

| Model | Notes |
|-------|-------|
| **`Llama 3.2 1B`** ⭐ | Recommended default |
| `Llama 3.2 3B` | Stronger phrasing |
| `Qwen3 1.7B` | Fast |
| `Qwen3 4B` | Stronger |
| `Ternary Bonsai 1.7B F16` | Experimental — Prism's Bonsai GGUF via Ollama |

> **Bonsai note** — Veyra uses Prism's `Ternary-Bonsai-1.7B-F16.gguf` through Ollama. The smaller Prism `Q2_0` variants aren't exposed because this Ollama build fails to load them reliably.

---

## Data

Veyra stores settings, database, logs, and downloaded models locally:

```text
%APPDATA%\com.typr.app\
```

Nothing is sent off-machine. No telemetry, no analytics.

---

## Build from source

**Requirements:** Windows · Node.js · Rust

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

---

## Design

Veyra ships with the **Glacier** visual system:

- **Palette** — ice / cyan electric / spark amber / black / white. Two-tone engine duality (cyan = STT, spark = Drafter) carries through every surface.
- **Typography** — Inter Tight (display + body), JetBrains Mono (kbd, eyebrows), Newsreader italic (sparing display accents).
- **Surfaces** — Light glass shells, hairline borders, rounded window with soft drop shadow.
- **Brand mark** — Inline SVG: dark squircle, cyan V waveform, warm spark at the base. Used at 22 px in the titlebar and 88 px in the Home hero.

See `docs/plans/` for the full design + implementation history (Glacier shell, overlay capsule + halo orb, rounded window, one-click wizard).

---

## Release rule

Every push to `main` ships a fresh GitHub Release with a Windows installer attached. See [Releases](https://github.com/SenSecurity/veyra/releases) for history.

---

<div align="center">
  <sub>
    Built with Tauri 2 · React · Rust · whisper.cpp · Ollama
    <br>
    <em>— quiet desk, ready when you are.</em>
  </sub>
</div>
