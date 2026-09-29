# rg studio

A cross-platform visual command composer and runner for
[ripgrep](https://github.com/BurntSushi/ripgrep), built with Rust and egui.
Compose a command in real time, run ripgrep, view its output in the app, or copy
the generated command to the clipboard.

## Features

- Inputs for the search pattern and search path.
- A visual regex builder with literals, character classes, groups, alternation,
  anchors, and quantifiers.
- Live validation of generated regular expressions.
- 95+ ripgrep 15.2.0 options organized into four categories.
- An option filter that searches both labels and flag names.
- Advanced arguments for repeatable options and flags from newer ripgrep versions.
- OS-appropriate quoting for patterns, paths, and option values.
- A live text preview that highlights matches in sample text.
- Background ripgrep execution without launching a shell; the results window
  shows stdout, stderr, and the exit code.
- Light and dark themes, readable typography, and responsive option cards.
- An About window with the version, Git SHA, build time, Rust version, and target
  reported by vergen 10.

The live preview tests the main pattern against editable sample text using the
Rust regex engine. It supports selected regex-related options, but does not run
ripgrep or apply filesystem filters. PCRE2 and the `auto` regex engine are not
available in the preview. For an actual search, use **Run ripgrep**. The app
starts the ripgrep CLI directly, not through a shell; advanced arguments are
parsed as command-line arguments rather than shell commands.

## Documentation

- [User guide (English)](docs/user-guide.en.md)
- [Руководство пользователя (русский)](docs/user-guide.ru.md)
- [Documentation index](docs/README.md)

## Architecture

- `src/app.rs` stores application state and configures the live preview.
- `src/ui.rs` renders the interface, dialogs, option catalog, and generated command.
- `src/theme.rs` defines the shared palette, contrast, and typography.
- `src/preview.rs` validates regexes and calculates match ranges in sample text.
- `src/search.rs` runs ripgrep in the background and returns its output to the UI.
- `src/command.rs`, `src/options.rs`, and `src/regex_builder.rs` define the command,
  option catalog, and visual regex builder.

## Build and run

Install [Rustup](https://rustup.rs/) and the stable Rust toolchain. Install the
ripgrep CLI as well; version 15.2.0 matches the option catalog. Make sure `rg`
is available in `PATH`, or place `rg.exe` (Windows) / `rg` (Unix) next to the
app executable.

```powershell
rustup update stable
cargo run --release
```

To build a Linux executable from Windows with Docker Desktop, run:

```powershell
.\scripts\build-linux-docker.ps1
```

The script writes the Linux binary and `rg-studio-linux.tar.gz` to `dist\linux`.
The archive preserves the executable bit. Docker must be running; Cargo
dependencies and Linux build outputs are cached in Docker volumes. The binary
requires the ripgrep CLI to be installed separately.

At runtime, the app looks for `rg.exe` or `rg` next to its executable and then
in the executable's parent directory before falling back to `PATH`. The
ripgrep source code and executable are not bundled with this repository. The
project uses the stable toolchain configured in `rust-toolchain.toml`.

## Quality checks

`cargo test` includes a test that launches the ripgrep CLI, so make sure `rg`
can be found before running the full test suite. Install `cargo-audit` if it is
not already available:

```powershell
rustup update stable
cargo install cargo-audit --locked
rg --version
```

Run the local checks:

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo doc --no-deps
cargo audit
```

Public items are documented with `///`, and Clippy warnings are treated as
errors. `cargo audit` checks the dependency tree locked in `Cargo.lock` against
the RustSec advisory database.

## CI/CD

Run **Manual quality checks** from the GitHub Actions tab to execute the
formatting, Clippy, test, documentation, and dependency-audit checks. The
workflow installs ripgrep because one of the tests launches the CLI.

Pushing a version tag such as `v0.1.0` starts **Release builds**. Before building,
the workflow verifies that the tag exactly matches the package version in
`Cargo.toml`. It then builds on Linux, Windows, and macOS and publishes the
platform archives as assets on a GitHub Release.
Dependabot checks Cargo dependencies and GitHub Actions versions monthly.

## Technology

- Rust 1.98.0 stable, edition 2024.
- egui and eframe 0.36.1.
- vergen and vergen-git2 10.0.2.
- ripgrep 15.2.0 as the source for the option catalog.

## License

[MIT](LICENSE)
