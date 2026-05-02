---
type: spec
phase: 2
parent: docs/superpowers/specs/2026-04-23-wispr-flow-parity-design.md
created: 2026-04-26
---

# Phase 2 — Pipeline Core (sub-spec)

> Sub-spec of the Wispr Flow parity master design. Master document:
> `docs/superpowers/specs/2026-04-23-wispr-flow-parity-design.md`. This file
> covers Phase 2 only; sections in the master spec remain authoritative for
> shape decisions not contradicted here.

## 1 — Scope & exit criteria

**In Phase 2:**
- Pipeline orchestrator (`pipeline/mod.rs`) for Dictation mode.
- `audio/vad.rs` energy-based VAD + 120s ring buffer hard cap on `audio/recorder.rs`.
- Format rules v2: fillers (PT+EN) + explicit commands (PT+EN) + dictionary pass + snippets pass. Reads `dictionary` and `snippets` repos from Phase 0.
- DB persistence per session: `TranscriptionRepo.insert` + `StatsRepo.bump_day` in a single transaction; word-count cap purge if `data.purge_on_exceed=true` and rows exceed `data.word_count_cap`.
- Whisper local: `-otxt` parse from sidecar `.txt` file, fallback to stdout scrape with timestamp strip.
- Per-session tmp WAV at `%LOCALAPPDATA%\com.typr.app\tmp\<uuid>.wav` with boot-time crash sweep (purge `*.wav` older than 10min).
- Default model `large-v3-turbo`; runtime gate rejects retired `tiny`/`small`/`medium`.
- Settings v2 → v3 migrator: remap `medium`→`turbo` (and defensive `tiny`/`small`→`turbo`), emit `MigrationEvent::ModelRemapped`, stamp `app_meta.settings_version=3`.
- `recorder.rs` deleted; Tauri commands wire to `pipeline::run_session` directly. Remaining `legacy_v1::Settings` import in pipeline-adjacent code removed.
- `tracing::instrument` spans on every stage (`capture`, `transcribe`, `format`, `inject`, `persist`) with structured fields (`session_id`, `engine`, `model`, `language`, `duration_ms`, `word_count`).

**Out of Phase 2 (later phases):**
- Command Mode `Shift+F24` (Phase 4).
- Compose Mode (newly captured backlog item — Phase 4 sub-feature; see §13).
- Groq Enhance LLM pass (Phase 4).
- Window context capture for tone hint (Phase 4).
- New overlay UI / tray menu (Phase 3 — Phase 2 keeps existing v0 overlay).
- F24 Win32-direct hotkey rewrite (Phase 5).
- Auto-download UX for missing model file (Phase 5 first-run wizard).

**Exit criteria:**
- F24 dictation E2E: capture → VAD auto-stop on push-to-talk OR manual stop on toggle → whisper turbo OR Groq → format rules v2 → enigo paste, no quality regression vs V0 for PT+EN.
- DB row inserted per session, stats bumped, cap purge fires when exceeded.
- All stages traced; `typr.log` shows complete span tree per session with structured fields.
- `cargo test --lib` + integration tests green; new pipeline + format + VAD + tmp tests added.
- `cargo build --release` + `npx tauri build --no-bundle` green.
- Manual smoke (Bruno): `medium` config auto-remaps to `turbo`, error if turbo `.bin` missing, dictation works once `.bin` is in place.

## 2 — Module structure

