#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use typr_lib::audio::{self, AudioRecorder};
use typr_lib::downloader;
use typr_lib::recording_state::RecordingState;
use typr_lib::settings::adapter::{apply_v1_v2_fields, to_v1_view, write_v1_keyring};
use typr_lib::settings::keyring::{KeyringBackend, SystemBackend};
use typr_lib::settings::legacy_v1::Settings as V1Settings;
use typr_lib::settings::schema::Settings as V2Settings;
use typr_lib::settings::{self, LoadOutcome, MigrationEvent};
use typr_lib::storage::app_meta::AppMetaRepo;
use typr_lib::storage::dictionary::{DictionaryRepo, DictionaryTerm, NewDictionaryTerm};
use typr_lib::storage::scratchpad::{NewNote, ScratchpadNote, ScratchpadRepo};
use typr_lib::storage::snippets::{NewSnippet, Snippet, SnippetRepo};
use typr_lib::storage::stats::{DailyStats, StatsRepo, StreakInfo, Totals};
use typr_lib::storage::transcriptions::{Transcription, TranscriptionRepo};
use typr_lib::storage::Db;
use typr_lib::telemetry;
use typr_lib::transcribe_local;

struct AppState {
    settings: Mutex<V2Settings>,
    keyring: Box<dyn KeyringBackend>,
    app_dir: PathBuf,
    db: Db,
    audio: Mutex<AudioRecorder>,
    recording_state: Mutex<RecordingState>,
}

fn get_app_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.typr.app")
}

/// Tauri-specific overlay UI side-effect. Lives here (not in the pipeline)
/// because the pipeline must remain headless and reusable from Phase 4
/// command mode without dragging the overlay window dependency along.
fn update_overlay(app: &tauri::AppHandle, state: &RecordingState) {
    if let Some(overlay) = app.get_webview_window("overlay") {
        let class = match state {
            RecordingState::Ready => "mic",
            RecordingState::Recording => "mic recording",
            RecordingState::Transcribing => "mic transcribing",
        };
        let js = format!("document.getElementById('mic').className = '{}';", class);
        let _ = overlay.eval(&js);
    }
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> V1Settings {
    // Snapshot v2 under the lock, drop the guard, then hit the keyring.
    // `to_v1_view` calls `backend.get()` which on `SystemBackend` is a
    // Credential Manager syscall — holding `Mutex<V2Settings>` across that
    // would serialise every settings read with the recorder hot path.
    let v2_snapshot = state.settings.lock().unwrap().clone();
    to_v1_view(&v2_snapshot, state.keyring.as_ref())
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: V1Settings) -> Result<(), String> {
    let mut v2 = state.settings.lock().unwrap();
    let snapshot = v2.clone();

    // Step 1: mutate v2 in-memory only — keyring untouched.
    apply_v1_v2_fields(&mut v2, &settings);

    // Step 2: write JSON. On failure, roll back the in-memory tree so we
    // don't drift from disk. No OS-side state has moved yet.
    if let Err(e) = settings::save(&state.app_dir, &v2) {
        *v2 = snapshot;
        return Err(format!("settings save failed: {e}"));
    }

    // Step 3: write keyring. If this fails, JSON + in-memory already reflect
    // the payload but the keyring lags. Surface the error so the frontend can
    // retry; full atomicity needs Phase 2's payload reshape (Option<String>
    // for "unchanged" vs "cleared" so a failed read can't round-trip an empty
    // sentinel back through save).
    write_v1_keyring(&settings, state.keyring.as_ref())
        .map_err(|e| format!("keyring write failed: {e}"))?;
    Ok(())
}

#[tauri::command]
fn list_microphones() -> Vec<audio::MicDevice> {
    audio::list_microphones()
}

#[tauri::command]
fn get_recording_state(state: State<AppState>) -> RecordingState {
    state.recording_state.lock().unwrap().clone()
}

#[tauri::command]
fn check_model_downloaded(state: State<AppState>, model_size: String) -> bool {
    let model_file = transcribe_local::model_filename(&model_size);
    state.app_dir.join(&model_file).exists()
}

#[tauri::command]
async fn download_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    model_size: String,
) -> Result<(), String> {
    let url = transcribe_local::model_download_url(&model_size);
    let model_file = transcribe_local::model_filename(&model_size);
    let dest = state.app_dir.join(&model_file);
    downloader::download_model(app, &url, &dest).await
}

