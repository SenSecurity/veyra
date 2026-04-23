---
type: spec
created: 2026-04-23
project: typr
status: approved
---

# Wispr Flow Feature Parity — Design Spec

## Section 1 — Overview

### Mission
Replace Wispr Flow personally. Local-first dictation app for a single user (Bruno), running on Windows. Zero mandatory cloud. Groq optional cloud engine. Custom visual design inspired by Notion, not a clone of Wispr Flow.

### Principles
- **Local-first.** Transcription, storage, settings all work offline. Cloud only when user opts in (Groq).
- **Modular.** Every subsystem (storage, transcribe, format, input, overlay) is a self-contained module with a clear interface.
- **Settings-driven UX.** Anything that could be a preference is a preference. No hard-coded UX choices where reasonable alternatives exist.
- **Privacy-native.** No telemetry, no crash reporting cloud, API keys in OS keyring, logs local only.
- **Custom design.** Notion-like warm, light-first aesthetic with generous spacing. No imitation of Wispr Flow's visual language.

### Stack

**Backend (Rust + Tauri 2)**
- Tauri 2 framework
- `rusqlite` (bundled, FTS5) for storage
- `whisper.cpp` sidecar for local transcription
- `reqwest` for Groq API
- `arboard` clipboard + `enigo` for Ctrl+V injection
- `keyring` (Windows Credential Manager) for secrets
- `tracing` + `tracing-appender` for logs
- `cpal` for audio capture
- `rodio` for sound playback

**Frontend (React 19 + Vite)**
- React 19 + TypeScript
- Vite build
- Tailwind v4 with `@theme` tokens (oklch color space)
- shadcn/ui components (copied, not packaged)
- zustand for state
- tanstack-router (code-based) + tanstack-virtual
- framer-motion for overlay animations
- cmdk for command palette
- Inter Variable + JetBrains Mono Variable fonts

### Scope (V1)
- Windows only
- Single user
- Dictation + Command mode (Shift+F24 clipboard hijack)
- History with FTS search
- Dictionary (custom terms + abbreviations + auto-add)
- Snippets (trigger → expansion)
- Scratchpad (quick notes)
- Stats (daily word count, streak, milestones)
- Settings (all user-facing)
- Tray icon + menu
- Overlay (three styles: pill, bar, tray-only)
- Light/dark theme
- Launch-at-login, close-to-tray
- Sounds on dictate
- Mute music on dictate (best-effort, Windows audio sessions)
- Window context capture (foreground window title → app_context)

### Non-goals (V1)
- Team/collab/cloud sync
- Billing/subscriptions
- macOS/Linux/mobile
- Auto-updater
- Local LLM for enhance (use Groq only)
- IDE extensions / Vibe Coding companion (Wispr Flow has this; we use window-title context only)
- Creator mode
- HIPAA/SOC2/SSO
- Crash telemetry

---

## Section 2 — Data model

### SQLite file
`%APPDATA%\com.typr.app\typr.db`

### Schema (`migrations/001_initial.sql`)

```sql
CREATE TABLE transcriptions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL,
    raw_text TEXT NOT NULL,
    final_text TEXT NOT NULL,
    word_count INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    language TEXT NOT NULL,
    engine TEXT NOT NULL,        -- 'local' | 'groq'
    model TEXT,
    app_context TEXT,
    mode TEXT NOT NULL,          -- 'dictation' | 'command'
    enhanced INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_transcriptions_created ON transcriptions(created_at DESC);

CREATE VIRTUAL TABLE transcriptions_fts USING fts5(
    final_text, app_context,
    content='transcriptions', content_rowid='id'
);

-- External-content FTS5 sync triggers (mandatory — FTS5 does NOT auto-sync)
CREATE TRIGGER transcriptions_ai AFTER INSERT ON transcriptions BEGIN
    INSERT INTO transcriptions_fts(rowid, final_text, app_context)
    VALUES (new.id, new.final_text, new.app_context);
END;
CREATE TRIGGER transcriptions_ad AFTER DELETE ON transcriptions BEGIN
    INSERT INTO transcriptions_fts(transcriptions_fts, rowid, final_text, app_context)
    VALUES ('delete', old.id, old.final_text, old.app_context);
END;
CREATE TRIGGER transcriptions_au AFTER UPDATE ON transcriptions BEGIN
    INSERT INTO transcriptions_fts(transcriptions_fts, rowid, final_text, app_context)
    VALUES ('delete', old.id, old.final_text, old.app_context);
    INSERT INTO transcriptions_fts(rowid, final_text, app_context)
    VALUES (new.id, new.final_text, new.app_context);
END;

CREATE TABLE dictionary_terms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    term TEXT NOT NULL UNIQUE,
    replacement TEXT,
    is_abbreviation INTEGER NOT NULL DEFAULT 0,
    auto_added INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE snippets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    trigger TEXT NOT NULL UNIQUE,
    expansion TEXT NOT NULL,
    description TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    use_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE scratchpad_notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    title TEXT,
    body TEXT NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE stats_daily (
    day TEXT PRIMARY KEY,              -- YYYY-MM-DD
    word_count INTEGER NOT NULL DEFAULT 0,
    session_count INTEGER NOT NULL DEFAULT 0,
    total_duration_ms INTEGER NOT NULL DEFAULT 0,
    avg_wpm REAL
);

CREATE TABLE app_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL
);
```

### Settings shape (TypeScript)