```
src-tauri/src/
├── pipeline/
│   ├── mod.rs            -- run_session() orchestrator + PipelineDeps + PipelineMode + PipelineError + StageError
│   ├── capture.rs        -- thin wrapper over audio::AudioRecorder; owns tmp WAV path + 120s cap enforcement
│   ├── transcribe.rs     -- dispatch(engine, wav, lang) -> TranscriptionResult; calls into transcribe_local/groq
│   ├── format.rs         -- run_format(raw, settings, db) -> final_text; chains rule passes
│   ├── inject.rs         -- paste(final_text); enigo + arboard; fallback "copied — paste manually" toast
│   ├── commit.rs         -- single-tx insert + bump_day + maybe purge
│   └── tmp.rs            -- session_wav_path() + sweep_stale_wavs(older_than)
├── audio/
│   ├── mod.rs            -- re-exports AudioRecorder
│   ├── recorder.rs       -- ex-audio.rs renamed (cpal, ring buffer, save_wav) + 120s cap + current_duration_ms()
│   └── vad.rs            -- EnergyVad { threshold, silence_window_ms } + tick()
├── format/
│   ├── mod.rs
│   ├── fillers.rs        -- drop_fillers(text, fillers[])
│   ├── commands.rs       -- replace_commands(text); EN + PT alias maps
│   ├── dictionary.rs     -- dictionary_pass(text, repo); auto_add_unseen(raw, repo, threshold)
│   └── snippets.rs       -- expand_snippets(text, repo)
├── transcribe_local.rs   -- internals switch to -otxt parse + stdout fallback; signature returns TranscriptionResult
├── transcribe_groq.rs    -- returns TranscriptionResult
├── settings/             -- v3 migrator added (§3)
├── storage/              -- unchanged (Phase 0)
└── recorder.rs           -- DELETED post-cutover
```

**`pipeline/mod.rs` shape:**
```rust
pub struct PipelineDeps<'a> {
    pub db: &'a Db,
    pub settings: &'a Settings,
    pub audio: &'a Mutex<AudioRecorder>,
    pub app: &'a AppHandle,
    pub app_dir: &'a Path,
}

pub enum PipelineMode { Dictation, Command }

pub enum StageError {
    Capture(String),
    Transcribe(String),
    Format(String),
    Inject(String),
    Persist(String),
}

pub struct PipelineError {
    pub stage: StageError,
}

pub async fn run_session(
    deps: PipelineDeps<'_>,
    mode: PipelineMode,
) -> Result<i64, PipelineError>; // returns inserted transcription row id
```

Phase 2: only `Dictation` arm implemented; `Command` arm returns `Err(StageError::Capture("command mode is Phase 4".into()))`.

**Why split `pipeline/format.rs` from `format/`:** rule modules under `format/` are pure data transforms tested in isolation (string-in, string-out, no IO except where DB access is required for dict/snippets). `pipeline/format.rs` orchestrates them with deps and emits stage events. Clean seam between pure transforms and orchestration.

## 3 — Settings v2 → v3 migrator

**Trigger:** boot reads `config.json`; `detect_version` returns 2; `whisperModel` ∈ `{tiny, small, medium}`.

**Action:**
1. Mutate in-memory `Settings`: set `transcription.whisper_model = "turbo"`.
2. Persist via `settings::save`.
3. Stamp `app_meta.settings_version=3`.
4. Emit `MigrationEvent::ModelRemapped { from: <old>, to: "turbo" }`.

**Mechanics:** add `migrate_v2_to_v3` in `settings/migrations.rs`. Hooked into `load()` after the existing v1→v2 path. No `.bak` (v2 is not being structurally rewritten — single field bumped). Idempotent: re-running on already-v3 file is a no-op.

**Schema bump:** `Settings::default().schema_version = 3` (was 2). All v2 JSON parses fine into v3 struct (no field added or removed) — `schemaVersion` is the only bump.

**Rationale for version bump on a single field:** future-proofing. Once we say "tiny/small/medium retired", we want a hard sentinel that says "this config was sanitized post-Phase-2." Otherwise a downgrade or external edit could re-introduce the dead model name and re-trigger the remap event noise on every boot.

**Rejected alternative — runtime guard only without schema bump.** Rejected because it leaves the JSON file inconsistent with what runtime accepts; every boot would re-trigger the remap event.

**Tests** (`settings/migrations.rs`):
- `migrate_v2_to_v3_remaps_medium_to_turbo`
- `migrate_v2_to_v3_remaps_small_to_turbo` (defensive — covers users who skipped v1→v2 but still hold `small` somehow)
- `migrate_v2_to_v3_remaps_tiny_to_turbo`
- `migrate_v2_to_v3_idempotent_on_turbo` (no remap, no event)
- Integration on tempdir: `load_v2_with_medium_writes_v3_and_emits_event`
- Integration: `load_v3_no_migration` (passes through, zero events)

