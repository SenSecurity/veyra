#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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

// Glacier overlay bounding-box table. The visible chrome is rendered
// by CSS; the OS window is transparent + frameless + always-on-top,
// so these constants only define the size and bottom-margin of the
// transparent webview that hosts the React overlay tree.
//
// Capsule: compact horizontal pill, sized closer to Wispr Flow than the
// earlier full-width Glacier mockup. Window includes small shadow/hint slack.
//
// Halo Orb: transparent box sized to the largest concentric ring plus optional
// chip/hint clearance. The "smaller" orb is intentionally just a tiny orb.
const OVERLAY_BOTTOM_MARGIN: i32 = 12;
const OVERLAY_EDGE_MARGIN: i32 = 18;

/// Returns (width, height) for the (style, size) pair, falling back to
/// capsule + medium for unknown values. Mirrors SIZE_SPECS in
/// `src/overlay/halo-orb.tsx` and the capsule width ladder in
/// `src/overlay/pill.tsx`. Keep these in lock-step.
pub fn overlay_dims(style: &str, size: &str) -> (i32, i32) {
    match (style, size) {
        ("capsule", "smaller") => (212, 54),
        ("capsule", "small") => (244, 56),
        ("capsule", "medium") => (292, 60),
        ("capsule", "large") => (352, 64),
        ("orb", "smaller") => (92, 92),
        ("orb", "small") => (112, 124),
        ("orb", "medium") => (140, 152),
        ("orb", "large") => (172, 184),
        // Fallback: capsule + medium.
        _ => (292, 60),
    }
}

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
fn active_monitor_position(
    width: i32,
    height: i32,
    placement: &str,
) -> Option<tauri::PhysicalPosition<i32>> {
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

    unsafe fn position_for_screen_point(
        point: POINT,
        width: i32,
        height: i32,
        placement: &str,
    ) -> Option<tauri::PhysicalPosition<i32>> {
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
        let work_width = work.right - work.left;
        let work_height = work.bottom - work.top;
        let x = if placement.ends_with("-left") {
            work.left + OVERLAY_EDGE_MARGIN
        } else if placement.ends_with("-right") {
            work.right - OVERLAY_EDGE_MARGIN - width
        } else {
            work.left + ((work_width - width) / 2)
        };
        let y = if placement.starts_with("top-") {
            work.top + OVERLAY_EDGE_MARGIN
        } else if placement.starts_with("center-") || placement == "center" {
            work.top + ((work_height - height) / 2)
        } else {
            work.bottom - OVERLAY_BOTTOM_MARGIN - height
        };
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
        position_for_screen_point(point, width, height, placement)
    }
}

#[cfg(not(target_os = "windows"))]
fn active_monitor_position(
    _width: i32,
    _height: i32,
    _placement: &str,
) -> Option<tauri::PhysicalPosition<i32>> {
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
    overlay_layout_revision: AtomicU64,
    overlay_layout: Mutex<OverlayLayoutPayload>,
    overlay_preview_generation: AtomicU64,
}

