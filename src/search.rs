use std::{
    collections::HashSet,
    ffi::OsString,
    ops::Range,
    process::Command,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use eframe::egui::Context;
use regex::{Regex, RegexBuilder};

use crate::command::CommandState;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const OUTPUT_PAGE_BYTES: usize = 64 * 1024;
const READABLE_RESULT_LIMIT: usize = 200;
const READABLE_RESULT_PREVIEW_CHARS: usize = 220;

pub(crate) struct SearchOutput {
    pub(crate) executable: String,
    pub(crate) command: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) success: bool,
    pub(crate) elapsed: Duration,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) readable_results: Option<ReadableResults>,
    pub(crate) stdout_pages: Vec<Range<usize>>,
    pub(crate) stderr_pages: Vec<Range<usize>>,
}

pub(crate) struct ReadableResults {
    pub(crate) total_lines: usize,
    pub(crate) file_count: usize,
    pub(crate) lines: Vec<ReadableResult>,
    pub(crate) truncated: bool,
}

pub(crate) struct ReadableResult {
    pub(crate) path: String,
    pub(crate) line_number: Option<usize>,
    pub(crate) preview: String,
    pub(crate) match_range: Option<Range<usize>>,
    pub(crate) shortened: bool,
}

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub(crate) enum OutputStream {
    #[default]
    Stdout,
    Stderr,
}

#[derive(Default)]
pub(crate) struct SearchController {
    pub(crate) running: bool,
    pub(crate) show_results: bool,
    pub(crate) output: Option<SearchOutput>,
    pub(crate) error: Option<String>,
    pub(crate) output_page: usize,
    pub(crate) output_stream: OutputStream,
    receiver: Option<Receiver<Result<SearchOutput, String>>>,
}

impl SearchController {
    pub(crate) fn start(&mut self, state: CommandState) {
        if self.running {
            return;
        }

        self.show_results = true;
        self.output = None;
        self.error = None;
        self.output_page = 0;
        self.output_stream = OutputStream::Stdout;

        let (sender, receiver) = mpsc::channel();
        let worker = thread::Builder::new()
            .name("rg-studio-search".to_owned())
            .spawn(move || {
                let _ = sender.send(execute(&state));
            });

        match worker {
            Ok(_) => {
                self.running = true;
                self.receiver = Some(receiver);
            }
            Err(error) => {
                self.error = Some(format!("Unable to start the search worker: {error}"));
                self.receiver = None;
            }
        }
    }

    pub(crate) fn poll(&mut self, context: &Context) {
        let Some(receiver) = &self.receiver else {
            return;
        };

        match receiver.try_recv() {
            Ok(Ok(output)) => {
                self.output_stream = if !output.success && !output.stderr.is_empty() {
                    OutputStream::Stderr
                } else {
                    OutputStream::Stdout
                };
                self.output = Some(output);
                self.error = None;
                self.running = false;
                self.receiver = None;
            }
            Ok(Err(error)) => {
                self.output = None;
                self.error = Some(error);
                self.running = false;
                self.receiver = None;
            }
            Err(TryRecvError::Empty) => {
                context.request_repaint_after(Duration::from_millis(80));
            }
            Err(TryRecvError::Disconnected) => {
                self.output = None;
                self.error = Some("The ripgrep worker stopped without returning a result.".into());
                self.running = false;
                self.receiver = None;
            }
        }
    }
}

fn execute(state: &CommandState) -> Result<SearchOutput, String> {
    let arguments = state.arguments()?;
    let readable_line_numbers = readable_output_line_numbers(state);
    let command = state.command()?;
    let executable = resolve_executable();
    let mut process = Command::new(&executable);
    process.args(arguments);
    #[cfg(windows)]
    process.creation_flags(CREATE_NO_WINDOW);

    let started_at = Instant::now();
    let output = process.output().map_err(|error| {
        format!(
            "Unable to start ripgrep ({}): {error}. Build the bundled CLI or install `rg` in PATH.",
            executable.display()
        )
    })?;
    let elapsed = started_at.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let readable_results = readable_line_numbers
        .and_then(|line_numbers| parse_readable_results(state, &stdout, line_numbers));

    Ok(SearchOutput {
        executable: executable.to_string_lossy().into_owned(),
        command,
        exit_code: output.status.code(),
        success: output.status.success(),
        elapsed,
        readable_results,
        stdout_pages: output_page_ranges(&stdout),
        stderr_pages: output_page_ranges(&stderr),
        stdout,
        stderr,
    })
}