```ts
type Settings = {
  schemaVersion: 2;
  microphone: string;                            // device name
  transcription: {
    engine: 'local' | 'groq';                    // default 'groq' if key exists else 'local'
    whisperModel: 'turbo' | 'large-v3' | 'base'; // default 'turbo'
    // NOTE: groqApiKey lives ONLY in Windows Credential Manager via keyring crate.
    //       It is NEVER part of the Settings JSON shape. Frontend reads presence
    //       via `system_cmd::has_groq_key()` returning bool.
    languages: string[];                         // ['pt', 'en']
    autoDetect: boolean;                         // default true
    gpuAcceleration: 'auto' | 'cpu' | 'cuda' | 'vulkan'; // default 'auto'
    vadEnabled: boolean;                         // default true
    noSpeechThreshold: number;                   // default 0.6
  };
  hotkeys: {
    dictation: string;                           // default 'F24'
    commandMode: string;                         // default 'Shift+F24'
    recordingMode: 'toggle' | 'push-to-talk';    // default 'push-to-talk'
  };
  overlay: {
    style: 'pill' | 'bar' | 'tray';              // default 'pill'
    position: 'near-cursor' | 'bottom-center' | 'custom';
    customPos?: { x: number; y: number };
  };
  formatting: {
    enhanceEnabled: boolean;                     // default false
    removeFillers: boolean;                      // default true
    fillerWords: string[];                       // default ['uh','um','né','tipo']
    explicitCommands: boolean;                   // default true
  };
  dictionary: {
    autoAdd: boolean;                            // default false
  };
  stats: {
    enabled: boolean;                            // default true
    milestoneNotifications: boolean;             // default true
  };
  data: {
    wordCountCap: number;                        // default 500000
    purgeOnExceed: boolean;                      // default true
  };
  system: {
    launchAtLogin: boolean;                      // default false
    closeToTray: boolean;                        // default true
    dictationSounds: boolean;                    // default true
    muteMusicOnDictate: boolean;                 // default false
  };
  ui: {
    language: 'en' | 'pt';                       // default 'en'
    theme: 'light' | 'dark' | 'system';          // default 'system'
    accent: 'indigo';                            // V1 ships single accent; multi-accent deferred
  };
  // NOTE: window-title capture is always-on, gated only by `formatting.enhanceEnabled`
  //       (where app_context tunes the prompt). No separate toggle — "Vibe Coding"
  //       ghost field removed.
};
```

### Secrets
- `groqApiKey` stored via `keyring::Entry::new("com.typr.app", "groq_api_key")`
- Never written to `settings.json`
- Migrated out of legacy `config.json` on first boot post-upgrade

### Storage layer module tree

```
src-tauri/src/storage/
├── mod.rs              -- Db struct, connection pool, run_migrations()
├── migrations.rs       -- include_str! SQL files
├── transcriptions.rs   -- TranscriptionRepo
├── dictionary.rs       -- DictionaryRepo
├── snippets.rs         -- SnippetRepo
├── scratchpad.rs       -- ScratchpadRepo
└── stats.rs            -- StatsRepo
```

Key methods:
- `TranscriptionRepo::insert`, `list_paginated`, `search_fts`, `delete_older_than_words`, `group_by_day`
- `DictionaryRepo::upsert`, `list`, `delete`, `find_matches`
- `SnippetRepo::upsert`, `list`, `find_by_trigger`, `increment_use`
- `ScratchpadRepo::upsert`, `list_ordered`, `delete`
- `StatsRepo::bump_day`, `streak_info`, `totals`

### Word-count cap purge
When `data.purgeOnExceed=true` and total word count exceeds `data.wordCountCap`:
1. Acquire a `Mutex<()>` guard (`AppState.purge_lock`) — only one purge runs at a time
2. `BEGIN IMMEDIATE` transaction (blocks concurrent inserts)
3. Sum `word_count` across all transcriptions
4. Delete oldest rows (`ORDER BY created_at ASC`) until sum ≤ cap
5. `COMMIT` — FTS trigger (`transcriptions_ad`) deletes matching FTS rows automatically
6. Release guard

Purge runs synchronously at the end of `commit_transcription()`, never mid-recording.
Race window with new inserts is bounded by the `IMMEDIATE` lock; reads (history, stats)
use deferred transactions and are never blocked by the purge beyond single-row-delete latency.

---

## Section 3 — UI + UX

### Tailwind v4 theme (`src/styles/globals.css`)

```css
@theme {
  --font-sans: 'Inter Variable', 'Segoe UI Variable', system-ui, sans-serif;
  --font-mono: 'JetBrains Mono Variable', 'Cascadia Code', monospace;

  --color-bg: oklch(99% 0.005 90);
  --color-surface: oklch(98% 0.008 90);
  --color-border: oklch(92% 0.008 90);
  --color-muted: oklch(70% 0.01 90);
  --color-fg: oklch(22% 0.01 90);
  --color-fg-soft: oklch(45% 0.01 90);

  --color-bg-dark: oklch(16% 0.008 270);
  --color-surface-dark: oklch(20% 0.01 270);

  --color-accent: oklch(55% 0.18 265);
  --color-accent-soft: oklch(95% 0.05 265);
  --color-success: oklch(65% 0.15 145);
  --color-danger: oklch(60% 0.22 25);
  --color-focus-ring: oklch(65% 0.20 265);        /* 2px outline, 2px offset */
  --color-focus-ring-on-accent: oklch(98% 0.01 265);

  --radius-sm: 6px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-xl: 16px;

  --shadow-1: 0 1px 2px rgba(0,0,0,0.04);
  --shadow-2: 0 4px 12px rgba(0,0,0,0.06);
  --shadow-3: 0 12px 32px rgba(0,0,0,0.08);
}
```

