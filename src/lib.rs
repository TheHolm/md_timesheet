//! Non-GUI logic for `md_timesheet`.
//!
//! Everything that can be exercised without a GTK main loop lives here: the
//! storage destinations, the Markdown records format, file I/O, Markdown
//! generation and the pure state transitions used by the "Start" and
//! "Worked for" buttons, including the duration preview shown on the latter. Keeping this out of `main.rs` lets the GUI and the
//! integration tests share exactly one implementation.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::prelude::*;
use chrono::Duration;

/// The timestamp format written on the final line of the document and used
/// to parse when the previous action happened.
const TIMESTAMP_FORMAT: &str = "%d/%m/%Y %H:%M";

/// Connection details for a Joplin note used as storage.
///
/// Joplin support is not implemented yet; the type exists so that
/// [`Destination`] can carry the configuration once it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoplinNote {
    /// The Joplin note identifier.
    pub id: String,
    /// The Joplin REST API base URL.
    pub url: String,
    /// The Joplin REST API token.
    pub api_key: String,
}

/// Which columns a table header and its rows contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordsFormat {
    /// Include a "Start Time" column.
    pub start_time: bool,
    /// Include an "End Time" column.
    pub end_time: bool,
    /// Include a "Duration" column.
    pub duration: bool,
    /// Round durations up to the next multiple of this many minutes.
    pub duration_rounding: i32,
}

/// Where the timesheet document is stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// A Markdown file at the contained path.
    TextFile(String),
    /// A Joplin note (not implemented yet).
    JoplinNote(JoplinNote),
}

/// Default record format: all columns enabled, durations rounded up to 10
/// minutes.
pub const RECORD_FORMAT: RecordsFormat = RecordsFormat {
    start_time: true,
    end_time: true,
    duration: true,
    duration_rounding: 10,
};

/// The application configuration loaded from a config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Where the timesheet document is stored.
    pub destination: Destination,
    /// Which columns the tables contain and how durations are rounded.
    pub format: RecordsFormat,
}

impl Default for Config {
    /// The built-in configuration used when no values are configured: the
    /// timesheet lives in `./timesheet.markdown` and [`RECORD_FORMAT`] applies.
    fn default() -> Self {
        Config {
            destination: Destination::TextFile("./timesheet.markdown".to_string()),
            format: RECORD_FORMAT,
        }
    }
}

/// Parses the contents of a `key = value` configuration file.
///
/// Blank lines and lines starting with `#` are ignored. Every supported key is
/// required; unknown keys and malformed or missing values produce a descriptive
/// error rather than falling back to defaults. A leading `~/` in `file_path` is
/// expanded to `home` (see [`expand_tilde`]).
pub fn parse_config(contents: &str, home: Option<&str>) -> Result<Config, String> {
    let mut file_path: Option<String> = None;
    let mut start_time: Option<bool> = None;
    let mut end_time: Option<bool> = None;
    let mut duration: Option<bool> = None;
    let mut duration_rounding: Option<i32> = None;

    for (index, raw_line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(|| {
            format!(
                "Line {}: expected 'key = value', got '{}'",
                line_number, raw_line
            )
        })?;
        let key = key.trim();
        let value = value.trim();
        match key {
            "file_path" => file_path = Some(expand_tilde(value, home)),
            "start_time" => start_time = Some(parse_bool(key, value, line_number)?),
            "end_time" => end_time = Some(parse_bool(key, value, line_number)?),
            "duration" => duration = Some(parse_bool(key, value, line_number)?),
            "duration_rounding" => {
                let parsed: i32 = value.parse().map_err(|_| {
                    format!(
                        "Line {}: '{}' must be an integer, got '{}'",
                        line_number, key, value
                    )
                })?;
                if parsed <= 0 {
                    return Err(format!(
                        "Line {}: '{}' must be positive, got {}",
                        line_number, key, parsed
                    ));
                }
                duration_rounding = Some(parsed);
            }
            _ => {
                return Err(format!("Line {}: unknown key '{}'", line_number, key));
            }
        }
    }

    let file_path = file_path.ok_or_else(|| "Missing 'file_path' setting.".to_string())?;
    let start_time = start_time.ok_or_else(|| "Missing 'start_time' setting.".to_string())?;
    let end_time = end_time.ok_or_else(|| "Missing 'end_time' setting.".to_string())?;
    let duration = duration.ok_or_else(|| "Missing 'duration' setting.".to_string())?;
    let duration_rounding =
        duration_rounding.ok_or_else(|| "Missing 'duration_rounding' setting.".to_string())?;

    Ok(Config {
        destination: Destination::TextFile(file_path),
        format: RecordsFormat {
            start_time,
            end_time,
            duration,
            duration_rounding,
        },
    })
}