#[tauri::command]
async fn toggle_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    do_toggle_recording(&app, &state).await
}

/// Drive the pipeline once we are committed to the `Recording → Transcribing
/// → Ready` transition. Used by both the toggle command and the PTT release
/// branch so emit/overlay/state housekeeping stays in lock-step.
///
/// Lock discipline: `recording_state` and `settings` are `std::sync::Mutex`,
/// so every guard is acquired in a `{}` scope and dropped before any `.await`.
/// `pipeline::run_session` re-locks `state.audio` internally inside
/// `capture::stop_and_save`, so we must NOT hold any lock around that call.
async fn run_pipeline_and_reset_state(app: &tauri::AppHandle, state: &AppState) {
    {
        let mut rs = state.recording_state.lock().unwrap();
        *rs = RecordingState::Transcribing;
    }
    let _ = app.emit("recording-state", RecordingState::Transcribing);
    update_overlay(app, &RecordingState::Transcribing);

    let settings = state.settings.lock().unwrap().clone();
    let groq_key = state.keyring.get().ok().flatten();

    let outcome = {
        let deps = typr_lib::pipeline::PipelineDeps {
            db: &state.db,
            settings: &settings,
            audio: &state.audio,
            app,
            app_dir: &state.app_dir,
            groq_key: groq_key.as_deref(),
        };
        typr_lib::pipeline::run_session(deps, typr_lib::pipeline::PipelineMode::Dictation).await
    };

    {
        let mut rs = state.recording_state.lock().unwrap();
        *rs = RecordingState::Ready;
    }
    let _ = app.emit("recording-state", RecordingState::Ready);
    update_overlay(app, &RecordingState::Ready);

    match outcome {
        Ok(row_id) => {
            let _ = app.emit(
                "transcription:new",
                serde_json::json!({ "rowId": row_id }),
            );
            tracing::info!(row_id, "[Typr] Transcription persisted");
        }
        Err(e) => tracing::error!(error = %e, "[Typr] Pipeline error"),
    }
}

