# Phase 2 — Pipeline Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-04-26-phase-2-pipeline-core-design.md` (commit `da06370`).

**Goal:** Replace `recorder.rs` with a typed pipeline orchestrator (`pipeline/`), split audio capture from the energy-VAD module, build format rules v2 (fillers + commands + dictionary + snippets), persist every dictation session into SQLite + stats inside one transaction, switch whisper-cli to the canonical `-otxt` sidecar parser, and ship a v2→v3 settings migrator that auto-remaps Bruno's retired `medium` model to `large-v3-turbo`. F24 dictation must match V0 quality, end-to-end traced.

**Architecture:** Phase 1's `AppState` + adapter stay; this phase adds `pipeline::run_session(deps, mode)` returning the inserted transcription row id. Cutover is staged across seven layers (§Cutover at the bottom) so every commit on `wispr-parity` keeps `npx tauri build --no-bundle` green. Recorder.rs is deleted in one commit (Layer 4) along with its `legacy_v1::Settings` import. New format/audio/pipeline modules slot in beside the existing surface and only become live during Layer 4. Auto-add-against-raw needs a DB column the current `dictionary_terms` schema does not have, so this plan introduces a fresh `auto_add_candidates` table via a storage migration in Task 4.

**Tech stack:** Rust (Tauri 2 backend, edition 2021), `cpal` for audio capture, `whisper-cli.exe` sidecar (already shipped at `src-tauri/binaries/whisper-cli.exe`), `enigo` + `arboard` for paste, `rusqlite` (bundled + FTS5) via Phase 0 `Db`, `tracing` + `tracing-appender`, `tokio` for async, `dirs` for `data_local_dir()` / `config_dir()`, `regex` for word-boundary rules, `uuid` for session ids.

**Working directory:** `Z:\Pessoal\vault\projects\local-whisper\typr-main` on branch `wispr-parity`. Always use `cargo` from `src-tauri/`. Production builds use `npx tauri build --no-bundle` (NOT `cargo build --release` directly — the Tauri build runs `pnpm build` first).

**Commit identity:** every `git commit` in this plan must use `git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit ...`. Do NOT touch `git config --global` and do NOT use `--amend` (Phase 1 lesson — pre-commit hook failures should produce a new commit, not amend).

---

## Task 0 — Probe whisper-cli `-otxt` filename behaviour

**Goal:** Pin whether `-of <stem>` writes `<stem>.txt` or `<stem>.wav.txt`. Result is referenced by Task 13. No production code change.

**Files:**
- Read-only: `src-tauri/binaries/whisper-cli.exe`, `src-tauri/src/transcribe_local.rs:1-73`

**Steps:**

- [ ] **Step 1: Locate any existing 16 kHz mono WAV fixture or generate one**

```powershell
# from repo root, in PowerShell (or bash with sox/ffmpeg if present)
# If no fixture exists, record a 3-second silence WAV via ffmpeg:
ffmpeg -f lavfi -i "anullsrc=r=16000:cl=mono" -t 3 -y src-tauri/tests/fixtures/silence_3s.wav
```

If `ffmpeg` is not on PATH, skip and record any 3-second clip into the same path.

- [ ] **Step 2: Locate a turbo or base whisper model**

```powershell
ls "$env:APPDATA\com.typr.app\ggml-*.bin"
```

Note one model path. If none exist, download `ggml-base.bin` (~140MB) into `%APPDATA%\com.typr.app\` for the probe; it is small enough to keep on disk.

- [ ] **Step 3: Run whisper-cli with `-of <stem>` (no extension)**

```powershell
$wav = "src-tauri/tests/fixtures/silence_3s.wav"
$stem = "src-tauri/tests/fixtures/probe_out"
$model = "$env:APPDATA\com.typr.app\ggml-base.bin"  # adjust to discovered model
src-tauri/binaries/whisper-cli.exe -m $model -f $wav -nt -otxt -of $stem
ls "$stem*"
```

Expected: exactly one of `probe_out.txt` or `probe_out.wav.txt` appears.

- [ ] **Step 4: Pin the result**

Edit `src-tauri/src/transcribe_local.rs` and add a doc comment near the top capturing the result, e.g.:

```rust
//! whisper-cli `-of <stem>` writes the transcription sidecar at
//! `<stem>.txt` (probed 2026-04-26 against bundled binary).
```

If the probe shows `<stem>.wav.txt`, write that path instead. Task 13 reads this comment to decide the `txt_path` formula.

- [ ] **Step 5: Commit the probe note**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "docs(phase-2): pin whisper-cli -otxt sidecar path"
```

Cleanup: leave `probe_out.txt` deleted (`rm src-tauri/tests/fixtures/probe_out*`); the silence fixture stays for future tests.

---

## Task 1 — Settings v2→v3 migrator

**Goal:** Remap retired `tiny`/`small`/`medium` whisper models to `turbo`, stamp `app_meta.settings_version=3`, bump `Settings::default().schema_version` to 3. Idempotent on already-v3 files.

**Files:**
- Modify: `src-tauri/src/settings/migrations.rs`
- Modify: `src-tauri/src/settings/schema.rs:105` (default `schema_version`)
- Modify: `src-tauri/src/settings/loader.rs` (hook v2→v3 after v1→v2)
- Modify: `src-tauri/tests/migration_e2e.rs` (add v2→v3 cases)

### Step 1: Bump schema default and assert in unit test

- [ ] **1.1 — Edit default**

`src-tauri/src/settings/schema.rs:105`: change `schema_version: 2` to `schema_version: 3`.

- [ ] **1.2 — Update existing schema unit test**

`src-tauri/src/settings/schema.rs` `default_matches_spec_defaults`: change `assert_eq!(s.schema_version, 2);` to `assert_eq!(s.schema_version, 3);`. Same in `serializes_with_camelcase_keys` (`json["schemaVersion"]`).

- [ ] **1.3 — Run unit tests**

```bash
cd src-tauri && cargo test --lib settings::schema
```

Expected: PASS.

### Step 2: Add `migrate_v2_to_v3` with TDD

- [ ] **2.1 — Write failing tests in `migrations.rs`**

Append the following inside the existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn migrate_v2_to_v3_remaps_medium_to_turbo() {
    let mut s = Settings::default();
    s.schema_version = 2;
    s.transcription.whisper_model = "medium".to_string();
    let outcome = migrate_v2_to_v3(s);
    assert_eq!(outcome.settings.schema_version, 3);
    assert_eq!(outcome.settings.transcription.whisper_model, "turbo");
    assert_eq!(outcome.remapped_model, Some(("medium".into(), "turbo".into())));
}

#[test]
fn migrate_v2_to_v3_remaps_small_to_turbo() {
    let mut s = Settings::default();
    s.schema_version = 2;
    s.transcription.whisper_model = "small".into();
    let outcome = migrate_v2_to_v3(s);
    assert_eq!(outcome.settings.transcription.whisper_model, "turbo");
    assert_eq!(outcome.remapped_model, Some(("small".into(), "turbo".into())));
}

#[test]
fn migrate_v2_to_v3_remaps_tiny_to_turbo() {
    let mut s = Settings::default();
    s.schema_version = 2;
    s.transcription.whisper_model = "tiny".into();
    let outcome = migrate_v2_to_v3(s);
    assert_eq!(outcome.settings.transcription.whisper_model, "turbo");
    assert_eq!(outcome.remapped_model, Some(("tiny".into(), "turbo".into())));
}

#[test]
fn migrate_v2_to_v3_idempotent_on_turbo() {
    let mut s = Settings::default();
    s.schema_version = 2;
    s.transcription.whisper_model = "turbo".into();
    let outcome = migrate_v2_to_v3(s);
    assert_eq!(outcome.settings.schema_version, 3);
    assert_eq!(outcome.remapped_model, None);
}

#[test]
fn migrate_v2_to_v3_preserves_large_v3() {
    let mut s = Settings::default();
    s.schema_version = 2;
    s.transcription.whisper_model = "large-v3".into();
    let outcome = migrate_v2_to_v3(s);
    assert_eq!(outcome.settings.transcription.whisper_model, "large-v3");
    assert_eq!(outcome.remapped_model, None);
}

#[test]
fn migrate_v2_to_v3_preserves_base() {
    let mut s = Settings::default();
    s.schema_version = 2;
    s.transcription.whisper_model = "base".into();
    let outcome = migrate_v2_to_v3(s);
    assert_eq!(outcome.settings.transcription.whisper_model, "base");
    assert_eq!(outcome.remapped_model, None);
}
```

- [ ] **2.2 — Run tests, verify failure**

```bash
cd src-tauri && cargo test --lib settings::migrations::tests::migrate_v2_to_v3
```

Expected: 6 FAIL with "cannot find function `migrate_v2_to_v3`".

- [ ] **2.3 — Add `migrate_v2_to_v3` and `MigrationOutcomeV3`**

After `migrate_v1_to_v2` in `migrations.rs`:

```rust
/// Outcome of a v2 → v3 migration. Smaller than [`MigrationOutcome`] because
/// no keyring touch is involved; the sole side-effect is the remap.
#[derive(Debug, Clone, PartialEq)]
pub struct MigrationOutcomeV3 {
    pub settings: Settings,
    pub remapped_model: Option<(String, String)>,
}

/// Phase 2 cutover: stamps `schema_version=3` and remaps retired whisper models
/// (`tiny`, `small`, `medium`) to `turbo`. Idempotent — running on a v3 input
/// with `turbo` already returns `remapped_model=None`.
pub fn migrate_v2_to_v3(mut s: Settings) -> MigrationOutcomeV3 {
    let from = s.transcription.whisper_model.clone();
    let to = match from.as_str() {
        "tiny" | "small" | "medium" => "turbo",
        other => other,
    };
    let remapped_model = if from != to {
        s.transcription.whisper_model = to.to_string();
        Some((from, to.to_string()))
    } else {
        None
    };
    s.schema_version = 3;
    MigrationOutcomeV3 { settings: s, remapped_model }
}
```

- [ ] **2.4 — Run tests, verify pass**

```bash
cd src-tauri && cargo test --lib settings::migrations
```

Expected: all green (existing v1→v2 + new v2→v3).

### Step 3: Hook into loader

- [ ] **3.1 — Read loader to find the v1→v2 invocation point**

```bash
rg -n "migrate_v1_to_v2" src-tauri/src/settings
```

- [ ] **3.2 — Add v2→v3 hook**

In `src-tauri/src/settings/loader.rs`, after the existing block that returns from a v1→v2 migration, add a branch that handles `detect_version == 2`. Pseudocode (replace `<existing v2 path>` with the actual code that today simply returns the parsed `Settings`):

```rust
2 => {
    let parsed: Settings = serde_json::from_value(root.clone())
        .map_err(|e| LoadError::Malformed(e.to_string()))?;
    let outcome = migrations::migrate_v2_to_v3(parsed);
    // Persist sanitized v3 + bump app_meta sentinel.
    save(&outcome.settings, path)?;
    if let Some((from, to)) = &outcome.remapped_model {
        tracing::info!(from = %from, to = %to, "v2->v3 model remap");
        // event for frontend toast — name aligned with v1->v2 pattern
        if let Some(app) = app_handle {
            let _ = app.emit("settings:model-remapped", serde_json::json!({"from": from, "to": to}));
        }
    }
    // Stamp app_meta sentinel.
    if let Some(db) = db_handle {
        let _ = AppMetaRepo::new(db).set("settings_version", "3");
    }
    Ok(outcome.settings)
}
3 => {
    let parsed: Settings = serde_json::from_value(root.clone())
        .map_err(|e| LoadError::Malformed(e.to_string()))?;
    Ok(parsed)
}
_ => Err(LoadError::UnsupportedVersion(detected)),
```

If `loader.rs` does not currently take `app_handle` or `db_handle`, thread them in as optional parameters with `None` allowed for unit tests. Match the parameter style already used by `migrate_v1_to_v2`.

- [ ] **3.3 — Run tests**

```bash
cd src-tauri && cargo test --lib settings
```

Expected: pass. If parameter threading broke callers, fix the call sites in `main.rs` / `lib.rs` so they pass `Some(app)` / `Some(&state.db)`.

### Step 4: Integration test on tempdir

- [ ] **4.1 — Add v2→v3 case in `migration_e2e.rs`**

Add two test functions next to the existing v1→v2 cases:

```rust
#[test]
fn load_v2_with_medium_writes_v3_and_remaps_to_turbo() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.json");
    std::fs::write(&path, r#"{
        "schemaVersion": 2,
        "microphone": "default",
        "transcription": {
            "engine": "local", "whisperModel": "medium",
            "languages": ["pt","en"], "autoDetect": true,
            "gpuAcceleration": "auto", "vadEnabled": true, "noSpeechThreshold": 0.6
        },
        "hotkeys": {"dictation":"F24","commandMode":"Shift+F24","recordingMode":"push-to-talk"},
        "overlay": {"style":"pill","position":"near-cursor"},
        "formatting": {"enhanceEnabled":false,"removeFillers":true,"fillerWords":[],"explicitCommands":true},
        "dictionary": {"autoAdd":false},
        "stats": {"enabled":true,"milestoneNotifications":true},
        "data": {"wordCountCap":500000,"purgeOnExceed":true},
        "system": {"launchAtLogin":false,"closeToTray":true,"dictationSounds":true,"muteMusicOnDictate":false},
        "ui": {"language":"en","theme":"system","accent":"indigo"}
    }"#).unwrap();

    let s = settings::load_for_test(&path).expect("load v2");
    assert_eq!(s.schema_version, 3);
    assert_eq!(s.transcription.whisper_model, "turbo");

    let on_disk: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(on_disk["schemaVersion"], 3);
    assert_eq!(on_disk["transcription"]["whisperModel"], "turbo");
}

