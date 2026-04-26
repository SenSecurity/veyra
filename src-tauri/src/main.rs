#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use typr_lib::audio;
use typr_lib::downloader;
use typr_lib::recorder::{Recorder, RecordingState};
use typr_lib::settings::adapter::{apply_v1_v2_fields, to_v1_view, write_v1_keyring};
use typr_lib::settings::keyring::{KeyringBackend, SystemBackend};
use typr_lib::settings::legacy_v1::Settings as V1Settings;
use typr_lib::settings::schema::Settings as V2Settings;
use typr_lib::settings::{self, LoadOutcome, MigrationEvent};
use typr_lib::storage::Db;
use typr_lib::telemetry;
use typr_lib::transcribe_local;

struct AppState {
    recorder: Recorder,
    settings: Mutex<V2Settings>,
    keyring: Box<dyn KeyringBackend>,
    app_dir: PathBuf,
    #[allow(dead_code)] // Held so SQLite stays open for the app lifetime; consumers come in Phase 2+.
    db: Db,
}

fn get_app_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.typr.app")
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
    state.recorder.get_state()
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

/// Build a fresh `V1Settings` snapshot for the recorder by reading the v2 tree
/// and keyring at call time. The recorder still consumes the legacy six-field
/// shape; Phase 2 will replace this with direct v2 access.
///
/// Lock scope: clone v2 under the guard, drop the guard, then read the
/// keyring. The keyring backend hits the OS Credential Manager — holding the
/// settings Mutex across that syscall would serialise the recorder's hot path
/// with any concurrent `get_settings`/`save_settings` invocation.
fn legacy_view(state: &AppState) -> V1Settings {
    let v2_snapshot = state.settings.lock().unwrap().clone();
    to_v1_view(&v2_snapshot, state.keyring.as_ref())
}

/// Shared logic for toggle recording, used by both the Tauri command and hotkey handler.
async fn do_toggle_recording(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<String, String> {
    let current_state = state.recorder.get_state();
    match current_state {
        RecordingState::Ready => {
            let mic = state.settings.lock().unwrap().microphone.clone();
            state.recorder.start_recording(app, &mic)?;
            Ok("recording".to_string())
        }
        RecordingState::Recording => {
            let settings = legacy_view(state);
            let result = state
                .recorder
                .stop_and_transcribe(app, &settings, &state.app_dir)
                .await?;
            Ok(result)
        }
        RecordingState::Transcribing => {
            Err("Currently transcribing, please wait".to_string())
        }
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

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            recorder: Recorder::new(),
            settings: Mutex::new(v2_settings),
            keyring,
            app_dir,
            db,
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_microphones,
            get_recording_state,
            check_model_downloaded,
            download_model,
            toggle_recording,
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
                                        let current = state.recorder.get_state();
                                        tracing::debug!(current = ?current, "[Typr] PTT mode");
                                        if current == RecordingState::Ready {
                                            let mic = state.settings.lock().unwrap().microphone.clone();
                                            match state.recorder.start_recording(&handle, &mic) {
                                                Ok(_) => tracing::info!("[Typr] Recording started"),
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
                                    let current = state.recorder.get_state();
                                    if current == RecordingState::Recording {
                                        let settings = legacy_view(state.inner());
                                        match state.recorder.stop_and_transcribe(
                                            &handle,
                                            &settings,
                                            &state.app_dir,
                                        ).await {
                                            Ok(result) => tracing::info!(result = %result, "[Typr] Transcription"),
                                            Err(e) => tracing::error!(error = %e, "[Typr] Transcription error"),
                                        }
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
