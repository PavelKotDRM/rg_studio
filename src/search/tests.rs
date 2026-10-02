use std::path::PathBuf;

use crate::command::CommandState;

use super::{
    OUTPUT_PAGE_BYTES, execute, output_page_ranges, parse_readable_results,
    raw_match_ranges_for_page, raw_output_text, readable_result_page, resolve_executable,
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
            .raw_highlight_matcher
            .as_ref()
            .is_some_and(|matcher| { matcher.is_match(&result.stdout) })
    );
}

#[test]
fn preserves_ansi_color_sequences_from_ripgrep() {
    let state = CommandState {
        pattern: "fn main".into(),
        path: "src".into(),
        options: vec!["--color=always".into()],
        ..CommandState::default()
    };

    let result = execute(&state).expect("ripgrep should start");

    assert!(result.success, "ripgrep failed: {}", result.stderr);
    assert!(result.stdout.contains("\x1b["), "stdout had no ANSI colors");
}

#[test]
fn keeps_match_highlighting_available_for_changed_output_formats() {
    let state = CommandState {
        pattern: "fn main".into(),
        path: "src".into(),
        options: vec!["--vimgrep".into()],
        ..CommandState::default()
    };

    let result = execute(&state).expect("ripgrep should start");

    assert!(result.success, "ripgrep failed: {}", result.stderr);
    assert!(
        result
            .raw_highlight_matcher
            .as_ref()
            .is_some_and(|matcher| matcher.is_match(&result.stdout))
    );
}

#[test]
fn context_output_uses_only_selected_coordinate_options() {
    let state = CommandState {
        pattern: "fn main".into(),
        path: "src".into(),
        options: vec!["--after-context=1".into(), "--color=auto".into()],
        ..CommandState::default()
    };

    let result = execute(&state).expect("ripgrep should start");
    let matching_line = result
        .stdout
        .lines()
        .find(|line| line.ends_with("fn main() -> eframe::Result {"))
        .expect("context output should contain a matching line");

    assert!(result.success, "ripgrep failed: {}", result.stderr);
    assert_eq!(matching_line, r"src\main.rs:fn main() -> eframe::Result {");
    for option in [
        "--line-number",
        "--with-filename",
        "--column",
        "--byte-offset",
    ] {
        assert!(!result.command.contains(option));
    }
    assert!(
        result
            .raw_highlight_matcher
            .as_ref()
            .is_some_and(|matcher| matcher.is_match(&result.stdout))
    );
}