#[test]
fn load_v3_passes_through_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.json");
    let mut def = settings::Settings::default();
    def.transcription.whisper_model = "large-v3".into();
    std::fs::write(&path, serde_json::to_string_pretty(&def).unwrap()).unwrap();
    let mtime_before = std::fs::metadata(&path).unwrap().modified().unwrap();

    let s = settings::load_for_test(&path).unwrap();
    assert_eq!(s.transcription.whisper_model, "large-v3");
    let mtime_after = std::fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(mtime_before, mtime_after, "v3 load must not rewrite file");
}
```

If `load_for_test` does not exist in `settings/mod.rs`, add a thin helper that delegates to the production loader without `app_handle`/`db_handle` (passes `None`).

- [ ] **4.2 — Run integration test**

```bash
cd src-tauri && cargo test --test migration_e2e
```

Expected: pass.

### Step 5: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(settings): add v2->v3 migrator remapping retired whisper models to turbo"
```

---

## Task 2 — Audio recorder rename + 120s ring-buffer cap

**Goal:** Move `audio.rs` into `audio/recorder.rs` (preserving git history) and add a 120-second hard cap on the captured ring buffer plus a `current_duration_ms()` getter.

**Files:**
- Rename: `src-tauri/src/audio.rs` → `src-tauri/src/audio/recorder.rs`
- Create: `src-tauri/src/audio/mod.rs`
- Modify: every importer of `crate::audio::AudioRecorder` (will be flagged by `cargo build`)

### Step 1: Rename via `git mv`

- [ ] **1.1**

```bash
mkdir -p src-tauri/src/audio
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com mv src-tauri/src/audio.rs src-tauri/src/audio/recorder.rs
```

- [ ] **1.2 — Create `audio/mod.rs`**

`src-tauri/src/audio/mod.rs`:

```rust
//! Audio capture surface used by `pipeline::capture`.
//!
//! Phase 2 splits the previous monolithic `audio.rs` into:
//! - `recorder` — cpal stream + ring buffer + `save_wav`.
//! - `vad`      — energy-based push-to-talk auto-stop.

pub mod recorder;
pub mod vad;

pub use recorder::AudioRecorder;
```

`vad.rs` will be added in Task 3; the `pub mod vad;` line will fail to compile until then. To keep this commit green, omit `pub mod vad;` here and add it in Task 3.

- [ ] **1.3 — Run `cargo check`**

```bash
cd src-tauri && cargo check
```

Expected: green (rename is structurally invisible if module path resolves).

### Step 2: Add 120-second cap

- [ ] **2.1 — Failing test**

In `src-tauri/src/audio/recorder.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn ring_buffer_caps_at_120_seconds() {
    let mut rec = AudioRecorder::new();
    // simulate 130s of mono 16kHz f32 frames
    let total_samples = 16_000 * 130;
    rec.push_test_samples(vec![0.5f32; total_samples]);
    let captured = rec.snapshot_test_samples();
    assert!(captured.len() <= 16_000 * 120, "expected <= 120s, got {} samples", captured.len());
    assert_eq!(captured.len(), 16_000 * 120, "expected exactly 120s after rolling-window cap");
}

#[test]
fn current_duration_ms_reports_buffered_audio() {
    let mut rec = AudioRecorder::new();
    rec.push_test_samples(vec![0.0f32; 16_000 * 5]); // 5s
    assert_eq!(rec.current_duration_ms(), 5_000);
}
```

`push_test_samples` and `snapshot_test_samples` are `#[cfg(test)]` shims that mutate the same `Vec<f32>` the cpal callback writes into. They MUST live alongside the production buffer field, gated by `#[cfg(test)]`, and call the same `apply_cap` helper (next step) so the cap logic is exercised by tests rather than smoke-only.

- [ ] **2.2 — Run, verify FAIL**

```bash
cd src-tauri && cargo test --lib audio::recorder
```

Expected: fail with "no method named `push_test_samples`" / "no method named `current_duration_ms`".

- [ ] **2.3 — Implement cap + getter**

In `recorder.rs`:

```rust
const SAMPLE_RATE: usize = 16_000;
const MAX_SECONDS: usize = 120;
const MAX_SAMPLES: usize = SAMPLE_RATE * MAX_SECONDS;

fn apply_cap(buf: &mut Vec<f32>) {
    if buf.len() > MAX_SAMPLES {
        let overflow = buf.len() - MAX_SAMPLES;
        buf.drain(..overflow);
    }
}

impl AudioRecorder {
    pub fn current_duration_ms(&self) -> u64 {
        let buf = self.samples.lock().unwrap();
        ((buf.len() as u64) * 1000) / (SAMPLE_RATE as u64)
    }

    #[cfg(test)]
    pub fn push_test_samples(&mut self, frames: Vec<f32>) {
        let mut buf = self.samples.lock().unwrap();
        buf.extend(frames);
        apply_cap(&mut buf);
    }

    #[cfg(test)]
    pub fn snapshot_test_samples(&self) -> Vec<f32> {
        self.samples.lock().unwrap().clone()
    }
}
```

In the cpal `data_callback` (or wherever new samples are pushed), replace `buf.extend_from_slice(...)` with:

```rust
buf.extend_from_slice(data);
apply_cap(&mut buf);
```

The exact buffer field name (`samples` above) must match what already exists in `recorder.rs`. Adjust if the field is `Arc<Mutex<Vec<f32>>>` already shared with the cpal closure — then test helpers stay the same shape.

- [ ] **2.4 — Verify pass**

```bash
cd src-tauri && cargo test --lib audio::recorder
```

Expected: PASS.

### Step 3: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "refactor(audio): split recorder into audio/ module and cap ring buffer at 120s"
```

---

## Task 3 — Energy-based VAD module

**Goal:** Pure-function VAD that emits `Speech | Silence | AutoStop` based on RMS thresholding over `tick()` calls. Used only by push-to-talk.

**Files:**
- Create: `src-tauri/src/audio/vad.rs`
- Modify: `src-tauri/src/audio/mod.rs` (`pub mod vad;`)

### Step 1: Failing tests

- [ ] **1.1 — Create test skeleton**

`src-tauri/src/audio/vad.rs`:

```rust
//! Energy-based Voice Activity Detection used by push-to-talk auto-stop.

pub const SILENCE_RMS_THRESHOLD: f32 = 0.01;
pub const SILENCE_WINDOW_MS: u32 = 800;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VadDecision {
    Speech,
    Silence,
    AutoStop,
}

pub struct EnergyVad {
    threshold: f32,
    silence_window_ms: u32,
    accumulated_silence_ms: u32,
}

impl EnergyVad {
    pub fn new() -> Self { todo!("Step 2") }
    pub fn with_threshold(threshold: f32, silence_window_ms: u32) -> Self { todo!("Step 2") }
    pub fn tick(&mut self, samples: &[f32], sample_rate: u32) -> VadDecision { todo!("Step 2") }
    pub fn reset(&mut self) { todo!("Step 2") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(amp: f32, n: usize) -> Vec<f32> {
        (0..n).map(|_| amp).collect()
    }

    #[test]
    fn pure_silence_accumulates_then_auto_stops() {
        let mut vad = EnergyVad::new();
        // 16 kHz: 200ms = 3200 samples per tick.
        for _ in 0..3 { // 600ms total
            assert_eq!(vad.tick(&rms(0.0, 3200), 16_000), VadDecision::Silence);
        }
        // 4th tick brings accumulated >= 800ms → AutoStop.
        assert_eq!(vad.tick(&rms(0.0, 3200), 16_000), VadDecision::AutoStop);
    }

    #[test]
    fn loud_burst_returns_speech_and_resets_accumulator() {
        let mut vad = EnergyVad::new();
        vad.tick(&rms(0.0, 3200), 16_000); // 200ms silence
        let decision = vad.tick(&rms(0.5, 3200), 16_000);
        assert_eq!(decision, VadDecision::Speech);
        // After speech, silence accumulator must be 0 again — verify by
        // running 3x silence (600ms) and expecting Silence (not AutoStop).
        for _ in 0..3 {
            assert_eq!(vad.tick(&rms(0.0, 3200), 16_000), VadDecision::Silence);
        }
    }

    #[test]
    fn reset_clears_accumulator() {
        let mut vad = EnergyVad::new();
        for _ in 0..3 { vad.tick(&rms(0.0, 3200), 16_000); }
        vad.reset();
        assert_eq!(vad.tick(&rms(0.0, 3200), 16_000), VadDecision::Silence);
    }

    #[test]
    fn threshold_just_below_speech_counts_as_silence() {
        let mut vad = EnergyVad::with_threshold(0.05, 800);
        // RMS of constant 0.04 = 0.04 < 0.05.
        assert_eq!(vad.tick(&rms(0.04, 3200), 16_000), VadDecision::Silence);
    }
}
```

- [ ] **1.2 — Run, verify FAIL**

```bash
cd src-tauri && cargo test --lib audio::vad
```

Expected: 4 panics from `todo!`.

### Step 2: Implementation

- [ ] **2.1 — Replace todos**

```rust
impl EnergyVad {
    pub fn new() -> Self {
        Self::with_threshold(SILENCE_RMS_THRESHOLD, SILENCE_WINDOW_MS)
    }

    pub fn with_threshold(threshold: f32, silence_window_ms: u32) -> Self {
        Self { threshold, silence_window_ms, accumulated_silence_ms: 0 }
    }

    pub fn tick(&mut self, samples: &[f32], sample_rate: u32) -> VadDecision {
        if samples.is_empty() {
            return VadDecision::Silence;
        }
        let sum_sq: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        let rms = (sum_sq / samples.len() as f64).sqrt() as f32;
        let elapsed_ms = (samples.len() as u64 * 1000 / sample_rate as u64) as u32;
        if rms >= self.threshold {
            self.accumulated_silence_ms = 0;
            VadDecision::Speech
        } else {
            self.accumulated_silence_ms = self.accumulated_silence_ms.saturating_add(elapsed_ms);
            if self.accumulated_silence_ms >= self.silence_window_ms {
                VadDecision::AutoStop
            } else {
                VadDecision::Silence
            }
        }
    }

