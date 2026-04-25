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

use tauri::Emitter;
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

            let backend = crate::settings::keyring::SystemBackend;
            let outcome = crate::settings::load(&app_dir, &db, &backend)
                .expect("load settings");

            for ev in &outcome.events {
                match ev {
                    crate::settings::MigrationEvent::Migrated => {
                        tracing::info!(stage = "settings", "migrated v1 → v2");
                        let _ = app.emit("settings:migrated", ());
                    }
                    crate::settings::MigrationEvent::ModelRemapped { from, to } => {
                        tracing::info!(stage = "settings", %from, %to, "whisper model remapped");
                        let _ = app.emit("settings:model-remapped", serde_json::json!({ "from": from, "to": to }));
                    }
                    crate::settings::MigrationEvent::NeedsGroqKey => {
                        tracing::info!(stage = "settings", "legacy Groq key — prompting for re-entry");
                        let _ = app.emit("settings:needs-groq-key", ());
                    }
                    crate::settings::MigrationEvent::UnknownVersion(v) => {
                        tracing::warn!(stage = "settings", version = v, "unknown settings version; using defaults");
                        let _ = app.emit("settings:unknown-version", *v);
                    }
                    crate::settings::MigrationEvent::MigrationFailed(msg) => {
                        tracing::error!(stage = "settings", error = %msg, "migration failed; .bak preserved");
                        let _ = app.emit("settings:migration-failed", msg.clone());
                    }
                }
            }

            app.manage(db);
            app.manage(outcome.settings);

            tracing::info!(stage = "boot", "storage + telemetry + settings initialised");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