/// Shared logic for toggle recording, used by both the Tauri command and the
/// hotkey handler in toggle mode. PTT mode bypasses this and drives state
/// directly from the `Pressed`/`Released` branches in `setup`.
async fn do_toggle_recording(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<String, String> {
    let current = {
        let rs = state.recording_state.lock().map_err(|e| e.to_string())?;
        rs.clone()
    };
    match current {
        RecordingState::Ready => {
            // Snapshot mic outside the recording_state lock to keep critical
            // sections short. `audio.start` does the cpal handshake — we
            // hold only `state.audio`'s Mutex for that, never `settings`.
            let mic = state.settings.lock().unwrap().microphone.clone();
            state
                .audio
                .lock()
                .map_err(|e| e.to_string())?
                .start(&mic)?;

            {
                let mut rs = state.recording_state.lock().map_err(|e| e.to_string())?;
                *rs = RecordingState::Recording;
            }
            let _ = app.emit("recording-state", RecordingState::Recording);
            update_overlay(app, &RecordingState::Recording);
            Ok("recording".into())
        }
        RecordingState::Recording => {
            run_pipeline_and_reset_state(app, state).await;
            Ok("ok".into())
        }
        RecordingState::Transcribing => Err("Currently transcribing, please wait".into()),
    }
}

// ---------------------------------------------------------------------------
// Phase 3 UI commands — CRUD/stats/wizard/groq-key.
//
// All commands return `Result<T, String>` so the JS side gets a string error.
// The storage repos already provide the heavy lifting; these are thin
// pass-throughs with payload-shape conversion at the boundary.
// ---------------------------------------------------------------------------

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn today_iso() -> String {
    use time::format_description::well_known::Iso8601;
    time::OffsetDateTime::now_utc()
        .date()
        .format(&Iso8601::DATE)
        .unwrap_or_else(|_| "1970-01-01".into())
}

// --- transcriptions --------------------------------------------------------

#[tauri::command]
fn list_transcriptions(
    state: State<AppState>,
    limit: u32,
    offset: u32,
) -> Result<Vec<Transcription>, String> {
    TranscriptionRepo::new(&state.db)
        .list_paginated(limit as i64, offset as i64)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn search_transcriptions(
    state: State<AppState>,
    query: String,
    limit: u32,
) -> Result<Vec<Transcription>, String> {
    TranscriptionRepo::new(&state.db)
        .search_fts(&query, limit as i64)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_transcription(state: State<AppState>, id: i64) -> Result<(), String> {
    TranscriptionRepo::new(&state.db)
        .delete_by_id(id)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// --- dictionary ------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewDictionaryTermPayload {
    term: String,
    replacement: Option<String>,
    is_abbreviation: bool,
    auto_added: bool,
    enabled: bool,
}

#[tauri::command]
fn list_dictionary_terms(state: State<AppState>) -> Result<Vec<DictionaryTerm>, String> {
    DictionaryRepo::new(&state.db).list().map_err(|e| e.to_string())
}

#[tauri::command]
fn upsert_dictionary_term(
    state: State<AppState>,
    term: NewDictionaryTermPayload,
) -> Result<i64, String> {
    DictionaryRepo::new(&state.db)
        .upsert(
            now_secs(),
            NewDictionaryTerm {
                term: &term.term,
                replacement: term.replacement.as_deref(),
                is_abbreviation: term.is_abbreviation,
                auto_added: term.auto_added,
                enabled: term.enabled,
            },
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_dictionary_term(state: State<AppState>, id: i64) -> Result<(), String> {
    DictionaryRepo::new(&state.db)
        .delete(id)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// --- snippets --------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewSnippetPayload {
    trigger: String,
    expansion: String,
    description: Option<String>,
    enabled: bool,
}

#[tauri::command]
fn list_snippets(state: State<AppState>) -> Result<Vec<Snippet>, String> {
    SnippetRepo::new(&state.db).list().map_err(|e| e.to_string())
}

#[tauri::command]
fn upsert_snippet(state: State<AppState>, snippet: NewSnippetPayload) -> Result<i64, String> {
    SnippetRepo::new(&state.db)
        .upsert(
            now_secs(),
            NewSnippet {
                trigger: &snippet.trigger,
                expansion: &snippet.expansion,
                description: snippet.description.as_deref(),
                enabled: snippet.enabled,
            },
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_snippet(state: State<AppState>, id: i64) -> Result<(), String> {
    SnippetRepo::new(&state.db)
        .delete(id)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// --- scratchpad ------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewNotePayload {
    id: Option<i64>,
    title: Option<String>,
    body: String,
    pinned: bool,
}

#[tauri::command]
fn list_scratchpad_notes(state: State<AppState>) -> Result<Vec<ScratchpadNote>, String> {
    ScratchpadRepo::new(&state.db)
        .list_ordered()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn upsert_scratchpad_note(state: State<AppState>, note: NewNotePayload) -> Result<i64, String> {
    ScratchpadRepo::new(&state.db)
        .upsert(
            now_secs(),
            note.id,
            NewNote {
                title: note.title.as_deref(),
                body: &note.body,
                pinned: note.pinned,
            },
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_scratchpad_note(state: State<AppState>, id: i64) -> Result<(), String> {
    ScratchpadRepo::new(&state.db)
        .delete(id)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn pin_scratchpad_note(state: State<AppState>, id: i64, pinned: bool) -> Result<(), String> {
    ScratchpadRepo::new(&state.db)
        .set_pinned(id, pinned)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// --- stats -----------------------------------------------------------------

#[tauri::command]
fn get_stats_totals(state: State<AppState>) -> Result<Totals, String> {
    StatsRepo::new(&state.db).totals().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_stats_streak(state: State<AppState>) -> Result<StreakInfo, String> {
    StatsRepo::new(&state.db)
        .streak_info(&today_iso())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_stats_by_day(state: State<AppState>) -> Result<Vec<DailyStats>, String> {
    StatsRepo::new(&state.db)
        .list_all_days()
        .map_err(|e| e.to_string())
}

// --- wizard ---------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WizardStatus {
    completed: bool,
}

#[tauri::command]
fn wizard_status(state: State<AppState>) -> Result<WizardStatus, String> {
    let completed = AppMetaRepo::new(&state.db)
        .get("wizard_completed")
        .map_err(|e| e.to_string())?
        .as_deref()
        == Some("1");
    Ok(WizardStatus { completed })
}

#[tauri::command]
fn mark_wizard_complete(state: State<AppState>) -> Result<(), String> {
    AppMetaRepo::new(&state.db)
        .set("wizard_completed", "1")
        .map_err(|e| e.to_string())
}

// --- groq key test --------------------------------------------------------

#[tauri::command]
async fn test_groq_key(key: String) -> Result<(), String> {
    if key.is_empty() {
        return Err("API key is empty".into());
    }
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.groq.com/openai/v1/models")
        .bearer_auth(&key)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?;
    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Groq API error ({status}): {body}"))
    }
}

fn main() {
    let app_dir = get_app_dir();
    let log_dir = app_dir.join("logs");
    let _ = telemetry::init_tracing(&log_dir);

    let db = Db::open(&app_dir.join("typr.db")).expect("open typr.db");
    let keyring: Box<dyn KeyringBackend> = Box::new(SystemBackend);

    // Loader returns LoadOutcome with embedded events for recoverable cases
    // (MigrationFailed, UnknownVersion) and Errs only on hard I/O failures.
    // We refuse to crash boot on those — fall back to defaults + a synthetic
    // MigrationFailed event so the frontend gets a toast instead of a silent
    // exit. Phase 1 design explicitly preserves .bak; a panic here would
    // short-circuit that recovery flow.
    let outcome = settings::load(&app_dir, &db, keyring.as_ref()).unwrap_or_else(|e| {
        tracing::error!(error = %e, "failed to load settings; falling back to defaults");
        LoadOutcome {
            settings: V2Settings::default(),
            events: vec![MigrationEvent::MigrationFailed(format!(
                "settings load failed: {e}"
            ))],
        }
    });

    let v2_settings = outcome.settings.clone();
    let initial_hotkey = v2_settings.hotkeys.dictation.clone();
    let migration_events = outcome.events;

    // Boot-time tmp sweep: any *.wav / *.txt left in the per-session tmp dir
    // older than 10 minutes is junk from a crashed prior run. Best-effort —
    // failure modes are logged inside `sweep_stale_wavs` and a zero count
    // here is the common case.
    let purged =
        typr_lib::pipeline::tmp::sweep_stale_wavs(std::time::Duration::from_secs(600));
    if purged > 0 {
        tracing::info!(purged, "swept stale tmp wav files at boot");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            settings: Mutex::new(v2_settings),
            keyring,
            app_dir,
            db,
            audio: Mutex::new(AudioRecorder::new()),
            recording_state: Mutex::new(RecordingState::Ready),
        })
        .invoke_handler(tauri::generate_handler![
            // Phase 1+2 (existing)
            get_settings,
            save_settings,
            list_microphones,
            get_recording_state,
            check_model_downloaded,
            download_model,
            toggle_recording,
            // Phase 3: transcriptions
            list_transcriptions,
            search_transcriptions,
            delete_transcription,
            // Phase 3: dictionary
            list_dictionary_terms,
            upsert_dictionary_term,
            delete_dictionary_term,
            // Phase 3: snippets
            list_snippets,
            upsert_snippet,
            delete_snippet,
            // Phase 3: scratchpad
            list_scratchpad_notes,
            upsert_scratchpad_note,
            delete_scratchpad_note,
            pin_scratchpad_note,
            // Phase 3: stats
            get_stats_totals,
            get_stats_streak,
            get_stats_by_day,
            // Phase 3: wizard
            wizard_status,
            mark_wizard_complete,
            // Phase 3: groq
            test_groq_key,
        ])
        .setup(move |app| {
            // Replay the migration events captured pre-Builder so the frontend
            // can render the same toasts the lib.rs reference flow produced.
            for ev in &migration_events {
                match ev {
                    MigrationEvent::Migrated => {
                        tracing::info!(stage = "settings", "migrated v1 -> v2");
                        if let Err(e) = app.emit("settings:migrated", ()) {
                            tracing::warn!(error = %e, event = "settings:migrated", "failed to emit migration event");
                        }
                    }
                    MigrationEvent::ModelRemapped { from, to } => {
                        tracing::info!(stage = "settings", %from, %to, "whisper model remapped");
                        if let Err(e) = app.emit(
                            "settings:model-remapped",
                            serde_json::json!({ "from": from, "to": to }),
                        ) {
                            tracing::warn!(error = %e, event = "settings:model-remapped", "failed to emit migration event");
                        }
                    }
                    MigrationEvent::NeedsGroqKey => {
                        tracing::info!(stage = "settings", "legacy Groq key - prompting for re-entry");
                        if let Err(e) = app.emit("settings:needs-groq-key", ()) {
                            tracing::warn!(error = %e, event = "settings:needs-groq-key", "failed to emit migration event");
                        }
                    }
                    MigrationEvent::UnknownVersion(v) => {
                        tracing::warn!(stage = "settings", version = v, "unknown settings version; using defaults");
                        if let Err(e) = app.emit("settings:unknown-version", *v) {
                            tracing::warn!(error = %e, event = "settings:unknown-version", "failed to emit migration event");
                        }
                    }
                    MigrationEvent::MigrationFailed(msg) => {
                        tracing::error!(stage = "settings", error = %msg, "migration failed; .bak preserved");
                        if let Err(e) = app.emit("settings:migration-failed", msg.clone()) {
                            tracing::warn!(error = %e, event = "settings:migration-failed", "failed to emit migration event");
                        }
                    }
                }
            }

            // Create the overlay window (small mic icon, top-right, always on top)
            let monitor = app.primary_monitor().ok().flatten();
            let (x, y) = if let Some(m) = monitor {
                let size = m.size();
                let scale = m.scale_factor();
                let logical_w = size.width as f64 / scale;
                ((logical_w - 60.0) as i32, 10_i32)
            } else {
                (1380, 10)
            };

            let overlay = WebviewWindowBuilder::new(
                app,
                "overlay",
                WebviewUrl::App("src/overlay.html".into()),
            )
            .title("")
            .inner_size(50.0, 50.0)
            .position(x as f64, y as f64)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .shadow(false)
            .build();

            match overlay {
                Ok(_) => tracing::info!("[Typr] Overlay window created"),
                Err(e) => tracing::error!(error = %e, "[Typr] Failed to create overlay"),
            }

            let handle = app.handle().clone();

            tracing::info!(hotkey = %initial_hotkey, "[Typr] Registering global shortcut");

            match app.global_shortcut().on_shortcut(
                initial_hotkey.as_str(),
                move |_app, shortcut, event| {
                    tracing::debug!(?shortcut, state = ?event.state, "[Typr] Hotkey event");
                    let handle = handle.clone();
                    let state = handle.state::<AppState>();
                    let mode = state.settings.lock().unwrap().hotkeys.recording_mode.clone();
                    tracing::debug!(mode = %mode, "[Typr] Recording mode");

                    match event.state {
                        ShortcutState::Pressed => {
                            tauri::async_runtime::spawn(async move {
                                let state = handle.state::<AppState>();
                                match mode.as_str() {
                                    "toggle" => {
                                        tracing::debug!("[Typr] Toggle mode: calling do_toggle_recording");
                                        match do_toggle_recording(&handle, state.inner()).await {
                                            Ok(result) => tracing::info!(result = %result, "[Typr] Toggle result"),
                                            Err(e) => tracing::error!(error = %e, "[Typr] Toggle error"),
                                        }
                                    }
                                    "push-to-talk" => {
                                        let current = {
                                            let rs = state.recording_state.lock().unwrap();
                                            rs.clone()
                                        };
                                        tracing::debug!(current = ?current, "[Typr] PTT mode");
                                        if current == RecordingState::Ready {
                                            let mic = state.settings.lock().unwrap().microphone.clone();
                                            let start_res = state.audio.lock().unwrap().start(&mic);
                                            match start_res {
                                                Ok(_) => {
                                                    {
                                                        let mut rs = state.recording_state.lock().unwrap();
                                                        *rs = RecordingState::Recording;
                                                    }
                                                    let _ = handle.emit("recording-state", RecordingState::Recording);
                                                    update_overlay(&handle, &RecordingState::Recording);
                                                    tracing::info!("[Typr] Recording started");
                                                }
                                                Err(e) => tracing::error!(error = %e, "[Typr] Start recording error"),
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            });
                        }
                        ShortcutState::Released => {
                            if mode == "push-to-talk" {
                                tauri::async_runtime::spawn(async move {
                                    let state = handle.state::<AppState>();
                                    let current = {
                                        let rs = state.recording_state.lock().unwrap();
                                        rs.clone()
                                    };
                                    if current == RecordingState::Recording {
                                        run_pipeline_and_reset_state(&handle, state.inner()).await;
                                    }
                                });
                            }
                        }
                    }
                },
            ) {
                Ok(_) => tracing::info!("[Typr] Global shortcut registered successfully"),
                Err(e) => tracing::error!(error = %e, "[Typr] Failed to register global shortcut"),
            }

            tracing::info!(stage = "boot", "storage + telemetry + settings initialised");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
