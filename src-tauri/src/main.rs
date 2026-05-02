#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use typr_lib::audio::{self, AudioRecorder};
use typr_lib::downloader;
use typr_lib::pipeline::PipelineMode;
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

const OVERLAY_WIDTH: i32 = 210;
const OVERLAY_HEIGHT: i32 = 36;
const OVERLAY_BOTTOM_MARGIN: i32 = 8;
const TRAY_SHOW_ID: &str = "tray_show";
const TRAY_HIDE_ID: &str = "tray_hide";
const TRAY_EXIT_ID: &str = "tray_exit";

#[cfg(target_os = "windows")]
fn play_transition_sound(state: &RecordingState) {
    let (kind, message_beep) = match state {
        RecordingState::Recording => (ChimeKind::Start, 0x00000040),
        RecordingState::Transcribing => (ChimeKind::Transcribing, 0x00000030),
        RecordingState::Ready => return,
    };
    tauri::async_runtime::spawn_blocking(move || unsafe {
        if play_output_chime(kind).is_err() {
            let _ = windows_sys::Win32::System::Diagnostics::Debug::Beep(
                if kind == ChimeKind::Start { 740 } else { 520 },
                90,
            );
            let _ = windows_sys::Win32::System::Diagnostics::Debug::MessageBeep(message_beep);
        }
    });
}

#[cfg(not(target_os = "windows"))]
fn play_transition_sound(_state: &RecordingState) {}

#[cfg(target_os = "windows")]
fn active_monitor_bottom_position() -> Option<tauri::PhysicalPosition<i32>> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{
        ClientToScreen, GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId,
        GUITHREADINFO,
    };

    unsafe fn active_caret_point() -> Option<POINT> {
        let foreground = GetForegroundWindow();
        if foreground.is_null() {
            return None;
        }
        let thread_id = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
        if thread_id == 0 {
            return None;
        }
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            flags: 0,
            hwndActive: std::ptr::null_mut(),
            hwndFocus: std::ptr::null_mut(),
            hwndCapture: std::ptr::null_mut(),
            hwndMenuOwner: std::ptr::null_mut(),
            hwndMoveSize: std::ptr::null_mut(),
            hwndCaret: std::ptr::null_mut(),
            rcCaret: std::mem::zeroed(),
        };
        if GetGUIThreadInfo(thread_id, &mut info) == 0 || info.hwndCaret.is_null() {
            return None;
        }

        let mut point = POINT {
            x: info.rcCaret.left,
            y: info.rcCaret.bottom,
        };
        if ClientToScreen(info.hwndCaret, &mut point) == 0 {
            return None;
        }
        Some(point)
    }

    unsafe fn bottom_center_for_screen_point(point: POINT) -> Option<tauri::PhysicalPosition<i32>> {
        let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
        if monitor.is_null() {
            return None;
        }

        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: std::mem::zeroed(),
            rcWork: std::mem::zeroed(),
            dwFlags: 0,
        };
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }

        let work = info.rcWork;
        let x = work.left + ((work.right - work.left - OVERLAY_WIDTH) / 2);
        let y = work.bottom - OVERLAY_BOTTOM_MARGIN - OVERLAY_HEIGHT;
        Some(tauri::PhysicalPosition::new(x, y))
    }

    unsafe {
        let point = active_caret_point().or_else(|| {
            let mut point = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut point) != 0 {
                Some(point)
            } else {
                None
            }
        })?;
        bottom_center_for_screen_point(point)
    }
}

