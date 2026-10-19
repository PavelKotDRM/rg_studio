use std::{
    io::Write,
    process::{Command, Stdio},
};

fn cli() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rg-studio"));
    command
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_remove("RIPGREP_CONFIG_PATH")
        .stdin(Stdio::null());
    command
}

#[test]
fn help_and_version_work_without_opening_a_window() {
    for args in [
        vec!["--help"],
        vec!["gui", "--help"],
        vec!["search", "--help"],
        vec!["command", "--help"],
        vec!["--version"],
    ] {
        let output = cli().args(args).output().unwrap();
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn command_prints_shell_quoted_arguments_without_running_ripgrep() {
    let output = cli()
        .args([
            "command",
            "hello world",
            "source files",
            "--",
            "--ignore-case",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "rg --ignore-case --regexp 'hello world' -- 'source files'"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn search_preserves_matches_no_matches_and_regex_error_exit_codes() {
    for (pattern, code) in [
        ("fn main", 0),
        ("RG_STUDIO_NO_SUCH_MATCH_629C1B", 1),
        ("[", 2),
    ] {
        let output = cli()
            .args(["search", pattern])
            .arg(std::path::Path::new("src").join("main.rs"))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(code));
        if code == 0 {
            assert!(String::from_utf8_lossy(&output.stdout).contains("fn main"));
        } else if code == 1 {
            assert!(output.stdout.is_empty());
            assert!(output.stderr.is_empty());
        } else {
            assert!(String::from_utf8_lossy(&output.stderr).contains("regex parse error"));
        }
    }
}

#[test]
fn search_supports_stdin_pipelines() {
    let mut child = cli()
        .args(["search", "needle"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"needle\nnot a match\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"needle\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn patternless_ripgrep_mode_works() {
    let output = cli()
        .args(["search", "--", "--files", "src"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("main.rs"));
}

#[test]
fn cli_search_expands_environment_paths() {
    let output = cli()
        .env("RG_STUDIO_CLI_TEST_PATH", "src")
        .args([
            "search",
            "fn main",
            "${RG_STUDIO_CLI_TEST_PATH}",
            "--",
            "--glob=main.rs",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("fn main"));
    assert!(output.stderr.is_empty());
}

#[test]
fn usage_and_path_errors_use_stderr_and_exit_code_two() {
    for args in [
        vec!["unknown"],
        vec!["search"],
        vec!["command"],
        vec!["command", "needle", "${UNCLOSED"],
        vec!["gui", "needle", "${UNCLOSED"],
    ] {
        let output = cli().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}