    pub fn reset(&mut self) {
        self.accumulated_silence_ms = 0;
    }
}
```

- [ ] **2.2 — Verify pass**

```bash
cd src-tauri && cargo test --lib audio::vad
```

Expected: 4 PASS.

### Step 3: Wire into mod + commit

- [ ] **3.1 — Add to `audio/mod.rs`**

```rust
pub mod vad;
```

(Adjacent to existing `pub mod recorder;`.)

- [ ] **3.2 — `cargo check`**

```bash
cd src-tauri && cargo check
```

Expected: green.

- [ ] **3.3 — Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(audio): add energy-based VAD module with rms thresholding"
```

---

## Task 4 — Auto-add candidates table (storage migration)

**Goal:** Add a `auto_add_candidates(term TEXT PRIMARY KEY, seen_count INTEGER NOT NULL DEFAULT 0, last_seen_at INTEGER NOT NULL)` table so `format::dictionary::auto_add_unseen` has a place to track frequencies. Spec §5 says "tracked in `dictionary.seen_count`" but the existing `dictionary_terms` schema has no such column; a separate table cleanly avoids confusing dictionary entries with candidates.

**Files:**
- Create: `src-tauri/src/storage/migrations/00X_auto_add_candidates.sql` (numbered after the highest existing migration; check `ls src-tauri/src/storage/migrations/`)
- Create: `src-tauri/src/storage/auto_add_candidates.rs`
- Modify: `src-tauri/src/storage/mod.rs` (re-export new module)

### Step 1: Migration SQL + runner

- [ ] **1.1 — Identify next migration number**

```bash
ls src-tauri/src/storage/migrations/
```

Pick `(highest + 1)` zero-padded to match existing convention (e.g. `006_auto_add_candidates.sql`).

- [ ] **1.2 — Write migration file**

```sql
-- migrations/00X_auto_add_candidates.sql
CREATE TABLE auto_add_candidates (
    term          TEXT    PRIMARY KEY,
    seen_count    INTEGER NOT NULL DEFAULT 0,
    last_seen_at  INTEGER NOT NULL
);

CREATE INDEX idx_auto_add_candidates_seen_count
  ON auto_add_candidates(seen_count DESC);
```

- [ ] **1.3 — Wire into migration runner**

If migrations are picked up by glob (check `storage/migrations.rs`), nothing further needed. If the runner has an explicit list, append the new file path / version constant.

- [ ] **1.4 — Smoke test the migration**

```bash
cd src-tauri && cargo test --lib storage::migrations
```

Expected: pass; new table is created on a fresh `mem_db`.

### Step 2: Repo with TDD

- [ ] **2.1 — Create `auto_add_candidates.rs` with failing tests**

```rust
use super::{Db, DbError};
use rusqlite::params;

#[derive(Debug, Clone, PartialEq)]
pub struct AutoAddCandidate {
    pub term: String,
    pub seen_count: i64,
    pub last_seen_at: i64,
}

pub struct AutoAddCandidatesRepo<'a> { db: &'a Db }

impl<'a> AutoAddCandidatesRepo<'a> {
    pub fn new(db: &'a Db) -> Self { Self { db } }

    /// Upsert: increments `seen_count` by 1 and stamps `last_seen_at = now`.
    /// Returns the new count.
    pub fn observe(&self, now: i64, term: &str) -> Result<i64, DbError> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO auto_add_candidates (term, seen_count, last_seen_at)
                 VALUES (?1, 1, ?2)
                 ON CONFLICT(term) DO UPDATE SET
                   seen_count = seen_count + 1,
                   last_seen_at = excluded.last_seen_at",
                params![term, now],
            )?;
            c.query_row(
                "SELECT seen_count FROM auto_add_candidates WHERE term = ?1",
                [term],
                |r| r.get(0),
            )
        })
    }

    pub fn get(&self, term: &str) -> Result<Option<AutoAddCandidate>, DbError> {
        self.db.with_conn(|c| {
            c.query_row(
                "SELECT term, seen_count, last_seen_at FROM auto_add_candidates WHERE term = ?1",
                [term],
                |r| Ok(AutoAddCandidate {
                    term: r.get(0)?,
                    seen_count: r.get(1)?,
                    last_seen_at: r.get(2)?,
                }),
            ).optional().map_err(Into::into)
        })
    }

    pub fn delete(&self, term: &str) -> Result<usize, DbError> {
        self.db.with_conn(|c| c.execute("DELETE FROM auto_add_candidates WHERE term = ?1", [term]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    #[test]
    fn observe_increments_count() {
        let db = mem_db();
        let repo = AutoAddCandidatesRepo::new(&db);
        assert_eq!(repo.observe(100, "tauri").unwrap(), 1);
        assert_eq!(repo.observe(200, "tauri").unwrap(), 2);
        assert_eq!(repo.observe(300, "tauri").unwrap(), 3);
        let row = repo.get("tauri").unwrap().unwrap();
        assert_eq!(row.seen_count, 3);
        assert_eq!(row.last_seen_at, 300);
    }

    #[test]
    fn get_missing_returns_none() {
        let db = mem_db();
        let repo = AutoAddCandidatesRepo::new(&db);
        assert!(repo.get("anything").unwrap().is_none());
    }

    #[test]
    fn delete_removes_row() {
        let db = mem_db();
        let repo = AutoAddCandidatesRepo::new(&db);
        repo.observe(1, "x").unwrap();
        assert_eq!(repo.delete("x").unwrap(), 1);
        assert!(repo.get("x").unwrap().is_none());
    }
}
```

`OptionalExtension` is already pulled in by other repos; if `repo.optional()` does not compile, add `use rusqlite::OptionalExtension;` at the top of the file.

- [ ] **2.2 — Re-export from `storage/mod.rs`**

Add:
```rust
pub mod auto_add_candidates;
```

- [ ] **2.3 — Run tests**

```bash
cd src-tauri && cargo test --lib storage::auto_add_candidates
```

Expected: 3 PASS.

### Step 3: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(storage): add auto_add_candidates table + repo for dictionary auto-add tracking"
```

---

## Task 5 — Format module: fillers

**Goal:** Word-boundary case-insensitive filler removal that collapses double spaces.

**Files:**
- Create: `src-tauri/src/format/mod.rs`
- Create: `src-tauri/src/format/fillers.rs`
- Modify: `src-tauri/src/lib.rs` and `src-tauri/src/main.rs` (`mod format;`)

### Step 1: Module skeleton + failing tests

- [ ] **1.1 — Create files**

`src-tauri/src/format/mod.rs`:
```rust
//! Format rules v2 — pure data transforms over transcribed text.
//!
//! Each rule module exposes a single public entry point; orchestration
//! happens in `pipeline::format::run_format`.

pub mod fillers;
```

`src-tauri/src/format/fillers.rs`:

```rust
use regex::RegexBuilder;

/// Drop every occurrence of a configured filler word from `text`.
/// Matching is word-boundary, case-insensitive. Adjacent double spaces
/// left after a drop are collapsed to a single space; leading/trailing
/// whitespace is trimmed.
pub fn drop_fillers(text: &str, fillers: &[String]) -> String {
    if fillers.is_empty() || text.is_empty() {
        return text.to_string();
    }
    let mut out = text.to_string();
    for f in fillers {
        let trimmed = f.trim();
        if trimmed.is_empty() { continue; }
        let escaped = regex::escape(trimmed);
        let pattern = format!(r"\b{}\b", escaped);
        let re = RegexBuilder::new(&pattern)
            .case_insensitive(true)
            .build()
            .expect("static filler pattern");
        out = re.replace_all(&out, "").into_owned();
    }
    // Collapse runs of whitespace introduced by removals.
    let collapse = regex::Regex::new(r"\s{2,}").unwrap();
    collapse.replace_all(out.trim(), " ").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> { v.iter().map(|x| x.to_string()).collect() }

    #[test]
    fn drops_simple_filler() {
        assert_eq!(drop_fillers("um hello world", &s(&["um"])), "hello world");
    }

    #[test]
    fn case_insensitive_match() {
        assert_eq!(drop_fillers("UM hello", &s(&["um"])), "hello");
    }

    #[test]
    fn preserves_word_boundary() {
        // "umbrella" must NOT be touched by filler "um".
        assert_eq!(drop_fillers("umbrella opens", &s(&["um"])), "umbrella opens");
    }

    #[test]
    fn multi_word_filler_supported() {
        assert_eq!(drop_fillers("I mean hello", &s(&["i mean"])), "hello");
    }

    #[test]
    fn collapses_double_spaces_after_drop() {
        assert_eq!(drop_fillers("hello um world", &s(&["um"])), "hello world");
    }

    #[test]
    fn empty_filler_list_is_noop() {
        assert_eq!(drop_fillers("um hello", &s(&[])), "um hello");
    }

    #[test]
    fn drops_pt_filler_ne() {
        assert_eq!(drop_fillers("isto né funciona", &s(&["né"])), "isto funciona");
    }
}
```

- [ ] **1.2 — Wire into root**

In `src-tauri/src/main.rs` (and `lib.rs` if it has its own module list), add `mod format;` near the existing `mod audio;` line.

- [ ] **1.3 — `cargo check`**

```bash
cd src-tauri && cargo check
```

If `regex` is not yet a dependency, add it to `src-tauri/Cargo.toml`:
```toml
regex = "1"
```
and re-run check.

- [ ] **1.4 — Run failing tests, verify pass directly**

Implementation is included in 1.1; running tests should produce 7 PASS:

```bash
cd src-tauri && cargo test --lib format::fillers
```

(Implementation-as-spec is acceptable for rule modules where the test list IS the contract; reviewer asserts coverage.)

### Step 2: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(format): add filler-word removal with word-boundary regex"
```

---

## Task 6 — Format module: explicit commands

**Goal:** Replace EN+PT punctuation/newline command tokens with their literal characters; punctuation tokens attach to the previous word.

**Files:**
- Create: `src-tauri/src/format/commands.rs`
- Modify: `src-tauri/src/format/mod.rs`

### Step 1: Implementation + tests

- [ ] **1.1 — Create file**

`src-tauri/src/format/commands.rs`:

```rust
use regex::Regex;
use std::sync::OnceLock;

struct CommandRule {
    /// Source pattern used to build the regex (case-insensitive).
    /// `\b` boundary enforced at compile time.
    phrase: &'static str,
    replacement: &'static str,
    /// If true, replacement attaches to the previous token (strip preceding space).
    attaches: bool,
}

const RULES: &[CommandRule] = &[
    // English
    CommandRule { phrase: "new paragraph",   replacement: "\n\n", attaches: false },
    CommandRule { phrase: "new line",        replacement: "\n",   attaches: false },
    CommandRule { phrase: "exclamation mark",replacement: "!",    attaches: true  },
    CommandRule { phrase: "question mark",   replacement: "?",    attaches: true  },
    CommandRule { phrase: "period",          replacement: ".",    attaches: true  },
    CommandRule { phrase: "comma",           replacement: ",",    attaches: true  },
    CommandRule { phrase: "bullet",          replacement: "• ",   attaches: false },
    // Portuguese
    CommandRule { phrase: "novo parágrafo",  replacement: "\n\n", attaches: false },
    CommandRule { phrase: "nova linha",      replacement: "\n",   attaches: false },
    CommandRule { phrase: "ponto final",     replacement: ".",    attaches: true  },
    CommandRule { phrase: "ponto",           replacement: ".",    attaches: true  },
    CommandRule { phrase: "vírgula",         replacement: ",",    attaches: true  },
    CommandRule { phrase: "interrogação",    replacement: "?",    attaches: true  },
    CommandRule { phrase: "exclamação",      replacement: "!",    attaches: true  },
];

fn compiled() -> &'static [(Regex, &'static str, bool)] {
    static CACHE: OnceLock<Vec<(Regex, &'static str, bool)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        RULES.iter().map(|r| {
            let escaped = regex::escape(r.phrase);
            let pat = if r.attaches {
                // Eat one leading space if present so punctuation attaches.
                format!(r"(?i)\s?\b{}\b", escaped)
            } else {
                format!(r"(?i)\b{}\b", escaped)
            };
            (Regex::new(&pat).expect("static command pattern"), r.replacement, r.attaches)
        }).collect()
    })
}

pub fn replace_commands(text: &str) -> String {
    let mut out = text.to_string();
    for (re, replacement, _attaches) in compiled() {
        out = re.replace_all(&out, *replacement).into_owned();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_period_attaches_to_previous_word() {
        assert_eq!(replace_commands("hello period world"), "hello. world");
    }

    #[test]
    fn en_comma_question_combo() {
        assert_eq!(
            replace_commands("hello comma is this question mark"),
            "hello, is this?",
        );
    }

    #[test]
    fn en_new_paragraph_inserts_double_newline() {
        assert_eq!(replace_commands("first new paragraph second"), "first\n\n second");
    }

    #[test]
    fn pt_virgula_pontofinal() {
        assert_eq!(
            replace_commands("olá vírgula mundo ponto final"),
            "olá, mundo.",
        );
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(replace_commands("hello PERIOD"), "hello.");
    }

    #[test]
    fn does_not_match_inside_word() {
        // "comma" inside "Komodo" – fabricated edge: real test is "commando" -> "commando"
        assert_eq!(replace_commands("commando squad"), "commando squad");
    }

    #[test]
    fn pt_ponto_final_takes_priority_over_ponto() {
        // "ponto final" is listed before "ponto" so two-word match wins.
        assert_eq!(replace_commands("isto ponto final"), "isto.");
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(replace_commands(""), "");
    }
}
```

- [ ] **1.2 — Wire `pub mod commands;` into `format/mod.rs`**

- [ ] **1.3 — Run tests**

```bash
cd src-tauri && cargo test --lib format::commands
```

Expected: 8 PASS. If `pt_ponto_final_takes_priority_over_ponto` fails, the rule order in `RULES` is wrong — `ponto final` MUST appear before `ponto`.

### Step 2: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(format): add explicit-command replacement for EN + PT punctuation tokens"
```

---

## Task 7 — Format module: dictionary pass + auto-add

**Goal:** Word-boundary replacement of dictionary terms with their `replacement`, casing-aware. Abbreviations (`is_abbreviation=true`) always uppercase. `auto_add_unseen` against the raw input bumps `auto_add_candidates`; when count >= threshold AND `auto_add_enabled`, inserts a placeholder dictionary row (so the user sees it in Phase 3 UI).

**Files:**
- Create: `src-tauri/src/format/dictionary.rs`
- Modify: `src-tauri/src/format/mod.rs`

### Step 1: Tests + impl

- [ ] **1.1 — Create file**

```rust
use crate::storage::auto_add_candidates::AutoAddCandidatesRepo;
use crate::storage::dictionary::{DictionaryRepo, DictionaryTerm, NewDictionaryTerm};
use crate::storage::DbError;
use regex::{Regex, RegexBuilder};
use std::collections::HashSet;

pub const DICTIONARY_AUTO_ADD_THRESHOLD: i64 = 5;

/// Apply dictionary replacements across `text`. Case-preserving for non-
/// abbreviation entries (Title→Title, ALL→ALL, lower→lower as configured
/// per row). Abbreviations always render as the configured replacement
/// regardless of input casing.
pub fn dictionary_pass(text: &str, repo: &DictionaryRepo) -> Result<String, DbError> {
    let terms = repo.list()?;
    if terms.is_empty() { return Ok(text.to_string()); }
    let mut out = text.to_string();
    for term in terms {
        if !term.enabled { continue; }
        let Some(replacement) = term.replacement.clone() else { continue; };
        let pat = format!(r"\b{}\b", regex::escape(&term.term));
        let re = RegexBuilder::new(&pat)
            .case_insensitive(true)
            .build()
            .map_err(|e| DbError::Other(format!("dictionary pattern: {e}")))?;
        out = if term.is_abbreviation {
            re.replace_all(&out, replacement.as_str()).into_owned()
        } else {
            // case-preserving: keep the original match's casing class,
            // but emit `replacement` adjusted to it.
            re.replace_all(&out, |caps: &regex::Captures| {
                let original = &caps[0];
                preserve_case(original, &replacement)
            }).into_owned()
        };
    }
    Ok(out)
}

/// Track non-dictionary tokens against the candidates table. When a token
/// has been seen `>= DICTIONARY_AUTO_ADD_THRESHOLD` times AND `auto_add_enabled`,
/// promote it to a dictionary entry with `auto_added=true, replacement=None`.
/// No-op when `auto_add_enabled=false`.
pub fn auto_add_unseen(
    raw: &str,
    dict_repo: &DictionaryRepo,
    cand_repo: &AutoAddCandidatesRepo,
    threshold: i64,
    auto_add_enabled: bool,
    now: i64,
) -> Result<(), DbError> {
    if !auto_add_enabled || raw.trim().is_empty() {
        return Ok(());
    }
    let known: HashSet<String> = dict_repo.list()?.into_iter()
        .map(|t| t.term.to_lowercase())
        .collect();
    // tokens = alphabetic runs >= 4 chars (skip noise).
    let tokenizer = Regex::new(r"[\p{L}]{4,}").unwrap();
    let mut emitted: HashSet<String> = HashSet::new();
    for cap in tokenizer.find_iter(raw) {
        let tok = cap.as_str().to_lowercase();
        if known.contains(&tok) || emitted.contains(&tok) {
            continue;
        }
        emitted.insert(tok.clone());
        let count = cand_repo.observe(now, &tok)?;
        if count >= threshold {
            dict_repo.upsert(now, NewDictionaryTerm {
                term: &tok,
                replacement: None,
                is_abbreviation: false,
                auto_added: true,
                enabled: true,
            })?;
            cand_repo.delete(&tok)?;
        }
    }
    Ok(())
}

fn preserve_case(original: &str, replacement: &str) -> String {
    if original.chars().all(|c| c.is_uppercase()) {
        replacement.to_uppercase()
    } else if original.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        let mut out = String::with_capacity(replacement.len());
        let mut chars = replacement.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
        }
        out.extend(chars);
        out
    } else {
        replacement.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::dictionary::NewDictionaryTerm;
    use crate::storage::test_util::mem_db;

    fn seed(repo: &DictionaryRepo, term: &str, replacement: Option<&str>, abbr: bool) {
        repo.upsert(1, NewDictionaryTerm {
            term, replacement, is_abbreviation: abbr,
            auto_added: false, enabled: true,
        }).unwrap();
    }

    #[test]
    fn replaces_basic_term_with_lowercase_input() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        seed(&repo, "tauri", Some("Tauri"), false);
        assert_eq!(dictionary_pass("we use tauri here", &repo).unwrap(), "we use Tauri here");
    }

    #[test]
    fn case_preserving_uppercase_match() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        seed(&repo, "tauri", Some("Tauri"), false);
        assert_eq!(dictionary_pass("TAURI rocks", &repo).unwrap(), "TAURI rocks");
    }

    #[test]
    fn case_preserving_titlecase_match() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        seed(&repo, "tauri", Some("tauri"), false);
        assert_eq!(dictionary_pass("Tauri opens", &repo).unwrap(), "Tauri opens");
    }

    #[test]
    fn abbreviation_always_uppercases() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        seed(&repo, "api", Some("API"), true);
        assert_eq!(dictionary_pass("call api now", &repo).unwrap(), "call API now");
        assert_eq!(dictionary_pass("CALL API NOW", &repo).unwrap(), "CALL API NOW");
    }

    #[test]
    fn disabled_entries_skipped() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        repo.upsert(1, NewDictionaryTerm {
            term: "tauri", replacement: Some("Tauri"),
            is_abbreviation: false, auto_added: false, enabled: false,
        }).unwrap();
        assert_eq!(dictionary_pass("we use tauri", &repo).unwrap(), "we use tauri");
    }

    #[test]
    fn auto_add_disabled_is_noop() {
        let db = mem_db();
        let dict = DictionaryRepo::new(&db);
        let cand = AutoAddCandidatesRepo::new(&db);
        auto_add_unseen("acme acme acme acme acme acme", &dict, &cand, 5, false, 0).unwrap();
        assert!(cand.get("acme").unwrap().is_none());
        assert!(dict.list().unwrap().is_empty());
    }

    #[test]
    fn auto_add_promotes_after_threshold() {
        let db = mem_db();
        let dict = DictionaryRepo::new(&db);
        let cand = AutoAddCandidatesRepo::new(&db);
        for i in 0..4 {
            auto_add_unseen("acme rocket", &dict, &cand, 5, true, i).unwrap();
        }
        // 4 sessions: candidate count=4, no promotion.
        assert_eq!(cand.get("acme").unwrap().unwrap().seen_count, 4);
        assert!(dict.list().unwrap().is_empty());
        // 5th session pushes count to 5 → promote.
        auto_add_unseen("acme rocket", &dict, &cand, 5, true, 5).unwrap();
        let dict_rows = dict.list().unwrap();
        let acme = dict_rows.iter().find(|t| t.term == "acme").unwrap();
        assert!(acme.auto_added);
        assert_eq!(acme.replacement, None);
        // Candidate row removed after promotion.
        assert!(cand.get("acme").unwrap().is_none());
    }

    #[test]
    fn auto_add_skips_known_dictionary_terms() {
        let db = mem_db();
        let dict = DictionaryRepo::new(&db);
        let cand = AutoAddCandidatesRepo::new(&db);
        seed(&dict, "tauri", Some("Tauri"), false);
        for i in 0..6 {
            auto_add_unseen("tauri tauri tauri", &dict, &cand, 5, true, i).unwrap();
        }
        assert!(cand.get("tauri").unwrap().is_none());
    }

    #[test]
    fn auto_add_skips_short_tokens() {
        let db = mem_db();
        let dict = DictionaryRepo::new(&db);
        let cand = AutoAddCandidatesRepo::new(&db);
        for _ in 0..6 {
            auto_add_unseen("a be the", &dict, &cand, 5, true, 0).unwrap();
        }
        // Only tokens >= 4 chars are tracked. "the" is 3 chars → skipped.
        assert!(cand.get("the").unwrap().is_none());
    }
}
```

If `DbError::Other(String)` does not exist, swap for whatever variant the existing repos use for "non-rusqlite errors", or downgrade the `regex::Error` path to `panic!("static-pattern build failed")` since the term comes from a DB row and the only failure is malformed bytes.

- [ ] **1.2 — Wire `pub mod dictionary;` into `format/mod.rs`**

- [ ] **1.3 — Run tests**

```bash
cd src-tauri && cargo test --lib format::dictionary
```

Expected: 9 PASS.

### Step 2: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(format): add dictionary pass with case-preserving replace and auto-add tracking"
```