## 4 — Pipeline orchestrator wiring

**Tauri command surface stays the same shape:**
- `start_recording(state, app)` — unchanged signature.
- `stop_recording(state, app)` — calls `pipeline::run_session(deps, PipelineMode::Dictation)` instead of `recorder.stop_and_transcribe`.

**`AppState` evolves:**
```rust
pub struct AppState {
    pub app_dir: PathBuf,
    pub settings: Mutex<Settings>,        // v2/v3 (Phase 1 + Phase 2 §3)
    pub keyring: Box<dyn KeyringBackend>, // Phase 1
    pub db: Db,                           // Phase 0
    pub audio: Mutex<AudioRecorder>,      // moved out of recorder.rs
    pub recording_state: Mutex<RecordingState>, // moved out of recorder.rs
}
```

**`start_recording` flow:**
1. Lock `recording_state`. If state ≠ `Ready` → return error.
2. Snapshot `settings.microphone` (clone under lock, drop guard).
3. `audio.lock().start(&mic_name)?`
4. Set state to `Recording`, emit `recording-state` event, `update_overlay`.

**`stop_recording` flow:**
1. Lock `recording_state`. If state ≠ `Recording` → return error.
2. Set state to `Transcribing`, emit, `update_overlay`. Drop guard.
3. Snapshot `settings`, `keyring.get()` (Groq key) — both before any `.await`.
4. Build `PipelineDeps` with refs to `db`, snapshot `settings`, `audio` mutex, `app`, `app_dir`.
5. `pipeline::run_session(deps, Dictation).await`.
6. On any error: log, set state to `Ready`, emit `recording-state` ready event + error event, `update_overlay` error/idle, return `Err`.
7. On `Ok(row_id)`: set state to `Ready`, emit `transcription:new { row_id }` and `recording-state` ready, return `Ok`.

**`run_session` internal flow** (Dictation):
```
[span session_id=<uuid>]
├─ stage=capture
│   ├─ audio.lock().stop_and_save(&tmp_wav)
│   └─ if file size < N bytes (no audio captured) → Err(Capture("zero speech"))
├─ stage=transcribe
│   ├─ engine = settings.transcription.engine
│   ├─ runtime model gate: if engine == "local" && model ∉ {turbo, large-v3, base}
│   │    → Err(Transcribe("model retired: <name>"))
│   ├─ if engine == "local" && model file missing at <path>
│   │    → Err(Transcribe("model file not found at <path>"))
│   ├─ dispatch → TranscriptionResult { text, language, duration_ms }
├─ stage=format
│   ├─ ctx = WindowCtx { app_context: "" }   // Phase 4 fills it
│   ├─ run_format(raw, &settings, &db) → final_text
├─ stage=inject
│   ├─ if final_text.is_empty() → skip (no commit either)
│   ├─ arboard.set(final_text); sleep 30ms; enigo Ctrl+V
│   └─ on enigo failure → toast "copied — paste manually"; commit still proceeds
├─ stage=persist
│   ├─ tx: TranscriptionRepo.insert(row); StatsRepo.bump_day(today, words, dur_ms)
│   ├─ if settings.data.purge_on_exceed → purge_over_cap(cap)
└─ cleanup: remove tmp_wav + sidecar .txt (best-effort; failures logged, not surfaced)
```

**Cancellation:** Phase 2 ships without a cancellation token. Esc / tray-pause is Phase 3 territory (overlay UI). If a session is in flight, blocks until done. Mic disconnect mid-recording surfaces as `Capture` error and unblocks the state machine.

**Why snapshot settings before await:** Phase 1 lock-discipline lesson — never hold `Mutex<Settings>` across `.await`. Snapshot is a clone, cheap.

## 5 — Format rules v2

**Pipeline order** (settings-gated each step):
1. Trim whitespace (always).
2. Filler drop — if `formatting.removeFillers=true`.
3. Explicit commands — if `formatting.explicitCommands=true`.
4. Dictionary pass — always (no toggle; if dict empty, no-op).
5. Snippets pass — always (if no triggers match, no-op).
6. Auto-add against **raw** input (not the formatted output) if `dictionary.autoAdd=true` and word seen-count crosses threshold.

