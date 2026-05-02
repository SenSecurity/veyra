use super::{Db, DbError};
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStats {
    pub day: String,
    pub word_count: i64,
    pub session_count: i64,
    pub total_duration_ms: i64,
    pub avg_wpm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreakInfo {
    pub current: i64,
    pub longest: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub word_count: i64,
    pub session_count: i64,
    pub total_duration_ms: i64,
}

pub struct StatsRepo<'a> { db: &'a Db }

impl<'a> StatsRepo<'a> {
    pub fn new(db: &'a Db) -> Self { Self { db } }

    pub fn bump_day(&self, day: &str, words: i64, duration_ms: i64) -> Result<(), DbError> {
        self.db.with_conn(|c| {
            c.execute(
                "INSERT INTO stats_daily (day, word_count, session_count, total_duration_ms)
                 VALUES (?1, ?2, 1, ?3)
                 ON CONFLICT(day) DO UPDATE SET
                   word_count = word_count + excluded.word_count,
                   session_count = session_count + 1,
                   total_duration_ms = total_duration_ms + excluded.total_duration_ms",
                params![day, words, duration_ms],
            )?;
            Ok(())
        })
    }

    pub fn get_day(&self, day: &str) -> Result<Option<DailyStats>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT day, word_count, session_count, total_duration_ms, avg_wpm
                 FROM stats_daily WHERE day = ?1",
            )?;
            let mut rows = stmt.query_map([day], |r| Ok(DailyStats {
                day: r.get(0)?,
                word_count: r.get(1)?,
                session_count: r.get(2)?,
                total_duration_ms: r.get(3)?,
                avg_wpm: r.get(4)?,
            }))?;
            rows.next().transpose()
        })
    }

    pub fn totals(&self) -> Result<Totals, DbError> {
        self.db.with_conn(|c| {
            c.query_row(
                "SELECT COALESCE(SUM(word_count),0),
                        COALESCE(SUM(session_count),0),
                        COALESCE(SUM(total_duration_ms),0)
                 FROM stats_daily",
                [], |r| Ok(Totals {
                    word_count: r.get(0)?,
                    session_count: r.get(1)?,
                    total_duration_ms: r.get(2)?,
                }),
            )
        })
    }

    pub fn list_all_days(&self) -> Result<Vec<DailyStats>, DbError> {
        self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT day, word_count, session_count, total_duration_ms, avg_wpm
                 FROM stats_daily ORDER BY day DESC",
            )?;
            let rows = stmt.query_map([], |r| Ok(DailyStats {
                day: r.get(0)?,
                word_count: r.get(1)?,
                session_count: r.get(2)?,
                total_duration_ms: r.get(3)?,
                avg_wpm: r.get(4)?,
            }))?.collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
    }

    pub fn streak_info(&self, today: &str) -> Result<StreakInfo, DbError> {
        let days: Vec<String> = self.db.with_conn(|c| {
            let mut stmt = c.prepare(
                "SELECT day FROM stats_daily WHERE word_count > 0 ORDER BY day ASC",
            )?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            let result: Result<Vec<_>, _> = rows.collect();
            result
        })?;

        // Parse YYYY-MM-DD into a time::Date via the ISO 8601 DATE format.
        fn parse(d: &str) -> Option<time::Date> {
            time::Date::parse(d, &time::format_description::well_known::Iso8601::DATE).ok()
        }
        let today_d = parse(today).ok_or_else(|| DbError::Sqlite(
            rusqlite::Error::InvalidParameterName("today".into())
        ))?;

        let mut longest = 0i64;
        let mut run = 0i64;
        let mut prev: Option<time::Date> = None;
        for s in &days {
            let d = match parse(s) { Some(d) => d, None => continue };
            match prev {
                Some(p) if (d - p).whole_days() == 1 => run += 1,
                _ => run = 1,
            }
            if run > longest { longest = run; }
            prev = Some(d);
        }

        // Current streak: walk backward from today.
        let set: std::collections::HashSet<time::Date> =
            days.iter().filter_map(|s| parse(s)).collect();
        let mut current = 0i64;
        let mut cursor = today_d;
        while set.contains(&cursor) {
            current += 1;
            cursor -= time::Duration::days(1);
        }

        Ok(StreakInfo { current, longest })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_util::mem_db;

    #[test]
    fn bump_day_aggregates() {
        let db = mem_db();
        let repo = StatsRepo::new(&db);
        repo.bump_day("2026-04-23", 50, 10_000).unwrap();
        repo.bump_day("2026-04-23", 30, 5_000).unwrap();
        let day = repo.get_day("2026-04-23").unwrap().unwrap();
        assert_eq!(day.word_count, 80);
        assert_eq!(day.session_count, 2);
        assert_eq!(day.total_duration_ms, 15_000);
    }

    #[test]
    fn totals_sums_all_days() {
        let db = mem_db();
        let repo = StatsRepo::new(&db);
        repo.bump_day("2026-04-20", 10, 1_000).unwrap();
        repo.bump_day("2026-04-21", 20, 2_000).unwrap();
        let t = repo.totals().unwrap();
        assert_eq!(t.word_count, 30);
        assert_eq!(t.session_count, 2);
        assert_eq!(t.total_duration_ms, 3_000);
    }

    #[test]
    fn streak_counts_consecutive_days_including_today() {
        let db = mem_db();
        let repo = StatsRepo::new(&db);
        repo.bump_day("2026-04-21", 1, 1).unwrap();
        repo.bump_day("2026-04-22", 1, 1).unwrap();
        repo.bump_day("2026-04-23", 1, 1).unwrap();
        let s = repo.streak_info("2026-04-23").unwrap();
        assert_eq!(s.current, 3);
        assert_eq!(s.longest, 3);
    }

    #[test]
    fn list_all_days_returns_descending() {
        let db = mem_db();
        let repo = StatsRepo::new(&db);
        repo.bump_day("2026-04-20", 10, 1_000).unwrap();
        repo.bump_day("2026-04-22", 20, 2_000).unwrap();
        repo.bump_day("2026-04-21", 5, 500).unwrap();
        let days = repo.list_all_days().unwrap();
        assert_eq!(days.len(), 3);
        assert_eq!(days[0].day, "2026-04-22");
        assert_eq!(days[1].day, "2026-04-21");
        assert_eq!(days[2].day, "2026-04-20");
    }

    #[test]
    fn streak_resets_on_gap() {
        let db = mem_db();
        let repo = StatsRepo::new(&db);
        repo.bump_day("2026-04-10", 1, 1).unwrap();
        repo.bump_day("2026-04-11", 1, 1).unwrap();
        repo.bump_day("2026-04-20", 1, 1).unwrap();
        repo.bump_day("2026-04-23", 1, 1).unwrap(); // today
        let s = repo.streak_info("2026-04-23").unwrap();
        assert_eq!(s.current, 1, "today is active but yesterday is missing");
        assert_eq!(s.longest, 2, "Apr 10-11 streak");
    }
}
