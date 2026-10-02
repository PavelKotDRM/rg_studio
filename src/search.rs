use std::{
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

#[cfg(test)]
const READABLE_RESULT_PAGE_SIZE: usize = 100;
#[cfg(test)]
const READABLE_RESULT_PREVIEW_CHARS: usize = 220;

pub(crate) struct SearchOutput {
    pub(crate) executable: String,
    pub(crate) command: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) success: bool,
    pub(crate) elapsed: Duration,
    pub(crate) stdout: String,
    pub(crate) stdout_is_utf8: bool,
    pub(crate) stderr: String,
    pub(crate) stderr_is_utf8: bool,
    pub(crate) raw_highlight_matcher: Option<Regex>,
    pub(crate) stdout_pages: Vec<Range<usize>>,
    pub(crate) stderr_pages: Vec<Range<usize>>,
}

#[cfg(test)]
pub(crate) struct ReadableResults {
    pub(crate) total_lines: usize,
    pub(crate) page_offsets: Vec<usize>,
    pub(crate) highlight_raw_output: bool,
    output_fields: OutputFields,
    search_path: String,
    matcher: Regex,
}

#[cfg(test)]
#[derive(Clone, Copy, Default)]
struct OutputFields {
    line_number: bool,
    column: bool,
    byte_offset: bool,
}

#[cfg(test)]
struct ReadableResultParts<'a> {
    path: &'a str,
    line_number: Option<usize>,
    column: Option<usize>,
    byte_offset: Option<usize>,
    content: &'a str,
    content_offset: usize,
}

