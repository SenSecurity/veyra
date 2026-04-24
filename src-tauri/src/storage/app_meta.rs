use super::{Db, DbError};

pub struct AppMetaRepo<'a> {
    db: &'a Db,
}

impl<'a> AppMetaRepo<'a> {
    pub fn new(db: &'a Db) -> Self {
        Self { db }
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare("SELECT value FROM app_meta WHERE key = ?1")?;
            let mut rows = stmt.query_map([key], |r| r.get::<_, String>(0))?;
            rows.next().transpose()
        })
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), DbError> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO app_meta (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                rusqlite::params![key, value],
            )?;
            Ok(())
        })
    }

    pub fn delete(&self, key: &str) -> Result<(), DbError> {
        self.db.with_conn(|c| {
            c.execute("DELETE FROM app_meta WHERE key = ?1", [key])?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    #[test]
    fn set_then_get() {
        let db = mem_db();
        let repo = AppMetaRepo::new(&db);
        assert_eq!(repo.get("k").unwrap(), None);
        repo.set("k", "v1").unwrap();
        assert_eq!(repo.get("k").unwrap(), Some("v1".to_string()));
        repo.set("k", "v2").unwrap();
        assert_eq!(repo.get("k").unwrap(), Some("v2".to_string()));
    }

    #[test]
    fn delete_removes_key() {
        let db = mem_db();
        let repo = AppMetaRepo::new(&db);
        repo.set("k", "v").unwrap();
        repo.delete("k").unwrap();
        assert_eq!(repo.get("k").unwrap(), None);
    }

    #[test]
    fn stores_large_json_blob() {
        let db = mem_db();
        let repo = AppMetaRepo::new(&db);
        let big = "x".repeat(100_000);
        repo.set("settings_json", &big).unwrap();
        assert_eq!(repo.get("settings_json").unwrap().unwrap().len(), 100_000);
    }
}
