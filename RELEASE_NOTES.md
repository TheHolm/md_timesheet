# Release Notes

History of tagged `md_timesheet` releases. Each entry has a **User-facing
changes** summary (what also appears in the GitHub Release body) and a
**Details** section with the low-level technical narrative. New entries are
added at the top. `scripts/extract-release-notes.sh` prints just one release's
section for the GitHub Release body.

## v0.5.0 — Aligned table columns and asynchronous file I/O

### User-facing changes

- Markdown table columns (Description, Start Time, End Time, Duration) are
  now padded to a common width within each day's table, so the raw file is
  readable as aligned text, not just as rendered Markdown. Only the day's
  table that was just added to or amended gets re-padded; every other day's
  table is left exactly as it was.
- The window no longer freezes while the timesheet file is read or written:
  all file I/O now runs on a background thread pool instead of the GTK main
  loop. The app still terminates immediately after "Start"/"Worked for"/amend,
  same as before.

### Details

- `src/lib.rs`: added `table_cells` (replaces the old exact-match
  `strip_prefix("| ")`/`strip_suffix(" |")` parsing in `first_cell` and
  `entry_start_time` with trimmed, whitespace-tolerant cell splitting, so
  padded cells still parse correctly), `is_table_row`, `pad_table_rows`,
  `table_bounds` and `pad_last_table`. `apply_worked` calls `pad_last_table`
  right after appending each new row (before any trailing day-boundary or
  timestamp line is pushed), and `amend_last_entry` calls `table_bounds` +
  `pad_table_rows` after rewriting the amended row; both are what keep
  padding scoped to only the table that was actually touched. The separator
  row's dash count is never stretched (it stays exactly `---`), but its cell
  is still padded with trailing spaces like any other cell so the `|`
  characters line up with the rest of the table.
- `src/main.rs`: all file I/O moved off the GTK main loop onto `gio`'s I/O
  thread pool via `gio::spawn_blocking`. `click_start`/`record_worked`/
  `record_amend` end by calling `process::exit`, which works from any
  thread, so their background task needs no further synchronisation.
  `Preview::reload` and the new `begin_amend` (replacing `start_amend`,
  which returned a `bool` the caller used to decide event propagation)
  instead await that background task inside a `glib::spawn_future_local`
  block, so the widget updates they make afterwards happen back on the main
  thread. Since the amend lookup is now asynchronous, the `Key::Up` handler
  can no longer decide propagation based on whether an entry was found, so
  Arrow Up now always stops propagation (instead of only when amend mode was
  actually entered).
- `tests/`: added `tests/table_padding.rs` with exact-string assertions for
  the padding behaviour itself (widening/narrowing, an earlier day's table
  never being touched, a header wider than its data staying unpadded). The
  existing `actions.rs`/`amend.rs`/`elapsed.rs` table-row assertions were
  switched to cell-by-cell comparison via a new shared `tests/common/mod.rs`
  helper (`cells`, `assert_row_eq`, `assert_lines_eq`), so they don't need to
  encode the exact padded width. `tests/formatting.rs` and
  `tests/document_io.rs` needed no changes: padding only happens inside
  `apply_worked`/`amend_last_entry`, not in `new_day`/`new_entry` or the
  raw `read_document`/`write_document`/`peek_document`.

## v0.4.0 — Duration preview on the "Worked for" button

### User-facing changes

- The "Worked on" button is now **"Worked for (HH:MM)"**. The time in
  brackets is how much time would be recorded if you pressed it now, rounded
  up exactly as the Duration column will be. It updates every 30 seconds
  while the window is open.
- It shows `(--:--)` when the last timestamp is more than 24 hours old, is in
  the future, or when there is no usable timestamp yet.
- In amend mode (Arrow Up) the button shows the amended entry's new duration,
  from that entry's start time until now.
- The window title is now "MD Timesheet" instead of "Enter Text".
- The application ID changed from the placeholder `com.example.textpopup` to
  `io.github.TheHolm.md_timesheet`. Desktop rules or settings tied to the old
  ID (e.g. window rules for your launch shortcut) need to be set up again.

### Details

- `src/lib.rs`: added `round_up_minutes` and `format_hhmm` (now also used by
  `new_entry`, so the preview and the written row share one implementation),
  `last_timestamp`, `pending_duration` (with `MAX_PENDING_MINUTES` = 24h),
  `button_label`, and `peek_document`, which reads the timesheet without
  creating it so opening the window has no side effects on disk.
- `last_entry_start` was split out of `amend_last_entry`, which now uses it;
  the amend preview and the amended row therefore use the same start time and
  midnight rule.
- `src/main.rs`: a `Preview` struct holds the button and the cached start
  points. The timestamp is re-read on the window's `map` signal (covering the
  first-run onboarding path) and a `glib::timeout_add_seconds_local` timer
  refreshes the label. Saving settings refreshes it too, since the rounding
  may have changed.
- Added `tests/elapsed.rs` and `peek_document` tests in
  `tests/document_io.rs`.

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
