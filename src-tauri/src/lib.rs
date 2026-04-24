pub mod telemetry;
pub mod storage;
pub mod settings;
pub mod audio;
pub mod transcribe_local;
pub mod transcribe_groq;
pub mod cleanup;
pub mod paste;
pub mod recorder;
pub mod downloader;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir().expect("app_data_dir");
            let log_dir = app_dir.join("logs");
            let _ = crate::telemetry::init_tracing(&log_dir);

            let db_path = app_dir.join("typr.db");
            let db = crate::storage::Db::open(&db_path)
                .expect("open typr.db");
            app.manage(db);

            tracing::info!(stage = "boot", "storage + telemetry initialised");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