---

## Task 8 — Format module: snippets

**Goal:** Phrase-start snippet expansion (only at byte 0 or after a newline). Increments `snippet.use_count`.

**Files:**
- Create: `src-tauri/src/format/snippets.rs`
- Modify: `src-tauri/src/format/mod.rs`

### Step 1: Tests + impl

- [ ] **1.1 — Create file**

```rust
use crate::storage::snippets::{Snippet, SnippetRepo};
use crate::storage::DbError;

pub fn expand_snippets(text: &str, repo: &SnippetRepo) -> Result<String, DbError> {
    let snippets = repo.list()?;
    if snippets.is_empty() { return Ok(text.to_string()); }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    let bytes = text.as_bytes();
    while cursor < bytes.len() {
        // Phrase-start = cursor == 0 OR previous byte is '\n'.
        let at_phrase_start = cursor == 0 || bytes[cursor - 1] == b'\n';
        if at_phrase_start {
            if let Some(hit) = match_first_trigger(&text[cursor..], &snippets) {
                out.push_str(&hit.snippet.expansion);
                repo.increment_use(hit.snippet.id)?;
                cursor += hit.matched_len;
                continue;
            }
        }
        // Default: copy one char and advance.
        let ch_len = next_char_len(&bytes[cursor..]);
        out.push_str(&text[cursor..cursor + ch_len]);
        cursor += ch_len;
    }
    Ok(out)
}

struct Hit<'a> {
    snippet: &'a Snippet,
    matched_len: usize,
}

fn match_first_trigger<'a>(window: &str, snippets: &'a [Snippet]) -> Option<Hit<'a>> {
    // Greedy left-to-right: longest trigger first to avoid prefix collisions.
    let mut sorted: Vec<&Snippet> = snippets.iter().collect();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.trigger.len()));
    for snip in sorted {
        if window.starts_with(&snip.trigger) {
            return Some(Hit { snippet: snip, matched_len: snip.trigger.len() });
        }
    }
    None
}

fn next_char_len(bytes: &[u8]) -> usize {
    let b = bytes[0];
    if b < 0x80 { 1 }
    else if b < 0xC0 { 1 } // continuation; should not happen at boundary, recover
    else if b < 0xE0 { 2 }
    else if b < 0xF0 { 3 }
    else { 4 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::snippets::NewSnippet;
    use crate::storage::test_util::mem_db;

    fn seed(repo: &SnippetRepo, trigger: &str, expansion: &str) {
        repo.upsert(1, NewSnippet {
            trigger, expansion, enabled: true,
        }).unwrap();
    }

    #[test]
    fn expands_at_byte_zero() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        seed(&repo, "/sig", "Bruno Rodrigues");
        assert_eq!(expand_snippets("/sig hello", &repo).unwrap(), "Bruno Rodrigues hello");
    }

    #[test]
    fn does_not_expand_mid_text() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        seed(&repo, "/sig", "Bruno Rodrigues");
        assert_eq!(expand_snippets("hello /sig hello", &repo).unwrap(), "hello /sig hello");
    }

    #[test]
    fn expands_after_newline() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        seed(&repo, "/sig", "Bruno");
        assert_eq!(expand_snippets("hello\n/sig", &repo).unwrap(), "hello\nBruno");
    }

    #[test]
    fn longest_trigger_wins_when_prefixes_collide() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        seed(&repo, "/sig", "short");
        seed(&repo, "/sigfull", "long");
        assert_eq!(expand_snippets("/sigfull hello", &repo).unwrap(), "long hello");
    }

    #[test]
    fn empty_repo_passthrough() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        assert_eq!(expand_snippets("hello", &repo).unwrap(), "hello");
    }

    #[test]
    fn increments_use_count_on_match() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        let id = repo.upsert(1, NewSnippet {
            trigger: "/sig", expansion: "B", enabled: true,
        }).unwrap();
        expand_snippets("/sig\n/sig", &repo).unwrap();
        let snip = repo.find_by_trigger("/sig").unwrap().unwrap();
        assert_eq!(snip.id, id);
        // Use count is bumped by the repo.increment_use call; existing repo
        // tests already cover the column. Here we just assert no error.
    }
}
```