#[cfg(not(target_os = "windows"))]
fn active_monitor_bottom_position() -> Option<tauri::PhysicalPosition<i32>> {
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChimeKind {
    Start,
    Transcribing,
}

fn play_output_chime(kind: ChimeKind) -> Result<(), String> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "No default output device found".to_string())?;
    let supported_config = device.default_output_config().map_err(|e| e.to_string())?;
    let sample_format = supported_config.sample_format();
    let config: cpal::StreamConfig = supported_config.into();
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;
    let duration_ms = match kind {
        ChimeKind::Start => 420,
        ChimeKind::Transcribing => 360,
    };
    let total_frames = ((sample_rate * duration_ms as f32) / 1000.0).max(1.0) as u32;
    let mut frame_index = 0_u32;
    let err_fn = |err| tracing::warn!(error = %err, "transition tone stream error");

    let stream = match sample_format {
        cpal::SampleFormat::F32 => device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _| {
                    write_chime(
                        data,
                        channels,
                        sample_rate,
                        total_frames,
                        kind,
                        &mut frame_index,
                    )
                },
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?,
        cpal::SampleFormat::I16 => device
            .build_output_stream(
                &config,
                move |data: &mut [i16], _| {
                    write_chime(
                        data,
                        channels,
                        sample_rate,
                        total_frames,
                        kind,
                        &mut frame_index,
                    )
                },
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?,
        cpal::SampleFormat::U16 => device
            .build_output_stream(
                &config,
                move |data: &mut [u16], _| {
                    write_chime(
                        data,
                        channels,
                        sample_rate,
                        total_frames,
                        kind,
                        &mut frame_index,
                    )
                },
                err_fn,
                None,
            )
            .map_err(|e| e.to_string())?,
        _ => return Err("Unsupported output sample format".to_string()),
    };

    stream.play().map_err(|e| e.to_string())?;
    std::thread::sleep(std::time::Duration::from_millis(duration_ms + 40));
    Ok(())
}

trait ToneSample {
    fn from_unit(sample: f32) -> Self;
}

impl ToneSample for f32 {
    fn from_unit(sample: f32) -> Self {
        sample
    }
}

impl ToneSample for i16 {
    fn from_unit(sample: f32) -> Self {
        (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
    }
}

impl ToneSample for u16 {
    fn from_unit(sample: f32) -> Self {
        ((sample.clamp(-1.0, 1.0) * 0.5 + 0.5) * u16::MAX as f32) as u16
    }
}

fn write_chime<T: ToneSample>(
    output: &mut [T],
    channels: usize,
    sample_rate: f32,
    total_frames: u32,
    kind: ChimeKind,
    frame_index: &mut u32,
) {
    for frame in output.chunks_mut(channels) {
        let (left, right) = if *frame_index < total_frames {
            let t = *frame_index as f32 / sample_rate;
            chime_sample(t, kind)
        } else {
            (0.0, 0.0)
        };
        *frame_index = frame_index.saturating_add(1);
        for (index, channel) in frame.iter_mut().enumerate() {
            let sample = if index % 2 == 0 { left } else { right };
            *channel = T::from_unit(sample);
        }
    }
}

fn chime_sample(t: f32, kind: ChimeKind) -> (f32, f32) {
    let mono = match kind {
        ChimeKind::Start => {
            bell_note(t, 0.000, 0.34, 392.00, 0.030)
                + bell_note(t, 0.018, 0.32, 587.33, 0.038)
                + bell_note(t, 0.064, 0.30, 739.99, 0.032)
                + bell_note(t, 0.112, 0.24, 987.77, 0.010)
        }
        ChimeKind::Transcribing => {
            bell_note(t, 0.000, 0.26, 880.00, 0.026)
                + bell_note(t, 0.030, 0.30, 659.25, 0.038)
                + bell_note(t, 0.082, 0.28, 493.88, 0.034)
                + bell_note(t, 0.130, 0.18, 329.63, 0.012)
        }
    };
    let shimmer = (t * 9.0).sin() * 0.003;
    let side = mono * 0.08 + shimmer;
    (
        (mono - side).clamp(-0.25, 0.25),
        (mono + side).clamp(-0.25, 0.25),
    )
}

fn bell_note(t: f32, start: f32, duration: f32, frequency: f32, gain: f32) -> f32 {
    let local = t - start;
    if local < 0.0 || local > duration {
        return 0.0;
    }
    let attack = smoothstep((local / 0.026).clamp(0.0, 1.0));
    let decay = (-local * 9.0).exp();
    let release = smoothstep(((duration - local) / 0.08).clamp(0.0, 1.0));
    let envelope = attack * decay * release;
    let phase = local * frequency * std::f32::consts::TAU;
    let fundamental = phase.sin();
    let overtone = (phase * 2.01).sin() * 0.13;
    let airy = (phase * 3.02).sin() * 0.045;
    (fundamental + overtone + airy) * envelope * gain
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

#[cfg(target_os = "windows")]
struct SingleInstanceGuard(windows_sys::Win32::Foundation::HANDLE);

#[cfg(target_os = "windows")]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(target_os = "windows")]
fn acquire_single_instance() -> Option<SingleInstanceGuard> {
    let name: Vec<u16> = "Local\\TyprSingleInstance\0".encode_utf16().collect();
    unsafe {
        let handle = windows_sys::Win32::System::Threading::CreateMutexW(
            std::ptr::null_mut(),
            1,
            name.as_ptr(),
        );
        if handle.is_null() {
            return None;
        }
        if windows_sys::Win32::Foundation::GetLastError()
            == windows_sys::Win32::Foundation::ERROR_ALREADY_EXISTS
        {
            let _ = windows_sys::Win32::Foundation::CloseHandle(handle);
            return None;
        }
        Some(SingleInstanceGuard(handle))
    }
}

#[cfg(not(target_os = "windows"))]
struct SingleInstanceGuard;

#[cfg(not(target_os = "windows"))]
fn acquire_single_instance() -> Option<SingleInstanceGuard> {
    Some(SingleInstanceGuard)
}

struct AppState {
    settings: Mutex<V2Settings>,
    keyring: Box<dyn KeyringBackend>,
    app_dir: PathBuf,
    db: Db,
    audio: Mutex<AudioRecorder>,
    recording_state: Mutex<RecordingState>,
    active_pipeline_mode: Mutex<PipelineMode>,
    model_download_cancel: AtomicBool,
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

fn should_close_to_tray(app: &tauri::AppHandle) -> bool {
    app.state::<AppState>()
        .settings
        .lock()
        .map(|settings| settings.system.close_to_tray)
        .unwrap_or(true)
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, TRAY_SHOW_ID, "Show Veyra", true, None::<&str>)?;
    let hide = MenuItem::with_id(app, TRAY_HIDE_ID, "Hide Window", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let exit = MenuItem::with_id(app, TRAY_EXIT_ID, "Exit Veyra", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &hide, &separator, &exit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip("Veyra")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_SHOW_ID => show_main_window(app),
            TRAY_HIDE_ID => hide_main_window(app),
            TRAY_EXIT_ID => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            }
            | TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => show_main_window(tray.app_handle()),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    tray.build(app)?;
    Ok(())
}

fn get_app_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.typr.app")
}

/// Tauri-specific overlay UI side-effect. Lives here (not in the pipeline)
/// because the pipeline must remain headless and reusable from Phase 4
/// command mode without dragging the overlay window dependency along.
fn pipeline_mode_label(mode: PipelineMode) -> &'static str {
    match mode {
        PipelineMode::Dictation => "dictation",
        PipelineMode::Command => "command",
    }
}

fn emit_overlay_mode(app: &tauri::AppHandle, mode: PipelineMode) {
    let payload = serde_json::json!({ "mode": pipeline_mode_label(mode) });
    let _ = app.emit_to("overlay", "overlay:mode", payload.clone());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        let _ = app.emit_to(
            "overlay",
            "overlay:mode",
            serde_json::json!({ "mode": pipeline_mode_label(mode) }),
        );
    });
}