#[test]
fn context_output_contains_selected_line_column_and_byte_offset_fields() {
    let state = CommandState {
        pattern: "fn main".into(),
        path: "src".into(),
        options: vec![
            "--after-context=1".into(),
            "--line-number".into(),
            "--with-filename".into(),
            "--column".into(),
            "--byte-offset".into(),
        ],
        ..CommandState::default()
    };

    let result = execute(&state).expect("ripgrep should start");
    let matching_line = result
        .stdout
        .lines()
        .find(|line| line.ends_with("fn main() -> eframe::Result {"))
        .expect("context output should contain a matching line");
    let path_separator = super::result_path_separator(matching_line, "src").unwrap();
    let fields = matching_line[path_separator + 1..]
        .splitn(4, ':')
        .collect::<Vec<_>>();

    assert!(result.success, "ripgrep failed: {}", result.stderr);
    assert_eq!(fields.len(), 4);
    assert!(fields[0].parse::<usize>().is_ok());
    assert!(fields[1].parse::<usize>().is_ok());
    assert!(fields[2].parse::<usize>().is_ok());
    assert_eq!(fields[3], "fn main() -> eframe::Result {");
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
fn keeps_no_matches_separate_from_process_errors() {
    let state = CommandState {
        pattern: format!("rg_studio_no_match_{}__test__", std::process::id()),
        path: "src".into(),
        ..CommandState::default()
    };

    let result = execute(&state).expect("ripgrep should start");

    assert_eq!(result.exit_code, Some(1));
    assert!(result.stdout.is_empty());
    assert!(result.stdout_pages.is_empty());
}

#[test]
fn formats_long_windows_results_as_file_and_match_excerpt() {
    let first = format!(
        r"D:\work\rg.d:1:9:8:D:\work\rg.exe: {}",
        "dependency ".repeat(40)
    );
    let second = format!(
        r"D:\work\deps\rg.d:2:14:13:D:\work\deps\rg.exe: {}",
        "dependency ".repeat(40)
    );
    let output = format!("{first}\n{second}");
    let state = CommandState {
        pattern: "rg[.]exe".into(),
        path: r"D:\work".into(),
        options: vec![
            "--word-regexp".into(),
            "--line-number".into(),
            "--with-filename".into(),
            "--column".into(),
            "--byte-offset".into(),
        ],
        ..CommandState::default()
    };

    let results = parse_readable_results(&state, &output).unwrap();
    let page = readable_result_page(&output, &results, 0).unwrap();
    let result = &page[0];
    let matched = &result.match_ranges[0];

    assert_eq!(results.total_lines, 2);
    assert_eq!(result.path, r"D:\work\rg.d");
    assert_eq!(result.line_number, Some(1));
    assert_eq!(result.column, Some(9));
    assert_eq!(result.byte_offset, Some(8));
    assert!(result.shortened);
    assert_eq!(&result.preview[matched.clone()], "rg.exe");
}

#[test]
fn preserves_line_numbers_in_readable_results() {
    let state = CommandState {
        pattern: "needle".into(),
        path: r"D:\work".into(),
        options: vec![
            "--line-number".into(),
            "--with-filename".into(),
            "--column".into(),
            "--byte-offset".into(),
        ],
        ..CommandState::default()
    };
    let output = r"D:\work\sample.txt:42:3:102:a needle in the line";
    let results = parse_readable_results(&state, output).unwrap();
    let page = readable_result_page(output, &results, 0).unwrap();

    assert_eq!(page[0].path, r"D:\work\sample.txt");
    assert_eq!(page[0].line_number, Some(42));
    assert_eq!(page[0].column, Some(3));
    assert_eq!(page[0].byte_offset, Some(102));
    assert_eq!(page[0].preview, "a needle in the line");
}

#[test]
fn highlights_every_match_and_parses_all_result_lines() {
    let state = CommandState {
        pattern: "needle".into(),
        path: "src".into(),
        options: vec![
            "--line-number".into(),
            "--with-filename".into(),
            "--column".into(),
            "--byte-offset".into(),
        ],
        ..CommandState::default()
    };
    let output = (1..=250)
        .map(|line_number| format!("src/sample.txt:{line_number}:1:0:needle and needle"))
        .collect::<Vec<_>>()
        .join("\n");

    let results = parse_readable_results(&state, &output).unwrap();
    let first_page = readable_result_page(&output, &results, 0).unwrap();
    let last_page = readable_result_page(&output, &results, 2).unwrap();

    assert_eq!(results.total_lines, 250);
    assert_eq!(results.page_offsets.len(), 3);
    assert_eq!(first_page[0].match_ranges, [0..6, 11..17]);
    assert_eq!(last_page.len(), 50);
    assert_eq!(
        &output[first_page[0].raw_range.clone()],
        "src/sample.txt:1:1:0:needle and needle\n"
    );
    assert!(readable_result_page(&output, &results, 3).is_none());
}

#[test]
fn auto_color_highlights_the_raw_page_without_changing_source_text() {
    let state = CommandState {
        pattern: "needle".into(),
        path: "src".into(),
        options: vec![
            "--color=auto".into(),
            "--line-number".into(),
            "--with-filename".into(),
            "--column".into(),
            "--byte-offset".into(),
        ],
        ..CommandState::default()
    };
    let output = format!(
        "src/sample.txt:1:{}:{}:{}needle\n",
        OUTPUT_PAGE_BYTES + 1,
        OUTPUT_PAGE_BYTES,
        "x".repeat(OUTPUT_PAGE_BYTES)
    );
    let results = parse_readable_results(&state, &output).unwrap();
    let line = readable_result_page(&output, &results, 0)
        .unwrap()
        .remove(0);
    let raw_line = &output[line.raw_range.clone()];
    let pages = output_page_ranges(raw_line);
    let page = pages[1].clone();
    let highlights = raw_match_ranges_for_page(&output, &results, &line, page.clone());

    assert!(results.highlight_raw_output);
    assert_eq!(highlights.len(), 1);
    assert_eq!(
        &raw_line[page.start + highlights[0].start..page.start + highlights[0].end],
        "needle"
    );
    assert!(!raw_line.contains("\x1b["));
}

#[test]
fn highlights_only_matches_allowed_by_word_and_line_modes() {
    let word_state = CommandState {
        pattern: "cat".into(),
        path: "src".into(),
        options: vec![
            "--word-regexp".into(),
            "--line-number".into(),
            "--with-filename".into(),
            "--column".into(),
            "--byte-offset".into(),
        ],
        ..CommandState::default()
    };
    let word_output = "src/sample.txt:1:9:8:catalog cat";
    let word_results = parse_readable_results(&word_state, word_output).unwrap();
    let word_page = readable_result_page(word_output, &word_results, 0).unwrap();
    assert_eq!(word_page[0].match_ranges.len(), 1);
    assert_eq!(word_page[0].match_ranges[0], 8..11);

    let line_state = CommandState {
        pattern: "needle".into(),
        path: "src".into(),
        options: vec![
            "--line-regexp".into(),
            "--line-number".into(),
            "--with-filename".into(),
            "--column".into(),
            "--byte-offset".into(),
        ],
        ..CommandState::default()
    };
    assert!(parse_readable_results(&line_state, "src/sample.txt:1:1:0:prefix needle").is_none());
    let line_output = "src/sample.txt:1:1:0:needle";
    let line_results = parse_readable_results(&line_state, line_output).unwrap();
    let line_page = readable_result_page(line_output, &line_results, 0).unwrap();
    assert_eq!(line_page[0].match_ranges.len(), 1);
    assert_eq!(line_page[0].match_ranges[0], 0..6);
}

#[test]
fn empty_standard_output_is_a_valid_empty_result() {
    let state = CommandState {
        pattern: "needle".into(),
        path: "src".into(),
        ..CommandState::default()
    };

    let results = parse_readable_results(&state, "").unwrap();

    assert!(results.page_offsets.is_empty());
    assert_eq!(results.total_lines, 0);
}

#[test]
fn leaves_nonstandard_ripgrep_output_unformatted() {
    let state = CommandState {
        pattern: "needle".into(),
        path: "src".into(),
        options: vec!["--json".into()],
        ..CommandState::default()
    };

    assert!(parse_readable_results(&state, "{}").is_none());
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

#[test]
fn raw_output_escapes_invalid_utf8_bytes_without_loss() {
    let bytes = [b'r', b'g', 0xFF, b'\n'];

    assert_eq!(raw_output_text(bytes.to_vec()), ("rg\\xFF\n".into(), false));
    assert_eq!(
        raw_output_text(vec![b'\\', 0xFE]),
        (r"\\\xFE".into(), false)
    );
}