### Routes (tanstack-router, code-based)

**Primary navigation (sidebar):**
- `/` — Home (stats cards, quick actions, recent 5)
- `/history` — virtualized list + filters (date, engine, language, mode) + FTS search
- `/dictionary` — table CRUD for user terms + abbreviations
- `/snippets` — table CRUD for triggers + expansions
- `/scratchpad` — list of notes, markdown render

**Settings (accessed via sidebar "Settings" button → tab layout, NOT sidebar items):**
- `/settings/general`
- `/settings/transcription`
- `/settings/hotkeys`
- `/settings/overlay`
- `/settings/formatting` — includes auto-add toggle from old `/settings/dictionary`
- `/settings/system`
- `/settings/stats`
- `/settings/data`
- `/settings/about`

Dedup note: dictionary CRUD lives at top-level `/dictionary`; `/settings/dictionary`
was folded into `/settings/formatting` (auto-add is a formatting preference, the
terms themselves live in storage accessed from the primary nav).

**First-run wizard** (`/wizard`, modal route, blocks everything until complete):
1. Welcome — "Typr needs a mic and a hotkey. 60 seconds."
2. Mic picker — enumerate `cpal` devices, live level meter, test record (2s loopback)
3. Engine picker — radio: "Local (private, slower)" / "Groq (fast, cloud)"; if Groq → paste key → `system_cmd::test_groq_key` button → green check or error
4. Hotkey test — default F24 pre-bound; user presses to confirm; if no event within 5s → show "Your keyboard may not have F24. Pick another" + `hotkey-input`
5. Language — checkboxes pt/en, autoDetect toggle
6. Done — "Press F24 anywhere to dictate."

Wizard writes `app_meta.wizard_completed=1`. Skippable via "Configure later" which writes `=0` but routes to `/` anyway. Re-runnable from `/settings/about`.

### Layout
- Sidebar 220px, collapsible to 56px (Ctrl+\)
- Main pane, page header `28px/600` weight
- Command palette Ctrl+K — FTS history search + page jump + quick-add

### Overlay state machine

```
idle
  └─ hotkey pressed → recording
recording
  ├─ hotkey released (ptt) → transcribing
  ├─ hotkey pressed again (toggle) → transcribing
  ├─ Esc pressed → cancelled → idle
  ├─ mic disconnected (cpal stream err) → error("Mic lost") → idle
  ├─ tray "Pause hotkeys" toggled → cancelled → idle
  └─ timeout 120s → transcribing (auto-stop, warn on overlay last 10s)
transcribing
  ├─ hotkey pressed during transcribe → queued ignore + toast "Wait — still transcribing"
  ├─ success → inject → success (flash 400ms, overlay tint=--color-success) → idle
  ├─ zero-speech (no_speech_prob > threshold) → silent fade → idle, no commit
  └─ error → error (flash 1.2s, overlay tint=--color-danger) → idle
```

Tray icon mirrors the overlay state dot: idle = grey outline, recording = red filled,
transcribing = spinner, success = brief green pulse (400ms), error = red `!` for 1.2s
then auto-clears to idle.

### Tray menu
- Status dot + label ("Idle" / "Recording" / "Transcribing")
- "Open Typr"
- "Pause hotkeys" (toggle)
- Submenu: Language (radios from `languages[]`)
- Submenu: Recording mode (Toggle / Push-to-talk)
- Separator
- "Quit"

---

## Section 4 — Transcription pipeline

### Audio capture
- `cpal` stream, device from `settings.microphone`, 16kHz mono f32
- Ring buffer in memory, max 120s hard cap
- VAD (energy threshold + silence window) on push-to-talk release auto-stop
- Toggle mode: user presses hotkey again to stop