/// Serialises a configuration to the `key = value` text format.
///
/// This is the inverse of [`parse_config`], so `parse_config(&serialize_config(c))`
/// yields `c`. Only a [`Destination::TextFile`] can be represented; a Joplin
/// destination is rejected because Joplin support is not implemented.
pub fn serialize_config(config: &Config) -> Result<String, String> {
    let file_path = match &config.destination {
        Destination::TextFile(path) => path,
        Destination::JoplinNote(_) => {
            return Err("Cannot serialise a Joplin destination to a config file.".to_string());
        }
    };

    Ok(format!(
        r#"# md_timesheet configuration.
# Path to the timesheet markdown file (~/ expands to your home directory).
file_path = {file_path}

# Which table columns to include.
start_time = {}
end_time = {}
duration = {}

# Round durations up to the next multiple of this many minutes.
duration_rounding = {}
"#,
        config.format.start_time,
        config.format.end_time,
        config.format.duration,
        config.format.duration_rounding
    ))
}

/// Returns the default configuration serialised to the config file format.
pub fn default_config_contents() -> String {
    serialize_config(&Config::default()).expect("the default config is always serialisable")
}

/// Returns the `md_timesheet` directory inside the XDG config directory.
///
/// Uses `$XDG_CONFIG_HOME` when set and non-empty, otherwise `$HOME/.config`.
/// Returns an error when neither is available.
pub fn xdg_config_dir(
    xdg_config_home: Option<&str>,
    home: Option<&str>,
) -> Result<PathBuf, String> {
    if let Some(xdg) = xdg_config_home.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(xdg).join("md_timesheet"));
    }
    match home.filter(|value| !value.is_empty()) {
        Some(home) => Ok(PathBuf::from(home).join(".config").join("md_timesheet")),
        None => Err("Neither $XDG_CONFIG_HOME nor $HOME is set.".to_string()),
    }
}

/// Returns the canonical config file path inside the XDG config directory.
pub fn canonical_config_path(
    xdg_config_home: Option<&str>,
    home: Option<&str>,
) -> Result<PathBuf, String> {
    Ok(xdg_config_dir(xdg_config_home, home)?.join("config"))
}

/// Returns the config file path inside the current folder.
pub fn cwd_config_path(cwd: &Path) -> PathBuf {
    cwd.join("md_timesheet.config")
}

/// Finds the config file to use, following the documented search order.
///
/// The path given by the `-c`/`--config` option wins and must exist. Otherwise
/// the current folder (`./md_timesheet.config`) is checked, then the XDG config
/// directory. The XDG file may itself be a pointer containing `config_path =`,
/// in which case the referenced file is used. Returns `Ok(None)` when no config
/// file exists.
pub fn locate_config(
    cli_config: Option<&Path>,
    cwd: &Path,
    xdg_config_home: Option<&str>,
    home: Option<&str>,
) -> Result<Option<PathBuf>, String> {
    if let Some(path) = cli_config {
        if path.is_file() {
            return Ok(Some(path.to_path_buf()));
        }
        return Err(format!("Config file '{}' does not exist.", path.display()));
    }

    let cwd_path = cwd_config_path(cwd);
    if cwd_path.is_file() {
        return Ok(Some(cwd_path));
    }

    if let Ok(xdg_path) = canonical_config_path(xdg_config_home, home) {
        if xdg_path.is_file() {
            let contents = fs::read_to_string(&xdg_path).map_err(|e| {
                format!("Error reading config file '{}': {}", xdg_path.display(), e)
            })?;
            return match pointer_target(&contents) {
                Some(target) => {
                    if target.is_file() {
                        Ok(Some(target))
                    } else {
                        Err(format!(
                            "Config pointer '{}' points at '{}', which does not exist.",
                            xdg_path.display(),
                            target.display()
                        ))
                    }
                }
                None => Ok(Some(xdg_path)),
            };
        }
    }

    Ok(None)
}