fn readable_output_line_numbers(state: &CommandState) -> Option<bool> {
    if !state.extra_arguments.trim().is_empty() {
        return None;
    }

    let mut line_numbers = false;
    for option in &state.options {
        let flag = option
            .split_once('=')
            .map_or(option.as_str(), |(flag, _)| flag);
        match flag {
            "--line-number" => line_numbers = true,
            "--no-line-number" => line_numbers = false,
            "--after-context"
            | "--before-context"
            | "--byte-offset"
            | "--color"
            | "--colors"
            | "--column"
            | "--context"
            | "--context-separator"
            | "--count"
            | "--count-matches"
            | "--debug"
            | "--field-context-separator"
            | "--field-match-separator"
            | "--files"
            | "--files-with-matches"
            | "--files-without-match"
            | "--generate"
            | "--heading"
            | "--help"
            | "--hyperlink-format"
            | "--json"
            | "--max-columns"
            | "--max-columns-preview"
            | "--multiline"
            | "--multiline-dotall"
            | "--no-filename"
            | "--null"
            | "--null-data"
            | "--only-matching"
            | "--passthru"
            | "--path-separator"
            | "--pcre2-version"
            | "--pretty"
            | "--quiet"
            | "--replace"
            | "--stats"
            | "--trace"
            | "--type-list"
            | "--version"
            | "--vimgrep" => return None,
            _ => {}
        }
    }

    Some(line_numbers)
}

fn parse_readable_results(
    state: &CommandState,
    output: &str,
    line_numbers: bool,
) -> Option<ReadableResults> {
    let total_lines = output.lines().count();
    let matcher = result_matcher(state);
    let mut lines = Vec::with_capacity(total_lines.min(READABLE_RESULT_LIMIT));
    let mut files = HashSet::new();

    for line in output.lines().take(READABLE_RESULT_LIMIT) {
        let path_separator = result_path_separator(line, &state.path)?;
        let path = &line[..path_separator];
        if path.is_empty() {
            return None;
        }
        files.insert(path);

        let output_text = &line[path_separator + 1..];
        let (line_number, content) = if line_numbers {
            let (number, content) = output_text.split_once(':')?;
            (Some(number.parse().ok()?), content)
        } else {
            (None, output_text)
        };
        let (preview, match_range, shortened) = result_excerpt(content, matcher.as_ref());

        lines.push(ReadableResult {
            path: path.to_owned(),
            line_number,
            preview,
            match_range,
            shortened,
        });
    }

    Some(ReadableResults {
        total_lines,
        file_count: files.len(),
        truncated: total_lines > lines.len(),
        lines,
    })
}

fn result_path_separator(line: &str, search_path: &str) -> Option<usize> {
    let search_path = search_path.trim_end_matches(['\\', '/']);
    if !search_path.is_empty()
        && let Some(rest) = line.strip_prefix(search_path)
    {
        let offset = if rest.starts_with(['\\', '/']) {
            search_path.len() + 1
        } else if rest.starts_with(':') {
            search_path.len()
        } else {
            0
        };
        if offset > 0
            && let Some(separator) = line[offset..].find(':')
        {
            return Some(offset + separator);
        }
    }

    let start = if cfg!(windows)
        && line.as_bytes().get(1) == Some(&b':')
        && line
            .as_bytes()
            .get(2)
            .is_some_and(|byte| matches!(byte, b'\\' | b'/'))
    {
        2
    } else {
        0
    };
    line[start..].find(':').map(|separator| start + separator)
}

fn result_matcher(state: &CommandState) -> Option<Regex> {
    if state.pattern.is_empty() {
        return None;
    }

    let fixed_strings = state
        .options
        .iter()
        .any(|option| option == "--fixed-strings");
    let ignore_case = state.options.iter().any(|option| option == "--ignore-case");
    let smart_case = state.options.iter().any(|option| option == "--smart-case");
    let pattern = if fixed_strings {
        regex::escape(&state.pattern)
    } else {
        state.pattern.clone()
    };
    let mut builder = RegexBuilder::new(&pattern);
    builder.case_insensitive(
        ignore_case || (smart_case && !state.pattern.chars().any(char::is_uppercase)),
    );
    builder.build().ok()
}

