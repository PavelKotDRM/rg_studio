use std::ffi::OsString;

use crate::options::{Category, OPTIONS};

/// Mutable state used to compose a ripgrep command.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommandState {
    /// Search pattern passed to ripgrep.
    pub pattern: String,
    /// Path in which ripgrep should search.
    pub path: String,
    /// Option arguments to pass before the pattern and path.
    pub options: Vec<String>,
    /// Advanced command-line arguments, parsed without invoking a shell.
    pub extra_arguments: String,
}

impl CommandState {
    /// Returns the command formatted for the current operating system shell.
    #[must_use]
    pub fn command(&self) -> Result<String, String> {
        let extra_arguments = shlex::split(&self.extra_arguments)
            .ok_or_else(|| "Additional arguments contain unmatched quotes.".to_owned())?;
        let mut parts = vec!["rg".to_owned()];
        parts.extend(self.options.iter().map(|value| quote(value)));
        parts.extend(extra_arguments.iter().map(|value| quote(value)));
        if !self.pattern.is_empty() {
            parts.push("--regexp".to_owned());
            parts.push(quote(&self.pattern));
        }
        if !self.path.is_empty() {
            parts.push("--".to_owned());
            parts.push(quote(&self.path));
        }
        Ok(parts.join(" "))
    }

    /// Returns process arguments for executing ripgrep without invoking a shell.
    pub fn arguments(&self) -> Result<Vec<OsString>, String> {
        let extra_arguments = shlex::split(&self.extra_arguments)
            .ok_or_else(|| "Additional arguments contain unmatched quotes.".to_owned())?;
        let mut arguments = self
            .options
            .iter()
            .cloned()
            .chain(extra_arguments)
            .map(OsString::from)
            .collect::<Vec<_>>();

        let has_pattern = !self.pattern.is_empty()
            || arguments.iter().any(|argument| {
                let Some(argument) = argument.to_str() else {
                    return false;
                };
                argument == "--regexp"
                    || argument == "-e"
                    || argument.starts_with("--regexp=")
                    || (argument.starts_with("-e") && argument.len() > 2)
                    || argument == "--file"
                    || argument == "-f"
                    || argument.starts_with("--file=")
                    || (argument.starts_with("-f") && argument.len() > 2)
            });
        let patternless_mode = arguments.iter().any(|argument| {
            matches!(
                argument.to_str(),
                Some(
                    "--files"
                        | "--help"
                        | "-h"
                        | "--version"
                        | "-V"
                        | "--type-list"
                        | "--pcre2-version"
                        | "--generate"
                )
            ) || argument
                .to_str()
                .is_some_and(|value| value.starts_with("--generate="))
        });

        if !has_pattern && !patternless_mode {
            return Err("Enter a pattern or select a patternless ripgrep mode.".to_owned());
        }

        if !self.pattern.is_empty() {
            arguments.push(OsString::from("--regexp"));
            arguments.push(OsString::from(&self.pattern));
        }
        if !self.path.is_empty() {
            arguments.push(OsString::from("--"));
            arguments.push(OsString::from(&self.path));
        }

        Ok(arguments)
    }

