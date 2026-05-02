use crate::storage::auto_add_candidates::AutoAddCandidatesRepo;
use crate::storage::dictionary::{DictionaryRepo, NewDictionaryTerm};
use crate::storage::DbError;
use regex::{Regex, RegexBuilder};
use std::collections::HashSet;

pub const DICTIONARY_AUTO_ADD_THRESHOLD: i64 = 5;

/// Apply dictionary replacements across `text`. Case-preserving for non-
/// abbreviation entries (Title→Title, ALL→ALL, lower→lower as configured
/// per row). Abbreviations always render as the configured replacement
/// regardless of input casing.
pub fn dictionary_pass(text: &str, repo: &DictionaryRepo) -> Result<String, DbError> {
    let terms = repo.list()?;
    if terms.is_empty() {
        return Ok(text.to_string());
    }
    let mut out = text.to_string();
    for term in terms {
        if !term.enabled {
            continue;
        }
        let Some(replacement) = term.replacement.clone() else {
            continue;
        };
        let pat = format!(r"\b{}\b", regex::escape(&term.term));
        let re = RegexBuilder::new(&pat)
            .case_insensitive(true)
            .build()
            .expect("pattern from escaped dictionary term cannot fail");
        out = if term.is_abbreviation {
            re.replace_all(&out, replacement.as_str()).into_owned()
        } else {
            re.replace_all(&out, |caps: &regex::Captures| {
                let original = &caps[0];
                preserve_case(original, &replacement)
            })
            .into_owned()
        };
    }
    Ok(out)
}

/// Track non-dictionary tokens against the candidates table. When a token
/// has been seen `>= DICTIONARY_AUTO_ADD_THRESHOLD` times AND `auto_add_enabled`,
/// promote it to a dictionary entry with `auto_added=true, replacement=None`.
/// No-op when `auto_add_enabled=false`.
pub fn auto_add_unseen(
    raw: &str,
    dict_repo: &DictionaryRepo,
    cand_repo: &AutoAddCandidatesRepo,
    threshold: i64,
    auto_add_enabled: bool,
    now: i64,
) -> Result<(), DbError> {
    if !auto_add_enabled || raw.trim().is_empty() {
        return Ok(());
    }
    let known: HashSet<String> = dict_repo
        .list()?
        .into_iter()
        .map(|t| t.term.to_lowercase())
        .collect();
    let tokenizer = Regex::new(r"[\p{L}]{4,}").unwrap();
    let mut emitted: HashSet<String> = HashSet::new();
    for cap in tokenizer.find_iter(raw) {
        let tok = cap.as_str().to_lowercase();
        if known.contains(&tok) || emitted.contains(&tok) {
            continue;
        }
        emitted.insert(tok.clone());
        let count = cand_repo.observe(now, &tok)?;
        if count >= threshold {
            dict_repo.upsert(
                now,
                NewDictionaryTerm {
                    term: &tok,
                    replacement: None,
                    is_abbreviation: false,
                    auto_added: true,
                    enabled: true,
                },
            )?;
            cand_repo.delete(&tok)?;
        }
    }
    Ok(())
}

fn preserve_case(original: &str, replacement: &str) -> String {
    if original.chars().all(|c| c.is_uppercase()) {
        replacement.to_uppercase()
    } else if original
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false)
    {
        let mut out = String::with_capacity(replacement.len());
        let mut chars = replacement.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
        }
        out.extend(chars);
        out
    } else {
        replacement.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::dictionary::NewDictionaryTerm;
    use crate::storage::test_util::mem_db;

    fn seed(repo: &DictionaryRepo, term: &str, replacement: Option<&str>, abbr: bool) {
        repo.upsert(
            1,
            NewDictionaryTerm {
                term,
                replacement,
                is_abbreviation: abbr,
                auto_added: false,
                enabled: true,
            },
        )
        .unwrap();
    }

    #[test]
    fn replaces_basic_term_with_lowercase_input() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        seed(&repo, "tauri", Some("Tauri"), false);
        assert_eq!(
            dictionary_pass("we use tauri here", &repo).unwrap(),
            "we use Tauri here"
        );
    }

    #[test]
    fn case_preserving_uppercase_match() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        seed(&repo, "tauri", Some("Tauri"), false);
        assert_eq!(
            dictionary_pass("TAURI rocks", &repo).unwrap(),
            "TAURI rocks"
        );
    }

    #[test]
    fn case_preserving_titlecase_match() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        seed(&repo, "tauri", Some("tauri"), false);
        assert_eq!(
            dictionary_pass("Tauri opens", &repo).unwrap(),
            "Tauri opens"
        );
    }

    #[test]
    fn abbreviation_always_uppercases() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        seed(&repo, "api", Some("API"), true);
        assert_eq!(
            dictionary_pass("call api now", &repo).unwrap(),
            "call API now"
        );
        assert_eq!(
            dictionary_pass("CALL API NOW", &repo).unwrap(),
            "CALL API NOW"
        );
    }

    #[test]
    fn disabled_entries_skipped() {
        let db = mem_db();
        let repo = DictionaryRepo::new(&db);
        repo.upsert(
            1,
            NewDictionaryTerm {
                term: "tauri",
                replacement: Some("Tauri"),
                is_abbreviation: false,
                auto_added: false,
                enabled: false,
            },
        )
        .unwrap();
        assert_eq!(
            dictionary_pass("we use tauri", &repo).unwrap(),
            "we use tauri"
        );
    }

    #[test]
    fn auto_add_disabled_is_noop() {
        let db = mem_db();
        let dict = DictionaryRepo::new(&db);
        let cand = AutoAddCandidatesRepo::new(&db);
        auto_add_unseen("acme acme acme acme acme acme", &dict, &cand, 5, false, 0).unwrap();
        assert!(cand.get("acme").unwrap().is_none());
        assert!(dict.list().unwrap().is_empty());
    }

    #[test]
    fn auto_add_promotes_after_threshold() {
        let db = mem_db();
        let dict = DictionaryRepo::new(&db);
        let cand = AutoAddCandidatesRepo::new(&db);
        for i in 0..4 {
            auto_add_unseen("acme rocket", &dict, &cand, 5, true, i).unwrap();
        }
        assert_eq!(cand.get("acme").unwrap().unwrap().seen_count, 4);
        assert!(dict.list().unwrap().is_empty());
        auto_add_unseen("acme rocket", &dict, &cand, 5, true, 5).unwrap();
        let dict_rows = dict.list().unwrap();
        let acme = dict_rows.iter().find(|t| t.term == "acme").unwrap();
        assert!(acme.auto_added);
        assert_eq!(acme.replacement, None);
        assert!(cand.get("acme").unwrap().is_none());
    }

    #[test]
    fn auto_add_skips_known_dictionary_terms() {
        let db = mem_db();
        let dict = DictionaryRepo::new(&db);
        let cand = AutoAddCandidatesRepo::new(&db);
        seed(&dict, "tauri", Some("Tauri"), false);
        for i in 0..6 {
            auto_add_unseen("tauri tauri tauri", &dict, &cand, 5, true, i).unwrap();
        }
        assert!(cand.get("tauri").unwrap().is_none());
    }

    #[test]
    fn auto_add_skips_short_tokens() {
        let db = mem_db();
        let dict = DictionaryRepo::new(&db);
        let cand = AutoAddCandidatesRepo::new(&db);
        for _ in 0..6 {
            auto_add_unseen("a be the", &dict, &cand, 5, true, 0).unwrap();
        }
        assert!(cand.get("the").unwrap().is_none());
    }
}
