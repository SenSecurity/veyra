use typr_lib::storage::{
    transcriptions::{NewTranscription, TranscriptionRepo},
    Db,
};

#[test]
fn open_on_disk_runs_migrations_and_round_trips() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("typr.db");

    let db = Db::open(&db_path).expect("open on disk");
    let repo = TranscriptionRepo::new(&db);
    repo.insert(NewTranscription {
        created_at: 1776902400,
        raw_text: "hello world",
        final_text: "hello world",
        word_count: 2,
        duration_ms: 1000,
        language: "en",
        engine: "local",
        model: Some("turbo"),
        app_context: Some("Notepad"),
        mode: "dictation",
        enhanced: false,
    })
    .unwrap();

    // Reopen the same file — migrations must be idempotent.
    drop(db);
    let db2 = Db::open(&db_path).expect("reopen");
    let repo2 = TranscriptionRepo::new(&db2);
    let hits = repo2.search_fts("hello", 10).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].final_text, "hello world");

    let has_wal = std::fs::read_dir(tmp.path())
        .unwrap()
        .any(|e| e.unwrap().file_name().to_string_lossy().ends_with("-wal"));
    assert!(has_wal, "WAL journal mode should leave a -wal file");
}
