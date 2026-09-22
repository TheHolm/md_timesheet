# Release Notes

History of tagged `md_timesheet` releases. Each entry has a **User-facing
changes** summary (what also appears in the GitHub Release body) and a
**Details** section with the low-level technical narrative. New entries are
added at the top. `scripts/extract-release-notes.sh` prints just one release's
section for the GitHub Release body.

## v0.3.1 — Prebuilt packages for every release

### User-facing changes

- No changes to `md_timesheet` itself.
- Tagged releases now ship ready-to-install packages on the GitHub Releases
  page: a Debian trixie `.deb`, an Ubuntu 26.04 `.deb`, and a best-effort
  FreeBSD 15 `.pkg` (all amd64), alongside GitHub's own source archive.

### Details

- Added `.woodpecker/release.yaml`, a tag-triggered workflow (`event: tag`,
  `ref: refs/tags/v*`). Its three package steps are independent
  (`depends_on: []`): `deb-trixie`, `deb-ubuntu2604` and `freebsd-pkg`;
  `publish-github-release` waits for all three and uploads them to a GitHub
  Release. The `freebsd-pkg` step is `failure: ignore`, so a FreeBSD
  breakage never blocks the Linux release.
- Added `scripts/fetch-freebsd-gtk.py`, which assembles the GTK4/libadwaita
  cross sysroot by resolving the `gtk4`/`libadwaita` closure from
  `pkg.freebsd.org`'s `packagesite.pkg` and unpacking it, stubbing any
  build-only `Requires.private` `.pc` files that are missing.
- Added `scripts/build-freebsd-pkg.py` (writes a `.pkg` from a staged tree,
  recording the GTK runtime deps) and `scripts/extract-release-notes.sh`
  (renders one release's section as the GitHub Release body).
- `Cargo.toml`: added `[package.metadata.deb]`, `description` and `license`,
  and built `reqwest` with `default-features = false`. The unused TLS stack it
  used to pull in (OpenSSL/native-tls) could not be satisfied for the FreeBSD
  cross-link.
- Documented FreeBSD as best effort in `README.md` and `AGENTS.md`.

## v0.3.0 — Configuration files, settings window and amend mode

### User-facing changes

- Configuration file support: the timesheet path, which table columns to
  include and the duration rounding are read from a small `key = value` config
  file. It is looked up via `-c`/`--config`, then `./md_timesheet.config`, then
  `$XDG_CONFIG_HOME/md_timesheet/config`. See the README for the keys.
- First-run dialog: when no config file is found the program offers to create
  one in the XDG config directory, in the current folder, or at a path you
  choose. For the last two a small pointer file is written to the XDG config
  directory so the config is found again on the next launch.
- Settings window: the gear button next to Start/Worked on opens a window to
  edit the path, columns and rounding and save them back to the config file. A
  path inside the home directory is saved back using `~`.
- Keyboard shortcuts: Enter is equivalent to "Worked on"; Arrow Up loads the
  last entry for amendment, where Enter replaces it and Escape cancels.

### Details

- Added configuration to the library: the `Config`, `Destination` and
  `RecordsFormat` types plus parsing, serialisation and discovery
  (`parse_config`, `serialize_config`, `locate_config`, `load_config_from`,
  `write_config`, `write_pointer`, `xdg_config_dir`, `cwd_config_path`,
  `collapse_tilde`, `default_config_contents`), covered by the new
  `tests/config.rs`.
- Added `last_entry_description` and `amend_last_entry` to the library, with
  coverage for description-only rows.
- Replaced the `once_cell` dependency with `clap` for `-c`/`--config` parsing.
- The GUI gained the first-run dialog, the settings window and the keyboard
  handlers; the settings gear was made flat and compact.

## v0.2.0 — Library split and an integration-test suite

### User-facing changes

- No behaviour change; the program works as before. The user-visible part is
  documentation: the README now covers testing and coverage, and notes that
  pushing is the user's responsibility.

### Details

- Split the program into a library (`src/lib.rs`) holding all non-GUI logic and
  a thin binary (`src/main.rs`) that wires up the GUI.
- Added an integration-test suite in `tests/` (`formatting.rs`,
  `document_io.rs`, `actions.rs`) covering Markdown generation, file I/O and the
  `apply_start`/`apply_worked` state transitions, including `read_document`
  failure paths.
- Committed `Cargo.lock` for reproducible builds and fixed the Docker build
  accordingly.

## v0.1.1 — AGPL-3.0 license and a "Start" fix

### User-facing changes

- Licensed under the GNU Affero General Public License v3.
- Fixed starting a new day via the "Start" button.

### Details

- Added the AGPL v3 `LICENSE`.
- Corrected the "Start" handler so a new day is opened as intended.
- Added `AGENTS.md` and updated the README and the TODO list.

## v0.1.0 — Initial release

### User-facing changes

- A GTK4/libadwaita program that keeps a timesheet in a Markdown file: enter a
  task description and press "Start" to begin tracking or "Worked on" to record
  the completed entry; the program updates the file and exits.
- Days are `#  <day> <Month>` headings followed by a Markdown table
  (Description, Start Time, End Time, Duration); durations are rounded up to
  the next multiple of ten minutes and the last line holds the timestamp of the
  last operation.

### Details

- Initial GTK4 front-end built on the `gtk4` and `libadwaita` (`adw`) crates,
  with Markdown generation and timestamp handling. `reqwest` is reserved for
  planned Joplin support.
