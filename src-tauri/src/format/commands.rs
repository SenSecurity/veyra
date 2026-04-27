use regex::Regex;
use std::sync::OnceLock;

struct CommandRule {
    /// Source pattern used to build the regex (case-insensitive).
    /// `\b` boundary enforced at compile time.
    phrase: &'static str,
    replacement: &'static str,
}

// NOTE: spec template proposed two regex modes (`attaches: true|false`) but the
// spec test `en_new_paragraph_inserts_double_newline` required the leading
// space to be eaten even for non-attaching tokens (newlines, bullets). Unified
// to `\s?\b{phrase}\b` for ALL rules — every command absorbs one optional
// leading space. Behaviorally cleaner: punctuation glues, breaks don't dangle.
const RULES: &[CommandRule] = &[
    // English
    CommandRule { phrase: "new paragraph",   replacement: "\n\n" },
    CommandRule { phrase: "new line",        replacement: "\n"   },
    CommandRule { phrase: "exclamation mark",replacement: "!"    },
    CommandRule { phrase: "question mark",   replacement: "?"    },
    CommandRule { phrase: "period",          replacement: "."    },
    CommandRule { phrase: "comma",           replacement: ","    },
    CommandRule { phrase: "bullet",          replacement: "• "   },
    // Portuguese
    CommandRule { phrase: "novo parágrafo",  replacement: "\n\n" },
    CommandRule { phrase: "nova linha",      replacement: "\n"   },
    CommandRule { phrase: "ponto final",     replacement: "."    },
    CommandRule { phrase: "ponto",           replacement: "."    },
    CommandRule { phrase: "vírgula",         replacement: ","    },
    CommandRule { phrase: "interrogação",    replacement: "?"    },
    CommandRule { phrase: "exclamação",      replacement: "!"    },
];

fn compiled() -> &'static [(Regex, &'static str)] {
    static CACHE: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        RULES.iter().map(|r| {
            let escaped = regex::escape(r.phrase);
            let pat = format!(r"(?i)\s?\b{}\b", escaped);
            (Regex::new(&pat).expect("static command pattern"), r.replacement)
        }).collect()
    })
}

pub fn replace_commands(text: &str) -> String {
    let mut out = text.to_string();
    for (re, replacement) in compiled() {
        out = re.replace_all(&out, *replacement).into_owned();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_period_attaches_to_previous_word() {
        assert_eq!(replace_commands("hello period world"), "hello. world");
    }

    #[test]
    fn en_comma_question_combo() {
        assert_eq!(
            replace_commands("hello comma is this question mark"),
            "hello, is this?",
        );
    }

    #[test]
    fn en_new_paragraph_inserts_double_newline() {
        assert_eq!(replace_commands("first new paragraph second"), "first\n\n second");
    }

    #[test]
    fn pt_virgula_pontofinal() {
        assert_eq!(
            replace_commands("olá vírgula mundo ponto final"),
            "olá, mundo.",
        );
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(replace_commands("hello PERIOD"), "hello.");
    }

    #[test]
    fn does_not_match_inside_word() {
        // "comma" inside "Komodo" – fabricated edge: real test is "commando" -> "commando"
        assert_eq!(replace_commands("commando squad"), "commando squad");
    }

    #[test]
    fn pt_ponto_final_takes_priority_over_ponto() {
        // "ponto final" is listed before "ponto" so two-word match wins.
        assert_eq!(replace_commands("isto ponto final"), "isto.");
    }

    #[test]
    fn empty_input_returns_empty() {
        assert_eq!(replace_commands(""), "");
    }
}