#[derive(Debug, Clone, Serialize)]
struct OverlayLayoutPayload {
    style: String,
    size: String,
    revision: u64,
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

fn parse_overlay_mode(mode: &str) -> Result<PipelineMode, String> {
    match mode {
        "dictation" => Ok(PipelineMode::Dictation),
        "command" => Ok(PipelineMode::Command),
        other => Err(format!("Unsupported overlay preview mode `{other}`")),
    }
}

fn parse_preview_recording_state(state: &str) -> Result<RecordingState, String> {
    match state {
        "Recording" => Ok(RecordingState::Recording),
        "Transcribing" => Ok(RecordingState::Transcribing),
        other => Err(format!("Unsupported overlay preview state `{other}`")),
    }
}

fn preview_level_at(tick: u64) -> f32 {
    const LEVELS: [f32; 16] = [
        0.10, 0.34, 0.58, 0.42, 0.72, 0.28, 0.49, 0.84, 0.38, 0.64, 0.18, 0.52, 0.76, 0.31, 0.46,
        0.68,
    ];
    LEVELS[(tick as usize) % LEVELS.len()]
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
                let (style, size) = current_overlay_layout(app);
                apply_overlay_layout(app, &style, &size);
                if *state == RecordingState::Recording {
                    let (w, h) = current_overlay_dims(app);
                    if let Some(position) = active_monitor_overlay_position(app, w, h) {
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

/// Look up current overlay dimensions from the AppState's settings snapshot.
fn current_overlay_dims(app: &tauri::AppHandle) -> (i32, i32) {
    let state: tauri::State<AppState> = app.state::<AppState>();
    let s = state.settings.lock().unwrap();
    overlay_dims(&s.overlay.style, &s.overlay.size)
}

fn current_overlay_layout(app: &tauri::AppHandle) -> (String, String) {
    let state: tauri::State<AppState> = app.state::<AppState>();
    let s = state.settings.lock().unwrap();
    (s.overlay.style.clone(), s.overlay.size.clone())
}

fn current_overlay_position(app: &tauri::AppHandle) -> String {
    let state: tauri::State<AppState> = app.state::<AppState>();
    let s = state.settings.lock().unwrap();
    normalize_overlay_position(&s.overlay.position)
}

fn normalize_overlay_position(value: &str) -> String {
    match value {
        "top-left" | "top-center" | "top-right" | "center-left" | "center" | "center-right"
        | "bottom-left" | "bottom-center" | "bottom-right" => value.to_string(),
        "near-cursor" | "" => "bottom-center".to_string(),
        _ => "bottom-center".to_string(),
    }
}

fn active_monitor_overlay_position(
    app: &tauri::AppHandle,
    width: i32,
    height: i32,
) -> Option<tauri::PhysicalPosition<i32>> {
    let placement = current_overlay_position(app);
    active_monitor_position(width, height, &placement)
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
fn save_settings(
    app: tauri::AppHandle,
    state: State<AppState>,
    settings: V1Settings,
) -> Result<(), String> {
    let layout_changed: bool;
    let new_style: String;
    let new_size: String;
    {
        let mut v2 = state.settings.lock().unwrap();
        let snapshot = v2.clone();

        layout_changed = snapshot.overlay.style != settings.overlay_style
            || snapshot.overlay.size != settings.overlay_size
            || normalize_overlay_position(&snapshot.overlay.position)
                != normalize_overlay_position(&settings.overlay_position);
        new_style = settings.overlay_style.clone();
        new_size = settings.overlay_size.clone();

        // Step 1: mutate v2 in-memory only — keyring untouched.
        apply_v1_v2_fields(&mut v2, &settings);

        // Step 2: write JSON. On failure, roll back the in-memory tree so we
        // don't drift from disk. No OS-side state has moved yet.
        if let Err(e) = settings::save(&state.app_dir, &v2) {
            *v2 = snapshot;
            return Err(format!("settings save failed: {e}"));
        }
    }

    // Step 3: write keyring. If this fails, JSON + in-memory already reflect
    // the payload but the keyring lags. Surface the error so the frontend can
    // retry; full atomicity needs Phase 2's payload reshape (Option<String>
    // for "unchanged" vs "cleared" so a failed read can't round-trip an empty
    // sentinel back through save).
    write_v1_keyring(&settings, state.keyring.as_ref())
        .map_err(|e| format!("keyring write failed: {e}"))?;

    // Step 4: if the overlay layout changed, resize the OS window now so the
    // user sees the new dimensions on the next dictation without restarting.
    if layout_changed {
        apply_overlay_layout(&app, &new_style, &new_size);
    }
    Ok(())
}

/// Apply the configured overlay (style, size) to the live overlay window:
/// resize via `set_size`, anchor against the configured work-area position,
/// and emit `overlay:layout` so the overlay webview's React tree switches
/// components without waiting for an unrelated settings refresh. The
/// overlay webview has its own zustand store, isolated from the main
/// window — without this event the React tree would render the previous
/// style inside the freshly-resized OS window.
fn apply_overlay_layout(app: &tauri::AppHandle, style: &str, size: &str) -> u64 {
    let (w, h) = overlay_dims(style, size);
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.set_size(tauri::LogicalSize::new(w as f64, h as f64));
        if let Some(position) = active_monitor_overlay_position(app, w, h) {
            let _ = overlay.set_position(position);
        }
    }
    let revision = app
        .try_state::<AppState>()
        .map(|state| {
            let revision = state.overlay_layout_revision.fetch_add(1, Ordering::SeqCst) + 1;
            if let Ok(mut layout) = state.overlay_layout.lock() {
                *layout = OverlayLayoutPayload {
                    style: style.to_string(),
                    size: size.to_string(),
                    revision,
                };
            }
            revision
        })
        .unwrap_or(0);
    let payload = OverlayLayoutPayload {
        style: style.to_string(),
        size: size.to_string(),
        revision,
    };
    let _ = app.emit_to("overlay", "overlay:layout", payload.clone());

    let app_for_retry = app.clone();
    let payload_for_retry = payload.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        let current_revision = app_for_retry
            .try_state::<AppState>()
            .map(|state| state.overlay_layout_revision.load(Ordering::SeqCst))
            .unwrap_or(revision);
        if current_revision == revision {
            let _ = app_for_retry.emit_to("overlay", "overlay:layout", payload_for_retry);
        }
    });
    revision
}

#[tauri::command]
fn get_overlay_layout(state: State<AppState>) -> OverlayLayoutPayload {
    state
        .overlay_layout
        .lock()
        .map(|layout| layout.clone())
        .unwrap_or_else(|_| OverlayLayoutPayload {
            style: "capsule".to_string(),
            size: "medium".to_string(),
            revision: 0,
        })
}

#[tauri::command]
fn set_overlay_layout(app: tauri::AppHandle, style: String, size: String) -> Result<(), String> {
    apply_overlay_layout(&app, &style, &size);
    Ok(())
}

#[tauri::command]
fn preview_overlay(
    app: tauri::AppHandle,
    state: State<AppState>,
    style: String,
    size: String,
    mode: String,
    recording_state: String,
) -> Result<(), String> {
    let mode = parse_overlay_mode(&mode)?;
    let recording_state = parse_preview_recording_state(&recording_state)?;
    let generation = state
        .overlay_preview_generation
        .fetch_add(1, Ordering::SeqCst)
        + 1;

    let layout_revision = apply_overlay_layout(&app, &style, &size);
    let (w, h) = overlay_dims(&style, &size);
    if let Some(overlay) = app.get_webview_window("overlay") {
        if let Some(position) = active_monitor_overlay_position(&app, w, h) {
            let _ = overlay.set_position(position);
        }
        let _ = overlay.show();
    }

    let payload = serde_json::json!({
        "active": true,
        "mode": pipeline_mode_label(mode),
        "state": recording_state,
        "style": style,
        "size": size,
        "revision": layout_revision,
    });
    let _ = app.emit_to("overlay", "overlay:preview", payload.clone());

    let app_for_preview_retry = app.clone();
    let payload_for_preview_retry = payload.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(180)).await;
        let current_generation = app_for_preview_retry
            .state::<AppState>()
            .overlay_preview_generation
            .load(Ordering::SeqCst);
        if current_generation == generation {
            let _ = app_for_preview_retry.emit_to(
                "overlay",
                "overlay:preview",
                payload_for_preview_retry,
            );
        }
    });

    let app_for_task = app.clone();
    tauri::async_runtime::spawn(async move {
        for tick in 0..40_u64 {
            let current_generation = app_for_task
                .state::<AppState>()
                .overlay_preview_generation
                .load(Ordering::SeqCst);
            if current_generation != generation {
                return;
            }

            if recording_state == RecordingState::Recording {
                let _ = app_for_task.emit_to(
                    "overlay",
                    "overlay:level",
                    serde_json::json!({ "level": preview_level_at(tick) }),
                );
            }

            tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        }

        let state = app_for_task.state::<AppState>();
        if state.overlay_preview_generation.load(Ordering::SeqCst) == generation {
            state
                .overlay_preview_generation
                .fetch_add(1, Ordering::SeqCst);
            let _ = app_for_task.emit_to(
                "overlay",
                "overlay:preview",
                serde_json::json!({ "active": false }),
            );
            let ready = state
                .recording_state
                .lock()
                .map(|recording_state| *recording_state == RecordingState::Ready)
                .unwrap_or(false);
            if ready {
                if let Some(overlay) = app_for_task.get_webview_window("overlay") {
                    let _ = overlay.hide();
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
fn hide_overlay_preview(app: tauri::AppHandle, state: State<AppState>) -> Result<(), String> {
    state
        .overlay_preview_generation
        .fetch_add(1, Ordering::SeqCst);
    let _ = app.emit_to(
        "overlay",
        "overlay:preview",
        serde_json::json!({ "active": false }),
    );
    let ready = state
        .recording_state
        .lock()
        .map_err(|e| e.to_string())
        .map(|recording_state| *recording_state == RecordingState::Ready)?;
    if ready {
        if let Some(overlay) = app.get_webview_window("overlay") {
            let _ = overlay.hide();
        }
    }
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
fn get_recording_mode(state: State<AppState>) -> String {
    let mode = state.active_pipeline_mode.lock().unwrap();
    pipeline_mode_label(*mode).to_string()
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

fn wizard_completed(state: &AppState) -> bool {
    AppMetaRepo::new(&state.db)
        .get("wizard_completed")
        .ok()
        .flatten()
        .as_deref()
        == Some("1")
}

fn ensure_wizard_completed(state: &AppState) -> Result<(), String> {
    if wizard_completed(state) {
        Ok(())
    } else {
        Err("Finish first boot setup before recording.".to_string())
    }
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

    let session_timeout = match mode {
        PipelineMode::Dictation => std::time::Duration::from_secs(240),
        PipelineMode::Command => std::time::Duration::from_secs(35),
    };

    let outcome = match tokio::time::timeout(session_timeout, async {
        let deps = typr_lib::pipeline::PipelineDeps {
            db: &state.db,
            settings: &settings,
            audio: &state.audio,
            app,
            app_dir: &state.app_dir,
            groq_key: groq_key.as_deref(),
        };
        typr_lib::pipeline::run_session(deps, mode).await
    })
    .await
    {
        Ok(outcome) => outcome,
        Err(_) => Err(typr_lib::pipeline::PipelineError {
            stage: match mode {
                PipelineMode::Dictation => typr_lib::pipeline::StageError::Transcribe(format!(
                    "session timed out after {} seconds",
                    session_timeout.as_secs()
                )),
                PipelineMode::Command => typr_lib::pipeline::StageError::Draft(format!(
                    "session timed out after {} seconds",
                    session_timeout.as_secs()
                )),
            },
        }),
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
    ensure_wizard_completed(state)?;
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
) -> bool {
    let hotkey = hotkey.trim();
    if hotkey.is_empty() {
        tracing::warn!(label, "[Typr] Skipping empty global shortcut");
        return false;
    }

    let handle = app.handle().clone();
    tracing::info!(hotkey, label, "[Typr] Registering global shortcut");

    match app
        .global_shortcut()
        .on_shortcut(hotkey, move |_app, shortcut, event| {
            tracing::debug!(
                ?shortcut,
                state = ?event.state,
                mode = ?pipeline_mode,
                "[Typr] Hotkey event"
            );
            handle_recording_hotkey_event(handle.clone(), pipeline_mode, event.state);
        }) {
        Ok(_) => {
            tracing::info!(hotkey, label, "[Typr] Global shortcut registered");
            true
        }
        Err(e) => {
            tracing::error!(error = %e, hotkey, label, "[Typr] Failed to register global shortcut");
            false
        }
    }
}

fn handle_recording_hotkey_event(
    handle: tauri::AppHandle,
    pipeline_mode: PipelineMode,
    shortcut_state: ShortcutState,
) {
    let state = handle.state::<AppState>();
    let recording_mode = state
        .settings
        .lock()
        .unwrap()
        .hotkeys
        .recording_mode
        .clone();

    match shortcut_state {
        ShortcutState::Pressed => {
            tauri::async_runtime::spawn(async move {
                let state = handle.state::<AppState>();
                match recording_mode.as_str() {
                    "toggle" => {
                        match do_toggle_recording(&handle, state.inner(), pipeline_mode).await {
                            Ok(result) => {
                                tracing::info!(result = %result, mode = ?pipeline_mode, "[Typr] Toggle result")
                            }
                            Err(e) => {
                                tracing::error!(error = %e, mode = ?pipeline_mode, "[Typr] Toggle error")
                            }
                        }
                    }
                    "push-to-talk" => {
                        if let Err(error) = ensure_wizard_completed(state.inner()) {
                            tracing::warn!(%error, "[Typr] Ignoring hotkey before first boot setup");
                            return;
                        }
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
                                        let mut active_mode =
                                            state.active_pipeline_mode.lock().unwrap();
                                        *active_mode = pipeline_mode;
                                    }
                                    emit_overlay_mode(&handle, pipeline_mode);
                                    let _ =
                                        handle.emit("recording-state", RecordingState::Recording);
                                    update_overlay(&handle, &RecordingState::Recording);
                                    spawn_level_emitter(handle.clone());
                                    tracing::info!(mode = ?pipeline_mode, "[Typr] Recording started");
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, mode = ?pipeline_mode, "[Typr] Start recording error")
                                }
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
fn list_email_drafts(
    state: State<AppState>,
    limit: u32,
    offset: u32,
) -> Result<Vec<Transcription>, String> {
    TranscriptionRepo::new(&state.db)
        .list_by_mode("command", limit as i64, offset as i64)
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
async fn download_email_draft_model(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    engine: String,
    model: String,
) -> Result<(), String> {
    typr_lib::draft_email::download_email_draft_model(
        Some(app),
        Some(&state.app_dir),
        &engine,
        &model,
    )
    .await
}

/// Cheap synchronous probe: is the Ollama CLI present on the user's
/// machine? Used by the first-boot wizard to render the right CTA
/// without invoking the heavier `check_email_draft_model` (which also
/// triggers a model pull). Mirrors the file-existence checks in
/// `src-tauri/windows/hooks.nsh`.
#[tauri::command]
fn is_ollama_installed() -> bool {
    let candidates: Vec<std::path::PathBuf> = ollama_candidate_paths();
    if candidates.iter().any(|p| p.is_file()) {
        return true;
    }
    // PATH lookup as final fallback.
    if let Ok(path) = std::env::var("PATH") {
        let exe = if cfg!(target_os = "windows") {
            "ollama.exe"
        } else {
            "ollama"
        };
        for dir in std::env::split_paths(&path) {
            if dir.join(exe).is_file() {
                return true;
            }
        }
    }
    false
}

#[cfg(target_os = "windows")]
fn ollama_candidate_paths() -> Vec<std::path::PathBuf> {
    let mut out: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        out.push(std::path::PathBuf::from(local).join("Programs/Ollama/ollama.exe"));
    }
    if let Ok(pf) = std::env::var("ProgramFiles") {
        out.push(std::path::PathBuf::from(pf).join("Ollama/ollama.exe"));
    }
    if let Ok(pf86) = std::env::var("ProgramFiles(x86)") {
        out.push(std::path::PathBuf::from(pf86).join("Ollama/ollama.exe"));
    }
    out
}

#[cfg(target_os = "macos")]
fn ollama_candidate_paths() -> Vec<std::path::PathBuf> {
    vec![
        std::path::PathBuf::from("/Applications/Ollama.app/Contents/Resources/ollama"),
        std::path::PathBuf::from("/usr/local/bin/ollama"),
        std::path::PathBuf::from("/opt/homebrew/bin/ollama"),
    ]
}

#[cfg(all(unix, not(target_os = "macos")))]
fn ollama_candidate_paths() -> Vec<std::path::PathBuf> {
    vec![
        std::path::PathBuf::from("/usr/local/bin/ollama"),
        std::path::PathBuf::from("/usr/bin/ollama"),
        std::path::PathBuf::from("/opt/ollama/ollama"),
    ]
}

#[tauri::command]
fn open_ollama_download() -> Result<(), String> {
    let url = "https://ollama.com/download";
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .status();
    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("open").arg(url).status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let status = std::process::Command::new("xdg-open").arg(url).status();

    status
        .map_err(|e| format!("Failed to open Ollama download page: {e}"))
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err(format!("Failed to open Ollama download page: {status}"))
            }
        })
}

#[tauri::command]
async fn install_ollama_runtime() -> Result<(), String> {
    if is_ollama_installed() {
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        install_ollama_runtime_windows().await
    }

    #[cfg(not(target_os = "windows"))]
    {
        open_ollama_download()
    }
}

#[cfg(target_os = "windows")]
async fn install_ollama_runtime_windows() -> Result<(), String> {
    let installer_path = std::env::temp_dir().join("Veyra-OllamaSetup.exe");
    let has_cached_installer = installer_path
        .metadata()
        .map(|m| m.len() > 1_000_000)
        .unwrap_or(false);

    if !has_cached_installer {
        let url = "https://ollama.com/download/OllamaSetup.exe";
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("Failed to prepare Ollama installer download: {e}"))?;
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to download Ollama installer: {e}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "Failed to download Ollama installer: HTTP {}",
                response.status()
            ));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read Ollama installer download: {e}"))?;
        if bytes.len() < 1_000_000 {
            return Err("Downloaded Ollama installer is unexpectedly small".to_string());
        }
        std::fs::write(&installer_path, &bytes)
            .map_err(|e| format!("Failed to save Ollama installer: {e}"))?;
    }

    let mut child = tokio::process::Command::new(&installer_path)
        .args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"])
        .spawn()
        .map_err(|e| format!("Failed to start silent Ollama installer: {e}"))?;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5 * 60);
    loop {
        if is_ollama_installed() {
            return Ok(());
        }

        match child
            .try_wait()
            .map_err(|e| format!("Failed to poll silent Ollama installer: {e}"))?
        {
            Some(status) => {
                if is_ollama_installed() {
                    return Ok(());
                }
                tracing::warn!(%status, "silent Ollama installer exited without installed runtime");
                break;
            }
            None if std::time::Instant::now() >= deadline => {
                tracing::warn!(
                    "silent Ollama installer is still running; wizard will keep polling"
                );
                return Ok(());
            }
            None => {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
    }

    std::process::Command::new(&installer_path)
        .spawn()
        .map_err(|e| format!("Failed to launch Ollama installer fallback: {e}"))?;
    Ok(())
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
    let command_mode_hotkey = v2_settings.hotkeys.command_mode.trim();
    if command_mode_hotkey.is_empty()
        || matches!(
            command_mode_hotkey,
            "Shift+F24" | "F12" | "Shift+F12" | "Ctrl+Alt+M" | "Ctrl+Alt+D"
        )
    {
        v2_settings.hotkeys.command_mode = "Pause".to_string();
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
    let initial_overlay_layout = OverlayLayoutPayload {
        style: v2_settings.overlay.style.clone(),
        size: v2_settings.overlay.size.clone(),
        revision: 0,
    };
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
            overlay_layout_revision: AtomicU64::new(0),
            overlay_layout: Mutex::new(initial_overlay_layout),
            overlay_preview_generation: AtomicU64::new(0),
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
            get_recording_mode,
            check_model_downloaded,
            download_model,
            cancel_model_download,
            cancel_recording,
            toggle_recording,
            get_overlay_layout,
            set_overlay_layout,
            preview_overlay,
            hide_overlay_preview,
            // Phase 3: transcriptions
            list_transcriptions,
            list_email_drafts,
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
            open_ollama_download,
            is_ollama_installed,
            install_ollama_runtime,
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
            // recording/transcribing, Wispr Flow style. Dimensions come from
            // the persisted overlay style/size and may be resized later via
            // the `set_overlay_layout` Tauri command.
            let (overlay_w, overlay_h) = current_overlay_dims(app.handle());
            let (x, y) = active_monitor_overlay_position(app.handle(), overlay_w, overlay_h)
                .map(|pos| (pos.x, pos.y))
                .unwrap_or((640, 720));

            let overlay = WebviewWindowBuilder::new(
                app,
                "overlay",
                WebviewUrl::App("src/overlay.html".into()),
            )
            .title("")
            .inner_size(overlay_w as f64, overlay_h as f64)
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

            let _ = register_recording_shortcut(
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
                let _ =
                    register_recording_shortcut(app, &command_hotkey, PipelineMode::Command, "command");
            }

            tracing::info!(stage = "boot", "storage + telemetry + settings initialised");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_mode_parser_accepts_supported_modes_only() {
        assert_eq!(parse_overlay_mode("dictation"), Ok(PipelineMode::Dictation));
        assert_eq!(parse_overlay_mode("command"), Ok(PipelineMode::Command));
        assert!(parse_overlay_mode("../dictation").is_err());
    }

    #[test]
    fn preview_state_parser_accepts_recording_and_transcribing_only() {
        assert_eq!(
            parse_preview_recording_state("Recording"),
            Ok(RecordingState::Recording)
        );
        assert_eq!(
            parse_preview_recording_state("Transcribing"),
            Ok(RecordingState::Transcribing)
        );
        assert!(parse_preview_recording_state("Ready").is_err());
    }

    #[test]
    fn preview_level_sequence_loops_inside_expected_bounds() {
        for tick in 0..64 {
            let level = preview_level_at(tick);
            assert!((0.0..=1.0).contains(&level));
        }

        assert_eq!(preview_level_at(0), preview_level_at(16));
    }

    #[test]
    fn halo_orb_overlay_dims_fit_full_orb_chrome() {
        assert_eq!(overlay_dims("orb", "smaller"), (92, 92));
        assert_eq!(overlay_dims("orb", "small"), (112, 124));
        assert_eq!(overlay_dims("orb", "medium"), (140, 152));
        assert_eq!(overlay_dims("orb", "large"), (172, 184));
    }
}