**`format/fillers.rs`:**
```rust
pub fn drop_fillers(text: &str, fillers: &[String]) -> String;
```
- Word-boundary regex per filler: `\b<filler>\b` case-insensitive.
- Compile per call (Phase 2). `Lazy<HashMap<settings_hash, RegexSet>>` cache deferred to Phase 5 polish.
- Collapses double spaces left after drops.
- Tests: `"um eh I mean hello"` + `["um","eh","i mean"]` → `"hello"`; preserves `"umbrella"` (no false match); preserves casing of non-filler tokens.

**`format/commands.rs`:**
```rust
pub fn replace_commands(text: &str) -> String;
```
- Static map, EN: `"new line"→"\n"`, `"new paragraph"→"\n\n"`, `"period"→"."`, `"comma"→","`, `"question mark"→"?"`, `"exclamation mark"→"!"`, `"bullet"→"• "`.
- PT aliases: `"vírgula"→","`, `"ponto"→"."`, `"ponto final"→"."`, `"interrogação"→"?"`, `"exclamação"→"!"`, `"nova linha"→"\n"`, `"novo parágrafo"→"\n\n"`.
- Word-boundary, case-insensitive.
- Punctuation tokens attach to previous word: `"hello comma world"` → `"hello, world"` (trim space before punctuation tokens).
- Tests cover both languages, attach/detach spacing, case preservation around tokens.

**`format/dictionary.rs`:**
```rust
pub fn dictionary_pass(text: &str, repo: &DictionaryRepo) -> Result<String, DbError>;
pub fn auto_add_unseen(
    raw: &str,
    repo: &DictionaryRepo,
    threshold: u32,
    auto_add_enabled: bool,
) -> Result<(), DbError>;
```
- For each entry: word-boundary regex match → replace with `replacement`, preserving the casing of the matched span (Title→Title, ALL→ALL, lower→lower as configured per entry).
- `is_abbreviation=true` → expand always to canonical form regardless of input case (`api`→`API`).
- Auto-add tracking: not in this pass. `auto_add_unseen` runs after all rules in the orchestrator, against the **raw** text. Threshold const = `DICTIONARY_AUTO_ADD_THRESHOLD: u32 = 5` in `format/dictionary.rs`. Exposing as a settings field is deferred to Phase 5 polish (current `Dictionary` struct only has `auto_add: bool` toggle).
- **Why auto-add against raw not final:** avoids feedback loop (replacing a word, then auto-adding the replacement). Frequency tracked in DB column `dictionary.seen_count`; threshold check is one query per session.

**`format/snippets.rs`:**
```rust
pub fn expand_snippets(text: &str, repo: &SnippetRepo) -> Result<String, DbError>;
```
- Phrase-start match only: trigger fires if it starts at byte index 0 OR is preceded by `\n` (paragraph start).
- Exact match (case-sensitive on trigger as configured per row); replace trigger token with expansion.
- `repo.increment_use(snippet_id)` on every match.
- Multiple non-overlapping triggers: greedy left-to-right.

**`pipeline/format.rs` orchestrator:**
```rust
pub fn run_format(
    raw: &str,
    settings: &Settings,
    db: &Db,
) -> Result<String, FormatError> {
    let mut text = raw.trim().to_string();
    if settings.formatting.remove_fillers {
        text = format::fillers::drop_fillers(&text, &settings.formatting.filler_words);
    }
    if settings.formatting.explicit_commands {
        text = format::commands::replace_commands(&text);
    }
    let dict_repo = DictionaryRepo::new(db);
    text = format::dictionary::dictionary_pass(&text, &dict_repo)?;
    let snip_repo = SnippetRepo::new(db);
    text = format::snippets::expand_snippets(&text, &snip_repo)?;
    format::dictionary::auto_add_unseen(
        raw,
        &dict_repo,
        DICTIONARY_AUTO_ADD_THRESHOLD,
        settings.dictionary.auto_add,
    )?;
    Ok(text)
}
```

