use std::{
    io::{self, Write},
    process::{Command, ExitStatus},
};

use clap::{Args, Parser, Subcommand};

use crate::{command::CommandState, search::resolve_executable};

#[derive(Debug, Parser)]
#[command(
    name = "rg-studio",
    version,
    about = "Visual and command-line workspace for ripgrep",
    after_help = "Without a subcommand, opens the GUI.\nPass ripgrep options after --, e.g. rg-studio search TODO src -- --ignore-case --glob=*.rs"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Action>,
}

#[derive(Debug, Subcommand)]
enum Action {
    /// Open the GUI with optional search fields and ripgrep arguments.
    Gui(SearchArgs),
    /// Run ripgrep in the terminal, preserving its output and exit code.
    Search(SearchArgs),
    /// Print the shell-quoted command without running it.
    Command(SearchArgs),
}

#[derive(Debug, Default, Args)]
struct SearchArgs {
    /// Regex for file contents (omit when using --files or -e/-f after --).
    pattern: Option<String>,
    /// Search directory or file; supports ~ and environment variables.
    path: Option<String>,
    /// Ripgrep arguments, passed literally without invoking a shell.
    #[arg(last = true, value_name = "RG_ARGS", allow_hyphen_values = true)]
    rg_args: Vec<String>,
}

impl SearchArgs {
    fn into_state(self) -> Result<CommandState, String> {
        let extra_arguments = shlex::try_join(self.rg_args.iter().map(String::as_str))
            .map_err(|error| format!("Invalid ripgrep argument: {error}"))?;
        Ok(CommandState {
            pattern: self.pattern.unwrap_or_default(),
            path: self.path.unwrap_or_default(),
            extra_arguments,
            ..CommandState::default()
        })
    }
}

pub(crate) fn run() -> Result<u8, String> {
    match Cli::parse().command {
        None => open_gui(CommandState::default()),
        Some(Action::Gui(args)) => open_gui(args.into_state()?),
        Some(Action::Search(args)) => {
            let state = args.into_state()?;
            let arguments = state.arguments()?;
            let executable = resolve_executable();
            let status = Command::new(&executable)
                .args(arguments)
                .status()
                .map_err(|error| {
                    format!(
                        "Unable to start ripgrep ({}): {error}. Install rg in PATH or next to rg-studio.",
                        executable.to_string_lossy()
                    )
                })?;
            Ok(exit_code(status))
        }
        Some(Action::Command(args)) => {
            let state = args.into_state()?;
            state.arguments()?;
            let command = state.command()?;
            match writeln!(io::stdout().lock(), "{command}") {
                Ok(()) => Ok(0),
                Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(0),
                Err(error) => Err(format!("Unable to write command to stdout: {error}")),
            }
        }
    }
}

fn exit_code(status: ExitStatus) -> u8 {
    if let Some(code) = status.code() {
        return u8::try_from(code).unwrap_or(2);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return u8::try_from(128 + signal).unwrap_or(2);
        }
    }
    2
}

fn open_gui(state: CommandState) -> Result<u8, String> {
    // Validate path expansion and argument quoting, but allow an empty GUI pattern.
    state.command()?;
    detach_console()?;
    crate::run_gui(state).map_err(|error| format!("Unable to open GUI: {error}"))?;
    Ok(0)
}

#[cfg(windows)]
fn detach_console() -> Result<(), String> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn FreeConsole() -> i32;
    }
    // Detach only in GUI mode; CLI mode must keep inherited console and pipe handles.
    if unsafe { FreeConsole() } == 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(6) {
            return Err(format!("Unable to detach the GUI console: {error}"));
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn detach_console() -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Action, Cli};
    use clap::{Parser, error::ErrorKind};

    #[test]
    fn no_arguments_open_the_gui() {
        assert!(
            Cli::try_parse_from(["rg-studio"])
                .unwrap()
                .command
                .is_none()
        );
    }

    #[test]
    fn parses_every_subcommand_with_pattern_path_and_ripgrep_flags() {
        for action in ["gui", "search", "command"] {
            let cli = Cli::try_parse_from([
                "rg-studio",
                action,
                "hello world",
                "source files",
                "--",
                "--ignore-case",
                "--glob=*.rs",
            ])
            .unwrap();
            let args = match cli.command.unwrap() {
                Action::Gui(args) | Action::Search(args) | Action::Command(args) => args,
            };
            let state = args.into_state().unwrap();
            assert_eq!(state.pattern, "hello world");
            assert_eq!(state.path, "source files");
            assert_eq!(
                state.arguments().unwrap(),
                [
                    "--ignore-case",
                    "--glob=*.rs",
                    "--regexp",
                    "hello world",
                    "--",
                    "source files"
                ]
                .map(std::ffi::OsString::from)
            );
        }
    }

    #[test]
    fn ripgrep_arguments_round_trip_without_shell_interpretation() {
        let cli = Cli::try_parse_from([
            "rg-studio",
            "search",
            "--",
            "-e",
            "-needle",
            "--glob",
            "source files/*.rs",
            "--replace",
            "a\"b'c;$HOME",
            "--no-config",
        ])
        .unwrap();
        let Some(Action::Search(args)) = cli.command else {
            panic!("expected search")
        };
        let state = args.into_state().unwrap();
        assert_eq!(
            state.arguments().unwrap(),
            [
                "-e",
                "-needle",
                "--glob",
                "source files/*.rs",
                "--replace",
                "a\"b'c;$HOME",
                "--no-config"
            ]
            .map(std::ffi::OsString::from)
        );
    }

    #[test]
    fn accepts_patternless_modes_but_rejects_an_empty_search() {
        let cli = Cli::try_parse_from(["rg-studio", "search", "--", "--files"]).unwrap();
        let Some(Action::Search(args)) = cli.command else {
            panic!("expected search")
        };
        assert!(args.into_state().unwrap().arguments().is_ok());
        let cli = Cli::try_parse_from(["rg-studio", "search"]).unwrap();
        let Some(Action::Search(args)) = cli.command else {
            panic!("expected search")
        };
        assert!(args.into_state().unwrap().arguments().is_err());
    }

    #[test]
    fn help_version_and_usage_errors_are_handled_by_the_parser() {
        for args in [vec!["--help"], vec!["search", "--help"], vec!["--version"]] {
            let error = Cli::try_parse_from(std::iter::once("rg-studio").chain(args)).unwrap_err();
            assert!(matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ));
        }
        for args in [
            vec!["unknown"],
            vec!["search", "one", "two", "three"],
            vec!["search", "needle", "--ignore-case"],
        ] {
            assert!(Cli::try_parse_from(std::iter::once("rg-studio").chain(args)).is_err());
        }
    }
}
