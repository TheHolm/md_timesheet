# AGENTS.md

## Repository visibility

**This repository is pushed to a public GitHub repo.** Before committing,
pushing, or writing anything into a tracked file (code, docs, scripts, test
fixtures, commit messages), make sure it contains no sensitive or identifying
information: no real hostnames/IPs, credentials/passwords/API keys/private
keys, personal file paths, or other details tied to a particular person's or
machine's identity. Genuinely throwaway material (e.g. ad hoc scripts written
for a single session) belongs outside the repo entirely (e.g. under `/tmp`),
never committed - see the "Never commit changes unless the user explicitly
asks to commit" rule below, which exists partly for this reason.

## Project overview

`md_timesheet` is a small GTK4/libadwaita desktop application for keeping a
timesheet formatted as Markdown. The user types a task description, then
presses "Start" (begin tracking) or "Worked on" (record the completed task),
and the program updates the timesheet file and exits. The package, library and
binary are all named `md_timesheet`. Version: declared as `version` in
`Cargo.toml`.

## Stack

- Rust 2021 edition
- `gtk4` (feature `v4_8`) - GUI toolkit
- `adw` / libadwaita (feature `v1_2`) - GNOME application shell
- `chrono` - date/time handling and duration maths
- `once_cell` - the lazily-initialised storage destination
- `reqwest` - reserved for the planned Joplin REST API support (currently unused)
- `tempfile` (dev-dependency) - temporary directories/files for integration tests
- Docker (Debian `bookworm` base) for building

## Target platforms

Generic Linux desktops with GTK4 and libadwaita. The `docker/` folder holds a
Dockerfile that provides a reproducible GTK4/libadwaita build environment. No
effort is made to support other platforms.

## Layout

- `src/main.rs` - the GTK4/libadwaita GUI: window, entry field and the
  "Start"/"Worked on" buttons. Deliberately thin; it only wires the widgets to
  the library and handles I/O errors and exit codes.
- `src/lib.rs` - all non-GUI logic, exposed publicly so the GUI and the
  integration tests share exactly one implementation: the `Destination` and
  `RecordsFormat` types, file reading/writing (`read_document`,
  `write_document`), Markdown generation (`new_day`, `new_entry`), the pure
  state transitions (`apply_start`, `apply_worked`) and the default
  `RECORD_FORMAT`.
- `tests/` - integration tests split by topic:
  - `formatting.rs` - `new_day`/`new_entry` column selection and duration
    rounding
  - `document_io.rs` - creating, reading back and overwriting a timesheet file
  - `actions.rs` - `apply_start`/`apply_worked` across empty files, same-day
    and previous-day timestamps, midnight rollover and unparsable last lines
- `Cargo.toml` - package metadata, dependencies and the `tempfile`
  dev-dependency
- `docker/Dockerfile` - build environment
- `README.md` - user-facing usage, note format and TODO list
- `LICENSE` - AGPL v3
- `timesheet.markdown` - the user's own runtime timesheet (gitignored, not
  checked in)

## Note format

The timesheet is a Markdown document; see `README.md` for a worked example.

- A day begins with a `#  <day> <Month>` heading, a blank line and a table
  header, e.g. `| Description | Start Time | End Time | Duration |`.
- Columns marked as enabled in `RecordsFormat` are included; disabled columns
  are omitted from the header and every row.
- Durations are rounded up to the next multiple of
  `RecordsFormat::duration_rounding` minutes (default 10) and printed as
  `HH:MM`.
- The very last line is the timestamp of the last operation in
  `%d/%m/%Y %H:%M` form. `apply_start`/`apply_worked` parse it to know when the
  previous action happened. **Never remove the timestamp on the last line.**

## Status / known gaps

Work in progress. Current known issues:

- Errors are written to STDERR (and to STDOUT for an unparsable last line on
  "Start"); a popup would be better.
- File reading/writing is synchronous and runs on the GTK main loop.
- The storage path (`./timesheet.markdown`) and the record format are
  hardcoded; configuration should move into the GUI.
- `apply_*` only inspects the last line; the last *non-empty* line would be
  more robust.
- Joplin note support is stubbed out (`Destination::JoplinNote` returns
  "Joplin support has not been implemented."); `reqwest` is reserved for it.
- Table columns are not padded to a common width, so the raw Markdown is not
  aligned as readable text.
- No packaging for Debian/Ubuntu yet.

## Commands

- Build/check: `cargo build`
- Run: `cargo run` (requires a GTK4/libadwaita desktop session)
- Test: `cargo test`
- Coverage: `cargo llvm-cov --tests` (requires the `llvm-tools` component and
  the `cargo-llvm-cov` subcommand)

One-time coverage tooling setup:

```
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

## Conventions

- `cargo fmt` style, no external formatters
- Every function and every test is documented with a `///` doc comment
  describing its purpose and any non-obvious behavior
- All new code must be covered by tests - integration tests in `tests/` for
  public behaviour and unit tests for private helpers; never add production
  code without accompanying tests
- Keep GUI code in `main.rs` and logic in `lib.rs`; anything testable belongs
  in the library
- Commits include a detailed description of what changed and why
- When starting work on each new branch, ask the user whether to bump the
  version number (and if so, to what value) before writing any code
- Version numbers follow `X.Y.Z`: `X` (major) is bumped only when the user
  explicitly asks for it; `Y` (minor) is bumped when a change adds a new
  feature; `Z` (patch) is bumped for bugfixes and other changes that don't
  add, remove, or change functionality
- Never commit changes unless the user explicitly asks to commit