/// Extracts the config file path from a `config_path =` pointer.
///
/// Returns `None` when `contents` is not a pointer, which is how the XDG config
/// file distinguishes a pointer from a regular configuration. Comments and
/// blank lines are ignored, matching [`parse_config`].
pub fn pointer_target(contents: &str) -> Option<PathBuf> {
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            if key.trim() == "config_path" {
                let value = value.trim();
                if !value.is_empty() {
                    return Some(PathBuf::from(value));
                }
            }
        }
    }
    None
}

/// Writes `contents` to the config file at `path`, creating parent directories.
pub fn write_config(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Error creating directory '{}': {}", parent.display(), e))?;
        }
    }
    fs::write(path, contents)
        .map_err(|e| format!("Error writing config file '{}': {}", path.display(), e))
}

/// Writes a pointer at `xdg_path` referencing the config file at `target`.
pub fn write_pointer(xdg_path: &Path, target: &Path) -> Result<(), String> {
    let contents = format!("config_path = {}\n", target.display());
    write_config(xdg_path, &contents)
}

/// Reads and parses the config file at `path`.
///
/// `home` is used to expand a leading `~/` in `file_path`.
pub fn load_config_from(path: &Path, home: Option<&str>) -> Result<Config, String> {
    let contents = fs::read_to_string(path)
        .map_err(|e| format!("Error reading config file '{}': {}", path.display(), e))?;
    parse_config(&contents, home)
}

/// Expands a leading `~` or `~/` in `path` to the `home` directory.
///
/// Paths that do not start with `~`, and any path when `home` is missing or
/// empty, are returned unchanged.
pub fn expand_tilde(path: &str, home: Option<&str>) -> String {
    let home = match home.filter(|value| !value.is_empty()) {
        Some(home) => home,
        None => return path.to_string(),
    };
    if path == "~" {
        return home.to_string();
    }
    match path.strip_prefix("~/") {
        Some(rest) => format!("{}/{}", home, rest),
        None => path.to_string(),
    }
}

/// Collapses a `home` prefix in `path` back to a leading `~`.
///
/// This is the inverse of [`expand_tilde`]: a path equal to `home` becomes `~`
/// and a path below `home` becomes `~/...`. Other paths, and any path when
/// `home` is missing or empty, are returned unchanged.
pub fn collapse_tilde(path: &str, home: Option<&str>) -> String {
    let home = match home.filter(|value| !value.is_empty()) {
        Some(home) => home,
        None => return path.to_string(),
    };
    if path == home {
        return "~".to_string();
    }
    let prefix = format!("{}/", home);
    match path.strip_prefix(&prefix) {
        Some(rest) => format!("~/{}", rest),
        None => path.to_string(),
    }
}

/// Parses a boolean config value, naming the key and line on failure.
fn parse_bool(key: &str, value: &str, line_number: usize) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!(
            "Line {}: '{}' must be 'true' or 'false', got '{}'",
            line_number, key, value
        )),
    }
}

/// Reads the whole document as a list of lines.
///
/// For [`Destination::TextFile`] the file is created when missing, so reading
/// a not-yet-existing timesheet yields an empty vector rather than an error.
/// [`Destination::JoplinNote`] is not implemented and always errors.
pub fn read_document(dest: &Destination) -> Result<Vec<String>, String> {
    match dest {
        Destination::JoplinNote(_) => Err("Joplin support has not been implemented.".to_string()),
        Destination::TextFile(file_path) => {
            let file: Result<std::fs::File, std::io::Error> = fs::OpenOptions::new()
                .write(true)
                .read(true)
                .create(true)
                .truncate(false)
                .open(file_path);
            match file {
                Err(e) => Err(format!(
                    "Error opening file for reading or creating a new file: {}",
                    e
                )),
                Ok(file) => {
                    let reader = BufReader::new(file);
                    let lines: Result<Vec<String>, std::io::Error> =
                        reader.lines().collect::<Result<_, _>>();
                    match lines {
                        Err(e) => Err(format!(
                            "Error while splitting file content into lines: {}",
                            e
                        )),
                        Ok(lines) => Ok(lines),
                    }
                }
            }
        }
    }
}

