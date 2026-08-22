/// Mutable state used to compose a ripgrep command.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommandState {
    /// Search pattern passed to ripgrep.
    pub pattern: String,
    /// Path in which ripgrep should search.
    pub path: String,
    /// Already validated option arguments, without the executable name.
    pub options: Vec<String>,
    /// Advanced arguments appended verbatim before positional arguments.
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
}