    #[cfg(test)]
    pub(crate) fn supports_readable_results(&self) -> bool {
        if self.pattern.is_empty() || !self.extra_arguments.trim().is_empty() {
            return false;
        }
        if !self.options.iter().any(|option| {
            option
                .split_once('=')
                .map_or(option.as_str(), |(flag, _)| flag)
                == "--with-filename"
        }) && !self.path.is_empty()
            && std::path::Path::new(&self.path).is_file()
        {
            return false;
        }
        self.options.iter().all(|option| {
            let flag = option
                .split_once('=')
                .map_or(option.as_str(), |(flag, _)| flag);
            if matches!(
                flag,
                "--after-context"
                    | "--auto-hybrid-regex"
                    | "--before-context"
                    | "--colors"
                    | "--context"
                    | "--context-separator"
                    | "--count"
                    | "--count-matches"
                    | "--debug"
                    | "--engine"
                    | "--field-context-separator"
                    | "--field-match-separator"
                    | "--file"
                    | "--files"
                    | "--files-with-matches"
                    | "--files-without-match"
                    | "--generate"
                    | "--heading"
                    | "--help"
                    | "--hyperlink-format"
                    | "--include-zero"
                    | "--invert-match"
                    | "--json"
                    | "--max-columns"
                    | "--max-columns-preview"
                    | "--multiline"
                    | "--multiline-dotall"
                    | "--hostname-bin"
                    | "--no-filename"
                    | "--no-pcre2-unicode"
                    | "--no-unicode"
                    | "--null"
                    | "--null-data"
                    | "--only-matching"
                    | "--passthru"
                    | "--path-separator"
                    | "--pcre2"
                    | "--pcre2-version"
                    | "--pretty"
                    | "--quiet"
                    | "--regexp"
                    | "--replace"
                    | "--stats"
                    | "--trace"
                    | "--type-list"
                    | "--trim"
                    | "--version"
                    | "--vimgrep"
            ) {
                return false;
            }

            let Some(spec) = OPTIONS.iter().find(|spec| spec.flag == flag) else {
                return false;
            };
            match spec.category {
                Category::Output => {
                    matches!(
                        flag,
                        "--block-buffered"
                            | "--byte-offset"
                            | "--column"
                            | "--line-buffered"
                            | "--line-number"
                            | "--no-line-number"
                            | "--sort"
                            | "--sort-files"
                            | "--sortr"
                            | "--with-filename"
                    ) || (flag == "--color"
                        && option
                            .split_once('=')
                            .is_some_and(|(_, mode)| matches!(mode, "auto" | "never")))
                }
                Category::Modes => {
                    matches!(
                        flag,
                        "--no-config" | "--no-ignore-messages" | "--no-messages"
                    )
                }
                Category::Search => matches!(
                    flag,
                    "--binary"
                        | "--case-sensitive"
                        | "--crlf"
                        | "--dfa-size-limit"
                        | "--fixed-strings"
                        | "--ignore-case"
                        | "--line-regexp"
                        | "--max-count"
                        | "--mmap"
                        | "--null-data"
                        | "--pre"
                        | "--pre-glob"
                        | "--regex-size-limit"
                        | "--search-zip"
                        | "--smart-case"
                        | "--stop-on-nonmatch"
                        | "--text"
                        | "--threads"
                        | "--word-regexp"
                ),
                Category::Files => true,
            }
        })
    }

    pub(crate) fn uses_external_ripgrep_config(&self) -> bool {
        std::env::var_os("RIPGREP_CONFIG_PATH").is_some()
            && !self.options.iter().any(|option| {
                option
                    .split_once('=')
                    .map_or(option.as_str(), |(flag, _)| flag)
                    == "--no-config"
            })
    }

    pub(crate) fn supports_raw_match_highlighting(&self) -> bool {
        if self.pattern.is_empty()
            || !self.extra_arguments.trim().is_empty()
            || self.uses_external_ripgrep_config()
        {
            return false;
        }

        self.options.iter().all(|option| {
            let flag = option
                .split_once('=')
                .map_or(option.as_str(), |(flag, _)| flag);
            let Some(spec) = OPTIONS.iter().find(|spec| spec.flag == flag) else {
                return false;
            };
            match spec.category {
                Category::Output => {
                    !matches!(flag, "--include-zero" | "--quiet" | "--replace")
                        && (flag != "--color"
                            || option.split_once('=').is_some_and(|(_, mode)| {
                                matches!(mode, "auto" | "always" | "ansi")
                            }))
                }
                Category::Modes => matches!(
                    flag,
                    "--debug"
                        | "--json"
                        | "--no-config"
                        | "--no-ignore-messages"
                        | "--no-messages"
                        | "--trace"
                ),
                Category::Search => matches!(
                    flag,
                    "--case-sensitive"
                        | "--crlf"
                        | "--dfa-size-limit"
                        | "--fixed-strings"
                        | "--ignore-case"
                        | "--line-regexp"
                        | "--max-count"
                        | "--mmap"
                        | "--null-data"
                        | "--pre"
                        | "--pre-glob"
                        | "--regex-size-limit"
                        | "--search-zip"
                        | "--smart-case"
                        | "--stop-on-nonmatch"
                        | "--text"
                        | "--threads"
                        | "--word-regexp"
                ),
                Category::Files => true,
            }
        })
    }
}

