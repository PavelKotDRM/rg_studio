use std::{
    ffi::OsString,
    ops::Range,
    process::Command,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Duration,
};

use eframe::egui::Context;

use crate::command::CommandState;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const OUTPUT_PAGE_BYTES: usize = 64 * 1024;

pub(crate) struct SearchOutput {
    pub(crate) executable: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) success: bool,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) stdout_pages: Vec<Range<usize>>,
    pub(crate) stderr_pages: Vec<Range<usize>>,
}

#[derive(Default)]
pub(crate) struct SearchController {
    pub(crate) running: bool,
    pub(crate) show_results: bool,
    pub(crate) output: Option<SearchOutput>,
    pub(crate) error: Option<String>,
    pub(crate) output_page: usize,
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
    let executable = resolve_executable();
    let mut process = Command::new(&executable);
    process.args(arguments);
    #[cfg(windows)]
    process.creation_flags(CREATE_NO_WINDOW);

    let output = process.output().map_err(|error| {
        format!(
            "Unable to start ripgrep ({}): {error}. Build the bundled CLI or install `rg` in PATH.",
            executable.display()
        )
    })?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    Ok(SearchOutput {
        executable: executable.to_string_lossy().into_owned(),
        exit_code: output.status.code(),
        success: output.status.success(),
        stdout_pages: output_page_ranges(&stdout),
        stderr_pages: output_page_ranges(&stderr),
        stdout,
        stderr,
    })
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

    use crate::command::CommandState;

    use super::{OUTPUT_PAGE_BYTES, execute, output_page_ranges, resolve_executable};

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
            path: "src/main.rs".into(),
            ..CommandState::default()
        };

        let result = execute(&state).expect("ripgrep should start");
        assert!(result.success, "ripgrep failed: {}", result.stderr);
        assert!(result.stdout.contains("fn main"));
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