**Tests:**
- Each rule module: 3-5 unit tests (happy path, edge case, no-op).
- `pipeline/format.rs`: integration test with in-memory `Db` + seeded dictionary/snippets → full pass.

## 6 — Audio capture + VAD + tmp WAV

**`audio/recorder.rs`** (renamed from `audio.rs`):
- Existing cpal stream + ring buffer kept.
- New: 120s hard cap. Once buffer hits `16000 * 120` samples (16kHz mono f32), oldest f32 chunks are dropped (rolling window). Prevents unbounded growth on stuck-toggle.
- New: `current_duration_ms()` getter for VAD + telemetry.

**`audio/vad.rs`:**
```rust
pub struct EnergyVad {
    threshold: f32,
    silence_window_ms: u32,
    accumulated_silence_ms: u32,
}

pub enum VadDecision { Speech, Silence, AutoStop }

impl EnergyVad {
    pub fn new() -> Self; // const SILENCE_RMS_THRESHOLD = 0.01, SILENCE_WINDOW_MS = 800
    pub fn tick(&mut self, samples: &[f32], sample_rate: u32) -> VadDecision;
    pub fn reset(&mut self);
}
```
- `tick` computes RMS of chunk. RMS < threshold → accumulate elapsed ms. RMS ≥ threshold → reset accumulator → return `Speech`.
- When `accumulated_silence_ms >= silence_window_ms` → return `AutoStop`.
- Used only in **push-to-talk** mode after key release: a poll thread checks recent samples every 50ms; on `AutoStop` it triggers `stop_recording`.
- **Toggle mode ignores VAD entirely** (user owns stop).
- Constants tuned for indoor speech; runtime-tunable via settings is Phase 5.

**`pipeline/tmp.rs`:**
```rust
pub fn session_wav_path(app_dir: &Path) -> PathBuf;
// %LOCALAPPDATA%\com.typr.app\tmp\<uuid>.wav

pub fn sweep_stale_wavs(app_dir: &Path, older_than: Duration);
// scan tmp/, delete *.wav with mtime < now - older_than; log count
```
- Sweep called once at boot (`main.rs::run` after settings load) with `Duration::from_secs(600)` (10 min).
- Cleanup on success: `commit.rs` removes `<uuid>.wav` + `<uuid>.wav.txt` (sidecar). Failure path leaves files; sweep collects them on next boot.
- Path uses `dirs::data_local_dir()` (`%LOCALAPPDATA%`) — different from `dirs::config_dir()` (`%APPDATA%`) used for `config.json`/`typr.db`. Aligns with master spec §4.

**Tests:**
- `vad.rs`: synthetic buffers — pure silence (1s) → `Silence` repeated, after 800ms accumulated → `AutoStop`; tone burst → `Speech`; `reset()` clears accumulator.
- `tmp.rs`: `session_wav_path` returns unique paths per call; `sweep_stale_wavs` deletes only stale files (use `filetime` crate or skip mtime mutation in test and assert the function scans the directory + deletes empty matching files).

## 7 — Whisper -otxt parse

**`transcribe_local.rs` change:**

Current behaviour: invokes `whisper-cli.exe` with `-nt -otxt -of <wav_stem>` already in args, then scrapes stdout for transcription.

Switch primary parser to read `<wav_stem>.txt`:

```rust
async fn transcribe_local(
    app: &AppHandle,
    model_path: &Path,
    wav_path: &Path,
    lang: &str,
) -> Result<TranscriptionResult, String> {
    let stem = wav_path.file_stem().unwrap().to_string_lossy();
    let txt_path = wav_path.with_file_name(format!("{stem}.txt"));
    // run sidecar with -of <stem>...
    if status.success() && txt_path.exists() {
        let raw = std::fs::read_to_string(&txt_path)?;
        let _ = std::fs::remove_file(&txt_path);
        return Ok(TranscriptionResult { text: raw.trim().into(), language: detected_language(&stderr), duration_ms });
    }
    // Fallback: re-run without -otxt, scrape stdout, strip [MM:SS.mmm --> MM:SS.mmm] timestamps
    fallback_stdout_scrape(/* ... */)
}
```

