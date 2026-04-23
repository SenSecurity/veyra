use super::{Db, DbError};
use rusqlite::params;

#[derive(Debug, Clone, PartialEq)]
pub struct Snippet {
    pub id: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub trigger: String,
    pub expansion: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub use_count: i64,
}

#[derive(Debug, Clone)]
pub struct NewSnippet<'a> {
    pub trigger: &'a str,
    pub expansion: &'a str,
    pub description: Option<&'a str>,
    pub enabled: bool,
}

pub struct SnippetRepo<'a> { db: &'a Db }

impl<'a> SnippetRepo<'a> {
    pub fn new(db: &'a Db) -> Self { Self { db } }

    pub fn upsert(&self, now: i64, row: NewSnippet) -> Result<i64, DbError> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO snippets
                   (created_at, updated_at, trigger, expansion, description, enabled, use_count)
                 VALUES (?1,?1,?2,?3,?4,?5,0)
                 ON CONFLICT(trigger) DO UPDATE SET
                   expansion=excluded.expansion,
                   description=excluded.description,
                   enabled=excluded.enabled,
                   updated_at=?1",
                params![now, row.trigger, row.expansion, row.description, row.enabled as i64],
            )?;
            c.query_row("SELECT id FROM snippets WHERE trigger = ?1", [row.trigger], |r| r.get(0))
        })
    }

    pub fn list(&self) -> Result<Vec<Snippet>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, created_at, updated_at, trigger, expansion, description, enabled, use_count
                 FROM snippets ORDER BY trigger ASC",
            )?;
            let rows = stmt.query_map([], map_snippet)?.collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }

    pub fn find_by_trigger(&self, trigger: &str) -> Result<Option<Snippet>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, created_at, updated_at, trigger, expansion, description, enabled, use_count
                 FROM snippets WHERE trigger = ?1 AND enabled = 1",
            )?;
            let mut rows = stmt.query_map([trigger], map_snippet)?;
            match rows.next() {
                Some(r) => Ok(Some(r?)),
                None => Ok(None),
            }
        })
    }

    pub fn increment_use(&self, id: i64) -> Result<(), DbError> {
        self.db.with_conn(|c| {
            c.execute("UPDATE snippets SET use_count = use_count + 1 WHERE id = ?1", [id])?;
            Ok(())
        })
    }
}

fn map_snippet(r: &rusqlite::Row) -> rusqlite::Result<Snippet> {
    Ok(Snippet {
        id: r.get(0)?,
        created_at: r.get(1)?,
        updated_at: r.get(2)?,
        trigger: r.get(3)?,
        expansion: r.get(4)?,
        description: r.get(5)?,
        enabled: r.get::<_, i64>(6)? != 0,
        use_count: r.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    #[test]
    fn upsert_then_find_by_trigger() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        repo.upsert(1, NewSnippet {
            trigger: ":email", expansion: "brunorodrigues2627@gmail.com",
            description: None, enabled: true,
        }).unwrap();
        let hit = repo.find_by_trigger(":email").unwrap().unwrap();
        assert_eq!(hit.expansion, "brunorodrigues2627@gmail.com");
        assert_eq!(hit.use_count, 0);
    }

    #[test]
    fn increment_use_bumps_counter() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        let id = repo.upsert(1, NewSnippet {
            trigger: ":sig", expansion: "- Bruno", description: None, enabled: true,
        }).unwrap();
        repo.increment_use(id).unwrap();
        repo.increment_use(id).unwrap();
        let hit = repo.find_by_trigger(":sig").unwrap().unwrap();
        assert_eq!(hit.use_count, 2);
    }

    #[test]
    fn find_by_trigger_returns_none_when_disabled() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        repo.upsert(1, NewSnippet {
            trigger: ":off", expansion: "x", description: None, enabled: false,
        }).unwrap();
        assert!(repo.find_by_trigger(":off").unwrap().is_none());
    }

    #[test]
    fn list_sorted_by_trigger() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        repo.upsert(1, NewSnippet { trigger: ":b", expansion: "b", description: None, enabled: true }).unwrap();
        repo.upsert(1, NewSnippet { trigger: ":a", expansion: "a", description: None, enabled: true }).unwrap();
        let rows = repo.list().unwrap();
        assert_eq!(rows[0].trigger, ":a");
        assert_eq!(rows[1].trigger, ":b");
    }
}