/// Reads the whole document as a list of lines without creating it.
///
/// Unlike [`read_document`], a missing [`Destination::TextFile`] yields an
/// empty vector and is left uncreated, so merely previewing the document (for
/// the "Worked for" label) has no side effects on disk.
/// [`Destination::JoplinNote`] is not implemented and always errors.
pub fn peek_document(dest: &Destination) -> Result<Vec<String>, String> {
    match dest {
        Destination::TextFile(file_path) if !Path::new(file_path).exists() => Ok(Vec::new()),
        _ => read_document(dest),
    }
}

/// Overwrites the document with `lines`, one line per element.
///
/// For [`Destination::TextFile`] the file is truncated first.
/// [`Destination::JoplinNote`] is not implemented and always errors.
pub fn write_document(dest: &Destination, lines: Vec<String>) -> Result<(), String> {
    match dest {
        Destination::JoplinNote(_) => Err("Joplin support has not been implemented.".to_string()),
        Destination::TextFile(file_path) => {
            let file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(file_path);
            match file {
                Err(_) => Err("Error opening file for write".to_string()),
                Ok(mut file) => {
                    for line in lines {
                        writeln!(file, "{}", line).map_err(|e| e.to_string())?;
                    }
                    Ok(())
                }
            }
        }
    }
}

/// Builds the lines that start a new day: the heading, a blank line and the
/// table header (with only the columns enabled in `format`).
pub fn new_day(date: &NaiveDateTime, format: &RecordsFormat) -> Vec<String> {
    let mut day_header: Vec<String> = Vec::new();
    day_header.push(format!("{}{}", "#  ", date.format("%e %B")));
    day_header.push("".to_string());

    let mut columns: Vec<String> = Vec::new();
    columns.push("Description".to_string());
    if format.start_time {
        columns.push("Start Time".to_string());
    }
    if format.end_time {
        columns.push("End Time".to_string());
    }
    if format.duration {
        columns.push("Duration".to_string());
    }

    day_header.push(format!("| {} |", columns.join(" | ")));
    day_header.push(format!(
        "| {} |",
        columns
            .iter()
            .map(|_| "---")
            .collect::<Vec<&str>>()
            .join(" | ")
    ));

    day_header
}

/// Rounds `minutes` up to the next multiple of `rounding`.
///
/// Exact multiples (including 0) are left unchanged. This is the rounding
/// applied to the "Duration" column, shared with the button preview so the two
/// can never disagree.
pub fn round_up_minutes(minutes: i64, rounding: i32) -> i64 {
    let rounding = rounding as i64;
    (minutes / rounding) * rounding + if minutes % rounding > 0 { rounding } else { 0 }
}

/// Formats a number of minutes as zero-padded `HH:MM`.
///
/// Hours are not wrapped at 24, so 1500 minutes is `25:00`.
pub fn format_hhmm(minutes: i64) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// The longest gap, in minutes, for which [`pending_duration`] still reports
/// a value (24 hours).
pub const MAX_PENDING_MINUTES: i64 = 24 * 60;

/// Returns the timestamp of the last operation stored on the final line.
///
/// Uses the same rule as [`apply_start`]/[`apply_worked`]: only the very last
/// line is inspected. Returns `None` for an empty document or an unparsable
/// final line.
pub fn last_timestamp(lines: &[String]) -> Option<NaiveDateTime> {
    let last_line = lines.last()?;
    NaiveDateTime::parse_from_str(last_line, TIMESTAMP_FORMAT).ok()
}

/// Computes the rounded duration that would be recorded for a task running
/// from `since` until `now`, formatted as `HH:MM`.
///
/// Returns `None` when there is no starting point, when `since` is in the
/// future (clock skew), or when more than [`MAX_PENDING_MINUTES`] have passed;
/// a gap of exactly 24 hours is still reported.
pub fn pending_duration(
    since: Option<NaiveDateTime>,
    now: NaiveDateTime,
    format: &RecordsFormat,
) -> Option<String> {
    let since = since?;
    if since > now {
        return None;
    }
    let minutes = now.signed_duration_since(since).num_minutes();
    if minutes > MAX_PENDING_MINUTES {
        return None;
    }
    Some(format_hhmm(round_up_minutes(
        minutes,
        format.duration_rounding,
    )))
}