**Sidecar path probe (Phase 2 plan Task 0):** confirm whether `-of <uuid>` writes `<uuid>.txt` or `-of <uuid>.wav` writes `<uuid>.wav.txt`. Plan ships a 30-second probe step against the existing whisper-cli binary before touching production code; pinned result lands in `transcribe_local.rs` as a doc comment.

**Why both paths:** the V0 stdout scrape is bug-prone (progress noise leaks into output). `-otxt` is canonical. Stdout fallback exists for whisper-cli builds that ignore `-otxt` silently.

**`transcribe_groq.rs`:** unchanged signature shape; returns `TranscriptionResult` instead of bare `String`. Small refactor.

**`TranscriptionResult` struct** in a new `pipeline/transcribe.rs`:
```rust
pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>, // local: parsed from stderr "auto-detected language: pt"; groq: from response.language
    pub duration_ms: u64,
}
```

**Tests:**
- `transcribe_local`: hard to unit-test without the actual binary. **Integration test** with real `whisper-cli.exe` + small fixture wav (3-second "hello world") gated `#[ignore]` or named `*_integration` so a dev box can run it and CI can skip.
- Stdout-scrape fallback gets a unit test: input string with `[00:00.000 --> 00:02.500]  hello world\n[Speaker progress]\n` → `"hello world"`.

## 8 — DB persistence + stats

**`pipeline/commit.rs`:**
```rust
pub fn commit_session(
    db: &Db,
    record: TranscriptionRecord,
    settings: &Settings,
) -> Result<i64, DbError>;
```
- Wraps `db.transaction()` (Phase 0 helper).
- Inside tx:
  1. `TranscriptionRepo::new(db).insert(record)` → row id.
  2. `StatsRepo::new(db).bump_day(today, record.word_count as u64, record.duration_ms)`.
  3. If `settings.data.purge_on_exceed` && current word_count > `settings.data.word_count_cap` → `TranscriptionRepo::purge_over_cap(cap)`.
- Returns row id on success.

**`TranscriptionRecord`** mirrors the Phase 0 `transcriptions` table column set: `raw_text`, `final_text`, `word_count`, `duration_ms`, `language`, `engine`, `model`, `app_context`, `mode`, `enhanced`. Phase 2 sets `app_context = ""` and `enhanced = false` (Phase 4 fills them).

**Tests** (`pipeline/commit.rs`):
- `commit_session_inserts_row_and_bumps_stats`
- `commit_session_purges_when_over_cap`
- `commit_session_skips_purge_when_disabled`
- `commit_session_atomic_on_stats_failure` (mock stats repo failure → verify transcription row also rolled back)

## 9 — Tracing instrumentation

**Per-session span tree:**
```
session{session_id=<uuid>, mode="dictation"}
├─ capture{tmp_wav=<path>, duration_ms=…}
├─ transcribe{engine="local"|"groq", model=…, language=…, duration_ms=…}
├─ format{rules_applied=N, dict_replacements=N, snippets_expanded=N}
├─ inject{method="enigo"|"clipboard-fallback"}
└─ persist{row_id=…, words=…, purged=N}
```

- All public stage functions get `#[tracing::instrument(skip(deps))]` with relevant fields.
- `tracing::info!` at stage boundaries; `tracing::warn!` on recoverable degradation (stdout-fallback hit, enigo failure → clipboard-only); `tracing::error!` on stage failure before bubbling.
- `tracing-appender` (already wired Phase 0) writes to `%LOCALAPPDATA%\com.typr.app\logs\typr.log`. JSON layer optional (deferred to Phase 5).

**Tests:** smoke test asserts a session emits expected span names by mounting a `tracing_subscriber::fmt::Layer` capture buffer. Not exhaustive — tracing structure is treated as observability surface, not contract.

## 10 — Testing strategy