fn update_overlay(app: &tauri::AppHandle, state: &RecordingState) {
    play_transition_sound(state);
    if let Some(overlay) = app.get_webview_window("overlay") {
        match state {
            RecordingState::Ready => {
                let _ = app.emit_to("overlay", "overlay:state", state.clone());
                let _ = overlay.hide();
            }
            RecordingState::Recording | RecordingState::Transcribing => {
                if *state == RecordingState::Recording {
                    if let Some(position) = active_monitor_bottom_position() {
                        let _ = overlay.set_position(position);
                    }
                }
                let _ = overlay.show();
                let _ = app.emit_to("overlay", "overlay:state", state.clone());
                let app = app.clone();
                let state = state.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
                    let _ = app.emit_to("overlay", "overlay:state", state);
                });
            }
        }
    }
}

fn spawn_level_emitter(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let state = app.state::<AppState>();
            let recording = {
                let rs = state.recording_state.lock().unwrap();
                *rs == RecordingState::Recording
            };
            if !recording {
                break;
            }
            let level = {
                let audio = state.audio.lock().unwrap();
                audio.current_level()
            };
            let _ = app.emit_to(
                "overlay",
                "overlay:level",
                serde_json::json!({ "level": level }),
            );
            tokio::time::sleep(std::time::Duration::from_millis(45)).await;
        }
    });
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
fn window_minimize(app: tauri::AppHandle) -> Result<(), String> {
    app.get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?
        .minimize()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn window_toggle_maximize(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    if window.is_maximized().map_err(|e| e.to_string())? {
        window.unmaximize().map_err(|e| e.to_string())
    } else {
        window.maximize().map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn window_close(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;
    if should_close_to_tray(&app) {
        window.hide().map_err(|e| e.to_string())
    } else {
        window.close().map_err(|e| e.to_string())
    }
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
fn get_recording_level(state: State<AppState>) -> f32 {
    state.audio.lock().unwrap().current_level()
}

#[tauri::command]
fn check_model_downloaded(state: State<AppState>, model_size: String) -> Result<bool, String> {
    let model_file = transcribe_local::model_filename(&model_size)?;
    Ok(state.app_dir.join(&model_file).exists())
}

#[tauri::command]
async fn download_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    model_size: String,
) -> Result<(), String> {
    state.model_download_cancel.store(false, Ordering::SeqCst);
    let url = transcribe_local::model_download_url(&model_size)?;
    let model_file = transcribe_local::model_filename(&model_size)?;
    let dest = state.app_dir.join(&model_file);
    downloader::download_model(app, &model_size, &url, &dest, &state.model_download_cancel).await
}

#[tauri::command]
fn cancel_model_download(state: State<AppState>) {
    state.model_download_cancel.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn cancel_recording(app: tauri::AppHandle, state: State<AppState>) -> Result<(), String> {
    {
        let current = state.recording_state.lock().map_err(|e| e.to_string())?;
        if *current != RecordingState::Recording {
            return Ok(());
        }
    }
    state.audio.lock().map_err(|e| e.to_string())?.cancel();
    {
        let mut rs = state.recording_state.lock().map_err(|e| e.to_string())?;
        *rs = RecordingState::Ready;
    }
    let _ = app.emit("recording-state", RecordingState::Ready);
    update_overlay(&app, &RecordingState::Ready);
    Ok(())
}

#[tauri::command]
async fn toggle_recording(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    do_toggle_recording(&app, &state, PipelineMode::Dictation).await
}

/// Drive the pipeline once we are committed to the `Recording → Transcribing
/// → Ready` transition. Used by both the toggle command and the PTT release
/// branch so emit/overlay/state housekeeping stays in lock-step.
///
/// Lock discipline: `recording_state` and `settings` are `std::sync::Mutex`,
/// so every guard is acquired in a `{}` scope and dropped before any `.await`.
/// `pipeline::run_session` re-locks `state.audio` internally inside
/// `capture::stop_and_save`, so we must NOT hold any lock around that call.
async fn run_pipeline_and_reset_state(
    app: &tauri::AppHandle,
    state: &AppState,
    mode: PipelineMode,
) {
    {
        let mut rs = state.recording_state.lock().unwrap();
        *rs = RecordingState::Transcribing;
    }
    emit_overlay_mode(app, mode);
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
        typr_lib::pipeline::run_session(deps, mode).await
    };

    {
        let mut rs = state.recording_state.lock().unwrap();
        *rs = RecordingState::Ready;
    }
    let _ = app.emit("recording-state", RecordingState::Ready);
    update_overlay(app, &RecordingState::Ready);

    match outcome {
        Ok(row_id) => {
            let _ = app.emit("transcription:new", serde_json::json!({ "rowId": row_id }));
            tracing::info!(row_id, mode = ?mode, "[Typr] Transcription persisted");
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
    mode: PipelineMode,
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
            state.audio.lock().map_err(|e| e.to_string())?.start(&mic)?;

            {
                let mut rs = state.recording_state.lock().map_err(|e| e.to_string())?;
                *rs = RecordingState::Recording;
            }
            {
                let mut active_mode = state
                    .active_pipeline_mode
                    .lock()
                    .map_err(|e| e.to_string())?;
                *active_mode = mode;
            }
            emit_overlay_mode(app, mode);
            let _ = app.emit("recording-state", RecordingState::Recording);
            update_overlay(app, &RecordingState::Recording);
            spawn_level_emitter(app.clone());
            Ok("recording".into())
        }
        RecordingState::Recording => {
            let active_mode = {
                let active_mode = state
                    .active_pipeline_mode
                    .lock()
                    .map_err(|e| e.to_string())?;
                *active_mode
            };
            run_pipeline_and_reset_state(app, state, active_mode).await;
            Ok("ok".into())
        }
        RecordingState::Transcribing => Err("Currently transcribing, please wait".into()),
    }
}

fn register_recording_shortcut(
    app: &mut tauri::App,
    hotkey: &str,
    pipeline_mode: PipelineMode,
    label: &'static str,
) {
    let hotkey = hotkey.trim();
    if hotkey.is_empty() {
        tracing::warn!(label, "[Typr] Skipping empty global shortcut");
        return;
    }

    let handle = app.handle().clone();
    tracing::info!(hotkey, label, "[Typr] Registering global shortcut");

    match app.global_shortcut().on_shortcut(
        hotkey,
        move |_app, shortcut, event| {
            tracing::debug!(
                ?shortcut,
                state = ?event.state,
                mode = ?pipeline_mode,
                "[Typr] Hotkey event"
            );
            let handle = handle.clone();
            let state = handle.state::<AppState>();
            let recording_mode = state.settings.lock().unwrap().hotkeys.recording_mode.clone();

            match event.state {
                ShortcutState::Pressed => {
                    tauri::async_runtime::spawn(async move {
                        let state = handle.state::<AppState>();
                        match recording_mode.as_str() {
                            "toggle" => {
                                match do_toggle_recording(&handle, state.inner(), pipeline_mode).await {
                                    Ok(result) => tracing::info!(result = %result, mode = ?pipeline_mode, "[Typr] Toggle result"),
                                    Err(e) => tracing::error!(error = %e, mode = ?pipeline_mode, "[Typr] Toggle error"),
                                }
                            }
                            "push-to-talk" => {
                                let current = {
                                    let rs = state.recording_state.lock().unwrap();
                                    rs.clone()
                                };
                                if current == RecordingState::Ready {
                                    let mic = state.settings.lock().unwrap().microphone.clone();
                                    let start_res = state.audio.lock().unwrap().start(&mic);
                                    match start_res {
                                        Ok(_) => {
                                            {
                                                let mut rs = state.recording_state.lock().unwrap();
                                                *rs = RecordingState::Recording;
                                            }
                                            {
                                                let mut active_mode = state.active_pipeline_mode.lock().unwrap();
                                                *active_mode = pipeline_mode;
                                            }
                                            emit_overlay_mode(&handle, pipeline_mode);
                                            let _ = handle.emit("recording-state", RecordingState::Recording);
                                            update_overlay(&handle, &RecordingState::Recording);
                                            spawn_level_emitter(handle.clone());
                                            tracing::info!(mode = ?pipeline_mode, "[Typr] Recording started");
                                        }
                                        Err(e) => tracing::error!(error = %e, mode = ?pipeline_mode, "[Typr] Start recording error"),
                                    }
                                }
                            }
                            _ => {}
                        }
                    });
                }
                ShortcutState::Released => {
                    if recording_mode == "push-to-talk" {
                        tauri::async_runtime::spawn(async move {
                            let state = handle.state::<AppState>();
                            let current = {
                                let rs = state.recording_state.lock().unwrap();
                                rs.clone()
                            };
                            let active_mode = {
                                let active_mode = state.active_pipeline_mode.lock().unwrap();
                                *active_mode
                            };
                            if current == RecordingState::Recording && active_mode == pipeline_mode {
                                run_pipeline_and_reset_state(&handle, state.inner(), pipeline_mode).await;
                            }
                        });
                    }
                }
            }
        },
    ) {
        Ok(_) => tracing::info!(hotkey, label, "[Typr] Global shortcut registered"),
        Err(e) => tracing::error!(error = %e, hotkey, label, "[Typr] Failed to register global shortcut"),
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
    DictionaryRepo::new(&state.db)
        .list()
        .map_err(|e| e.to_string())
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
    SnippetRepo::new(&state.db)
        .list()
        .map_err(|e| e.to_string())
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
    StatsRepo::new(&state.db)
        .totals()
        .map_err(|e| e.to_string())
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

#[tauri::command]
async fn check_email_draft_model(key: String, engine: String, model: String) -> Result<(), String> {
    typr_lib::draft_email::check_email_draft_model(&key, &engine, &model).await
}

#[tauri::command]
async fn download_email_draft_model(engine: String, model: String) -> Result<(), String> {
    typr_lib::draft_email::download_email_draft_model(&engine, &model).await
}

fn main() {
    let Some(_single_instance) = acquire_single_instance() else {
        return;
    };

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

    let mut v2_settings = outcome.settings.clone();
    let mut settings_changed = false;
    if v2_settings.hotkeys.command_mode == "Shift+F24" {
        v2_settings.hotkeys.command_mode = "F12".to_string();
        settings_changed = true;
    }
    let email_engine = v2_settings.transcription.email_draft_engine.as_str();
    let email_model = v2_settings.transcription.email_draft_model.as_str();
    let email_model_valid = match email_engine {
        "groq" => typr_lib::draft_email::ALLOWED_GROQ_DRAFT_MODELS.contains(&email_model),
        "ollama" => typr_lib::draft_email::ALLOWED_OLLAMA_DRAFT_MODELS.contains(&email_model),
        _ => false,
    };
    if !email_model_valid {
        v2_settings.transcription.email_draft_engine = "ollama".to_string();
        v2_settings.transcription.email_draft_model =
            typr_lib::draft_email::DEFAULT_OLLAMA_DRAFT_MODEL.to_string();
        settings_changed = true;
    }
    if settings_changed {
        if let Err(e) = settings::save(&app_dir, &v2_settings) {
            tracing::warn!(error = %e, "[Typr] Failed to save boot-normalized settings");
        }
    }
    let dictation_hotkey = v2_settings.hotkeys.dictation.clone();
    let command_hotkey = v2_settings.hotkeys.command_mode.clone();
    let migration_events = outcome.events;

    // Boot-time tmp sweep: any *.wav / *.txt left in the per-session tmp dir
    // older than 10 minutes is junk from a crashed prior run. Best-effort —
    // failure modes are logged inside `sweep_stale_wavs` and a zero count
    // here is the common case.
    let purged = typr_lib::pipeline::tmp::sweep_stale_wavs(std::time::Duration::from_secs(600));
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
            active_pipeline_mode: Mutex::new(PipelineMode::Dictation),
            model_download_cancel: AtomicBool::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            // Phase 1+2 (existing)
            window_minimize,
            window_toggle_maximize,
            window_close,
            get_settings,
            save_settings,
            list_microphones,
            get_recording_state,
            get_recording_level,
            check_model_downloaded,
            download_model,
            cancel_model_download,
            cancel_recording,
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
            check_email_draft_model,
            download_email_draft_model,
        ])
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            let WindowEvent::CloseRequested { api, .. } = event else {
                return;
            };

            let close_to_tray = window
                .app_handle()
                .state::<AppState>()
                .settings
                .lock()
                .map(|settings| settings.system.close_to_tray)
                .unwrap_or(true);

            if close_to_tray {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(move |app| {
            setup_tray(app)?;

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

            // Create the overlay window hidden by default. It appears only while
            // recording/transcribing, Wispr Flow style.
            let monitor = app.primary_monitor().ok().flatten();
            let (x, y) = if let Some(m) = monitor {
                let size = m.size();
                let scale = m.scale_factor();
                let logical_w = size.width as f64 / scale;
                let logical_h = size.height as f64 / scale;
                (
                    (logical_w / 2.0 - (OVERLAY_WIDTH as f64 / 2.0)) as i32,
                    (logical_h - 110.0) as i32,
                )
            } else {
                (640, 720)
            };

            let overlay = WebviewWindowBuilder::new(
                app,
                "overlay",
                WebviewUrl::App("src/overlay.html".into()),
            )
            .title("")
            .inner_size(OVERLAY_WIDTH as f64, OVERLAY_HEIGHT as f64)
            .position(x as f64, y as f64)
            .resizable(false)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .focused(false)
            .shadow(false)
            .visible(false)
            .build();

            match overlay {
                Ok(_) => tracing::info!("[Typr] Overlay window created"),
                Err(e) => tracing::error!(error = %e, "[Typr] Failed to create overlay"),
            }

            register_recording_shortcut(
                app,
                &dictation_hotkey,
                PipelineMode::Dictation,
                "dictation",
            );
            if command_hotkey.trim().eq_ignore_ascii_case(dictation_hotkey.trim()) {
                tracing::warn!(
                    hotkey = %command_hotkey,
                    "[Typr] Command hotkey matches dictation hotkey; command mode not registered"
                );
            } else {
                register_recording_shortcut(app, &command_hotkey, PipelineMode::Command, "command");
            }

            tracing::info!(stage = "boot", "storage + telemetry + settings initialised");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
