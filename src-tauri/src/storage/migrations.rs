use super::{Db, DbError};

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("migrations/001_initial.sql")),
];

pub fn run(db: &Db) -> Result<(), DbError> {
    db.with_conn_mut(|conn| {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                 version INTEGER PRIMARY KEY,
                 applied_at INTEGER NOT NULL
             );",
        )?;

        let tx = conn.transaction()?;
        for (version, sql) in MIGRATIONS {
            let already: i64 = tx.query_row(
                "SELECT count(*) FROM schema_migrations WHERE version = ?1",
                [version],
                |r| r.get(0),
            )?;
            if already == 0 {
                tx.execute_batch(sql)?;
                tx.execute(
                    "INSERT INTO schema_migrations(version, applied_at) VALUES (?1, strftime('%s','now'))",
                    [version],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    })
}