**Unit (Rust, `cargo test --lib`):**
- `format/fillers.rs` — happy path, word-boundary safety, casing preservation.
- `format/commands.rs` — EN + PT alias coverage, punctuation attach, multi-replace.
- `format/dictionary.rs` — case-preserving replace, abbreviation expansion, auto-add against raw + threshold gating.
- `format/snippets.rs` — phrase-start gating, paragraph-boundary trigger, increment_use side-effect.
- `audio/vad.rs` — synthetic buffers (silence/tone/speech), accumulator math.
- `pipeline/tmp.rs` — unique path generation, sweep age filter.
- `pipeline/commit.rs` — insert + bump + purge atomicity.
- `settings/migrations.rs` — v2→v3 remap + idempotence (§3).

**Integration (`tests/`):**
- `pipeline_dictation.rs` — stub audio fixture (pre-recorded 3s WAV) → mock transcribe (returns canned text) → real format (in-memory DB) → mock inject → real commit (in-memory DB) → assert row inserted, stats bumped, span tree shape.
- `migration_e2e.rs` extension — add v2→v3 case alongside existing v1→v2 cases.

**Manual smoke (Bruno, post-task-checklist):**
- `medium` config in `config.json` → boot → assert `whisperModel: "turbo"` + `schemaVersion: 3` + `app_meta.settings_version=3` + remap event in log.
- F24 push-to-talk: speak "olá mundo, vírgula, isto é um teste" → assert paste shows `"olá mundo, isto é um teste"` (PT command "vírgula" replaced).
- F24 toggle: speak then press again → assert paste, no auto-stop on silence in toggle.
- Engine switch local↔groq → both work end-to-end.
- Pull mic mid-recording → graceful error, state returns to Ready.
- DB inspection: `sqlite3 %APPDATA%\com.typr.app\typr.db "SELECT id, final_text, word_count FROM transcriptions ORDER BY id DESC LIMIT 5;"` shows recent sessions.

## 11 — Cutover gameplan

Ordered to keep `wispr-parity` always-buildable per commit:

1. **Pre-flight:** branch already on `wispr-parity`. Phase 1 closed. Snapshot Bruno's `config.json` to `~/Desktop/typr-config-backup.json` before any settings handling work.

2. **Layer 1 — Settings v3 migrator** (no behaviour change yet): add `migrate_v2_to_v3`, schema bump default, tests. Keep old runtime accepting `tiny/small/medium` for now. Commit. Run + verify Bruno's config remaps to `turbo`.

3. **Layer 2 — Pure modules** (parallel construction, recorder.rs untouched):
   - `audio/recorder.rs` rename (file-level git mv) + 120s cap + `current_duration_ms`. Commit.
   - `audio/vad.rs` new module + tests. Commit.
   - `format/{fillers,commands,dictionary,snippets}.rs` new modules + tests. Commit.
   - `pipeline/{tmp,commit}.rs` new modules + tests. Commit.

4. **Layer 3 — Pipeline orchestrator** (still parallel):
   - `pipeline/{transcribe,format,inject,capture,mod}.rs` new modules + tests. Recorder.rs still used by Tauri commands. `cargo build` green throughout.

5. **Layer 4 — Tauri command cutover**:
   - Modify `start_recording` / `stop_recording` in `main.rs` to call `pipeline::run_session`.
   - Move `audio` + `recording_state` from `Recorder` struct into `AppState`.
   - Delete `recorder.rs`. Drop `legacy_v1::Settings` import.
   - Add boot-time `tmp::sweep_stale_wavs(app_dir, Duration::from_secs(600))`.
   - Commit.

6. **Layer 5 — Whisper -otxt switch** (gated separate commit so revert is cheap):
   - Probe whisper-cli `-of` extension behaviour (Plan Task 0).
   - Switch `transcribe_local.rs` parser. Keep stdout-scrape fallback path.
   - Commit.

7. **Layer 6 — Runtime model gate**:
   - Add reject of non-{turbo, large-v3, base} models in pipeline/transcribe.rs.
   - Commit.

8. **Layer 7 — Manual smoke + reviewer pass + docs**: run Bruno smoke matrix; dispatch final code reviewer subagent; address findings; update Phase 2 plan completion section; commit; push.

