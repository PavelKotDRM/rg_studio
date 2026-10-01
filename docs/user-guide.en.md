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

## Configure a search

### Pattern and path

- **Search pattern** is a regular expression for file contents, not a filename
  glob. For example, `*.` is invalid because `*` must repeat a preceding
  expression; use `.*` to match any characters in a line. To filter file names,
  enable **Include/exclude glob** in **Files** and enter a glob such as `*.rs`.
  For exact text, enable **Literal strings** in **Search**.
- Enter a directory or file in **Search in**. If the field is empty, ripgrep
  searches its current working directory; `.` explicitly selects that directory.
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
immediately. Use **Hide** or **Show** to collapse or expand it.

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
elapsed time, executable, and the command used. Results (`stdout`) and messages
(`stderr`, such as warnings and errors) are on separate tabs, each with a line
count. For standard text results, cards show the source file, an optional line
number, and a highlighted excerpt. Long lines are shortened; expand **Raw
ripgrep output** to inspect the complete, paginated output. Generated `.d`
dependency files can contain many paths on a single line. Output modes that
change ripgrep's format are shown as-is. On failure, a plain-language
explanation is shown and the messages tab opens automatically. Exit code `1`
means no matches were found; it is a normal ripgrep result, not an app error.

Click **Copy command** to copy the displayed command for use in a terminal.
The command display applies OS-appropriate quoting to the pattern, path, and
selected option values.

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