#[cfg(test)]
pub(crate) struct ReadableResult {
    pub(crate) path: String,
    pub(crate) line_number: Option<usize>,
    pub(crate) column: Option<usize>,
    pub(crate) byte_offset: Option<usize>,
    pub(crate) preview: String,
    pub(crate) match_ranges: Vec<Range<usize>>,
    pub(crate) raw_range: Range<usize>,
    pub(crate) content_range: Range<usize>,
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
    pub(crate) result_id: u64,
    pub(crate) stdout_page: usize,
    pub(crate) stderr_page: usize,
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
        self.result_id = self.result_id.wrapping_add(1);
        self.stdout_page = 0;
        self.stderr_page = 0;
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
                self.output_stream = if !output.success
                    && output.exit_code != Some(1)
                    && !output.stderr.is_empty()
                {
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
    let exit_code = output.status.code();
    let success = output.status.success();
    let (stdout, stdout_is_utf8) = raw_output_text(output.stdout);
    let (stderr, stderr_is_utf8) = raw_output_text(output.stderr);
    let raw_highlight_matcher = if stdout_is_utf8 && state.supports_raw_match_highlighting() {
        result_matcher(state)
    } else {
        None
    };

    Ok(SearchOutput {
        executable: executable.to_string_lossy().into_owned(),
        command,
        exit_code,
        success,
        elapsed,
        raw_highlight_matcher,
        stdout_pages: output_page_ranges(&stdout),
        stderr_pages: output_page_ranges(&stderr),
        stdout_is_utf8,
        stderr_is_utf8,
        stdout,
        stderr,
    })
}

fn raw_output_text(bytes: Vec<u8>) -> (String, bool) {
    let bytes = match String::from_utf8(bytes) {
        Ok(text) => return (text, true),
        Err(error) => error.into_bytes(),
    };

    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut text = String::with_capacity(bytes.len());
    let mut offset = 0;
    while offset < bytes.len() {
        match std::str::from_utf8(&bytes[offset..]) {
            Ok(valid) => {
                push_lossless_text(&mut text, valid);
                break;
            }
            Err(error) => {
                let valid_end = offset + error.valid_up_to();
                let valid = std::str::from_utf8(&bytes[offset..valid_end])
                    .expect("the prefix before a UTF-8 error is valid");
                push_lossless_text(&mut text, valid);
                offset = valid_end;

                let invalid_len = error.error_len().unwrap_or(bytes.len() - offset);
                for byte in &bytes[offset..offset + invalid_len] {
                    text.push_str("\\x");
                    text.push(HEX[(byte >> 4) as usize] as char);
                    text.push(HEX[(byte & 0x0F) as usize] as char);
                }
                offset += invalid_len;
            }
        }
    }
    (text, false)
}

fn push_lossless_text(text: &mut String, valid: &str) {
    for character in valid.chars() {
        if character == '\\' {
            text.push_str("\\\\");
        } else {
            text.push(character);
        }
    }
}

#[cfg(test)]
fn parse_readable_results(state: &CommandState, output: &str) -> Option<ReadableResults> {
    if !state.supports_readable_results() {
        return None;
    }

    let matcher = result_matcher(state)?;
    let output_fields = output_fields(state);
    let mut page_offsets = Vec::new();
    let mut total_lines = 0;
    let mut line_offset = 0;

    for raw_line in output.split_inclusive('\n') {
        if total_lines % READABLE_RESULT_PAGE_SIZE == 0 {
            page_offsets.push(line_offset);
        }

        let line = strip_line_terminator(raw_line);
        let parts = readable_result_parts(line, &state.path, output_fields)?;
        if !matcher.is_match(parts.content) {
            return None;
        }
        total_lines += 1;
        line_offset += raw_line.len();
    }

    Some(ReadableResults {
        total_lines,
        page_offsets,
        highlight_raw_output: state.options.iter().any(|option| option == "--color=auto"),
        output_fields,
        search_path: state.path.clone(),
        matcher,
    })
}

#[cfg(test)]
pub(crate) fn readable_result_page(
    output: &str,
    results: &ReadableResults,
    page: usize,
) -> Option<Vec<ReadableResult>> {
    let start = *results.page_offsets.get(page)?;
    let end = results
        .page_offsets
        .get(page + 1)
        .copied()
        .unwrap_or(output.len());
    let page_output = output.get(start..end)?;
    let mut lines = Vec::with_capacity(READABLE_RESULT_PAGE_SIZE);
    let mut offset = start;

    for raw_line in page_output.split_inclusive('\n') {
        let line = strip_line_terminator(raw_line);
        lines.push(parse_readable_result_line(
            line,
            offset..offset + raw_line.len(),
            &results.search_path,
            &results.matcher,
            results.output_fields,
        )?);
        offset += raw_line.len();
    }

    Some(lines)
}

#[cfg(test)]
pub(crate) fn raw_match_ranges_for_page(
    output: &str,
    results: &ReadableResults,
    line: &ReadableResult,
    page_range: Range<usize>,
) -> Vec<Range<usize>> {
    if !results.highlight_raw_output {
        return Vec::new();
    }

    let page_start = line.raw_range.start + page_range.start;
    let page_end = line.raw_range.start + page_range.end;
    let content_start = page_start.max(line.content_range.start);
    let content_end = page_end.min(line.content_range.end);
    if content_start > content_end {
        return Vec::new();
    }
    let Some(content) = output.get(line.content_range.clone()) else {
        return Vec::new();
    };
    let content_page_start = content_start - line.content_range.start;
    let content_page_end = content_end - line.content_range.start;
    let mut ranges = Vec::new();

    for matched in results.matcher.find_iter(content) {
        if matched.start() > content_page_end {
            break;
        }
        let start = matched.start().max(content_page_start);
        let end = matched.end().min(content_page_end);
        if start <= end {
            ranges.push(
                line.content_range.start + start - page_start
                    ..line.content_range.start + end - page_start,
            );
        }
    }

    ranges
}

#[cfg(test)]
fn parse_readable_result_line(
    line: &str,
    raw_range: Range<usize>,
    search_path: &str,
    matcher: &Regex,
    output_fields: OutputFields,
) -> Option<ReadableResult> {
    let parts = readable_result_parts(line, search_path, output_fields)?;
    let (preview, match_ranges, shortened) = result_excerpt(parts.content, matcher)?;
    let content_range = raw_range.start + parts.content_offset..raw_range.start + line.len();

    Some(ReadableResult {
        path: parts.path.to_owned(),
        line_number: parts.line_number,
        column: parts.column,
        byte_offset: parts.byte_offset,
        preview,
        match_ranges,
        raw_range,
        content_range,
        shortened,
    })
}

#[cfg(test)]
fn readable_result_parts<'a>(
    line: &'a str,
    search_path: &str,
    output_fields: OutputFields,
) -> Option<ReadableResultParts<'a>> {
    let path_separator = result_path_separator(line, search_path)?;
    let path = &line[..path_separator];
    if path.is_empty() {
        return None;
    }

    let output_text = &line[path_separator + 1..];
    let mut content = output_text;
    let line_number = if output_fields.line_number {
        let (value, remainder) = content.split_once(':')?;
        content = remainder;
        Some(value.parse().ok()?)
    } else {
        None
    };
    let column = if output_fields.column {
        let (value, remainder) = content.split_once(':')?;
        content = remainder;
        Some(value.parse().ok()?)
    } else {
        None
    };
    let byte_offset = if output_fields.byte_offset {
        let (value, remainder) = content.split_once(':')?;
        content = remainder;
        Some(value.parse().ok()?)
    } else {
        None
    };
    Some(ReadableResultParts {
        path,
        line_number,
        column,
        byte_offset,
        content,
        content_offset: line.len() - content.len(),
    })
}