#[cfg(windows)]
fn quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_./:=+,-".contains(character))
    {
        return value.to_owned();
    }

    format!("\"{}\"", value.replace('"', "\\\""))
}

#[cfg(not(windows))]
fn quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "_./:=+,-".contains(character))
    {
        return value.to_owned();
    }

    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use crate::options::Category;

    use super::CommandState;

    #[test]
    fn composes_options_pattern_and_path() {
        let state = CommandState {
            pattern: "hello world".into(),
            path: "src folder".into(),
            options: vec!["--ignore-case".into(), "--glob=*.rs".into()],
            extra_arguments: String::new(),
        };

        let command = state.command().unwrap();
        assert!(command.starts_with("rg --ignore-case "));
        assert!(command.contains("--glob=*.rs"));
        assert!(command.contains("hello world"));
        assert!(command.contains("src folder"));
        assert!(command.contains("--regexp"));
        assert!(command.contains(" -- "));
    }

    #[test]
    fn omits_empty_positional_arguments() {
        assert_eq!(CommandState::default().command().unwrap(), "rg");
    }

    #[test]
    fn creates_shell_free_arguments_with_spaces_preserved() {
        let state = CommandState {
            pattern: "hello world".into(),
            path: "source files".into(),
            options: vec!["--ignore-case".into()],
            extra_arguments: "--glob \"*.rs\"".into(),
        };

        assert_eq!(
            state.arguments().unwrap(),
            [
                "--ignore-case",
                "--glob",
                "*.rs",
                "--regexp",
                "hello world",
                "--",
                "source files"
            ]
            .map(OsString::from)
        );
    }

    #[test]
    fn generated_command_keeps_file_globs_separate_from_the_regex_and_path() {
        let state = CommandState {
            pattern: "main".into(),
            path: "-src".into(),
            options: vec!["--glob=*.rs".into()],
            ..CommandState::default()
        };

        let arguments = state.arguments().unwrap();
        assert_eq!(
            arguments,
            ["--glob=*.rs", "--regexp", "main", "--", "-src"].map(OsString::from)
        );

        let command = state.command().unwrap();
        assert!(command.contains("--glob=*.rs"));
        assert!(command.contains("--regexp main --"));
    }

    #[test]
    fn command_contains_only_explicit_output_options() {
        let state = CommandState {
            pattern: "needle".into(),
            path: "source.txt".into(),
            ..CommandState::default()
        };

        assert_eq!(
            state.arguments().unwrap(),
            ["--regexp", "needle", "--", "source.txt"].map(OsString::from)
        );
        assert!(state.supports_readable_results());
    }

    #[test]
    fn coordinate_fields_are_added_only_when_the_options_are_selected() {
        let state = CommandState {
            pattern: "needle".into(),
            path: "source.txt".into(),
            options: vec![
                "--line-number".into(),
                "--with-filename".into(),
                "--column".into(),
                "--byte-offset".into(),
            ],
            ..CommandState::default()
        };

        assert_eq!(
            state.arguments().unwrap(),
            [
                "--line-number",
                "--with-filename",
                "--column",
                "--byte-offset",
                "--regexp",
                "needle",
                "--",
                "source.txt"
            ]
            .map(OsString::from)
        );
    }

    #[test]
    fn context_output_does_not_add_unselected_coordinate_flags() {
        let state = CommandState {
            pattern: "needle".into(),
            path: "source.txt".into(),
            options: vec!["--after-context=1".into()],
            ..CommandState::default()
        };

        assert_eq!(
            state.arguments().unwrap(),
            [
                "--after-context=1",
                "--regexp",
                "needle",
                "--",
                "source.txt"
            ]
            .map(OsString::from)
        );
        assert!(!state.supports_readable_results());
    }

    #[test]
    fn heading_vimgrep_and_pretty_modes_do_not_add_unselected_flags() {
        for option in ["--heading", "--vimgrep", "--pretty"] {
            let state = CommandState {
                pattern: "needle".into(),
                path: "source.txt".into(),
                options: vec![option.into()],
                ..CommandState::default()
            };
            let arguments = state.arguments().unwrap();

            assert_eq!(
                arguments,
                [option, "--regexp", "needle", "--", "source.txt"].map(OsString::from)
            );
            assert!(!state.supports_readable_results());
        }
    }

    #[test]
    fn preserves_explicit_no_line_number_setting() {
        let state = CommandState {
            pattern: "needle".into(),
            options: vec!["--no-line-number".into()],
            ..CommandState::default()
        };

        assert_eq!(
            state.arguments().unwrap(),
            ["--no-line-number", "--regexp", "needle"].map(OsString::from)
        );
        assert!(state.supports_readable_results());
    }

    #[test]
    fn supports_terminal_auto_and_never_color_modes_but_not_ansi_modes() {
        for mode in ["auto", "never"] {
            let state = CommandState {
                pattern: "needle".into(),
                options: vec![format!("--color={mode}")],
                ..CommandState::default()
            };
            assert!(state.supports_readable_results(), "mode: {mode}");
        }

        for mode in ["always", "ansi"] {
            let state = CommandState {
                pattern: "needle".into(),
                options: vec![format!("--color={mode}")],
                ..CommandState::default()
            };
            assert!(!state.supports_readable_results(), "mode: {mode}");
        }
    }

    #[test]
    fn ansi_color_output_does_not_add_unselected_coordinates() {
        for mode in ["always", "ansi"] {
            let state = CommandState {
                pattern: "needle".into(),
                options: vec![format!("--color={mode}")],
                ..CommandState::default()
            };

            assert_eq!(
                state.arguments().unwrap(),
                [
                    format!("--color={mode}"),
                    "--regexp".into(),
                    "needle".into()
                ]
                .map(OsString::from)
            );
            assert!(!state.supports_readable_results());
        }
    }

    #[test]
    fn raw_highlighting_covers_text_formats_but_skips_count_and_file_list_modes() {
        for option in [
            "--after-context=2",
            "--byte-offset",
            "--color=auto",
            "--color=always",
            "--column",
            "--context=3",
            "--heading",
            "--json",
            "--null",
            "--null-data",
            "--only-matching",
            "--vimgrep",
        ] {
            let state = CommandState {
                pattern: "needle".into(),
                options: vec![option.into()],
                ..CommandState::default()
            };
            assert!(
                state.supports_raw_match_highlighting(),
                "raw output should retain match highlighting for {option}"
            );
        }

        for option in [
            "--count",
            "--files",
            "--files-with-matches",
            "--files-without-match",
            "--invert-match",
            "--no-unicode",
            "--pcre2",
            "--quiet",
        ] {
            let state = CommandState {
                pattern: "needle".into(),
                options: vec![option.into()],
                ..CommandState::default()
            };
            assert!(
                !state.supports_raw_match_highlighting(),
                "raw output should not guess match spans for {option}"
            );
        }
    }

    #[test]
    fn raw_highlighting_classifies_every_catalogued_option() {
        for spec in crate::options::OPTIONS {
            let state = CommandState {
                pattern: "needle".into(),
                options: vec![spec.flag.to_owned()],
                ..CommandState::default()
            };
            let expected = match spec.category {
                Category::Output => !matches!(
                    spec.flag,
                    "--color" | "--include-zero" | "--quiet" | "--replace"
                ),
                Category::Modes => matches!(
                    spec.flag,
                    "--debug"
                        | "--json"
                        | "--no-config"
                        | "--no-ignore-messages"
                        | "--no-messages"
                        | "--trace"
                ),
                Category::Search => matches!(
                    spec.flag,
                    "--case-sensitive"
                        | "--crlf"
                        | "--dfa-size-limit"
                        | "--fixed-strings"
                        | "--ignore-case"
                        | "--line-regexp"
                        | "--max-count"
                        | "--mmap"
                        | "--null-data"
                        | "--pre"
                        | "--pre-glob"
                        | "--regex-size-limit"
                        | "--search-zip"
                        | "--smart-case"
                        | "--stop-on-nonmatch"
                        | "--text"
                        | "--threads"
                        | "--word-regexp"
                ),
                Category::Files => true,
            };

            assert_eq!(
                state.supports_raw_match_highlighting(),
                expected,
                "unexpected raw highlighting handling for {}",
                spec.flag
            );
        }
    }

    #[test]
    fn classifies_all_catalogued_options() {
        for spec in crate::options::OPTIONS {
            let state = CommandState {
                pattern: "needle".into(),
                options: vec![spec.flag.to_owned()],
                ..CommandState::default()
            };
            let expected = match spec.category {
                Category::Output => matches!(
                    spec.flag,
                    "--block-buffered"
                        | "--byte-offset"
                        | "--column"
                        | "--line-buffered"
                        | "--line-number"
                        | "--no-line-number"
                        | "--sort"
                        | "--sort-files"
                        | "--sortr"
                        | "--with-filename"
                ),
                Category::Modes => {
                    matches!(
                        spec.flag,
                        "--no-config" | "--no-ignore-messages" | "--no-messages"
                    )
                }
                Category::Search => matches!(
                    spec.flag,
                    "--binary"
                        | "--case-sensitive"
                        | "--crlf"
                        | "--dfa-size-limit"
                        | "--fixed-strings"
                        | "--ignore-case"
                        | "--line-regexp"
                        | "--max-count"
                        | "--mmap"
                        | "--pre"
                        | "--pre-glob"
                        | "--regex-size-limit"
                        | "--search-zip"
                        | "--smart-case"
                        | "--stop-on-nonmatch"
                        | "--text"
                        | "--threads"
                        | "--word-regexp"
                ),
                Category::Files => true,
            };

            assert_eq!(
                state.supports_readable_results(),
                expected,
                "unexpected handling for {}",
                spec.flag
            );
        }
    }

    #[test]
    fn rejects_invalid_extra_argument_quotes_and_missing_pattern() {
        let invalid_quotes = CommandState {
            extra_arguments: "\"unterminated".into(),
            ..CommandState::default()
        };
        assert!(invalid_quotes.arguments().is_err());
        assert!(invalid_quotes.command().is_err());

        assert!(CommandState::default().arguments().is_err());
    }

    #[test]
    fn permits_patternless_file_listing() {
        let state = CommandState {
            options: vec!["--files".into()],
            path: "src folder".into(),
            ..CommandState::default()
        };

        assert_eq!(
            state.arguments().unwrap(),
            ["--files", "--", "src folder"].map(OsString::from)
        );
    }

    #[test]
    fn permits_patterns_loaded_from_a_file() {
        let state = CommandState {
            options: vec!["--file=patterns.txt".into()],
            path: "source files".into(),
            ..CommandState::default()
        };

        assert_eq!(
            state.arguments().unwrap(),
            ["--file=patterns.txt", "--", "source files"].map(OsString::from)
        );
    }

    #[cfg(windows)]
    #[test]
    fn preserves_quoted_windows_paths_in_additional_arguments() {
        let state = CommandState {
            pattern: "needle".into(),
            extra_arguments: r#"--pre "C:\Program Files\filters\pre.cmd""#.into(),
            ..CommandState::default()
        };

        let arguments = state.arguments().unwrap();
        assert_eq!(arguments[0], "--pre");
        assert_eq!(arguments[1], r"C:\Program Files\filters\pre.cmd");
    }
}