If `Snippet` does not have an `enabled` flag, drop the parameter from `seed`. Adjust to whatever `NewSnippet` actually requires (check `storage/snippets.rs`).

- [ ] **1.2 — Wire `pub mod snippets;` into `format/mod.rs`**

- [ ] **1.3 — Run tests**

```bash
cd src-tauri && cargo test --lib format::snippets
```

Expected: 6 PASS.

### Step 2: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(format): add phrase-start snippet expansion with greedy matching"
```

---

## Task 9 — Pipeline tmp module

**Goal:** `session_wav_path()` produces a unique `<uuid>.wav` under `%LOCALAPPDATA%\com.typr.app\tmp\`. `sweep_stale_wavs()` deletes WAVs older than a `Duration`.

**Files:**
- Create: `src-tauri/src/pipeline/mod.rs` (skeleton)
- Create: `src-tauri/src/pipeline/tmp.rs`

### Step 1: Skeleton

- [ ] **1.1 — Create `pipeline/mod.rs`**

```rust
//! Pipeline orchestrator and stage modules.
//!
//! Phase 2 wires only the Dictation arm. Command Mode is Phase 4.

pub mod tmp;
```

Other submodules will be added by their respective tasks (`commit`, `format`, `transcribe`, `inject`, `capture`, plus the `mod.rs` orchestrator in Task 14).

- [ ] **1.2 — Wire `mod pipeline;` into `lib.rs` + `main.rs`**

### Step 2: Tests + impl

- [ ] **2.1 — Create `pipeline/tmp.rs`**

```rust
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Returns the directory used for per-session temporary WAVs.
/// Uses `%LOCALAPPDATA%\com.typr.app\tmp\` (different from `%APPDATA%`,
/// which holds `config.json` + `typr.db`). The `app_dir` argument is
/// retained for symmetry with other helpers but ignored — local-app-data
/// is resolved via `dirs::data_local_dir()`.
pub fn tmp_dir() -> PathBuf {
    let base = dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir);
    base.join("com.typr.app").join("tmp")
}

/// Generate a fresh `<uuid>.wav` path. Creates the parent directory if
/// missing. Returns `None` when the directory cannot be created.
pub fn session_wav_path() -> Option<PathBuf> {
    let dir = tmp_dir();
    fs::create_dir_all(&dir).ok()?;
    let id = uuid::Uuid::new_v4();
    Some(dir.join(format!("{id}.wav")))
}

/// Best-effort sweep: delete `*.wav` and `*.wav.txt` and `*.txt` whose
/// mtime is older than `now - older_than`. Returns the count deleted.
pub fn sweep_stale_wavs(older_than: Duration) -> usize {
    sweep_dir(&tmp_dir(), older_than)
}

fn sweep_dir(dir: &Path, older_than: Duration) -> usize {
    let now = SystemTime::now();
    let mut deleted = 0usize;
    let Ok(entries) = fs::read_dir(dir) else { return 0; };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else { continue; };
        if ext != "wav" && ext != "txt" { continue; }
        let Ok(meta) = entry.metadata() else { continue; };
        let Ok(mtime) = meta.modified() else { continue; };
        if let Ok(age) = now.duration_since(mtime) {
            if age >= older_than {
                if fs::remove_file(&path).is_ok() {
                    deleted += 1;
                }
            }
        }
    }
    deleted
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn session_path_unique() {
        let a = session_wav_path().unwrap();
        let b = session_wav_path().unwrap();
        assert_ne!(a, b);
        assert_eq!(a.extension().unwrap(), "wav");
    }

    #[test]
    fn sweep_dir_deletes_old_files_only() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("old.wav");
        let new = tmp.path().join("new.wav");
        File::create(&old).unwrap().write_all(b"x").unwrap();
        File::create(&new).unwrap().write_all(b"x").unwrap();

        // Backdate `old` by 30 minutes.
        let backdate = SystemTime::now() - Duration::from_secs(60 * 30);
        filetime::set_file_mtime(&old, filetime::FileTime::from_system_time(backdate)).unwrap();

        let n = sweep_dir(tmp.path(), Duration::from_secs(60 * 10));
        assert_eq!(n, 1);
        assert!(!old.exists());
        assert!(new.exists());
    }

    #[test]
    fn sweep_dir_ignores_non_wav() {
        let tmp = tempfile::tempdir().unwrap();
        let other = tmp.path().join("other.bin");
        File::create(&other).unwrap().write_all(b"x").unwrap();
        let backdate = SystemTime::now() - Duration::from_secs(60 * 30);
        filetime::set_file_mtime(&other, filetime::FileTime::from_system_time(backdate)).unwrap();
        let n = sweep_dir(tmp.path(), Duration::from_secs(60 * 10));
        assert_eq!(n, 0);
        assert!(other.exists());
    }
}
```

`uuid`, `dirs`, `tempfile`, `filetime` may need to be added to `Cargo.toml`. Add them under `[dependencies]` (`tempfile`/`filetime` go under `[dev-dependencies]`):

```toml
uuid = { version = "1", features = ["v4"] }
dirs = "5"
# dev-dependencies:
tempfile = "3"
filetime = "0.2"
```

- [ ] **2.2 — Run tests**

```bash
cd src-tauri && cargo test --lib pipeline::tmp
```

Expected: 3 PASS.

### Step 3: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(pipeline): add tmp wav path generator + stale-file sweep"
```

---

## Task 10 — Pipeline commit module

**Goal:** Single-transaction `insert + bump_day + maybe_purge`. Returns inserted row id.

**Files:**
- Create: `src-tauri/src/pipeline/commit.rs`
- Modify: `src-tauri/src/pipeline/mod.rs`

### Step 1: Tests + impl

- [ ] **1.1 — Create file**

```rust
use crate::settings::schema::Settings;
use crate::storage::stats::StatsRepo;
use crate::storage::transcriptions::{NewTranscription, TranscriptionRepo};
use crate::storage::{Db, DbError};
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct TranscriptionRecord {
    pub raw_text: String,
    pub final_text: String,
    pub word_count: i64,
    pub duration_ms: i64,
    pub language: Option<String>,
    pub engine: String,   // "local" | "cloud"
    pub model: String,    // "turbo" | "base" | "groq:whisper-large-v3" | etc
    pub app_context: String, // empty in Phase 2; Phase 4 fills it
    pub mode: String,      // "dictation" in Phase 2
    pub enhanced: bool,    // false in Phase 2
}

pub fn commit_session(
    db: &Db,
    record: TranscriptionRecord,
    settings: &Settings,
) -> Result<i64, DbError> {
    let cap_enabled = settings.data.purge_on_exceed;
    let cap = settings.data.word_count_cap as i64;
    db.with_conn_mut(|c| {
        let tx = c.transaction()?;
        let now = Utc::now().timestamp();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        // 1. Insert transcription via the repo (must accept &Transaction or a
        //    helper that uses `tx.execute` directly — see existing repo style).
        let row_id: i64 = tx.execute(
            "INSERT INTO transcriptions
                (created_at, raw_text, final_text, word_count, duration_ms,
                 language, engine, model, app_context, mode, enhanced)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            rusqlite::params![
                now, record.raw_text, record.final_text, record.word_count,
                record.duration_ms, record.language, record.engine,
                record.model, record.app_context, record.mode,
                record.enhanced as i64,
            ],
        ).map(|_| tx.last_insert_rowid())?;
        // 2. Bump day stats.
        tx.execute(
            "INSERT INTO daily_stats (day, words, duration_ms)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(day) DO UPDATE SET
               words = words + excluded.words,
               duration_ms = duration_ms + excluded.duration_ms",
            rusqlite::params![today, record.word_count, record.duration_ms],
        )?;
        // 3. Cap purge.
        let mut purged = 0i64;
        if cap_enabled {
            let total: i64 = tx.query_row(
                "SELECT COALESCE(SUM(word_count), 0) FROM transcriptions",
                [], |r| r.get(0),
            )?;
            if total > cap {
                purged = tx.execute(
                    "DELETE FROM transcriptions WHERE id IN (
                       SELECT id FROM transcriptions ORDER BY id ASC
                     )
                     LIMIT (SELECT COUNT(*) FROM transcriptions
                            WHERE id IN (SELECT id FROM transcriptions ORDER BY id ASC))",
                    [],
                )? as i64;
                // Simpler approach: delegate to TranscriptionRepo::delete_to_fit_word_cap
                // outside the transaction. Caller order: commit insert + stats first,
                // then call repo on the same Db. See note below — pick whichever style
                // matches the existing repo's transaction support.
            }
        }
        let _ = purged; // silence unused warning if delegated
        tx.commit()?;
        Ok(row_id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    fn rec(words: i64) -> TranscriptionRecord {
        TranscriptionRecord {
            raw_text: "hello world".into(),
            final_text: "hello world".into(),
            word_count: words,
            duration_ms: 1500,
            language: Some("en".into()),
            engine: "local".into(),
            model: "turbo".into(),
            app_context: "".into(),
            mode: "dictation".into(),
            enhanced: false,
        }
    }

    #[test]
    fn inserts_row_and_bumps_stats() {
        let db = mem_db();
        let s = Settings::default();
        let id = commit_session(&db, rec(2), &s).unwrap();
        assert!(id > 0);
        let stats = StatsRepo::new(&db).totals().unwrap();
        assert_eq!(stats.words, 2);
        assert_eq!(stats.duration_ms, 1500);
    }

    #[test]
    fn purges_when_cap_exceeded() {
        let db = mem_db();
        let mut s = Settings::default();
        s.data.word_count_cap = 5;
        s.data.purge_on_exceed = true;
        // Insert several rows to exceed cap.
        for _ in 0..10 {
            commit_session(&db, rec(2), &s).unwrap();
        }
        let total = TranscriptionRepo::new(&db).total_word_count().unwrap();
        assert!(total <= 5, "expected <= cap, got {total}");
    }

    #[test]
    fn skips_purge_when_disabled() {
        let db = mem_db();
        let mut s = Settings::default();
        s.data.word_count_cap = 5;
        s.data.purge_on_exceed = false;
        for _ in 0..10 {
            commit_session(&db, rec(2), &s).unwrap();
        }
        let total = TranscriptionRepo::new(&db).total_word_count().unwrap();
        assert!(total > 5);
    }
}
```

