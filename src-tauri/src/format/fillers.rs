use regex::RegexBuilder;

/// Drop every occurrence of a configured filler word from `text`.
/// Matching is word-boundary, case-insensitive. Adjacent double spaces
/// left after a drop are collapsed to a single space; leading/trailing
/// whitespace is trimmed.
pub fn drop_fillers(text: &str, fillers: &[String]) -> String {
    if fillers.is_empty() || text.is_empty() {
        return text.to_string();
    }
    let mut out = text.to_string();
    for f in fillers {
        let trimmed = f.trim();
        if trimmed.is_empty() {
            continue;
        }
        let escaped = regex::escape(trimmed);
        let pattern = format!(r"\b{}\b", escaped);
        let re = RegexBuilder::new(&pattern)
            .case_insensitive(true)
            .build()
            .expect("static filler pattern");
        out = re.replace_all(&out, "").into_owned();
    }
    // Collapse runs of whitespace introduced by removals.
    let collapse = regex::Regex::new(r"\s{2,}").unwrap();
    collapse.replace_all(out.trim(), " ").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn drops_simple_filler() {
        assert_eq!(drop_fillers("um hello world", &s(&["um"])), "hello world");
    }

    #[test]
    fn case_insensitive_match() {
        assert_eq!(drop_fillers("UM hello", &s(&["um"])), "hello");
    }

    #[test]
    fn preserves_word_boundary() {
        // "umbrella" must NOT be touched by filler "um".
        assert_eq!(
            drop_fillers("umbrella opens", &s(&["um"])),
            "umbrella opens"
        );
    }

    #[test]
    fn multi_word_filler_supported() {
        assert_eq!(drop_fillers("I mean hello", &s(&["i mean"])), "hello");
    }

    #[test]
    fn collapses_double_spaces_after_drop() {
        assert_eq!(drop_fillers("hello um world", &s(&["um"])), "hello world");
    }

    #[test]
    fn empty_filler_list_is_noop() {
        assert_eq!(drop_fillers("um hello", &s(&[])), "um hello");
    }

    #[test]
    fn drops_pt_filler_ne() {
        assert_eq!(
            drop_fillers("isto né funciona", &s(&["né"])),
            "isto funciona"
        );
    }
}
