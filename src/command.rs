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
            parts.push(quote(&self.resolved_search_path()?));
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
            arguments.push(OsString::from(self.resolved_search_path()?));
        }

        Ok(arguments)
    }

    fn resolved_search_path(&self) -> Result<String, String> {
        expand_search_path(&self.path, std::env::home_dir().as_deref(), |name| {
            std::env::var(name).map_err(|error| {
                format!("Search in: cannot read environment variable '{name}': {error}.")
            })
        })
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

fn expand_search_path(
    path: &str,
    home: Option<&std::path::Path>,
    mut variable: impl FnMut(&str) -> Result<String, String>,
) -> Result<String, String> {
    let mut expanded = String::new();
    let mut remaining = path;
    if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        let home = home
            .and_then(std::path::Path::to_str)
            .filter(|home| !home.is_empty())
            .ok_or_else(|| {
                "Search in: cannot determine a Unicode home directory path.".to_owned()
            })?;
        expanded.push_str(home);
        remaining = &path[1..];
        if !remaining.is_empty() {
            if !home.ends_with(std::path::is_separator) {
                expanded.push(std::path::MAIN_SEPARATOR);
            }
            remaining = &remaining[1..];
        }
    }

    while !remaining.is_empty() {
        let reference = if let Some(tail) = remaining.strip_prefix("${") {
            let end = tail.find('}').ok_or_else(|| {
                "Search in: environment variable reference is missing '}'.".to_owned()
            })?;
            Some((&tail[..end], end + 3))
        } else if let Some(tail) = remaining.strip_prefix('$') {
            let tail = tail
                .get(..4)
                .filter(|prefix| prefix.eq_ignore_ascii_case("env:"))
                .map_or(tail, |_| &tail[4..]);
            let length = tail
                .bytes()
                .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                .count();
            (length > 0).then_some((&tail[..length], remaining.len() - tail.len() + length))
        } else if let Some(tail) = remaining.strip_prefix('%') {
            tail.find('%').map(|end| (&tail[..end], end + 2))
        } else {
            None
        };

        if let Some((name, length)) = reference
            && !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            expanded.push_str(&variable(name)?);
            remaining = &remaining[length..];
        } else {
            let character = remaining
                .chars()
                .next()
                .expect("remaining path is not empty");
            expanded.push(character);
            remaining = &remaining[character.len_utf8()..];
        }
    }

    if !path.is_empty() && expanded.is_empty() {
        return Err("Search in: the expanded path is empty.".to_owned());
    }
    Ok(expanded)
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

    format!("'{}'", value.replace('\'', "''"))
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
mod tests;
