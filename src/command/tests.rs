use std::ffi::OsString;

use crate::options::Category;

use super::CommandState;

#[cfg(windows)]
#[test]
fn quotes_values_for_powershell() {
    assert_eq!(super::quote("a\"b's"), "'a\"b''s'");
}

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
