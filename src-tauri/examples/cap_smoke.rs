//! Smoke 1.7 helper: opens the production typr.db, inserts one fake
//! transcription via the real `commit_session` path, then prints the
//! resulting SUM(word_count). Run with `cargo run --release --example cap_smoke`.
//!
//! Requires typr.exe to be NOT running (SQLite WAL allows concurrent reads
//! but the writer here is single-connection and a live typr would race).

use std::path::PathBuf;

use typr_lib::pipeline::commit::{commit_session, TranscriptionRecord};
use typr_lib::settings::Settings;
use typr_lib::storage::Db;

fn main() {
    let app_dir: PathBuf = dirs::config_dir()
        .expect("no config dir")
        .join("com.typr.app");
    let db_path = app_dir.join("typr.db");
    let db = Db::open(&db_path).expect("open typr.db");

    // Build settings reflecting the on-disk config so cap_purge actually runs.
    // We can't reuse the migrator pipeline cheaply here; reading the JSON
    // directly is enough for the smoke check.
    let cfg_raw = std::fs::read_to_string(app_dir.join("config.json")).expect("read config.json");
    let settings: Settings = serde_json::from_str(&cfg_raw).expect("parse config");

    println!(
        "before: cap={} purge={} sum={}",
        settings.data.word_count_cap,
        settings.data.purge_on_exceed,
        sum(&db),
    );

    let record = TranscriptionRecord {
        raw_text: "smoke test row".into(),
        final_text: "smoke test row".into(),
        word_count: 3,
        duration_ms: 500,
        language: String::new(),
        engine: "local".into(),
        model: "smoke".into(),
        app_context: String::new(),
        mode: "dictation".into(),
        enhanced: false,
    };
    let row_id = commit_session(&db, record, &settings).expect("commit_session");
    println!("inserted row_id={row_id}, after sum={}", sum(&db),);
}

fn sum(db: &Db) -> i64 {
    db.with_conn(|c| {
        c.query_row(
            "SELECT COALESCE(SUM(word_count), 0) FROM transcriptions",
            [],
            |r| r.get(0),
        )
    })
    .expect("sum query")
}