/// Builds the "Worked for" button label from a [`pending_duration`] result.
///
/// `None` is shown as `--:--`.
pub fn button_label(pending: Option<&str>) -> String {
    format!("Worked for ({})", pending.unwrap_or("--:--"))
}

/// Builds the single table row describing a completed task.
///
/// The duration is rounded up to the next multiple of
/// `format.duration_rounding` minutes and printed as `HH:MM`.
pub fn new_entry(
    description: String,
    start_date: &NaiveDateTime,
    end_date: &NaiveDateTime,
    format: &RecordsFormat,
) -> Vec<String> {
    let time_format = "%H:%M";
    let mut day_header: Vec<String> = Vec::new();

    let duration: i64 = round_up_minutes(
        end_date.signed_duration_since(*start_date).num_minutes(),
        format.duration_rounding,
    );

    let mut columns: Vec<String> = Vec::new();
    columns.push(description);
    if format.start_time {
        columns.push(start_date.format(time_format).to_string());
    }
    if format.end_time {
        columns.push(end_date.format(time_format).to_string());
    }
    if format.duration {
        columns.push(format_hhmm(duration));
    }

    day_header.push(format!("| {} |", columns.join(" | ")));

    day_header
}

/// Applies the "Start" action to `lines` in place.
///
/// The current timestamp is written to the final line. On an empty document a
/// whole new day is created first; when the previous timestamp belongs to an
/// earlier day a fresh day is inserted before it.
///
/// Returns `Err` when the final line is not a parsable timestamp, leaving
/// `lines` unchanged in that case.
pub fn apply_start(
    lines: &mut Vec<String>,
    current_datetime: NaiveDateTime,
    format: &RecordsFormat,
) -> Result<(), String> {
    if lines.is_empty() {
        lines.append(&mut new_day(&current_datetime, format));
        lines.push(current_datetime.format(TIMESTAMP_FORMAT).to_string());
    } else if let Some(last_line) = lines.last() {
        match NaiveDateTime::parse_from_str(last_line, TIMESTAMP_FORMAT) {
            Ok(previous_datetime) => {
                if previous_datetime.date() == current_datetime.date() {
                    lines.pop();
                    lines.push(current_datetime.format(TIMESTAMP_FORMAT).to_string());
                } else {
                    lines.pop();
                    lines.push("".to_string());
                    lines.append(&mut new_day(&current_datetime, format));
                    lines.push(current_datetime.format(TIMESTAMP_FORMAT).to_string());
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

/// Applies the "Worked on" action to `lines` in place.
///
/// `description` is added as a table row spanning from the previous timestamp
/// to `current_datetime`, which is then written to the final line. On an empty
/// document a whole new day is created first; when the previous timestamp
/// belongs to an earlier day the entry is added to that day and a new day is
/// started afterwards. After the new row is added, the day's table it
/// belongs to (and only that one) is re-padded (see [`pad_last_table`]) so
/// its columns stay aligned.
///
/// Returns `Err` when the final line is not a parsable timestamp, leaving
/// `lines` unchanged in that case.
pub fn apply_worked(
    lines: &mut Vec<String>,
    description: String,
    current_datetime: NaiveDateTime,
    format: &RecordsFormat,
) -> Result<(), String> {
    if lines.is_empty() {
        lines.append(&mut new_day(&current_datetime, format));
        lines.append(&mut new_entry(
            description,
            &current_datetime,
            &current_datetime,
            format,
        ));
        pad_last_table(lines);
        lines.push(current_datetime.format(TIMESTAMP_FORMAT).to_string());
    } else if let Some(last_line) = lines.last() {
        match NaiveDateTime::parse_from_str(last_line, TIMESTAMP_FORMAT) {
            Ok(previous_datetime) => {
                if previous_datetime.date() == current_datetime.date() {
                    lines.pop();
                    lines.append(&mut new_entry(
                        description,
                        &previous_datetime,
                        &current_datetime,
                        format,
                    ));
                    pad_last_table(lines);
                    lines.push(current_datetime.format(TIMESTAMP_FORMAT).to_string());
                } else {
                    lines.pop();
                    lines.append(&mut new_entry(
                        description,
                        &previous_datetime,
                        &current_datetime,
                        format,
                    ));
                    pad_last_table(lines);
                    lines.push("".to_string());
                    lines.append(&mut new_day(&current_datetime, format));
                    lines.push(current_datetime.format(TIMESTAMP_FORMAT).to_string());
                }
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

/// Returns the index of the last entry row in `lines`, if there is one.
///
/// Only table data rows count; the day header's column header and its `---`
/// separator are ignored.
fn last_entry_index(lines: &[String]) -> Option<usize> {
    lines.iter().rposition(|line| is_entry_row(line))
}

/// Returns whether `line` is a table data row rather than the header or the
/// separator.
fn is_entry_row(line: &str) -> bool {
    matches!(first_cell(line), Some(cell) if cell != "Description" && cell != "---")
}

/// Splits a Markdown table `line` into its trimmed cells.
///
/// Returns `None` when `line` is not a `| ... |` row. Each cell is trimmed,
/// so this is tolerant of the column padding added by [`pad_table_rows`]:
/// `"| 12:00      |"` and `"| 12:00 |"` both yield `["12:00"]`.
fn table_cells(line: &str) -> Option<Vec<&str>> {
    let trimmed = line.trim();
    if trimmed.len() < 2 || !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return None;
    }
    Some(
        trimmed[1..trimmed.len() - 1]
            .split('|')
            .map(str::trim)
            .collect(),
    )
}

/// Returns whether `line` parses as a Markdown table row at all (header,
/// separator or data row).
fn is_table_row(line: &str) -> bool {
    table_cells(line).is_some()
}

/// Extracts the first cell of a Markdown table `line`.
///
/// Returns `None` when `line` is not a `| ... |` row.
fn first_cell(line: &str) -> Option<&str> {
    table_cells(line)?.into_iter().next()
}

/// Extracts the start time from an entry row that has a "Start Time" column.
fn entry_start_time(line: &str) -> Option<NaiveTime> {
    let cells = table_cells(line)?;
    NaiveTime::parse_from_str(cells.get(1)?, "%H:%M").ok()
}

/// Re-pads every cell in `rows` so each column has a common width.
///
/// `rows` must all be `| ... |` table rows (header, separator and data rows
/// alike) belonging to the same table; the separator row is padded like any
/// other row (its cell text stays exactly `"---"`, it just gets trailing
/// spaces so its `|` characters still line up with the rest of the table).
/// Column width is measured in `char`s. Rows with fewer cells than the
/// widest row are padded only for the cells they have.
fn pad_table_rows(rows: &mut [String]) {
    let parsed: Vec<Vec<String>> = rows
        .iter()
        .filter_map(|r| table_cells(r))
        .map(|cells| cells.into_iter().map(str::to_string).collect())
        .collect();
    let columns = parsed.iter().map(|cells| cells.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; columns];
    for cells in &parsed {
        for (i, cell) in cells.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }
    for (line, cells) in rows.iter_mut().zip(parsed.iter()) {
        let padded: Vec<String> = cells
            .iter()
            .enumerate()
            .map(|(i, cell)| format!("{:<width$}", cell, width = widths[i]))
            .collect();
        *line = format!("| {} |", padded.join(" | "));
    }
}

/// Returns the start and one-past-the-end index of the contiguous run of
/// table rows in `lines` that contains `index`.
///
/// Walks backward and forward from `index` while the neighbouring line
/// [`is_table_row`], stopping at the first blank line, day heading or
/// timestamp line in either direction. This is what keeps padding scoped to
/// a single day's table instead of a neighbouring one.
fn table_bounds(lines: &[String], index: usize) -> (usize, usize) {
    let mut start = index;
    while start > 0 && is_table_row(&lines[start - 1]) {
        start -= 1;
    }
    let mut end = index + 1;
    while end < lines.len() && is_table_row(&lines[end]) {
        end += 1;
    }
    (start, end)
}

/// Re-pads the table at the very end of `lines`.
///
/// Intended to be called right after appending a new row and before
/// appending anything else (a day heading, the trailing timestamp, ...), so
/// the backward walk finds exactly the table that row was just added to.
/// Does nothing when `lines` is empty.
fn pad_last_table(lines: &mut [String]) {
    let len = lines.len();
    if len == 0 {
        return;
    }
    let mut start = len;
    while start > 0 && is_table_row(&lines[start - 1]) {
        start -= 1;
    }
    pad_table_rows(&mut lines[start..len]);
}

/// Returns the description of the last entry in the document, if there is one.
///
/// The day header and its separator are ignored, so a document that only
/// contains day headers yields `None`.
pub fn last_entry_description(lines: &[String]) -> Option<String> {
    let index = last_entry_index(lines)?;
    first_cell(&lines[index]).map(|cell| cell.to_string())
}

/// Returns when the last entry started, as used by [`amend_last_entry`].
///
/// The stored `HH:MM` start time is placed on `current_datetime`'s date; when
/// that would be later than `current_datetime` (the entry actually started
/// before midnight) it is moved back one day.
///
/// Requires the format to include the "Start Time" column. Returns `Err` when
/// there is no entry, the start time cannot be parsed, or the final line is
/// not a valid timestamp - exactly the cases in which amending would fail.
pub fn last_entry_start(
    lines: &[String],
    current_datetime: NaiveDateTime,
    format: &RecordsFormat,
) -> Result<NaiveDateTime, String> {
    if !format.start_time {
        return Err("Cannot amend without a Start Time column.".to_string());
    }

    let index = last_entry_index(lines).ok_or_else(|| "There is no entry to amend.".to_string())?;

    let start_time = entry_start_time(&lines[index])
        .ok_or_else(|| "The last entry has no parsable start time.".to_string())?;

    if last_timestamp(lines).is_none() {
        return Err("The last line is not a valid timestamp.".to_string());
    }

    let mut start_datetime = current_datetime.date().and_time(start_time);
    if start_datetime > current_datetime {
        start_datetime -= Duration::days(1);
    }
    Ok(start_datetime)
}

/// Applies the "amend last entry" action to `lines` in place.
///
/// The last entry row keeps its start time (see [`last_entry_start`]) but gets
/// the new `description`, an end time of `current_datetime` and a recomputed
/// duration; the trailing timestamp is then updated. The whole day's table
/// the amended entry belongs to (header, separator and every entry; see
/// [`table_bounds`]) is then re-padded, since the new description's length
/// may change the required column width.
///
/// Requires the format to include the "Start Time" column. Returns `Err`,
/// leaving `lines` unchanged, when there is no entry, the start time cannot be
/// parsed, or the final line is not a valid timestamp.
pub fn amend_last_entry(
    lines: &mut [String],
    description: String,
    current_datetime: NaiveDateTime,
    format: &RecordsFormat,
) -> Result<(), String> {
    let start_datetime = last_entry_start(lines, current_datetime, format)?;
    let index = last_entry_index(lines).ok_or_else(|| "There is no entry to amend.".to_string())?;

    let mut row = new_entry(description, &start_datetime, &current_datetime, format);
    lines[index] = row
        .pop()
        .ok_or_else(|| "Could not build the amended entry.".to_string())?;

    let (start, end) = table_bounds(lines, index);
    pad_table_rows(&mut lines[start..end]);

    let last_index = lines.len() - 1;
    lines[last_index] = current_datetime.format(TIMESTAMP_FORMAT).to_string();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `parse_bool` accepts the two documented literals.
    #[test]
    fn parse_bool_accepts_true_and_false() {
        assert!(parse_bool("start_time", "true", 1).unwrap());
        assert!(!parse_bool("start_time", "false", 1).unwrap());
    }

    /// `parse_bool` rejects anything else and names the key.
    #[test]
    fn parse_bool_rejects_other_values() {
        let error = parse_bool("start_time", "yes", 3).unwrap_err();
        assert!(error.contains("start_time"));
        assert!(error.contains("Line 3"));
    }

    /// `table_cells` splits a row into trimmed cells.
    #[test]
    fn table_cells_splits_and_trims() {
        assert_eq!(
            table_cells("| Doing Stuff | 12:00 |"),
            Some(vec!["Doing Stuff", "12:00"])
        );
        assert_eq!(
            table_cells("| 12:00      | 13:02    |"),
            Some(vec!["12:00", "13:02"])
        );
    }

    /// `table_cells` returns `None` for anything that is not a `| ... |` row.
    #[test]
    fn table_cells_rejects_non_rows() {
        assert_eq!(table_cells("#  2 January"), None);
        assert_eq!(table_cells(""), None);
        assert_eq!(table_cells("24/12/2024 15:41"), None);
        assert_eq!(table_cells("|"), None);
    }

    /// `is_table_row` recognises header, separator and data rows alike.
    #[test]
    fn is_table_row_recognises_every_kind_of_row() {
        assert!(is_table_row("| Description | Start Time |"));
        assert!(is_table_row("| --- | --- |"));
        assert!(is_table_row("| Doing Stuff | 12:00 |"));
        assert!(!is_table_row("#  2 January"));
        assert!(!is_table_row(""));
    }

    /// `pad_table_rows` widens every row's cells to the widest cell in each
    /// column, including the separator row, and leaves shorter cells intact.
    #[test]
    fn pad_table_rows_aligns_columns() {
        let mut rows = vec![
            "| Description | Start Time |".to_string(),
            "| --- | --- |".to_string(),
            "| A longer task | 09:30 |".to_string(),
        ];
        pad_table_rows(&mut rows);
        assert_eq!(rows[0], "| Description   | Start Time |");
        assert_eq!(rows[1], "| ---           | ---        |");
        assert_eq!(rows[2], "| A longer task | 09:30      |");
    }

    /// `pad_table_rows` is idempotent: padding an already-padded table
    /// reproduces the same text.
    #[test]
    fn pad_table_rows_is_idempotent() {
        let mut rows = vec![
            "| Description | Start Time |".to_string(),
            "| --- | --- |".to_string(),
            "| A longer task | 09:30 |".to_string(),
        ];
        pad_table_rows(&mut rows);
        let once = rows.clone();
        pad_table_rows(&mut rows);
        assert_eq!(rows, once);
    }

    /// `table_bounds` finds exactly the contiguous table rows around a given
    /// index, stopping at the blank line separating two days.
    #[test]
    fn table_bounds_stops_at_day_boundary() {
        let lines = vec![
            "#  1 January".to_string(),
            "".to_string(),
            "| Description | Start Time |".to_string(),
            "| --- | --- |".to_string(),
            "| First | 09:00 |".to_string(),
            "".to_string(),
            "#  2 January".to_string(),
            "".to_string(),
            "| Description | Start Time |".to_string(),
            "| --- | --- |".to_string(),
            "| Second | 09:00 |".to_string(),
            "24/12/2024 15:41".to_string(),
        ];
        assert_eq!(table_bounds(&lines, 4), (2, 5));
        assert_eq!(table_bounds(&lines, 10), (8, 11));
    }

    /// `pad_last_table` re-pads only the table at the end of `lines`,
    /// leaving an earlier, already-closed day's table untouched.
    #[test]
    fn pad_last_table_only_touches_the_last_table() {
        let mut lines = vec![
            "#  1 January".to_string(),
            "".to_string(),
            "| Description | Start Time |".to_string(),
            "| --- | --- |".to_string(),
            "| First | 09:00 |".to_string(),
            "".to_string(),
            "#  2 January".to_string(),
            "".to_string(),
            "| Description | Start Time |".to_string(),
            "| --- | --- |".to_string(),
            "| A much longer second task | 09:00 |".to_string(),
        ];
        let first_day_before = lines[2..5].to_vec();
        pad_last_table(&mut lines);
        assert_eq!(lines[2..5], first_day_before[..]);
        assert_eq!(
            table_cells(&lines[8]),
            Some(vec!["Description", "Start Time"])
        );
        assert_eq!(
            table_cells(&lines[10]),
            Some(vec!["A much longer second task", "09:00"])
        );
        // The description column widened to fit the new entry, so both the
        // header and the entry row now share the same, longer, column width.
        assert_eq!(
            format!("| {:<25} | {:<10} |", "Description", "Start Time"),
            lines[8]
        );
        assert_eq!(
            format!("| {:<25} | {:<10} |", "A much longer second task", "09:00"),
            lines[10]
        );
    }
}