Each layer keeps the binary working. Worst-case revert is `git revert <layer-sha>` without cascading.

## 12 — Risks & mitigations

- **Whisper-cli `-of` quirks across versions** — mitigated by Plan Task 0 probe + stdout-scrape fallback retained indefinitely.
- **VAD constants too aggressive (cuts user mid-sentence)** — Phase 2 ships push-to-talk only with VAD; toggle mode unaffected. Bruno smoke catches it; constants tunable via settings in Phase 5.
- **Auto-add false positives** — gated by `settings.dictionary.auto_add` (default `false`). Bruno enables only after manual seed of dictionary in Phase 3 UI.
- **DB write blocking pipeline (rusqlite is sync)** — `commit.rs` runs in `tokio::task::spawn_blocking` to avoid stalling other awaits. Word-count cap purge inside same blocking task.
- **`%LOCALAPPDATA%` differs from `%APPDATA%` — confusion** — single helper `tmp::session_wav_path(app_dir)` takes the **app data dir** but resolves the local-data dir internally via `dirs::data_local_dir()`. Test asserts the produced path lives under `data_local_dir()`.
- **Recorder.rs deletion mid-cutover breaks bisect** — Layer 4 is one commit (delete + rewire) so any bisect lands either fully in old world or fully in new.

## 13 — Backlog captured this session

**Compose Mode (Phase 4 sub-feature, beyond master spec §4):**

User describes a third dictation flow distinct from current Dictation and planned Command Mode:

- "Write this in English: <speech>" — translation/transformation from instruction + verbal payload, no prior text selection.
- "Reply to this email saying <speech>" — assumes window context (Gmail/Outlook foreground), LLM composes from the verbal brief at the cursor location, formatting tone per app context.

This is "Compose" or "Auto" mode in Whispr Flow terminology. Master spec currently covers only "Edit" (Command Mode requires a selection).

**Action:** add §4.X to master spec under Phase 4 covering:
- Third hotkey (proposed: `Ctrl+Shift+F24` or tray "Compose").
- Pipeline path: capture → transcribe → LLM compose call (Groq `llama-3.1-8b-instant` or larger) with system prompt `"You are a {app_context}-aware composer. Write the requested content. Output only the body."` + user payload `<verbal brief>`.
- Inject at cursor (no clipboard hijack — there's nothing to replace).
- Window-context-driven tone hint reused from Groq Enhance.

Captured here so it does not vanish into chat history. Promoted to master spec on Phase 2 close.

---

## Self-review

Spec coverage vs Phase 2 master-spec exit criterion ("F24 dictation matches V0 quality, end-to-end traced"):
- Pipeline orchestrator → §2, §4.
- Audio capture + VAD + 120s cap → §2, §6.
- Whisper turbo + Groq + format rules + enigo → §2, §5, §7, §4.
- Tracing across stages → §9.
- Migration of Bruno's `medium` → `turbo` → §3.
- Tmp WAV lifecycle → §6.
- DB persistence + stats + cap purge → §8.

Placeholder scan: no TBDs. Ambiguity around the whisper-cli `.txt` filename is explicitly resolved by Plan Task 0 (probe step), not left to the implementer.

Scope check: single-phase deliverable, well-bounded. Compose Mode wishlist captured but explicitly deferred (§13).

Internal consistency: settings field names (`removeFillers`, `explicitCommands`, `autoAdd`, `wordCountCap`, `purgeOnExceed`, `vadEnabled`, `noSpeechThreshold`) confirmed against `settings/schema.rs` Phase 1. `DICTIONARY_AUTO_ADD_THRESHOLD` flagged as a code constant (not a settings field) because the existing `Dictionary` struct only carries `auto_add: bool`; exposing as a setting deferred to Phase 5.

`AppState` shape change (§4) is flagged in the cutover (§11 Layer 4) as a single commit so bisect remains clean.

The `legacy_v1::Settings` import in `recorder.rs` (Phase 1 deferral) is removed in §11 Layer 4 alongside the `recorder.rs` deletion — closing that backlog item.
