use std::ffi::OsString;

/// Mutable state used to compose a ripgrep command.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommandState {
    /// Search pattern passed to ripgrep.
    pub pattern: String,
    /// Path in which ripgrep should search.
    pub path: String,
    /// Already validated option arguments, without the executable name.
    pub options: Vec<String>,
    /// Advanced command-line arguments, parsed without invoking a shell.
    pub extra_arguments: String,
}

impl CommandState {
    /// Returns the command formatted for the current operating system shell.
    #[must_use]
    pub fn command(&self) -> String {
        let mut parts = vec!["rg".to_owned()];
        parts.extend(self.options.iter().map(|value| quote(value)));
        if !self.extra_arguments.trim().is_empty() {
            parts.push(self.extra_arguments.trim().to_owned());
        }
        if !self.pattern.is_empty() {
            parts.push(quote(&self.pattern));
        }
        if !self.path.is_empty() {
            parts.push(quote(&self.path));
        }
        parts.join(" ")
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

    use super::CommandState;

    #[test]
    fn composes_options_pattern_and_path() {
        let state = CommandState {
            pattern: "hello world".into(),
            path: "src folder".into(),
            options: vec!["--ignore-case".into(), "--glob=*.rs".into()],
            extra_arguments: String::new(),
        };

        let command = state.command();
        assert!(command.starts_with("rg --ignore-case "));
        assert!(command.contains("--glob=*.rs"));
        assert!(command.contains("hello world"));
        assert!(command.contains("src folder"));
    }

    #[test]
    fn omits_empty_positional_arguments() {
        assert_eq!(CommandState::default().command(), "rg");
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
    fn rejects_invalid_extra_argument_quotes_and_missing_pattern() {
        let invalid_quotes = CommandState {
            extra_arguments: "\"unterminated".into(),
            ..CommandState::default()
        };
        assert!(invalid_quotes.arguments().is_err());

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