fn result_excerpt(line: &str, matcher: Option<&Regex>) -> (String, Option<Range<usize>>, bool) {
    let match_range = matcher.and_then(|matcher| matcher.find(line).map(|matched| matched.range()));
    let character_count = line.chars().count();
    if character_count <= READABLE_RESULT_PREVIEW_CHARS {
        return (line.to_owned(), match_range, false);
    }

    let focus_byte = match_range.as_ref().map_or(0, |matched| matched.start);
    let focus_character = line[..focus_byte].chars().count();
    let start_character = focus_character.saturating_sub(READABLE_RESULT_PREVIEW_CHARS / 3);
    let end_character = (start_character + READABLE_RESULT_PREVIEW_CHARS).min(character_count);
    let start_byte = byte_index_at_character(line, start_character);
    let end_byte = byte_index_at_character(line, end_character);
    let prefix = if start_byte > 0 { "..." } else { "" };
    let suffix = if end_byte < line.len() { "..." } else { "" };
    let preview = format!("{prefix}{}{suffix}", &line[start_byte..end_byte]);
    let preview_match = match_range.and_then(|matched| {
        let start = matched.start.max(start_byte);
        let end = matched.end.min(end_byte);
        (start <= end && start >= start_byte && end <= end_byte)
            .then_some(prefix.len() + start - start_byte..prefix.len() + end - start_byte)
    });

    (preview, preview_match, true)
}

fn byte_index_at_character(text: &str, character: usize) -> usize {
    text.char_indices()
        .nth(character)
        .map_or(text.len(), |(byte, _)| byte)
}

fn output_page_ranges(output: &str) -> Vec<Range<usize>> {
    let mut pages = Vec::new();
    let mut start = 0;

    while start < output.len() {
        let mut end = start.saturating_add(OUTPUT_PAGE_BYTES).min(output.len());
        if end < output.len() {
            while !output.is_char_boundary(end) {
                end -= 1;
            }
            if let Some(newline) = output[start..end].rfind('\n') {
                let line_end = start + newline + 1;
                if line_end - start >= OUTPUT_PAGE_BYTES / 2 {
                    end = line_end;
                }
            }
        }

        pages.push(start..end);
        start = end;
    }

    pages
}

