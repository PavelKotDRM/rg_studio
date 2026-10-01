#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Category {
    Search,
    Files,
    Output,
    Modes,
}

impl Category {
    pub(crate) const ALL: [Self; 4] = [Self::Search, Self::Files, Self::Output, Self::Modes];

    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Search => "Search",
            Self::Files => "Files",
            Self::Output => "Output",
            Self::Modes => "Modes & diagnostics",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum OptionKind {
    Switch,
    Value(&'static str),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct OptionSpec {
    pub(crate) category: Category,
    pub(crate) flag: &'static str,
    pub(crate) label: &'static str,
    pub(crate) kind: OptionKind,
}

const fn switch(category: Category, flag: &'static str, label: &'static str) -> OptionSpec {
    OptionSpec {
        category,
        flag,
        label,
        kind: OptionKind::Switch,
    }
}

const fn value(
    category: Category,
    flag: &'static str,
    label: &'static str,
    hint: &'static str,
) -> OptionSpec {
    OptionSpec {
        category,
        flag,
        label,
        kind: OptionKind::Value(hint),
    }
}

pub(crate) const OPTIONS: &[OptionSpec] = &[
    value(
        Category::Search,
        "--regexp",
        "Additional pattern",
        "PATTERN",
    ),
    value(
        Category::Search,
        "--file",
        "Patterns from file",
        "PATTERNFILE",
    ),
    value(Category::Search, "--pre", "Preprocessor command", "COMMAND"),
    value(Category::Search, "--pre-glob", "Preprocessor glob", "GLOB"),
    switch(Category::Search, "--search-zip", "Search compressed files"),
    switch(Category::Search, "--case-sensitive", "Case sensitive"),
    switch(Category::Search, "--crlf", "Use CRLF line terminators"),
    value(
        Category::Search,
        "--dfa-size-limit",
        "DFA size limit",
        "NUM+SUFFIX",
    ),
    value(Category::Search, "--encoding", "Text encoding", "ENCODING"),
    value(
        Category::Search,
        "--engine",
        "Regex engine",
        "default|pcre2|auto",
    ),
    switch(Category::Search, "--fixed-strings", "Literal strings"),
    switch(Category::Search, "--ignore-case", "Ignore case"),
    switch(Category::Search, "--invert-match", "Invert matches"),
    switch(Category::Search, "--line-regexp", "Match whole line"),
    value(
        Category::Search,
        "--max-count",
        "Maximum matches per file",
        "NUM",
    ),
    switch(Category::Search, "--mmap", "Use memory maps"),
    switch(Category::Search, "--multiline", "Multiline search"),
    switch(
        Category::Search,
        "--multiline-dotall",
        "Dot matches newlines",
    ),
    switch(Category::Search, "--no-unicode", "Disable Unicode mode"),
    switch(Category::Search, "--null-data", "Use NUL line terminators"),
    switch(Category::Search, "--pcre2", "Use PCRE2"),
    value(
        Category::Search,
        "--regex-size-limit",
        "Compiled regex size limit",
        "NUM+SUFFIX",
    ),
    switch(Category::Search, "--smart-case", "Smart case"),
    switch(
        Category::Search,
        "--stop-on-nonmatch",
        "Stop after non-match",
    ),
    switch(Category::Search, "--text", "Search binary as text"),
    value(Category::Search, "--threads", "Worker threads", "NUM"),
    switch(Category::Search, "--word-regexp", "Match whole word"),
    switch(
        Category::Search,
        "--auto-hybrid-regex",
        "Automatic hybrid regex",
    ),
    switch(
        Category::Search,
        "--no-pcre2-unicode",
        "Disable PCRE2 Unicode",
    ),
    switch(Category::Search, "--binary", "Search binary files"),
    switch(Category::Files, "--follow", "Follow symbolic links"),
    value(Category::Files, "--glob", "Include or exclude glob", "GLOB"),
    switch(
        Category::Files,
        "--glob-case-insensitive",
        "Case-insensitive globs",
    ),
    switch(Category::Files, "--hidden", "Include hidden files"),
    value(Category::Files, "--iglob", "Case-insensitive glob", "GLOB"),
    value(
        Category::Files,
        "--ignore-file",
        "Additional ignore file",
        "PATH",
    ),
    switch(
        Category::Files,
        "--ignore-file-case-insensitive",
        "Case-insensitive ignore files",
    ),
    value(
        Category::Files,
        "--max-depth",
        "Maximum directory depth",
        "NUM",
    ),
    value(
        Category::Files,
        "--max-filesize",
        "Maximum file size",
        "NUM+SUFFIX",
    ),
    switch(Category::Files, "--no-ignore", "Ignore no ignore files"),
    switch(
        Category::Files,
        "--no-ignore-dot",
        "Ignore no .ignore files",
    ),
    switch(
        Category::Files,
        "--no-ignore-exclude",
        "Ignore no local exclusions",
    ),
    switch(
        Category::Files,
        "--no-ignore-files",
        "Ignore no --ignore-file files",
    ),
    switch(
        Category::Files,
        "--no-ignore-global",
        "Ignore no global rules",
    ),
    switch(
        Category::Files,
        "--no-ignore-parent",
        "Ignore no parent rules",
    ),
    switch(Category::Files, "--no-ignore-vcs", "Ignore no VCS rules"),
    switch(
        Category::Files,
        "--no-require-git",
        "Use gitignore outside repositories",
    ),
    switch(
        Category::Files,
        "--one-file-system",
        "Stay on one file system",
    ),
    value(Category::Files, "--type", "Include file type", "TYPE"),
    value(Category::Files, "--type-not", "Exclude file type", "TYPE"),
    value(Category::Files, "--type-add", "Add file type", "TYPESPEC"),
    value(Category::Files, "--type-clear", "Clear file type", "TYPE"),
    switch(Category::Files, "--unrestricted", "Reduce ignore filtering"),
    value(
        Category::Output,
        "--after-context",
        "Lines after match",
        "NUM",
    ),
    value(
        Category::Output,
        "--before-context",
        "Lines before match",
        "NUM",
    ),
    switch(
        Category::Output,
        "--block-buffered",
        "Block buffered output",
    ),
    switch(Category::Output, "--byte-offset", "Show byte offsets"),
    value(
        Category::Output,
        "--color",
        "Color mode",
        "never|auto|always|ansi",
    ),
    value(
        Category::Output,
        "--colors",
        "Color specification",
        "COLOR_SPEC",
    ),
    switch(Category::Output, "--column", "Show columns"),
    value(Category::Output, "--context", "Lines around match", "NUM"),
    value(
        Category::Output,
        "--context-separator",
        "Context separator",
        "SEPARATOR",
    ),
    value(
        Category::Output,
        "--field-context-separator",
        "Context field separator",
        "SEPARATOR",
    ),
    value(
        Category::Output,
        "--field-match-separator",
        "Match field separator",
        "SEPARATOR",
    ),
    switch(Category::Output, "--heading", "Group matches by file"),
    value(
        Category::Output,
        "--hostname-bin",
        "Hostname command",
        "COMMAND",
    ),
    value(
        Category::Output,
        "--hyperlink-format",
        "Terminal hyperlink format",
        "FORMAT",
    ),
    switch(
        Category::Output,
        "--include-zero",
        "Include zero-match files",
    ),
    switch(Category::Output, "--line-buffered", "Line buffered output"),
    switch(Category::Output, "--line-number", "Show line numbers"),
    switch(Category::Output, "--no-line-number", "Hide line numbers"),
    value(
        Category::Output,
        "--max-columns",
        "Maximum printed columns",
        "NUM",
    ),
    switch(
        Category::Output,
        "--max-columns-preview",
        "Preview long lines",
    ),
    switch(Category::Output, "--null", "NUL after file paths"),
    switch(Category::Output, "--only-matching", "Print matches only"),
    value(
        Category::Output,
        "--path-separator",
        "Printed path separator",
        "SEPARATOR",
    ),
    switch(
        Category::Output,
        "--passthru",
        "Print matching and non-matching lines",
    ),
    switch(Category::Output, "--pretty", "Pretty output"),
    switch(Category::Output, "--quiet", "Suppress output"),
    value(
        Category::Output,
        "--replace",
        "Replacement text",
        "REPLACEMENT",
    ),
    value(
        Category::Output,
        "--sort",
        "Ascending sort",
        "none|path|modified|accessed|created",
    ),
    value(
        Category::Output,
        "--sortr",
        "Descending sort",
        "none|path|modified|accessed|created",
    ),
    switch(Category::Output, "--trim", "Trim leading whitespace"),
    switch(Category::Output, "--vimgrep", "Vim-compatible output"),
    switch(Category::Output, "--with-filename", "Always show filenames"),
    switch(Category::Output, "--no-filename", "Never show filenames"),
    switch(Category::Output, "--sort-files", "Sort files (deprecated)"),
    switch(Category::Modes, "--count", "Count matching lines"),
    switch(
        Category::Modes,
        "--count-matches",
        "Count individual matches",
    ),
    switch(
        Category::Modes,
        "--files-with-matches",
        "List files with matches",
    ),
    switch(
        Category::Modes,
        "--files-without-match",
        "List files without matches",
    ),
    switch(Category::Modes, "--json", "JSON Lines output"),
    switch(Category::Modes, "--debug", "Debug logging"),
    switch(
        Category::Modes,
        "--no-ignore-messages",
        "Suppress ignore parse errors",
    ),
    switch(Category::Modes, "--no-messages", "Suppress file errors"),
    switch(Category::Modes, "--stats", "Print aggregate statistics"),
    switch(Category::Modes, "--trace", "Trace logging"),
    switch(Category::Modes, "--files", "List searchable files"),
    value(
        Category::Modes,
        "--generate",
        "Generate documentation",
        "KIND",
    ),
    switch(
        Category::Modes,
        "--no-config",
        "Disable configuration files",
    ),
    switch(Category::Modes, "--pcre2-version", "Print PCRE2 version"),
    switch(Category::Modes, "--type-list", "List supported file types"),
    switch(Category::Modes, "--version", "Print ripgrep version"),
    switch(Category::Modes, "--help", "Print ripgrep help"),
];

/// Groups of flags that contradict each other when passed to ripgrep together.
///
/// Enabling one flag in a group should disable the others so the generated
/// command never contains two options that override one another silently.
pub(crate) const MUTUALLY_EXCLUSIVE: &[&[&str]] = &[
    &["--case-sensitive", "--ignore-case", "--smart-case"],
    &["--line-number", "--no-line-number"],
    &["--with-filename", "--no-filename"],
    &["--sort", "--sortr"],
];

/// Returns the other flags that conflict with `flag`, if any.
pub(crate) fn conflicting_flags(flag: &str) -> &'static [&'static str] {
    MUTUALLY_EXCLUSIVE
        .iter()
        .find(|group| group.contains(&flag))
        .copied()
        .unwrap_or(&[])
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{MUTUALLY_EXCLUSIVE, OPTIONS, conflicting_flags};

    #[test]
    fn option_flags_are_unique_and_comprehensive() {
        let flags: HashSet<_> = OPTIONS.iter().map(|option| option.flag).collect();
        assert_eq!(flags.len(), OPTIONS.len());
        assert!(OPTIONS.len() >= 95);
    }

    #[test]
    fn mutually_exclusive_groups_only_reference_known_flags() {
        let flags: HashSet<_> = OPTIONS.iter().map(|option| option.flag).collect();
        for group in MUTUALLY_EXCLUSIVE {
            assert!(group.len() >= 2, "a conflict group needs at least 2 flags");
            for flag in *group {
                assert!(
                    flags.contains(flag),
                    "unknown flag in conflict group: {flag}"
                );
            }
        }
    }

    #[test]
    fn conflicting_flags_are_reported_both_ways() {
        assert_eq!(
            conflicting_flags("--ignore-case"),
            &["--case-sensitive", "--ignore-case", "--smart-case"]
        );
        assert_eq!(conflicting_flags("--sort"), &["--sort", "--sortr"]);
        assert!(conflicting_flags("--hidden").is_empty());
    }
}
