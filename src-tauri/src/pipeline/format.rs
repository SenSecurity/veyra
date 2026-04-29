//! Format orchestrator — composes the four format-rule modules in order:
//! trim → drop_fillers → replace_commands → dictionary_pass → expand_snippets.
//! After producing the formatted text, observes the *raw* transcription
//! against the auto-add candidates table (no transform on text).
//!
//! Each pass is gated by `settings.formatting`; dictionary and snippet
//! passes always run (DB-driven, with their own no-op fast paths when the
//! corresponding tables are empty).

use time::OffsetDateTime;

use crate::format;
use crate::settings::Settings;
use crate::storage::auto_add_candidates::AutoAddCandidatesRepo;
use crate::storage::dictionary::DictionaryRepo;
use crate::storage::snippets::SnippetRepo;
use crate::storage::{Db, DbError};

#[derive(Debug)]
pub enum FormatError {
    Db(DbError),
}

impl From<DbError> for FormatError {
    fn from(e: DbError) -> Self {
        Self::Db(e)
    }
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::Db(e) => write!(f, "format db error: {e}"),
        }
    }
}

impl std::error::Error for FormatError {}

/// Apply the dictation-arm format pipeline to `raw` and return the formatted
/// text. Side-effect: observes raw tokens against the auto-add candidates
/// table and may promote tokens past the threshold to dictionary entries.
pub fn run_format(raw: &str, settings: &Settings, db: &Db) -> Result<String, FormatError> {
    let mut text = raw.trim().to_string();

    if settings.formatting.remove_fillers {
        text = format::fillers::drop_fillers(&text, &settings.formatting.filler_words);
    }
    if settings.formatting.explicit_commands {
        text = format::commands::replace_commands(&text);
    }

    let dict_repo = DictionaryRepo::new(db);
    text = format::dictionary::dictionary_pass(&text, &dict_repo)?;

    let snip_repo = SnippetRepo::new(db);
    text = format::snippets::expand_snippets(&text, &snip_repo)?;

    let cand_repo = AutoAddCandidatesRepo::new(db);
    format::dictionary::auto_add_unseen(
        raw,
        &dict_repo,
        &cand_repo,
        format::dictionary::DICTIONARY_AUTO_ADD_THRESHOLD,
        settings.dictionary.auto_add,
        OffsetDateTime::now_utc().unix_timestamp(),
    )?;

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::dictionary::NewDictionaryTerm;
    use crate::storage::snippets::NewSnippet;
    use crate::storage::test_util::mem_db;

    #[test]
    fn end_to_end_dictation_path() {
        let db = mem_db();
        let dict = DictionaryRepo::new(&db);
        dict.upsert(
            1,
            NewDictionaryTerm {
                term: "tauri",
                replacement: Some("Tauri"),
                is_abbreviation: false,
                auto_added: false,
                enabled: true,
            },
        )
        .unwrap();
        let snips = SnippetRepo::new(&db);
        snips
            .upsert(
                1,
                NewSnippet {
                    trigger: "/sig",
                    expansion: "Bruno",
                    description: None,
                    enabled: true,
                },
            )
            .unwrap();

        let mut settings = Settings::default();
        settings.formatting.remove_fillers = true;
        settings.formatting.filler_words = vec!["um".to_string()];
        settings.formatting.explicit_commands = true;

        let out = run_format("/sig um we use tauri comma yes period", &settings, &db).unwrap();
        assert_eq!(out, "Bruno we use Tauri, yes.");
    }

    #[test]
    fn empty_input_returns_empty() {
        let db = mem_db();
        let out = run_format("", &Settings::default(), &db).unwrap();
        assert_eq!(out, "");
    }

    #[test]
    fn formatting_toggles_off_skips_passes() {
        let db = mem_db();
        let mut settings = Settings::default();
        settings.formatting.remove_fillers = false;
        settings.formatting.explicit_commands = false;
        settings.formatting.filler_words = vec!["um".to_string()];

        let out = run_format("um hello comma", &settings, &db).unwrap();
        assert_eq!(out, "um hello comma");
    }
}
