use std::ops::Range;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PreviewOptions {
    pub(crate) case_insensitive: bool,
    pub(crate) fixed_strings: bool,
    pub(crate) multi_line: bool,
    pub(crate) dot_matches_new_line: bool,
    pub(crate) no_unicode: bool,
    pub(crate) whole_word: bool,
    pub(crate) whole_line: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PreviewResult {
    EmptyPattern,
    UnsupportedEngine,
    Invalid(String),
    Matches(Vec<Range<usize>>),
}

pub(crate) fn evaluate(pattern: &str, text: &str, options: PreviewOptions) -> PreviewResult {
    if pattern.is_empty() {
        return PreviewResult::EmptyPattern;
    }

    let mut expression = if options.fixed_strings {
        regex::escape(pattern)
    } else {
        pattern.to_owned()
    };

    if options.whole_word {
        expression = format!(r"\b(?:{expression})\b");
    }
    if options.whole_line {
        expression = format!("^(?:{expression})$");
    }

    let mut builder = regex::RegexBuilder::new(&expression);
    builder
        .case_insensitive(options.case_insensitive)
        .multi_line(options.multi_line || options.dot_matches_new_line)
        .dot_matches_new_line(options.dot_matches_new_line)
        .unicode(!options.no_unicode);

    let regex = match builder.build() {
        Ok(regex) => regex,
        Err(error) => return PreviewResult::Invalid(error.to_string()),
    };

    PreviewResult::Matches(
        regex
            .find_iter(text)
            .map(|matched| matched.range())
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::{PreviewOptions, PreviewResult, evaluate};

    #[test]
    fn finds_match_ranges_in_the_sample_text() {
        let result = evaluate("TODO", "TODO then TODO", PreviewOptions::default());

        assert_eq!(result, PreviewResult::Matches(vec![0..4, 10..14]));
    }

    #[test]
    fn supports_case_insensitive_and_fixed_string_previews() {
        let case_insensitive = evaluate(
            "todo",
            "TODO todo",
            PreviewOptions {
                case_insensitive: true,
                ..PreviewOptions::default()
            },
        );
        assert_eq!(case_insensitive, PreviewResult::Matches(vec![0..4, 5..9]));

        let fixed = evaluate(
            "a+b",
            "a+b aaab",
            PreviewOptions {
                fixed_strings: true,
                ..PreviewOptions::default()
            },
        );
        assert_eq!(
            fixed,
            PreviewResult::Matches(std::iter::once(0..3).collect())
        );
    }

    #[test]
    fn reports_empty_and_invalid_patterns() {
        assert_eq!(
            evaluate("", "sample text", PreviewOptions::default()),
            PreviewResult::EmptyPattern
        );
        assert!(matches!(
            evaluate("(", "sample text", PreviewOptions::default()),
            PreviewResult::Invalid(_)
        ));
    }

    #[test]
    fn applies_word_line_and_multiline_options() {
        let whole_word = evaluate(
            "cat",
            "cat catalog",
            PreviewOptions {
                whole_word: true,
                ..PreviewOptions::default()
            },
        );
        assert_eq!(
            whole_word,
            PreviewResult::Matches(std::iter::once(0..3).collect())
        );

        let whole_line = evaluate(
            "cat",
            "cat\ncatalog\ncat",
            PreviewOptions {
                whole_line: true,
                multi_line: true,
                ..PreviewOptions::default()
            },
        );
        assert_eq!(whole_line, PreviewResult::Matches(vec![0..3, 12..15]));

        let multiline = evaluate(
            "alpha.*omega",
            "alpha\nbeta\nomega",
            PreviewOptions {
                multi_line: true,
                dot_matches_new_line: true,
                ..PreviewOptions::default()
            },
        );
        assert_eq!(
            multiline,
            PreviewResult::Matches(std::iter::once(0..16).collect())
        );
    }
}
