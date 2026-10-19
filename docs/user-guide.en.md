# rg studio user guide

[Русская версия](user-guide.ru.md) · [Project README](../README.md)

rg studio is a graphical command composer for
[ripgrep](https://github.com/BurntSushi/ripgrep). It helps you configure a
search, inspect the generated command, run the ripgrep CLI, and review its
output. The app does not include the ripgrep executable.

## Install and launch

To build from source, install [Rustup](https://rustup.rs/) and the stable Rust
toolchain:

```powershell
rustup update stable
cargo run --release
```

Install the ripgrep CLI separately. Version 15.2.0 is recommended because the
option catalog is based on that release. When you click **Run ripgrep**, the
app looks for `rg.exe` or `rg` next to its executable, then in the executable's
parent directory, and finally in `PATH`.

## Command-line interface

Running `rg-studio` without arguments opens the GUI. Available commands:

| Command | Purpose |
| --- | --- |
| `gui [PATTERN] [PATH] [-- RG_ARGS...]` | Open the GUI with prefilled fields; do not start a search |
| `search [PATTERN] [PATH] [-- RG_ARGS...]` | Run ripgrep directly in the terminal |
| `command [PATTERN] [PATH] [-- RG_ARGS...]` | Print the shell-quoted command without running it |
| `--help`, `--version` | Show help or the application version |

```powershell
rg-studio gui 'TODO|FIXME' src -- --ignore-case
rg-studio search TODO src -- --line-number --glob=*.rs
rg-studio command TODO '~' -- --hidden
rg-studio search -- --files src
rg-studio search -- -e '-pattern'
```

Run from source with `cargo run -- search TODO src`, or use the built executable.
Use `<command> --help` for command-specific help. Parameters after `--` belong
to ripgrep; preserve arguments containing spaces with your shell's quoting.
In GUI mode these parameters appear in **Advanced arguments**, opened
automatically when nonempty, rather than as checked catalog options.

`search` streams stdout and stderr directly and accepts piped stdin. With no
path, ripgrep searches the working directory unless stdin is piped. Exit codes
are 0 for matches, 1 for no matches, and 2 for errors; CLI validation errors
are also reported on stderr with code 2. No search history or output dialog
is created in CLI mode. The path supports the same shortcuts as **Search in**;
quote `~` or variable references if you want the app to expand them.
Shell commands and operators are not evaluated by the app.
On Windows, GUI mode detaches from the console, while CLI modes retain console
and redirected streams.

## Configure a search

### Pattern and path

- **Search pattern** is a regular expression for file contents, not a filename
  glob. For example, `*.` is invalid because `*` must repeat a preceding
  expression; use `.*` to match any characters in a line. To filter file names,
  enable **Include/exclude glob** in **Files** and enter a glob such as `*.rs`.
  For exact text, enable **Literal strings** in **Search**.
- Enter a directory or file in **Search in**. If the field is empty, ripgrep
  searches its current working directory; `.` explicitly selects that directory.
- **Search in** expands `~`, `~/folder`, and `~\folder` to your home directory.
  Environment variables support `%NAME%`, `$NAME`, `${NAME}`, and `$env:NAME`
  syntax (for example `%USERPROFILE%` on Windows or `$HOME` on Linux).
  Variables come from the application's environment; missing variables and paths
  that expand to an empty string are reported as errors. Expansion is single-pass,
  applies only to **Search in**, and is reflected in the generated command.
  Relative paths such as `.` and `..` keep their usual meaning. Other users'
  home shortcuts (`~user`), wildcards, and shell expressions such as `$(...)`
  are not expanded or executed.
- The generated command puts the path after `--`, so a path beginning with `-`
  is treated as a path rather than another option.

### Option categories and filter

The option catalog is split into **Search**, **Files**, **Output**, and
**Modes & diagnostics**. Enable a checkbox to add that option to the generated
command. Value fields show the expected kind of value; glob fields accept file
patterns, while the search pattern field accepts a regular expression.

Use **Filter options** to find options by either their displayed name or flag.
The filter only changes which cards are shown; it does not change selected
options.

Expand **Advanced arguments** to enter repeatable options or flags that are not
in the catalog. Arguments are split into command-line tokens, with quoted
values kept together. Unmatched quotes are shown as an error and prevent the
command from running.
The app does not invoke a shell for **Run ripgrep**, so shell operators such as
`|` are not executed as shell syntax. Options such as ripgrep's `--pre` still
have their normal ripgrep behavior; only use trusted values for those options.

## Build a regular expression

Select **Regex builder** beside the pattern field to open the visual editor.
If the pattern field already contains an expression, the editor imports it as
a **Raw regex** part so it is preserved.

1. Add an atom, or select an existing row and choose its type. The available
   types include literal text, raw regex, any character, digit, word character,
   whitespace, character classes, groups, word boundary, and alternation.
2. Enter the atom's value where needed. Literal text is escaped automatically.
   For a character class, enter the class contents (for example, `a-z0-9`);
   the builder adds the brackets.
3. Choose a quantifier: once, optional (`?`), zero or more (`*`), one or more
   (`+`), an exact count (`{n}`), or a range (`{min,max}`).
4. Use **Start of line** and **End of line** to add `^` and `$`. Use **Up**,
   **Down**, and **Remove** to edit rows, or **Clear** to remove all rows.

The **Generated regex** field is read-only. The editor validates it with the
default Rust regex engine. If it is invalid, the error is shown in the builder.

## Preview and actual search

The **Live text preview** lets you edit sample text and see matches highlighted
immediately. It starts collapsed. Use **Show** to expand it or **Hide** to
collapse it again.

The preview is intentionally not a full ripgrep run: it tests only the main
pattern against the sample text. It does not search files, read pattern files,
apply ignore files or globs, or reproduce every ripgrep option. It honors a
subset of regex-related options, including case handling, literal strings,
multiline matching, Unicode mode, and whole-word or whole-line matching.
Selecting PCRE2 or the `auto` engine makes the preview unavailable; those
settings can still be used for the actual CLI search if supported by your
ripgrep build.

For a filesystem search, review **Generated command** and click **Run ripgrep**.
The CLI runs in the background. The results window explains whether the search
completed, found no matches, or reported an error; it also shows the exit code,
elapsed time, executable, and the command used. The default **CLI output** view
shows the complete captured `stdout` and `stderr` in separate stream selectors.
Long lines wrap to the window width. Use Previous/Next page to browse large
output; Copy copies the displayed page.
The selected ripgrep options are passed unchanged. Output-changing modes such
as JSON, counts, context, and file lists keep ripgrep's original format.
Recognizable match text is highlighted where present. ANSI colors are rendered
without changing copied output; with `--color=auto`, match text is highlighted
even when redirected `stdout` contains no ANSI codes. Non-UTF-8 bytes use
reversible `\xNN` escapes with literal backslashes doubled. Exit code `1` means
no matches were found; it is a normal ripgrep result, not an app error.

Click **Copy command** to copy the displayed command for use in a terminal.
The command display uses PowerShell quoting on Windows and POSIX shell quoting
on Unix for the pattern, path, and selected option values.

## Troubleshooting

- **Unable to start ripgrep:** install the CLI, put it next to the app (or in
  its parent directory), or add it to `PATH`.
- **Enter a pattern or select a patternless ripgrep mode:** enter a pattern, or
  select a mode that does not require one, such as **List searchable files** in
  **Modes & diagnostics**.
- **No matches (exit code 1):** check the pattern, path, and enabled filters.
  An exit code of `1` simply means the search completed without a match.
- **Invalid regex:** **Search pattern** is a regular expression, not a filename
  glob. `*.` is invalid because `*` must follow an expression; use `.*` to
  match any characters in a line. To filter file names, use **Files** >
  **Include/exclude glob**, for example `*.rs`. PCRE2-specific syntax is not
  accepted by the default Rust regex validator.
- **An option is rejected by ripgrep:** make sure the installed CLI version
  supports it. The catalog targets ripgrep 15.2.0; use **Advanced arguments**
  for options not listed in the catalog.
