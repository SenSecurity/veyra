//! Format orchestrator — composes the four format-rule modules in order:
//! trim → drop_fillers → replace_commands → dictionary_pass → expand_snippets.
//! After producing the formatted text, observes the *raw* transcription
//! against the auto-add candidates table (no transform on text).
//!
//! Each pass is gated by `settings.formatting`; dictionary and snippet
//! passes always run (DB-driven, with their own no-op fast paths when the
//! corresponding tables are empty).

use std::sync::OnceLock;

use regex::Regex;
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

    // Final normalisation pass. Whisper auto-adds punctuation in some
    // languages (PT especially), so when a user dictates the explicit-command
    // word ("vírgula", "ponto final") they end up with a doubled mark like
    // "mundo,, " or "teste..". Collapse runs and strip any space that landed
    // before a punctuation char. Idempotent — re-running is a no-op once a
    // single mark + non-space follow.
    text = collapse_punctuation(&text);

    Ok(text)
}

fn collapse_punctuation(text: &str) -> String {
    // 1. Strip whitespace immediately before terminal punctuation marks.
    static SPACE_BEFORE: OnceLock<Regex> = OnceLock::new();
    let space_before = SPACE_BEFORE.get_or_init(|| Regex::new(r"\s+([,.!?])").unwrap());
    let mut out = space_before.replace_all(text, "$1").into_owned();

    // 2. Drop a comma that's immediately followed (optionally across one
    //    space) by a sentence terminator. Whisper auto-punctuation routinely
    //    inserts a comma where the user then dictates "ponto final" or
    //    similar — keep the stronger mark, lose the comma.
    static COMMA_BEFORE_TERMINATOR: OnceLock<Regex> = OnceLock::new();
    let comma_before_term =
        COMMA_BEFORE_TERMINATOR.get_or_init(|| Regex::new(r",(\s*[.!?])").unwrap());
    out = comma_before_term.replace_all(&out, "$1").into_owned();

    // 3. Collapse runs of the same terminal punctuation (optionally separated
    //    by whitespace) down to a single mark. The `regex` crate has no
    //    backreferences so each mark gets its own pattern.
    static COMMA_RUN: OnceLock<Regex> = OnceLock::new();
    static PERIOD_RUN: OnceLock<Regex> = OnceLock::new();
    static BANG_RUN: OnceLock<Regex> = OnceLock::new();
    static QUESTION_RUN: OnceLock<Regex> = OnceLock::new();
    let comma_run = COMMA_RUN.get_or_init(|| Regex::new(r",(?:\s*,)+").unwrap());
    let period_run = PERIOD_RUN.get_or_init(|| Regex::new(r"\.(?:\s*\.)+").unwrap());
    let bang_run = BANG_RUN.get_or_init(|| Regex::new(r"!(?:\s*!)+").unwrap());
    let question_run = QUESTION_RUN.get_or_init(|| Regex::new(r"\?(?:\s*\?)+").unwrap());
    out = comma_run.replace_all(&out, ",").into_owned();
    out = period_run.replace_all(&out, ".").into_owned();
    out = bang_run.replace_all(&out, "!").into_owned();
    out = question_run.replace_all(&out, "?").into_owned();

    out
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

    #[test]
    fn collapse_pt_double_punctuation_after_command_replace() {
        // Whisper auto-adds punctuation in PT, so the user dictating
        // "olá mundo vírgula isto é teste ponto final" arrives at the
        // formatter as roughly "olá mundo, vírgula, isto é teste, ponto final.".
        // After the command rule eats `\s?\bvírgula\b` and `\s?\bponto final\b`,
        // we end up with stacked marks. Final pass must collapse them.
        let db = mem_db();
        let mut settings = Settings::default();
        settings.formatting.remove_fillers = false;
        settings.formatting.explicit_commands = true;

        let out = run_format(
            "olá mundo, vírgula, isto é teste, ponto final.",
            &settings,
            &db,
        )
        .unwrap();
        assert_eq!(out, "olá mundo, isto é teste.");
    }

    #[test]
    fn collapse_handles_question_and_exclamation() {
        let out = collapse_punctuation("hello?? world!!");
        assert_eq!(out, "hello? world!");
    }

    #[test]
    fn collapse_strips_space_before_punctuation() {
        let out = collapse_punctuation("hello , world .");
        assert_eq!(out, "hello, world.");
    }

    #[test]
    fn collapse_idempotent() {
        let once = collapse_punctuation("a, b. c! d?");
        assert_eq!(once, "a, b. c! d?");
        let twice = collapse_punctuation(&once);
        assert_eq!(twice, once);
    }
}
