use super::Db;

pub fn mem_db() -> Db {
    Db::open_in_memory().expect("in-memory db")
}
