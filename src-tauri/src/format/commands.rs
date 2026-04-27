use regex::Regex;
use std::sync::OnceLock;

struct CommandRule {
    /// Source pattern used to build the regex (case-insensitive).
    /// `\b` boundary enforced at compile time.
    phrase: &'static str,
    replacement: &'static str,
    /// If true, replacement attaches to the previous token (strip preceding space).
    attaches: bool,
}

const RULES: &[CommandRule] = &[
    // English
    CommandRule { phrase: "new paragraph",   replacement: "\n\n", attaches: false },
    CommandRule { phrase: "new line",        replacement: "\n",   attaches: false },
    CommandRule { phrase: "exclamation mark",replacement: "!",    attaches: true  },
    CommandRule { phrase: "question mark",   replacement: "?",    attaches: true  },
    CommandRule { phrase: "period",          replacement: ".",    attaches: true  },
    CommandRule { phrase: "comma",           replacement: ",",    attaches: true  },
    CommandRule { phrase: "bullet",          replacement: "• ",   attaches: false },
    // Portuguese
    CommandRule { phrase: "novo parágrafo",  replacement: "\n\n", attaches: false },
    CommandRule { phrase: "nova linha",      replacement: "\n",   attaches: false },
    CommandRule { phrase: "ponto final",     replacement: ".",    attaches: true  },
    CommandRule { phrase: "ponto",           replacement: ".",    attaches: true  },
    CommandRule { phrase: "vírgula",         replacement: ",",    attaches: true  },
    CommandRule { phrase: "interrogação",    replacement: "?",    attaches: true  },
    CommandRule { phrase: "exclamação",      replacement: "!",    attaches: true  },
];

fn compiled() -> &'static [(Regex, &'static str, bool)] {
    static CACHE: OnceLock<Vec<(Regex, &'static str, bool)>> = OnceLock::new();
    CACHE.get_or_init(|| {
        RULES.iter().map(|r| {
            let escaped = regex::escape(r.phrase);
            // Eat one leading space if present so the replacement attaches
            // cleanly to the previous token (punctuation glues; newlines/bullets
            // also avoid leaving a stray space before the break).
            let _ = r.attaches;
            let pat = format!(r"(?i)\s?\b{}\b", escaped);
            (Regex::new(&pat).expect("static command pattern"), r.replacement, r.attaches)
        }).collect()
    })
}

pub fn replace_commands(text: &str) -> String {
    let mut out = text.to_string();
    for (re, replacement, _attaches) in compiled() {
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
