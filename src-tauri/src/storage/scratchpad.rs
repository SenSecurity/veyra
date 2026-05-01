use super::{Db, DbError};
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScratchpadNote {
    pub id: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub title: Option<String>,
    pub body: String,
    pub pinned: bool,
}

#[derive(Debug, Clone)]
pub struct NewNote<'a> {
    pub title: Option<&'a str>,
    pub body: &'a str,
    pub pinned: bool,
}

pub struct ScratchpadRepo<'a> { db: &'a Db }

impl<'a> ScratchpadRepo<'a> {
    pub fn new(db: &'a Db) -> Self { Self { db } }

    pub fn upsert(&self, now: i64, id: Option<i64>, note: NewNote) -> Result<i64, DbError> {
        self.db.with_conn(|c| match id {
            Some(i) => {
                c.execute(
                    "UPDATE scratchpad_notes
                       SET updated_at=?1, title=?2, body=?3, pinned=?4
                     WHERE id=?5",
                    params![now, note.title, note.body, note.pinned as i64, i],
                )?;
                Ok(i)
            }
            None => {
                c.execute(
                    "INSERT INTO scratchpad_notes
                       (created_at, updated_at, title, body, pinned)
                     VALUES (?1,?1,?2,?3,?4)",
                    params![now, note.title, note.body, note.pinned as i64],
                )?;
                Ok(c.last_insert_rowid())
            }
        })
    }

    pub fn list_ordered(&self) -> Result<Vec<ScratchpadNote>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT id, created_at, updated_at, title, body, pinned
                 FROM scratchpad_notes
                 ORDER BY pinned DESC, updated_at DESC, id DESC",
            )?;
            let rows = stmt.query_map([], |r| Ok(ScratchpadNote {
                id: r.get(0)?,
                created_at: r.get(1)?,
                updated_at: r.get(2)?,
                title: r.get(3)?,
                body: r.get(4)?,
                pinned: r.get::<_, i64>(5)? != 0,
            }))?.collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }

    pub fn delete(&self, id: i64) -> Result<usize, DbError> {
        self.db.with_conn(|c| c.execute("DELETE FROM scratchpad_notes WHERE id = ?1", [id]))
    }

    pub fn set_pinned(&self, id: i64, pinned: bool) -> Result<usize, DbError> {
        self.db.with_conn(|c| c.execute(
            "UPDATE scratchpad_notes SET pinned = ?1 WHERE id = ?2",
            params![pinned as i64, id],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    #[test]
    fn insert_and_list() {
        let db = mem_db();
        let repo = ScratchpadRepo::new(&db);
        let id = repo.upsert(100, None, NewNote { title: Some("t"), body: "b", pinned: false }).unwrap();
        assert!(id > 0);
        let notes = repo.list_ordered().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].body, "b");
    }

    #[test]
    fn update_existing_by_id() {
        let db = mem_db();
        let repo = ScratchpadRepo::new(&db);
        let id = repo.upsert(100, None, NewNote { title: None, body: "v1", pinned: false }).unwrap();
        repo.upsert(200, Some(id), NewNote { title: None, body: "v2", pinned: true }).unwrap();
        let notes = repo.list_ordered().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].body, "v2");
        assert!(notes[0].pinned);
        assert_eq!(notes[0].updated_at, 200);
    }

    #[test]
    fn list_pinned_first_then_by_updated_desc() {
        let db = mem_db();
        let repo = ScratchpadRepo::new(&db);
        repo.upsert(100, None, NewNote { title: None, body: "old-unpinned", pinned: false }).unwrap();
        repo.upsert(200, None, NewNote { title: None, body: "new-unpinned", pinned: false }).unwrap();
        repo.upsert(50, None, NewNote { title: None, body: "pinned", pinned: true }).unwrap();
        let notes = repo.list_ordered().unwrap();
        assert_eq!(notes[0].body, "pinned");
        assert_eq!(notes[1].body, "new-unpinned");
        assert_eq!(notes[2].body, "old-unpinned");
    }

    #[test]
    fn set_pinned_flips_flag() {
        let db = mem_db();
        let repo = ScratchpadRepo::new(&db);
        let id = repo.upsert(1, None, NewNote { title: None, body: "x", pinned: false }).unwrap();
        repo.set_pinned(id, true).unwrap();
        let notes = repo.list_ordered().unwrap();
        assert!(notes[0].pinned);
        repo.set_pinned(id, false).unwrap();
        let notes = repo.list_ordered().unwrap();
        assert!(!notes[0].pinned);
    }

    #[test]
    fn delete_removes() {
        let db = mem_db();
        let repo = ScratchpadRepo::new(&db);
        let id = repo.upsert(1, None, NewNote { title: None, body: "x", pinned: false }).unwrap();
        assert_eq!(repo.delete(id).unwrap(), 1);
        assert!(repo.list_ordered().unwrap().is_empty());
    }
}