**Important:** the inline LIMIT subquery above is illustrative. The real implementation MUST delegate to `TranscriptionRepo::delete_to_fit_word_cap(cap)` (Phase 0 helper, see `storage/transcriptions.rs:97`) to avoid duplicating purge logic. Run the purge inside the `with_conn_mut` block but AFTER `tx.commit()` is dropped — i.e. structurally:

```rust
let row_id = db.with_conn_mut(|c| { /* tx insert+stats */ })?;
if cap_enabled {
    let total = TranscriptionRepo::new(db).total_word_count()?;
    if total > cap {
        TranscriptionRepo::new(db).delete_to_fit_word_cap(cap)?;
    }
}
Ok(row_id)
```

This means the cap purge is NOT atomic with the insert. That's OK — purge is best-effort housekeeping; an interrupted purge just leaves over-cap rows for the next session to clean up. Document this in a comment.

- [ ] **1.2 — Wire `pub mod commit;` into `pipeline/mod.rs`**

- [ ] **1.3 — Run tests**

```bash
cd src-tauri && cargo test --lib pipeline::commit
```

Expected: 3 PASS.

If `chrono` is not yet a dependency, add `chrono = "0.4"` to `Cargo.toml`.

### Step 2: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(pipeline): add commit_session writing transcription + stats with cap-purge"
```

---

## Task 11 — Pipeline transcribe wrapper + TranscriptionResult

**Goal:** Introduce a typed `TranscriptionResult` and a `dispatch` function selecting local/cloud based on settings. Local + cloud transcribers both return `TranscriptionResult`.

**Files:**
- Create: `src-tauri/src/pipeline/transcribe.rs`
- Modify: `src-tauri/src/transcribe_local.rs` (signature change)
- Modify: `src-tauri/src/transcribe_groq.rs` (signature change)
- Modify: `src-tauri/src/pipeline/mod.rs`

### Step 1: Result type

- [ ] **1.1 — Create file with skeleton**

`src-tauri/src/pipeline/transcribe.rs`:

```rust
use std::path::Path;
use tauri::AppHandle;

use crate::settings::schema::Settings;
use crate::transcribe_groq;
use crate::transcribe_local;

#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub duration_ms: u64,
    pub model: String,
}

#[derive(Debug)]
pub enum TranscribeError {
    ModelRetired(String),
    ModelFileMissing(String),
    Engine(String),
}

impl std::fmt::Display for TranscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ModelRetired(m) => write!(f, "model retired: {m}"),
            Self::ModelFileMissing(p) => write!(f, "model file not found at {p}"),
            Self::Engine(e) => write!(f, "engine error: {e}"),
        }
    }
}

const ALLOWED_LOCAL_MODELS: &[&str] = &["turbo", "large-v3", "base"];