fn resolve_executable() -> OsString {
    let bundled_name = if cfg!(windows) { "rg.exe" } else { "rg" };
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(parent) = current_exe.parent()
    {
        let mut directories = vec![parent];
        if let Some(parent) = parent.parent() {
            directories.push(parent);
        }
        for directory in directories {
            let path = directory.join(bundled_name);
            if path.is_file() {
                return path.into_os_string();
            }
        }
    }

    OsString::from("rg")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::mpsc;

    use crate::command::CommandState;

    use super::{
        OUTPUT_PAGE_BYTES, OutputStream, SearchController, SearchOutput, execute,
        output_page_ranges, parse_readable_results, readable_output_line_numbers,
        resolve_executable,
    };

    #[test]
    fn resolves_a_bundled_binary_or_path_command() {
        let executable = resolve_executable();
        assert!(!executable.is_empty());
        let path = PathBuf::from(&executable);
        if path.is_absolute() {
            assert_eq!(
                path.file_name().unwrap(),
                if cfg!(windows) { "rg.exe" } else { "rg" }
            );
        } else {
            assert_eq!(executable, "rg");
        }
    }

    #[test]
    fn bundled_binary_path_is_a_sibling_of_the_application() {
        let executable = resolve_executable();
        let path = PathBuf::from(executable);
        if path.is_absolute() {
            assert!(
                path.file_name()
                    .is_some_and(|name| { name == if cfg!(windows) { "rg.exe" } else { "rg" } })
            );
        }
    }

    #[test]
    fn executes_ripgrep_and_captures_its_stdout() {
        let state = CommandState {
            pattern: "fn main".into(),
            path: "src".into(),
            options: vec!["--glob=*.rs".into()],
            ..CommandState::default()
        };

        let result = execute(&state).expect("ripgrep should start");
        assert!(result.success, "ripgrep failed: {}", result.stderr);
        assert!(result.stdout.contains("fn main"));
        assert_eq!(result.command, state.command().unwrap());
        assert!(
            result
                .readable_results
                .as_ref()
                .is_some_and(|results| !results.lines.is_empty())
        );
    }

    #[test]
    fn captures_a_clear_diagnostic_for_an_invalid_regex() {
        let state = CommandState {
            pattern: "*.".into(),
            path: "src".into(),
            ..CommandState::default()
        };

        let result = execute(&state).expect("ripgrep should start");
        assert!(!result.success);
        assert!(result.stderr.contains("regex parse error:"));
        assert!(
            result
                .stderr
                .contains("repetition operator missing expression")
        );
    }

    #[test]
    fn selects_diagnostics_automatically_when_search_fails() {
        let stderr = "regex parse error".to_owned();
        let stderr_pages = output_page_ranges(&stderr);
        let output = SearchOutput {
            executable: "rg".into(),
            command: "rg --regexp *.".into(),
            exit_code: Some(2),
            success: false,
            elapsed: std::time::Duration::ZERO,
            stdout: String::new(),
            stderr,
            readable_results: None,
            stdout_pages: Vec::new(),
            stderr_pages,
        };
        let (sender, receiver) = mpsc::channel::<Result<SearchOutput, String>>();
        sender.send(Ok(output)).unwrap();
        let mut controller = SearchController {
            running: true,
            receiver: Some(receiver),
            ..SearchController::default()
        };

        controller.poll(&eframe::egui::Context::default());

        assert_eq!(controller.output_stream, OutputStream::Stderr);
    }

    #[test]
    fn formats_long_windows_results_as_file_and_match_excerpt() {
        let first = format!(r"D:\work\rg.d:D:\work\rg.exe: {}", "dependency ".repeat(40));
        let second = format!(
            r"D:\work\deps\rg.d:D:\work\deps\rg.exe: {}",
            "dependency ".repeat(40)
        );
        let output = format!("{first}\n{second}");
        let state = CommandState {
            pattern: "rg[.]exe".into(),
            path: r"D:\work".into(),
            options: vec!["--word-regexp".into()],
            ..CommandState::default()
        };

        let results = parse_readable_results(&state, &output, false).unwrap();
        let result = &results.lines[0];
        let matched = result.match_range.as_ref().unwrap();

        assert_eq!(results.total_lines, 2);
        assert_eq!(results.file_count, 2);
        assert_eq!(result.path, r"D:\work\rg.d");
        assert!(result.shortened);
        assert_eq!(&result.preview[matched.clone()], "rg.exe");
    }

    #[test]
    fn preserves_line_numbers_in_readable_results() {
        let state = CommandState {
            pattern: "needle".into(),
            path: r"D:\work".into(),
            options: vec!["--line-number".into()],
            ..CommandState::default()
        };
        let results = parse_readable_results(
            &state,
            r"D:\work\sample.txt:42:a needle in the line",
            readable_output_line_numbers(&state).unwrap(),
        )
        .unwrap();

        assert_eq!(results.lines[0].path, r"D:\work\sample.txt");
        assert_eq!(results.lines[0].line_number, Some(42));
        assert_eq!(results.lines[0].preview, "a needle in the line");
    }

    #[test]
    fn leaves_nonstandard_ripgrep_output_unformatted() {
        let state = CommandState {
            pattern: "needle".into(),
            path: "src".into(),
            options: vec!["--json".into()],
            ..CommandState::default()
        };

        assert!(parse_readable_results(&state, "{}", false).is_none());
    }

    #[test]
    fn output_pages_cover_text_without_splitting_utf8() {
        let text = format!(
            "{}\n{}",
            "a".repeat(OUTPUT_PAGE_BYTES / 2),
            "🙂".repeat(OUTPUT_PAGE_BYTES / 2)
        );
        let pages = output_page_ranges(&text);
        let mut next_start = 0;

        assert!(text[..pages[0].end].ends_with('\n'));
        for page in pages {
            assert_eq!(page.start, next_start);
            assert!(page.end > page.start);
            assert!(page.end - page.start <= OUTPUT_PAGE_BYTES);
            assert!(text.is_char_boundary(page.start));
            assert!(text.is_char_boundary(page.end));
            next_start = page.end;
        }

        assert_eq!(next_start, text.len());
    }

    #[test]
    fn empty_output_has_no_pages() {
        assert!(output_page_ranges("").is_empty());
    }
}