### Whisper local (engine=local)
- Sidecar `whisper-cpp` (`whisper-cli.exe` in recent builds) via `tauri-plugin-shell`
- Args: `-m models/<model>.bin -f <wav> -l <lang> --no-speech-thold 0.6 --logprob-thold -1.0 -nt -otxt -of <wav_stem>`
- Dump buffer to WAV temp in `%LOCALAPPDATA%\com.typr.app\tmp\<uuid>.wav`
- Models resolved from `%APPDATA%\com.typr.app\models\`
- `autoDetect=true` → `-l auto`, else first entry in `languages[]`
- `-otxt` writes `<wav_stem>.txt` next to the WAV — parse that file, not stdout (stdout carries progress noise). Fallback if `.txt` missing: re-run without `-otxt` and scrape stdout strip of `[MM:SS.mmm --> MM:SS.mmm]` timestamps
- Delete WAV + sidecar `.txt` after commit (WAV crash sweep: on startup, purge `tmp/*.wav` older than 10min)

**Default model: `large-v3-turbo`** (809M params, ~8× faster than large-v3, PT supported, GGUF available). Options: `turbo`, `large-v3`, `base`. Legacy `tiny`/`small`/`medium` dropped (quality insufficient for PT, source of "Obrigado/Amara.org" hallucinations).

**GPU acceleration.** V1 ships CPU binary only. V1.1 adds Vulkan binary (`whisper-cpp-vulkan.exe`) for NVIDIA+AMD+Intel coverage in one artifact. Runtime probe picks adapter via `settings.transcription.gpuAcceleration`.

### Groq cloud (engine=groq)
- `reqwest` multipart POST `https://api.groq.com/openai/v1/audio/transcriptions`
- Model: `whisper-large-v3`
- Key: `keyring::Entry::new("com.typr.app", "groq_api_key").get_password()`
- Timeout 30s, 1 retry with 2s backoff
- On error → toast "Groq failed, try local" (no auto-switch)

### Rule-based formatting (always runs)
1. Trim whitespace
2. If `removeFillers=true` → regex drop `fillerWords[]` (word boundaries)
3. If `explicitCommands=true` → replace "new line"→`\n`, "new paragraph"→`\n\n`, "period"→`.`, "comma"→`,`, "question mark"→`?`, "bullet"→`• `
4. Dictionary pass — word-boundary regex, case-preserving replace; `is_abbreviation=true` → expand (e.g. "api"→"API"); auto-add if word unseen + freq ≥ N → insert `auto_added=1`
5. Snippets pass — trigger exact match at phrase start → expansion

### Groq Enhance (optional, `enhanceEnabled=true`)
- Model: `llama-3.1-8b-instant`
- Prompt: system "You are a dictation formatter. Clean grammar, preserve meaning. Match tone hint: {hint}." user: `raw_text`
- Tone hint derived from `app_context` (window title): Slack→casual, VS Code→code comment, Gmail→email, default→neutral
- Output → `final_text`, `enhanced=1`
- Disabled → `final_text = raw_text` after rules

### Command Mode (Shift+F24)
```
1. save_clipboard = arboard.get()
2. enigo.key(Ctrl+C)
3. poll clipboard 200ms until change (max 500ms)
4. selection = arboard.get()
5. start recording overlay (mode=command)
6. user releases hotkey → stop
7. transcribe → instruction
8. Groq chat completion:
     system "Edit the text per the instruction. Output only edited text."
     user: {selection}\n\nInstruction: {instruction}
9. arboard.set(edited)
10. enigo.key(Ctrl+V)
11. sleep 100ms, arboard.set(save_clipboard)  -- restore
```
Edge: step 3 timeout → abort, toast "Nothing selected".

**Clipboard fragility.** Step 3 polling is unreliable in:
- Electron apps (Slack/Discord/VS Code) where Ctrl+C is intercepted by JS handlers that may delay or swallow the copy
- Microsoft Office (Word/Excel) — clipboard writes go through OLE and can lag 300-800ms
- RDP / Citrix sessions — clipboard redirection adds 500ms+ round-trip
- Password managers / secure-input fields — clipboard write silently blocked

Mitigations:
- Extend poll window to 800ms (tunable via `settings.commandMode.clipboardTimeoutMs`, default 500)
- `app_context` denylist (KeePassXC, 1Password, Bitwarden, LastPass) → refuse Command Mode, toast "Not available in password managers"
- On zero-length selection after timeout → toast "Nothing selected" + don't start recording (no wasted audio)

### Injection (dictation)
- `arboard.set(final_text)` → `enigo.key(Ctrl+V)`
- Fallback if enigo fails: copy only, toast "Copied — paste manually"
- No clipboard restore in dictation (user expects new text there)

### Commit
```rust
TranscriptionRepo.insert(Transcription {
  raw_text, final_text, word_count, duration_ms,
  language, engine, model, app_context, mode, enhanced
});
StatsRepo.bump_day(today, word_count, duration_ms);
if settings.data.purge_on_exceed {
  purge_if_over_cap(settings.data.word_count_cap);
}
```

### Error states → overlay
- Mic busy → "Mic in use"
- Whisper sidecar crash → "Engine failed" + log
- Groq 401 → "Invalid API key" + link to settings
- Groq 429 → "Rate limit" + retry after 5s
- Zero speech detected → overlay fade silent, no commit

---

## Section 5 — Input / Hotkey / Overlay

### Global hotkeys
- Primary: `RegisterHotKey` Win32 via `windows` crate (see "F24 hotkey handling" below for rationale and mechanics)
- Fallback: `tauri-plugin-global-shortcut`
- Registered in `setup()`: `dictation` (default F24) + `commandMode` (default Shift+F24)
- Press/release separation via `GetAsyncKeyState` poll thread (see below)
- Dynamic re-registration when user changes in Settings (unregister all → register new)
- Validation: reject reserved combos (Alt+Tab, Win+L, Ctrl+Alt+Del) in UI before Rust call
- Conflict detection: `RegisterHotKey` returns `ERROR_HOTKEY_ALREADY_REGISTERED` → toast "Hotkey in use, pick another"

### Recording mode
- `toggle`: press → start, press again → stop
- `push-to-talk`: press → start, release → stop (`ShortcutState::Released`)
- State held in `Arc<Mutex<RecordingState>>`

### F24 hotkey handling

V0 lost F24 release events in release builds. Root cause never pinned down — likely `tauri-plugin-global-shortcut`'s internal hook thread missing key-up for F24 specifically, possibly interacted with `windows_subsystem = "windows"`. Rather than chase it, go Win32 direct as primary.

**Primary path: `RegisterHotKey` Win32 via `windows` crate.**
- Dedicated message-only window (`CreateWindowExW` with `HWND_MESSAGE` parent) on its own thread
- `RegisterHotKey(hwnd, id, modifiers, vk)` for F24 (`VK_F24 = 0x87`) and Shift+F24 (`MOD_SHIFT | VK_F24`)
- `GetMessageW` loop, filter `WM_HOTKEY` → emit `hotkey:pressed(id)` event into Tauri app handle
- For release detection (push-to-talk): after `WM_HOTKEY`, spawn a `GetAsyncKeyState(VK_F24)` poll at 30Hz until the key reads as up → emit `hotkey:released(id)`. This sidesteps the release-event bug entirely by polling state rather than waiting for an edge.
- Dynamic re-registration on settings change: `UnregisterHotKey` all → `RegisterHotKey` new

**Fallback: `tauri-plugin-global-shortcut`.**
- Only used if `RegisterHotKey` returns error (rare — means another process owns the combo)
- Toast "Hotkey in use by another app. Pick another" + open settings

**Logging.**
- `%LOCALAPPDATA%\com.typr.app\logs\typr.log` via `tracing-appender`
- Log at every stage: register OK/fail (with Win32 error code), `WM_HOTKEY` received, poll thread state transitions
- Reserved combo rejection (Alt+Tab, Win+L, Ctrl+Alt+Del) handled in UI before reaching Rust

### Overlay window
- Tauri child window: `decorations: false`, `always_on_top: true`, `skip_taskbar: true`, `transparent: true`, `focusable: false`
- Size: pill 180x44, bar 320x48, tray (icon anim only, no window)
- Position:
  - `near-cursor`: `GetCursorPos` + offset (24, 24), clamp to monitor bounds
  - `bottom-center`: monitor work area bottom minus 80
  - `custom`: coords in `settings.overlay.customPos`
- Render: React state machine `idle | recording | transcribing | success | error`
- Animations: Framer Motion, entry fade+scale 150ms, waveform CSS anim during recording
- IPC: Rust emits `overlay:state` events → React listener updates

### Tray icon (Tauri 2 built-in `tauri::tray`)
- Icon states: idle (mic outline), recording (mic filled red), transcribing (spinner), error (!)
- Menu items as in Section 3
- Left-click tray → toggle main window

### Clipboard / keyboard injection
- `arboard::Clipboard::new()` singleton in `AppState`
- `enigo::Enigo::new()` singleton
- Ctrl+V synthesis: `Press(Ctrl)`, `Click(v)`, `Release(Ctrl)`
- Delay 30ms between clipboard set and paste (Windows async)

### Window context capture
- `GetForegroundWindow` + `GetWindowTextW` via `windows` crate
- Cached 500ms to avoid syscall hot path
- Written to `transcription.app_context`

### Sounds (optional)
- Assets: `start.wav`, `stop.wav`, `error.wav` in `resources/sounds/`
- `rodio` player, volume 0.4
- Fire-and-forget on thread

### Mute music (optional)
- Windows `IAudioSessionManager2` enumerate sessions, mute all except own
- Restore volumes on stop
- Detect known players (Spotify, chrome.exe with media session) — mute only those

---

## Section 6 — Module breakdown

### Rust (`src-tauri/src/`)

```
src-tauri/src/
├── main.rs                 -- Tauri setup, plugin register, window create
├── lib.rs                  -- re-exports, run()
├── state.rs                -- AppState (Db, Settings, Clipboard, Enigo, RecordingState)
├── settings/
│   ├── mod.rs              -- Settings struct + load/save
│   ├── schema.rs           -- typed shape matching TS
│   ├── migrations.rs       -- v1→v2 JSON migrators
│   └── keyring.rs          -- Groq key get/set via keyring crate
├── storage/                -- see Section 2
├── audio/
│   ├── mod.rs
│   ├── capture.rs          -- cpal stream, ring buffer
│   ├── vad.rs              -- energy-based silence detector
│   └── wav.rs              -- f32 → WAV writer
├── transcribe/
│   ├── mod.rs              -- TranscriptionResult struct + dispatch fn (no Engine trait — only 2 impls, match on settings.engine)
│   ├── local.rs            -- whisper-cpp sidecar
│   └── groq.rs             -- reqwest multipart
├── format/
│   ├── mod.rs              -- pipeline(raw, ctx, settings) → final (orchestrates all passes)
│   ├── passes.rs           -- fillers, commands, dictionary_pass, snippets_pass (one file, each a free fn; small enough to live together)
│   └── enhance.rs          -- Groq llama-3.1-8b (separate — network I/O, different error semantics)
├── input/
│   ├── mod.rs
│   ├── hotkeys.rs
│   ├── clipboard.rs
│   ├── inject.rs
│   └── window_ctx.rs
├── overlay/
│   ├── mod.rs
│   └── events.rs
├── tray/
│   ├── mod.rs
│   └── icons.rs
├── pipeline/
│   ├── mod.rs              -- orchestrator
│   ├── dictation.rs
│   └── command.rs          -- clipboard hijack flow
├── stats/
│   └── mod.rs
├── sounds/
│   └── mod.rs              -- rodio playback
├── audio_control/
│   └── windows.rs          -- IAudioSessionManager2 mute
├── logging.rs              -- tracing + file appender
└── commands/               -- Tauri invoke handlers
    ├── mod.rs
    ├── settings_cmd.rs
    ├── history_cmd.rs
    ├── dictionary_cmd.rs
    ├── snippets_cmd.rs
    ├── scratchpad_cmd.rs
    ├── stats_cmd.rs
    └── system_cmd.rs       -- test_mic, test_groq_key, export_db
```

### React (`src/`)

```
src/
├── main.tsx
├── router.tsx              -- tanstack-router config (code-based)
├── app.tsx                 -- root layout
├── layout/
│   ├── sidebar.tsx
│   ├── topbar.tsx
│   └── command-palette.tsx -- Ctrl+K
├── routes/
│   ├── index.tsx           -- Home
│   ├── history.tsx         -- virtualized list + filters
│   ├── dictionary.tsx
│   ├── snippets.tsx
│   ├── scratchpad.tsx
│   └── settings/
│       ├── layout.tsx
│       ├── general.tsx
│       ├── transcription.tsx
│       ├── hotkeys.tsx
│       ├── overlay.tsx
│       ├── formatting.tsx   -- includes former /settings/dictionary auto-add toggle
│       ├── system.tsx
│       ├── stats.tsx
│       ├── data.tsx
│       └── about.tsx
├── overlay/
│   ├── overlay.tsx         -- separate window entry
│   ├── pill.tsx
│   ├── bar.tsx
│   └── waveform.tsx
├── components/
│   ├── ui/                 -- shadcn (button, dialog, input, ...)
│   ├── stat-card.tsx
│   ├── streak-calendar.tsx
│   ├── transcription-row.tsx
│   ├── hotkey-input.tsx
│   ├── engine-picker.tsx
│   ├── model-picker.tsx
│   └── empty-state.tsx
├── stores/
│   ├── settings-store.ts   -- zustand + Tauri sync
│   ├── session-store.ts
│   └── overlay-store.ts
├── lib/
│   ├── tauri.ts            -- typed invoke wrappers
│   ├── format-date.ts
│   ├── fts-highlight.ts
│   └── hotkey-utils.ts
├── hooks/
│   ├── use-settings.ts
│   ├── use-transcriptions.ts
│   ├── use-live-events.ts
│   └── use-shortcut.ts
├── styles/
│   ├── globals.css         -- @theme tokens
│   └── tailwind.css
└── types/
    ├── settings.ts
    ├── transcription.ts
    └── ipc.ts
```

### Pipeline orchestrator contract

The `pipeline/` module is the single entry point for a dictation or command-mode session. It owns the sequencing, error handling, and event emission that the rest of the stack reacts to. Shape:

```rust
// pipeline/mod.rs
pub struct PipelineDeps<'a> {
    pub db: &'a Db,
    pub settings: &'a Settings,
    pub clipboard: &'a Clipboard,
    pub enigo: &'a mut Enigo,
    pub audio: &'a AudioCapture,
    pub app: &'a AppHandle,  // for emit
}

pub enum PipelineMode { Dictation, Command }

pub async fn run_session(
    deps: PipelineDeps<'_>,
    mode: PipelineMode,
) -> Result<Transcription, PipelineError>;
```

Responsibilities:
1. Emit `session:state` events at each stage boundary (`recording`, `transcribing`, `formatting`, `injecting`, `done`/`error`)
2. Capture window context (`input::window_ctx`) once at session start — cached, passed down, never re-queried
3. Call `transcribe::dispatch(engine, wav_path, lang)` → `TranscriptionResult`
4. Call `format::pipeline(raw, ctx, settings)` → `final_text`
5. For `Command` mode: run `input::clipboard::hijack()` before recording, `input::inject::paste_and_restore()` after
6. For `Dictation`: `input::inject::paste(final_text)` (no restore)
7. Persist via `db::transcriptions::insert` + `db::stats::bump_day` in a single transaction
8. Clean temp WAV + sidecar `.txt`

`PipelineError` carries a stage enum (`Record | Transcribe | Format | Inject | Persist`) so the overlay can render the right error copy.

Why an orchestrator rather than chained free functions: each stage emits events and shares a cancellation token (Esc, mic disconnect, tray pause). Putting that control flow in one function keeps the state machine legible and the event sequence deterministic.

### IPC boundary
All Tauri commands typed in `types/ipc.ts`. Generated via `tauri-specta` if feasible, else hand-maintained.

Events Rust → React:
- `session:state`
- `session:progress`
- `overlay:state`
- `settings:changed`
- `transcription:new`
- `stats:updated`

### New dependencies

**Cargo.toml**
- `rusqlite = { version = "0.31", features = ["bundled", "backup", "fts5"] }` — `fts5` required for FTS5 virtual table support
- `keyring = "3"` — v3 API: `Entry::new(service, user)` returns `Result<Entry, Error>`; `get_password`/`set_password`/`delete_credential` all fallible. Wrap all calls; do NOT `.unwrap()`.
- `tracing`, `tracing-appender`, `tracing-subscriber`
- `reqwest = { features = ["json", "multipart", "rustls-tls"] }` — build clients with `.https_only(true)` to refuse accidental plaintext
- `rodio = "0.19"` (optional feature `sounds`)
- `windows = { features = ["Win32_Media_Audio", "Win32_Media_Audio_Endpoints", "Win32_System_Com", "Win32_UI_WindowsAndMessaging", "Win32_UI_Input_KeyboardAndMouse", "Win32_Foundation"] }` — `Endpoints` + `Com` needed for `IAudioSessionManager2`; `Input_KeyboardAndMouse` for `RegisterHotKey` / `VK_F24`
- `enigo = "0.2"` — v0.2 API is `Enigo::new(&Settings::default())?` returning Result; `.key(Key, Direction)` replaces old `.key_down`/`.key_up`
- `arboard = "3"`
- `cpal = "0.15"`
- (tray uses Tauri 2 built-in `tauri::tray` API, no plugin needed)

**package.json**
- `@tanstack/react-router`, `@tanstack/react-virtual`
- `zustand`
- `tailwindcss@4`, `@tailwindcss/vite`
- `framer-motion`
- `cmdk`
- `date-fns`
- `lucide-react`

---

## Section 7 — Migration plan

### Current Typr state
- `src-tauri/src/settings.rs` monolithic (107 lines, 4 tests)
- `config.json` in `%APPDATA%\com.typr.app\` with flat shape (microphone, engine, whisperModel, groqApiKey, recordingMode, hotkey)
- No SQLite, no persistent history
- Existing frontend discarded (modular rewrite)

### Strategy
Rewrite incrementally in branch `wispr-parity`. Keep `main` functional during dev. Release V1 = merge when Phases 1–5 pass smoke tests.

### Settings migration
- Detect legacy shape: absence of `schemaVersion` → v1
- Migrator v1→v2:
  ```
  old.microphone        → new.microphone
  old.engine            → new.transcription.engine
  old.whisperModel      → remap (see table) → new.transcription.whisperModel
  old.groqApiKey (JSON) → keyring; delete from JSON
  old.recordingMode     → new.hotkeys.recordingMode
  old.hotkey            → new.hotkeys.dictation
  defaults              → remaining fields
  ```

**Model remap table** (V1 drops low-quality models):

| v1 whisperModel | v2 whisperModel | Rationale |
|-----------------|-----------------|-----------|
| `tiny`          | `turbo`         | Quality insufficient for PT; turbo is roughly same speed CPU |
| `base`          | `base`          | Unchanged |
| `small`         | `turbo`         | Turbo beats small on quality at similar cost |
| `medium`        | `turbo`         | Turbo matches medium quality at 8× speed |
| `large-v3`      | `large-v3`      | Unchanged |
| `turbo`         | `turbo`         | Unchanged (future-proof) |
| (unknown)       | `turbo`         | Safe default |

Show toast on remap: "Your Whisper model was upgraded to `turbo` (faster, same quality)."

**Backup lifecycle:**
- Write `config.json.v1.bak` before rewrite
- After migrator returns Ok and settings reload succeeds → delete backup (we keep the DB-stored `app_meta.settings_version=2` as the migrated-marker; the `.bak` is only safety for the rewrite itself)
- On failure: keep backup, surface error toast "Settings migration failed — your old config is preserved at config.json.v1.bak", fall back to defaults
- Bump `app_meta` row `{key: "settings_version", value: "2"}`

### Database migration
- First run post-upgrade: create `typr.db`, run `001_initial.sql`
- No historical data to migrate (V0 had none)
- Future migrations: `002_*.sql`, tracked in `schema_migrations`

### Phases

| Phase | Scope | Exit criteria |
|-------|-------|---------------|
| 0 — Foundation | Scaffolding, deps, tracing, branch `wispr-parity`, storage layer (rusqlite bundled+FTS5), migrations runner, repos (transcriptions/dictionary/snippets/scratchpad/stats/settings), unit tests for repos | `cargo build` + `pnpm build` green; FTS query works; repo unit tests pass |
| 1 | Settings rewrite + v1→v2 migrator + keyring v3 wrapper + `.bak` lifecycle | Legacy config.json migrates clean, remap toast fires, backup preserved on failure |
| 2 | Pipeline core (audio + whisper turbo local + Groq + format rules + enigo inject) | F24 dictation matches V0 quality, end-to-end traced |
| 3 | UI rewrite (React 19 + Vite 6 + Tailwind v4 `@theme` + shadcn, tanstack-router code routes, overlay, tray) | Every route navigable, settings sync, dark/light parity |
| 4 | Advanced features (command mode with clipboard denylist + 800ms tunable, Groq enhance, window ctx denylist, sounds, mute music, stats, cap purge) | Manual matrix Phase 4 passes |
| 5 | Polish (first-run wizard, empty states, dark mode QA, F24 Win32-direct primary path, Vulkan build optional) | Clean install → zero wasted clicks |
| 6 | Packaging (MSI + NSIS, sidecars `whisper-cli.exe` + `ggml-large-v3-turbo.bin`, DLLs) | Install MSI on clean PC → dictation works cold |

### User data (on release)
- First launch post-upgrade: toast "Settings migrated to v2"
- If Groq key existed in JSON: prompt "Re-enter Groq key (now stored securely)" — do not silently move from corrupt JSON
- History starts empty (first DB use)

### Rollback plan
- User keeps `config.json.v1.bak`
- DB file separate → deleting restores pre-V1 state
- If phase fails in testing: reset branch, `main` still works

---

## Section 8 — Testing strategy

### Unit tests (Rust, `cargo test`)
- `settings/migrations.rs` — v1 → v2 shape, edge cases (missing fields, corrupt JSON, groqApiKey present/absent)
- `settings/keyring.rs` — mock backend, get/set/delete
- `storage/*` — in-memory SQLite per test
  - transcriptions: insert, paginate, FTS search, purge_over_cap
  - dictionary: upsert idempotent, case-preserving replace, auto-add freq threshold
  - snippets: trigger lookup, increment_use
  - scratchpad: CRUD, pin ordering
  - stats: bump_day cumulative, streak across gaps
- `format/*` — fillers (word-boundary), commands (standalone match), dictionary_pass, snippets_pass (phrase-start)
- `audio/vad.rs` — synthetic buffers (silence, tone, speech)
- `audio/wav.rs` — round-trip f32 → WAV

### Integration tests (`tests/`)
- `pipeline_dictation.rs` — stub audio → stub whisper → format → verify DB row + stats bumped
- `pipeline_command.rs` — mock clipboard + mock enigo → verify restore sequence
- `migration_e2e.rs` — v1 config fixture → full boot → v2 shape + keyring populated + backup file

### Frontend tests (Vitest)
- Stores: `settings-store` load/save/optimistic update
- Hooks: `use-transcriptions` pagination + FTS query building
- Components: `hotkey-input` capture, `transcription-row` variants, `streak-calendar` date math
- `fts-highlight.ts` — query tokenization + match span generation

### E2E smoke (Playwright + Tauri)
Deferred to V1.1. Single-user V1 relies on manual testing.

### Manual test matrix (pre-release)
- First-run wizard: mic pick, engine pick, hotkey test, Groq key paste
- Dictation F24 in: Notepad, VS Code, Chrome address bar, Slack, Discord, Word
- Command mode Shift+F24 in: Chrome selection, VS Code, Word
- Engine switch local↔groq during use
- Language change mid-session
- Overlay positioning: multi-monitor, DPI 125%/150%, portrait monitor
- Tray: pause hotkeys, language switch, recording mode switch
- Settings persist cross-restart
- Word-count cap purge with populated DB
- History FTS search PT + EN
- Dictionary auto-add after N uses
- Snippets trigger in dictation
- Light/dark/system theme
- Launch-at-login on/off
- Close-to-tray vs quit
- Sounds on/off
- Mute music with Spotify playing
- Groq 401/429 handling
- Whisper sidecar crash recovery
- Mic disconnect mid-recording

### Regression guard
- Local pre-commit hook: `cargo test` + `cargo clippy -- -D warnings` + `cargo fmt --check` + `pnpm test` + `pnpm lint` + `pnpm build`
- No GitHub Actions Windows runner V1 (cost)

### Perf sanity
Benchmark script: 10s audio sample, end-to-end hotkey-press → text-injected.
Targets:
- Groq: < 1500ms
- Local turbo Vulkan: < 800ms
- Local turbo CPU: < 3000ms
Recorded in `docs/benchmarks.md`, re-run per release.

### Data integrity
- DB backup before migrations (`typr.db` → `typr.db.v1.bak`)
- FTS consistency: `INSERT INTO transcriptions_fts(transcriptions_fts) VALUES('integrity-check')` after bulk ops
- Corrupt DB: detect via `PRAGMA integrity_check`, offer rebuild from backup

### Logging / observability
- `tracing` in every module, spans per pipeline stage
- Rotating `typr.log` (10MB × 3) in `%LOCALAPPDATA%\com.typr.app\logs\`
- Settings → About → "Open logs folder"
- No telemetry, no cloud crash reporting

---

## Section 9 — Roadmap

Same 7 phases as Section 7 migration table (Phase 0 — Foundation through Phase 6 — Packaging). Each phase is its own implementation plan. Next step: invoke `superpowers:writing-plans` for Phase 0 — Foundation.

---

## Section 10 — Risks + open questions

### Risks
- **F24 release no-op** — V1 ships `RegisterHotKey` Win32 direct as PRIMARY (not fallback) with `GetAsyncKeyState` 30Hz release poll; plugin path kept only as diagnostic comparison
- **Tauri 2 + Tailwind v4 compat** — v4 new, shadcn components may need patches
- **Whisper Turbo GGUF availability** — verify `ggml-large-v3-turbo.bin` exists in HuggingFace ggerganov repo (Phase 0 gate)
- **Vulkan whisper.cpp binary size** — ~80MB bundle bloat; V1 ships CPU only, Vulkan in Phase 5 / V1.1 if needed
- **Keyring on Windows** — `keyring` crate v3 uses Credential Manager → DPAPI under the hood; no separate fallback needed. If unlock fails (rare, no user password), prompt user to re-enter Groq key
- **Clipboard race in Command Mode** — 800ms tunable `clipboardTimeoutMs`, app denylist for KeePassXC/1Password/Bitwarden/LastPass; Electron/Office/RDP documented as fragile
- **Mute music IAudioSessionManager2** — some apps ignore session volume (DRM/exclusive); best-effort, not blocker

### Security / privacy
- **Log sanitisation (MANDATORY)** — `tracing` spans MUST NOT include:
  - `raw_text` / `final_text` / `command_selection` / `app_context` text (PII + user content)
  - `groqApiKey` or any credential string
  - Full WAV paths with usernames (log relative to tmp dir)
  Instead log: duration_ms, byte_len, char_count, stage name, error kind. Enforce via `#[tracing::instrument(skip(...))]` on every pipeline fn and code review checklist
- **DB encryption decision** — V1 ships **unencrypted SQLite**. Rationale: single-user local, OS account boundary is the trust perimeter, SQLCipher adds build complexity (bundled feature conflict) + perf hit. Groq key stays in Credential Manager (not DB). Revisit if multi-user or mobile ever on roadmap
- **Keyring access** — all `keyring::Entry` calls wrapped, no `.unwrap()`; failures surface as toast "Credential store unavailable, re-enter key" not panic

### Open questions (resolve in Phase 0 before coding)
- GPU detection: `nvidia-smi` via `Command` or WMI query? → pick simplest
- Groq free-tier rate limits — check current quotas and document in About page
- First-run wizard: skip button? → yes, "Configure later"
- Dark mode default: `system` or `light`? → `system`; Notion-like default is light
- Sounds bundling: custom protocol or resources? → resources via `tauri::path::resolve_resource`

### Deferred V1.1+
- macOS/Linux
- Auto-updater
- Cloud sync (explicit non-goal V1)
- Team features
- IDE extensions
- Local LLM for enhance (Ollama)
- Playwright E2E suite
- Crash telemetry (opt-in)
- Vulkan GPU build

---

## Approvals
- Section 1 (Overview) — approved
- Section 2 (Data model) — approved
- Section 3 (UI + UX) — approved
- Section 4 (Transcription pipeline) — approved (with Turbo default + Vulkan deferred)
- Section 5 (Input / Hotkey / Overlay) — approved
- Section 6 (Module breakdown) — approved
- Section 7 (Migration plan) — approved
- Section 8 (Testing strategy) — approved
- Section 9 (Roadmap) — approved
- Section 10 (Risks + open questions) — approved