#[cfg(test)]
fn output_fields(state: &CommandState) -> OutputFields {
    let has_option = |flag: &str| state.options.iter().any(|option| option == flag);
    let column = has_option("--column");
    OutputFields {
        line_number: has_option("--line-number") || (column && !has_option("--no-line-number")),
        column,
        byte_offset: has_option("--byte-offset"),
    }
}

#[cfg(test)]
fn strip_line_terminator(line: &str) -> &str {
    line.strip_suffix('\n')
        .unwrap_or(line)
        .strip_suffix('\r')
        .unwrap_or_else(|| line.strip_suffix('\n').unwrap_or(line))
}

#[cfg(test)]
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
    let mut pattern = if fixed_strings {
        regex::escape(&state.pattern)
    } else {
        state.pattern.clone()
    };
    if state.options.iter().any(|option| option == "--line-regexp") {
        pattern = format!("^(?:{pattern})$");
    }
    if state.options.iter().any(|option| option == "--word-regexp") {
        pattern = format!(r"\b(?:{pattern})\b");
    }
    let mut builder = RegexBuilder::new(&pattern);
    builder.case_insensitive(
        ignore_case || (smart_case && !state.pattern.chars().any(char::is_uppercase)),
    );
    builder.build().ok()
}

#[cfg(test)]
fn result_excerpt(line: &str, matcher: &Regex) -> Option<(String, Vec<Range<usize>>, bool)> {
    let first_match = matcher.find(line)?;
    let character_count = line.chars().count();
    if character_count <= READABLE_RESULT_PREVIEW_CHARS {
        let match_ranges = matcher
            .find_iter(line)
            .map(|matched| matched.range())
            .collect();
        return Some((line.to_owned(), match_ranges, false));
    }

    let focus_byte = first_match.start();
    let focus_character = line[..focus_byte].chars().count();
    let start_character = focus_character.saturating_sub(READABLE_RESULT_PREVIEW_CHARS / 3);
    let end_character = (start_character + READABLE_RESULT_PREVIEW_CHARS).min(character_count);
    let start_byte = byte_index_at_character(line, start_character);
    let end_byte = byte_index_at_character(line, end_character);
    let prefix = if start_byte > 0 { "..." } else { "" };
    let suffix = if end_byte < line.len() { "..." } else { "" };
    let preview = format!("{prefix}{}{suffix}", &line[start_byte..end_byte]);
    let mut preview_matches = Vec::new();
    for matched in matcher.find_iter(line) {
        if matched.start() > end_byte {
            break;
        }
        let start = matched.start().max(start_byte);
        let end = matched.end().min(end_byte);
        if start <= end {
            preview_matches
                .push(prefix.len() + start - start_byte..prefix.len() + end - start_byte);
        }
    }

    Some((preview, preview_matches, true))
}

#[cfg(test)]
fn byte_index_at_character(text: &str, character: usize) -> usize {
    text.char_indices()
        .nth(character)
        .map_or(text.len(), |(byte, _)| byte)
}

pub(crate) fn output_page_ranges(output: &str) -> Vec<Range<usize>> {
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
mod tests;