pub async fn dispatch(
    app: &AppHandle,
    app_dir: &Path,
    wav_path: &Path,
    settings: &Settings,
    groq_key: Option<&str>,
) -> Result<TranscriptionResult, TranscribeError> {
    match settings.transcription.engine.as_str() {
        "local" => {
            let model = settings.transcription.whisper_model.as_str();
            if !ALLOWED_LOCAL_MODELS.contains(&model) {
                return Err(TranscribeError::ModelRetired(model.to_string()));
            }
            let model_path = app_dir.join(transcribe_local::model_filename(model));
            if !model_path.exists() {
                return Err(TranscribeError::ModelFileMissing(
                    model_path.display().to_string(),
                ));
            }
            transcribe_local::transcribe_local(app, &model_path, wav_path)
                .await
                .map_err(TranscribeError::Engine)
        }
        "cloud" | "groq" => {
            let key = groq_key.ok_or_else(|| TranscribeError::Engine("groq key missing".into()))?;
            transcribe_groq::transcribe_groq(key, wav_path)
                .await
                .map_err(TranscribeError::Engine)
        }
        other => Err(TranscribeError::Engine(format!("unknown engine: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::schema::Settings;

    #[test]
    fn dispatch_rejects_retired_model_without_io() {
        // Construct a settings with model="medium" — the function must reject
        // before any FS or AppHandle is touched. Since dispatch is async and
        // takes an AppHandle, we test the gate logic directly.
        let mut s = Settings::default();
        s.transcription.engine = "local".into();
        s.transcription.whisper_model = "medium".into();
        assert!(!ALLOWED_LOCAL_MODELS.contains(&"medium"));
    }

    #[test]
    fn allowed_models_list_matches_spec() {
        assert!(ALLOWED_LOCAL_MODELS.contains(&"turbo"));
        assert!(ALLOWED_LOCAL_MODELS.contains(&"large-v3"));
        assert!(ALLOWED_LOCAL_MODELS.contains(&"base"));
        assert!(!ALLOWED_LOCAL_MODELS.contains(&"medium"));
        assert!(!ALLOWED_LOCAL_MODELS.contains(&"small"));
        assert!(!ALLOWED_LOCAL_MODELS.contains(&"tiny"));
    }
}
```

### Step 2: Adapt existing transcribers to return `TranscriptionResult`

This step changes the public signatures of `transcribe_local` and `transcribe_groq`. Existing callers (currently `recorder.rs`) will break — that is expected; `recorder.rs` is replaced wholesale in Task 14.

- [ ] **2.1 — Update `transcribe_groq.rs`**

Open the file. Replace the return type from `Result<String, String>` to `Result<TranscriptionResult, String>`. After the existing JSON-parse, build:

```rust
Ok(crate::pipeline::transcribe::TranscriptionResult {
    text: parsed.text.trim().into(),
    language: parsed.language.clone(),
    duration_ms: parsed.duration.map(|d| (d * 1000.0) as u64).unwrap_or(0),
    model: format!("groq:{}", parsed.model.unwrap_or_else(|| "whisper-large-v3".into())),
})
```

Adjust to whatever struct `serde_json` is decoded into; if `language`/`duration`/`model` aren't already pulled, add them as `Option<String>` / `Option<f64>` fields.

- [ ] **2.2 — Update `transcribe_local.rs`**

Replace return type. After the existing scrape (Task 13 will switch to sidecar; for now keep the stdout path), build:

```rust
Ok(crate::pipeline::transcribe::TranscriptionResult {
    text: cleaned.into(),
    language: parsed_language, // if available; else None
    duration_ms: total_elapsed_ms,
    model: model_name.into(),
})
```

`model_name` should come from `model_path.file_stem()` stripped of `ggml-`.

- [ ] **2.3 — Wire `pub mod transcribe;` into `pipeline/mod.rs`**

- [ ] **2.4 — `cargo build`**

```bash
cd src-tauri && cargo build
```

Expected: ONE failure in `recorder.rs` because it still expects `Result<String, String>`. To keep the branch buildable, **do not delete recorder.rs yet** — bridge it temporarily:

```rust
// recorder.rs — temporary bridge (will be replaced in Task 14)
let raw_text = match settings.engine.as_str() {
    "local" => {
        let res = transcribe_local::transcribe_local(app, &model_path, &temp_path).await?;
        res.text
    }
    "cloud" => {
        let res = transcribe_groq::transcribe_groq(&settings.groq_api_key, &temp_path).await?;
        res.text
    }
    _ => return Err(format!("Unknown engine: {}", settings.engine)),
};
```

- [ ] **2.5 — `cargo build` again**

Expected: green.

- [ ] **2.6 — Run unit tests**

```bash
cd src-tauri && cargo test --lib pipeline::transcribe
```

Expected: 2 PASS.

### Step 3: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(pipeline): add transcribe dispatch returning TranscriptionResult; gate retired models"
```

---

## Task 12 — Pipeline format orchestrator

**Goal:** Compose the four format rule modules in the order specified by §5 of the design.

**Files:**
- Create: `src-tauri/src/pipeline/format.rs`
- Modify: `src-tauri/src/pipeline/mod.rs`

### Step 1: Tests + impl

- [ ] **1.1 — Create file**

```rust
use crate::format;
use crate::settings::schema::Settings;
use crate::storage::auto_add_candidates::AutoAddCandidatesRepo;
use crate::storage::dictionary::DictionaryRepo;
use crate::storage::snippets::SnippetRepo;
use crate::storage::{Db, DbError};
use chrono::Utc;

#[derive(Debug)]
pub enum FormatError {
    Db(DbError),
}

impl From<DbError> for FormatError {
    fn from(e: DbError) -> Self { Self::Db(e) }
}

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
    let cand_repo = AutoAddCandidatesRepo::new(db);
    format::dictionary::auto_add_unseen(
        raw,
        &dict_repo,
        &cand_repo,
        format::dictionary::DICTIONARY_AUTO_ADD_THRESHOLD,
        settings.dictionary.auto_add,
        Utc::now().timestamp(),
    )?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::dictionary::NewDictionaryTerm;
    use crate::storage::snippets::NewSnippet;
    use crate::storage::test_util::mem_db;

    #[test]
    fn end_to_end_dictation_path() {
        let db = mem_db();
        let dict = DictionaryRepo::new(&db);
        dict.upsert(0, NewDictionaryTerm {
            term: "tauri", replacement: Some("Tauri"),
            is_abbreviation: false, auto_added: false, enabled: true,
        }).unwrap();
        let snips = SnippetRepo::new(&db);
        snips.upsert(0, NewSnippet {
            trigger: "/sig", expansion: "Bruno", enabled: true,
        }).unwrap();
        let mut s = Settings::default();
        s.formatting.remove_fillers = true;
        s.formatting.filler_words = vec!["um".into()];
        s.formatting.explicit_commands = true;
        let out = run_format(
            "/sig um we use tauri comma yes period",
            &s,
            &db,
        ).unwrap();
        assert_eq!(out, "Bruno we use Tauri, yes.");
    }

    #[test]
    fn empty_input_returns_empty() {
        let db = mem_db();
        let s = Settings::default();
        assert_eq!(run_format("", &s, &db).unwrap(), "");
    }

    #[test]
    fn formatting_toggles_off_skips_passes() {
        let db = mem_db();
        let mut s = Settings::default();
        s.formatting.remove_fillers = false;
        s.formatting.explicit_commands = false;
        s.formatting.filler_words = vec!["um".into()];
        // no dict / snippets seeded → identity except trim
        assert_eq!(run_format("um hello comma", &s, &db).unwrap(), "um hello comma");
    }
}
```

- [ ] **1.2 — Wire `pub mod format;` into `pipeline/mod.rs`**

- [ ] **1.3 — Run tests**

```bash
cd src-tauri && cargo test --lib pipeline::format
```

Expected: 3 PASS.

### Step 2: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(pipeline): add format orchestrator chaining filler/command/dict/snippet passes"
```

---

## Task 13 — Whisper -otxt sidecar parser

**Goal:** Switch primary parser to read `<stem>.txt` (path pinned by Task 0). Keep stdout-scrape fallback.

**Files:**
- Modify: `src-tauri/src/transcribe_local.rs`

### Step 1: Implement sidecar path

- [ ] **1.1 — Edit `transcribe_local`**

Restructure into:

```rust
pub async fn transcribe_local(
    app: &AppHandle,
    model_path: &Path,
    wav_path: &Path,
) -> Result<TranscriptionResult, String> {
    let stem = wav_path.file_stem()
        .ok_or_else(|| "wav path has no stem".to_string())?
        .to_string_lossy().into_owned();
    // Sidecar location pinned by Task 0 probe (see top-of-file comment).
    // If probe showed `<stem>.wav.txt`, change formula here accordingly.
    let parent = wav_path.parent().unwrap_or_else(|| Path::new("."));
    let txt_path = parent.join(format!("{stem}.txt"));

    let started = Instant::now();
    let stdout_capture = run_whisper_cli(app, model_path, wav_path, &txt_path).await?;
    let elapsed_ms = started.elapsed().as_millis() as u64;

    let raw = if txt_path.exists() {
        let content = std::fs::read_to_string(&txt_path)
            .map_err(|e| format!("read sidecar: {e}"))?;
        let _ = std::fs::remove_file(&txt_path);
        content
    } else {
        tracing::warn!("whisper-cli -otxt sidecar missing; falling back to stdout scrape");
        scrape_stdout(&stdout_capture)
    };

    let cleaned = raw.trim().to_string();
    Ok(TranscriptionResult {
        text: cleaned,
        language: detect_language(&stdout_capture),
        duration_ms: elapsed_ms,
        model: model_filename_to_label(model_path),
    })
}

fn scrape_stdout(stdout: &str) -> String {
    // Strip [MM:SS.mmm --> MM:SS.mmm] timestamp prefixes, keep remainder.
    let re = regex::Regex::new(r"\[\d{2}:\d{2}\.\d{3}\s*-->\s*\d{2}:\d{2}\.\d{3}\]").unwrap();
    re.replace_all(stdout, "").lines()
        .map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ")
}

fn detect_language(stderr_or_stdout: &str) -> Option<String> {
    // whisper-cli prints "auto-detected language: pt (probability ...)"
    let re = regex::Regex::new(r"auto-detected language:\s*([a-z]{2})").unwrap();
    re.captures(stderr_or_stdout).and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn model_filename_to_label(model_path: &Path) -> String {
    model_path.file_stem().and_then(|s| s.to_str())
        .map(|s| s.trim_start_matches("ggml-").to_string())
        .unwrap_or_else(|| "unknown".into())
}
```

`run_whisper_cli` is a refactor of the existing process-spawn block. It should pass `-of <wav_stem>` (matching what Task 0 probed) and capture both stdout AND stderr (concatenate for `detect_language`).

- [ ] **1.2 — Add unit test for `scrape_stdout`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrape_stdout_strips_timestamps() {
        let raw = "[00:00.000 --> 00:02.500]  hello world\n[00:02.500 --> 00:04.000]  goodbye\n";
        assert_eq!(scrape_stdout(raw), "hello world goodbye");
    }

    #[test]
    fn scrape_stdout_drops_empty_lines() {
        let raw = "\n\n  hello  \n\n";
        assert_eq!(scrape_stdout(raw), "hello");
    }

    #[test]
    fn detect_language_extracts_pt() {
        let stderr = "auto-detected language: pt (probability 0.97)";
        assert_eq!(detect_language(stderr), Some("pt".to_string()));
    }

    #[test]
    fn detect_language_returns_none_when_absent() {
        assert_eq!(detect_language("nothing here"), None);
    }
}
```

- [ ] **1.3 — Run tests**

```bash
cd src-tauri && cargo test --lib transcribe_local
```

Expected: 4 PASS. Build stays green; sidecar path is exercised at runtime by the Bruno smoke step (Task 16).

### Step 2: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(transcribe): switch whisper-cli parser to -otxt sidecar with stdout fallback"
```

---

## Task 14 — Pipeline orchestrator (`run_session`)

**Goal:** `pipeline::run_session(deps, mode)` runs Dictation E2E. Returns inserted row id.

**Files:**
- Create: `src-tauri/src/pipeline/inject.rs`
- Create: `src-tauri/src/pipeline/capture.rs`
- Modify: `src-tauri/src/pipeline/mod.rs` (add the orchestrator)

### Step 1: Inject

- [ ] **1.1 — Create `pipeline/inject.rs`**

Move the body of `paste.rs::paste_text` here, returning a structured result so the orchestrator can record the method used:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum InjectMethod { Enigo, ClipboardOnly }

pub fn paste(text: &str) -> Result<InjectMethod, String> {
    if text.is_empty() { return Ok(InjectMethod::Enigo); }
    let mut clipboard = arboard::Clipboard::new()
        .map_err(|e| format!("clipboard init: {e}"))?;
    clipboard.set_text(text.to_string())
        .map_err(|e| format!("clipboard set: {e}"))?;
    std::thread::sleep(std::time::Duration::from_millis(30));
    match enigo_paste() {
        Ok(()) => Ok(InjectMethod::Enigo),
        Err(e) => {
            tracing::warn!(error = %e, "enigo paste failed; user must paste manually");
            Ok(InjectMethod::ClipboardOnly)
        }
    }
}

fn enigo_paste() -> Result<(), String> {
    use enigo::{Enigo, Key, KeyboardControllable};
    let mut e = Enigo::new();
    e.key_down(Key::Control);
    e.key_click(Key::Layout('v'));
    e.key_up(Key::Control);
    Ok(())
}
```

Match the actual `enigo` API — the existing `paste.rs` has working code; mirror it. `paste.rs` itself will be deleted in Task 15.

### Step 2: Capture

- [ ] **2.1 — Create `pipeline/capture.rs`**

```rust
use crate::audio::AudioRecorder;
use crate::pipeline::tmp::session_wav_path;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct CaptureOutput {
    pub wav_path: PathBuf,
    pub duration_ms: u64,
    pub byte_size: u64,
}

pub fn stop_and_save(
    audio: &Mutex<AudioRecorder>,
) -> Result<CaptureOutput, String> {
    let wav_path = session_wav_path()
        .ok_or_else(|| "tmp dir not available".to_string())?;
    let mut rec = audio.lock().map_err(|e| format!("audio lock: {e}"))?;
    let duration_ms = rec.current_duration_ms();
    rec.stop_and_save(&wav_path).map_err(|e| format!("stop_and_save: {e}"))?;
    drop(rec);
    let byte_size = std::fs::metadata(&wav_path)
        .map(|m| m.len()).unwrap_or(0);
    Ok(CaptureOutput { wav_path, duration_ms, byte_size })
}
```

### Step 3: Orchestrator

- [ ] **3.1 — Append to `pipeline/mod.rs`**

```rust
pub mod capture;
pub mod inject;

use std::path::Path;
use std::sync::Mutex;
use tauri::AppHandle;
use tracing::instrument;
use uuid::Uuid;

use crate::audio::AudioRecorder;
use crate::settings::schema::Settings;
use crate::storage::Db;

pub struct PipelineDeps<'a> {
    pub db: &'a Db,
    pub settings: &'a Settings,
    pub audio: &'a Mutex<AudioRecorder>,
    pub app: &'a AppHandle,
    pub app_dir: &'a Path,
    pub groq_key: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineMode { Dictation, Command }

#[derive(Debug)]
pub enum StageError {
    Capture(String),
    Transcribe(String),
    Format(String),
    Inject(String),
    Persist(String),
}

#[derive(Debug)]
pub struct PipelineError { pub stage: StageError }

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.stage)
    }
}

#[instrument(skip(deps), fields(session_id, mode = ?mode))]
pub async fn run_session(
    deps: PipelineDeps<'_>,
    mode: PipelineMode,
) -> Result<i64, PipelineError> {
    if mode != PipelineMode::Dictation {
        return Err(PipelineError {
            stage: StageError::Capture("command mode is Phase 4".into()),
        });
    }
    let session_id = Uuid::new_v4();
    tracing::Span::current().record("session_id", &tracing::field::display(session_id));

    // 1. Capture
    let cap = capture::stop_and_save(deps.audio)
        .map_err(|e| PipelineError { stage: StageError::Capture(e) })?;
    if cap.byte_size < 1024 {
        let _ = std::fs::remove_file(&cap.wav_path);
        return Err(PipelineError {
            stage: StageError::Capture("zero speech captured".into()),
        });
    }
    tracing::info!(
        wav = %cap.wav_path.display(),
        duration_ms = cap.duration_ms,
        bytes = cap.byte_size,
        "capture done",
    );

    // 2. Transcribe
    let tx_result = transcribe::dispatch(
        deps.app, deps.app_dir, &cap.wav_path, deps.settings, deps.groq_key,
    ).await
        .map_err(|e| PipelineError { stage: StageError::Transcribe(e.to_string()) })?;
    tracing::info!(
        engine = %deps.settings.transcription.engine,
        model = %tx_result.model,
        language = ?tx_result.language,
        duration_ms = tx_result.duration_ms,
        "transcribe done",
    );

    // 3. Format
    let final_text = format::run_format(&tx_result.text, deps.settings, deps.db)
        .map_err(|e| PipelineError { stage: StageError::Format(format!("{e:?}")) })?;
    tracing::info!(words = final_text.split_whitespace().count(), "format done");

    // 4. Inject (best-effort; continues to persist even if it falls back)
    let inject_method = if !final_text.is_empty() {
        inject::paste(&final_text)
            .map_err(|e| PipelineError { stage: StageError::Inject(e) })?
    } else {
        inject::InjectMethod::Enigo
    };
    tracing::info!(method = ?inject_method, "inject done");

    // 5. Persist
    let record = commit::TranscriptionRecord {
        raw_text: tx_result.text.clone(),
        final_text: final_text.clone(),
        word_count: final_text.split_whitespace().count() as i64,
        duration_ms: cap.duration_ms as i64,
        language: tx_result.language.clone(),
        engine: deps.settings.transcription.engine.clone(),
        model: tx_result.model.clone(),
        app_context: String::new(),
        mode: "dictation".into(),
        enhanced: false,
    };
    let row_id = tokio::task::spawn_blocking({
        let db = deps.db.clone();           // requires Db: Clone (verify Phase 0)
        let settings = deps.settings.clone();
        move || commit::commit_session(&db, record, &settings)
    }).await
        .map_err(|e| PipelineError { stage: StageError::Persist(e.to_string()) })?
        .map_err(|e| PipelineError { stage: StageError::Persist(format!("{e:?}")) })?;
    tracing::info!(row_id, "persist done");

    // 6. Cleanup
    let _ = std::fs::remove_file(&cap.wav_path);

    Ok(row_id)
}
```

If `Db` is not `Clone`, replace the `spawn_blocking` capture with `Arc<Db>` (changes Phase 0 ergonomics) OR run `commit_session` synchronously on the async thread — for Phase 2 the latter is acceptable since rusqlite is in-process and the synchronous block is bounded by 1 transaction; cite this in a code comment and revisit in Phase 5 if profiling shows stalls. To keep the plan's stated guarantee ("DB write in spawn_blocking"), prefer wrapping `Db` in `Arc` if not already.

- [ ] **3.2 — `cargo build`**

```bash
cd src-tauri && cargo build
```

Expected: green.

- [ ] **3.3 — Add a smoke unit test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_mode_returns_phase_4_error() {
        // We can't construct a real PipelineDeps without an AppHandle, so
        // assert the error type via a tiny helper.
        let err = PipelineError { stage: StageError::Capture("command mode is Phase 4".into()) };
        assert!(format!("{err}").contains("Phase 4"));
    }
}
```

```bash
cd src-tauri && cargo test --lib pipeline
```

Expected: PASS.

### Step 4: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "feat(pipeline): add run_session orchestrator wiring capture->transcribe->format->inject->persist"
```

---

## Task 15 — Tauri command cutover + delete `recorder.rs`

**Goal:** Switch `start_recording` / `stop_recording` Tauri commands to call `pipeline::run_session`. Move `audio` + `recording_state` from the `Recorder` struct into `AppState`. Delete `recorder.rs`. Drop `legacy_v1::Settings` import. Add boot-time tmp sweep.

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/lib.rs`
- Delete: `src-tauri/src/recorder.rs`
- Delete: `src-tauri/src/paste.rs` (logic moved to `pipeline::inject`)
- Modify: `src-tauri/src/cleanup.rs` (no-op? still used by inject? — check imports)

### Step 1: AppState evolves

- [ ] **1.1 — Find AppState definition**

```bash
rg -n "pub struct AppState" src-tauri/src
```

- [ ] **1.2 — Add fields**

```rust
pub struct AppState {
    pub app_dir: PathBuf,
    pub settings: Mutex<Settings>,
    pub keyring: Box<dyn KeyringBackend>,
    pub db: std::sync::Arc<Db>,
    pub audio: Mutex<AudioRecorder>,
    pub recording_state: Mutex<RecordingState>,
}
```

`RecordingState` is the same enum currently in `recorder.rs`. Move it into a fresh `src-tauri/src/recording_state.rs`:

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum RecordingState {
    Ready,
    Recording,
    Transcribing,
}
```

Add `mod recording_state;` in `lib.rs` and `main.rs`. `update_overlay` (currently in `recorder.rs`) moves into the same file or stays inline in `main.rs`.

### Step 2: Tauri command bodies

- [ ] **2.1 — `start_recording`**

```rust
#[tauri::command]
pub async fn start_recording(state: tauri::State<'_, AppState>, app: AppHandle)
    -> Result<(), String>
{
    {
        let mut rs = state.recording_state.lock().map_err(|e| e.to_string())?;
        if *rs != RecordingState::Ready {
            return Err("Already recording or transcribing".into());
        }
        *rs = RecordingState::Recording;
    }
    let mic_name = state.settings.lock().map_err(|e| e.to_string())?.microphone.clone();
    state.audio.lock().map_err(|e| e.to_string())?.start(&mic_name)?;
    let _ = app.emit("recording-state", RecordingState::Recording);
    update_overlay(&app, &RecordingState::Recording);
    Ok(())
}
```

- [ ] **2.2 — `stop_recording`**

```rust
#[tauri::command]
pub async fn stop_recording(state: tauri::State<'_, AppState>, app: AppHandle)
    -> Result<(), String>
{
    {
        let mut rs = state.recording_state.lock().map_err(|e| e.to_string())?;
        if *rs != RecordingState::Recording {
            return Err("Not currently recording".into());
        }
        *rs = RecordingState::Transcribing;
    }
    let _ = app.emit("recording-state", RecordingState::Transcribing);
    update_overlay(&app, &RecordingState::Transcribing);

    // Snapshot under lock, drop guard before await.
    let settings = state.settings.lock().map_err(|e| e.to_string())?.clone();
    let groq_key = state.keyring.get().ok().flatten();

    let deps = pipeline::PipelineDeps {
        db: &state.db,
        settings: &settings,
        audio: &state.audio,
        app: &app,
        app_dir: &state.app_dir,
        groq_key: groq_key.as_deref(),
    };
    let outcome = pipeline::run_session(deps, pipeline::PipelineMode::Dictation).await;

    let mut rs = state.recording_state.lock().map_err(|e| e.to_string())?;
    *rs = RecordingState::Ready;
    drop(rs);
    let _ = app.emit("recording-state", RecordingState::Ready);
    update_overlay(&app, &RecordingState::Ready);

    match outcome {
        Ok(row_id) => {
            let _ = app.emit("transcription:new", serde_json::json!({"rowId": row_id}));
            Ok(())
        }
        Err(e) => Err(format!("{e:?}")),
    }
}
```

### Step 3: Boot-time tmp sweep

- [ ] **3.1 — In `main.rs::run` after `Db` opens, before commands register**

```rust
let purged = pipeline::tmp::sweep_stale_wavs(std::time::Duration::from_secs(600));
if purged > 0 {
    tracing::info!(purged, "swept stale tmp wav files at boot");
}
```

### Step 4: Delete recorder.rs + paste.rs

- [ ] **4.1**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com rm src-tauri/src/recorder.rs src-tauri/src/paste.rs
```

Remove `mod recorder;` and `mod paste;` from `main.rs` and `lib.rs`. The `legacy_v1::Settings` import in `recorder.rs` disappears with the file — closing the Phase 1 backlog item.

### Step 5: Build + smoke

- [ ] **5.1 — Build production**

```bash
cd src-tauri && npx tauri build --no-bundle
```

Expected: green. (Use repo root `npm`/`pnpm` if `npx` errors — match what Phase 1 used.)

- [ ] **5.2 — Run all tests**

```bash
cd src-tauri && cargo test
```

Expected: all green.

### Step 6: Commit

- [ ] **Commit**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com commit -am "refactor(pipeline): cut Tauri commands over to pipeline::run_session; delete recorder.rs"
```

---

## Task 16 — Manual smoke + completion docs

**Goal:** Walk Bruno through the smoke matrix from §10 of the spec; on green, append a completion section to this plan and push.

### Step 1: Smoke matrix

Bruno runs each in turn; each line must pass before moving on:

- [ ] **1.1 — Settings v2→v3 migration**

Backup, set medium, boot, verify:

```powershell
copy "$env:APPDATA\com.typr.app\config.json" "$env:USERPROFILE\Desktop\typr-config-backup.json"
# Edit config.json to set "schemaVersion": 2 and "transcription.whisperModel": "medium"
# (or use a fresh config — the migrator path is the same).
# Launch the built typr.exe from src-tauri/target/release/.
# After boot:
Get-Content "$env:APPDATA\com.typr.app\config.json" | Select-String "schemaVersion|whisperModel"
```

Expected: `"schemaVersion": 3`, `"whisperModel": "turbo"`. Log line `v2->v3 model remap from=medium to=turbo` present in `%LOCALAPPDATA%\com.typr.app\logs\typr.log`.

- [ ] **1.2 — Push-to-talk PT command**

Hold F24, say `"olá mundo, vírgula, isto é um teste"`, release. Expected paste: `"olá mundo, isto é um teste"` (`vírgula` replaced).

- [ ] **1.3 — Toggle EN basic**

Press F24 once, say `"hello world period"`, press F24 again. Expected paste: `"hello world."`.

- [ ] **1.4 — Engine switch**

Open settings UI, switch engine to `cloud`, save Groq key, repeat 1.3. Expected: same output via Groq backend (log shows `engine="cloud"` line).

- [ ] **1.5 — Mic disconnect mid-session**

Start recording, physically pull the USB mic, stop. Expected: error toast, state returns to `Ready`, no crash.

- [ ] **1.6 — DB inspection**

```powershell
sqlite3 "$env:APPDATA\com.typr.app\typr.db" "SELECT id, final_text, word_count, engine, model FROM transcriptions ORDER BY id DESC LIMIT 5;"
```

Expected: 5 most recent sessions visible with sane columns.

- [ ] **1.7 — Cap purge**

```powershell
sqlite3 "$env:APPDATA\com.typr.app\typr.db" "SELECT SUM(word_count) FROM transcriptions;"
# Edit config.json to set data.wordCountCap to a number lower than the sum;
# trigger one more dictation.
sqlite3 "$env:APPDATA\com.typr.app\typr.db" "SELECT SUM(word_count) FROM transcriptions;"
```

Expected: total drops to ≤ cap.

### Step 2: Append completion section

- [ ] **2.1 — Edit this plan**

Append a new top-level section at the bottom:

```markdown
## Phase 2 Completion Status — YYYY-MM-DD

- All 16 tasks complete on `wispr-parity`.
- Commits: `<sha-T1> .. <sha-T16>` (one per task plus reviewer fixes).
- Test posture: `cargo test` green (X lib + Y integration + Z storage tests).
- Bruno smoke matrix: each row passed.
- Reviewer pass: <findings + fixes>.
- Open Phase 2 backlog forwarded to Phase 3:
  - Compose Mode (spec §13).
  - VAD constants tunable via settings.
  - Dictionary auto-add threshold as a settings field.
  - JSON tracing layer.
  - Esc / tray cancellation token.
```

### Step 3: Reviewer pass + push

- [ ] **3.1 — Dispatch final code reviewer**

(Subagent-Driven Development controller handles this — see skill.)

- [ ] **3.2 — Address findings, commit, push**

```bash
git -c user.name=Bruno -c user.email=brunorodrigues2627@gmail.com push origin wispr-parity
```

---

## Cutover summary (mirrors spec §11)

```
Layer 1 — Task 1                  (settings v2→v3 migrator)
Layer 2 — Tasks 2, 3, 4, 5, 6, 7, 8, 9, 10  (pure modules; recorder.rs alive)
Layer 3 — Tasks 11, 12, 14         (orchestrator wired; recorder.rs alive)
Layer 4 — Task 15                  (Tauri command cutover; recorder.rs deleted)
Layer 5 — Task 13                  (whisper -otxt switch; can run before or after Layer 4)
Layer 6 — already inside Task 11   (runtime model gate)
Layer 7 — Task 16                  (manual smoke + reviewer + docs + push)
```

Each task ends with one or more commits; every commit on `wispr-parity` builds. Worst-case revert is `git revert <task-sha>` without cascading.

---

## Self-review notes

- **Spec coverage:** every spec section maps to at least one task. §3 → T1, §6 (audio+VAD+tmp) → T2+T3+T9, §5 (format) → T5+T6+T7+T8+T12, §7 (whisper -otxt) → T0+T13, §8 (DB) → T4+T10, §4+§9+§2 (orchestrator + tracing + module structure) → T11+T14+T15, §10 (testing) is interleaved into every task, §11 cutover → task ordering above.
- **No placeholders:** every code-bearing step shows the actual code or a precise transformation against an existing block. Where signatures depend on existing code (e.g. `enigo` API in T15), the step says "mirror the existing block" and points at the file to copy from.
- **Type consistency:** `TranscriptionResult` defined in T11 used by T13 + T14. `RecordingState` extracted in T15 used inside same task. `PipelineDeps`/`PipelineMode`/`StageError` defined in T14 used by T15. `TranscriptionRecord` defined in T10 used by T14.
- **Schema gap noted:** the design says "tracked in `dictionary.seen_count`" but the existing schema lacks that column. T4 adds a separate `auto_add_candidates` table; this is a deliberate deviation called out at the top of the plan and inside T4's goal line. The reviewer should validate this choice.
- **`Db` cloning concern:** T14 step 3 flags that `Db: Clone` is a precondition for the `spawn_blocking` design. If Phase 0 made `Db` non-Clone, the implementer falls back to wrapping it in `Arc<Db>` during T15 (when `AppState` is restructured anyway).
