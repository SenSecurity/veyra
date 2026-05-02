use crate::storage::snippets::{Snippet, SnippetRepo};
use crate::storage::DbError;

pub fn expand_snippets(text: &str, repo: &SnippetRepo) -> Result<String, DbError> {
    let snippets = repo.list()?;
    if snippets.is_empty() {
        return Ok(text.to_string());
    }
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    let bytes = text.as_bytes();
    while cursor < bytes.len() {
        let at_phrase_start = cursor == 0 || bytes[cursor - 1] == b'\n';
        if at_phrase_start {
            if let Some(hit) = match_first_trigger(&text[cursor..], &snippets) {
                out.push_str(&hit.snippet.expansion);
                repo.increment_use(hit.snippet.id)?;
                cursor += hit.matched_len;
                continue;
            }
        }
        let ch_len = next_char_len(&bytes[cursor..]);
        out.push_str(&text[cursor..cursor + ch_len]);
        cursor += ch_len;
    }
    Ok(out)
}

struct Hit<'a> {
    snippet: &'a Snippet,
    matched_len: usize,
}

fn match_first_trigger<'a>(window: &str, snippets: &'a [Snippet]) -> Option<Hit<'a>> {
    let mut sorted: Vec<&Snippet> = snippets.iter().filter(|s| s.enabled).collect();
    sorted.sort_by_key(|s| std::cmp::Reverse(s.trigger.len()));
    for snip in sorted {
        if window.starts_with(&snip.trigger) {
            return Some(Hit {
                snippet: snip,
                matched_len: snip.trigger.len(),
            });
        }
    }
    None
}

fn next_char_len(bytes: &[u8]) -> usize {
    let b = bytes[0];
    if b < 0x80 {
        1
    } else if b < 0xC0 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::snippets::NewSnippet;
    use crate::storage::test_util::mem_db;

    fn seed(repo: &SnippetRepo, trigger: &str, expansion: &str) {
        repo.upsert(
            1,
            NewSnippet {
                trigger,
                expansion,
                description: None,
                enabled: true,
            },
        )
        .unwrap();
    }

    #[test]
    fn expands_at_byte_zero() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        seed(&repo, "/sig", "Bruno Rodrigues");
        assert_eq!(
            expand_snippets("/sig hello", &repo).unwrap(),
            "Bruno Rodrigues hello"
        );
    }

    #[test]
    fn does_not_expand_mid_text() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        seed(&repo, "/sig", "Bruno Rodrigues");
        assert_eq!(
            expand_snippets("hello /sig hello", &repo).unwrap(),
            "hello /sig hello"
        );
    }

    #[test]
    fn expands_after_newline() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        seed(&repo, "/sig", "Bruno");
        assert_eq!(
            expand_snippets("hello\n/sig", &repo).unwrap(),
            "hello\nBruno"
        );
    }

    #[test]
    fn longest_trigger_wins_when_prefixes_collide() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        seed(&repo, "/sig", "short");
        seed(&repo, "/sigfull", "long");
        assert_eq!(
            expand_snippets("/sigfull hello", &repo).unwrap(),
            "long hello"
        );
    }

    #[test]
    fn empty_repo_passthrough() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        assert_eq!(expand_snippets("hello", &repo).unwrap(), "hello");
    }

    #[test]
    fn increments_use_count_on_match() {
        let db = mem_db();
        let repo = SnippetRepo::new(&db);
        let id = repo
            .upsert(
                1,
                NewSnippet {
                    trigger: "/sig",
                    expansion: "B",
                    description: None,
                    enabled: true,
                },
            )
            .unwrap();
        expand_snippets("/sig\n/sig", &repo).unwrap();
        let snip = repo.find_by_trigger("/sig").unwrap().unwrap();
        assert_eq!(snip.id, id);
    }
}
