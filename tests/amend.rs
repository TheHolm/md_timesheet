//! Integration tests for finding and amending the last timesheet entry.

use chrono::NaiveDate;
use md_timesheet::{amend_last_entry, last_entry_description, RecordsFormat, RECORD_FORMAT};

/// Builds a `NaiveDateTime` for the given calendar fields.
fn dt(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(year, month, day)
        .unwrap()
        .and_hms_opt(hour, minute, 0)
        .unwrap()
}

/// A pre-existing day header for 1 January.
fn january_first() -> Vec<String> {
    vec![
        "#   1 January".to_string(),
        "".to_string(),
        "| Description | Start Time | End Time | Duration |".to_string(),
        "| --- | --- | --- | --- |".to_string(),
    ]
}

/// A pre-existing day header for 2 January.
fn january_second() -> Vec<String> {
    vec![
        "#   2 January".to_string(),
        "".to_string(),
        "| Description | Start Time | End Time | Duration |".to_string(),
        "| --- | --- | --- | --- |".to_string(),
    ]
}

/// An empty document has no entry to describe.
#[test]
fn last_entry_description_empty_document() {
    assert_eq!(last_entry_description(&[]), None);
}

/// A day header alone (including its separator) is not an entry.
#[test]
fn last_entry_description_header_only() {
    assert_eq!(last_entry_description(&january_second()), None);
}

/// A description-only row (all optional columns disabled) is still an entry.
#[test]
fn last_entry_description_single_column_row() {
    let lines = vec!["| Doing Stuff |".to_string()];

    assert_eq!(
        last_entry_description(&lines),
        Some("Doing Stuff".to_string())
    );
}

/// The description of the single entry is returned.
#[test]
fn last_entry_description_single_entry() {
    let mut lines = january_second();
    lines.push("| Doing Stuff | 12:00 | 13:02 | 01:10 |".to_string());
    lines.push("02/01/2024 13:02".to_string());

    assert_eq!(
        last_entry_description(&lines),
        Some("Doing Stuff".to_string())
    );
}

/// When there are several entries the last one wins.
#[test]
fn last_entry_description_returns_last_of_many() {
    let mut lines = january_second();
    lines.push("| First | 09:00 | 10:00 | 01:00 |".to_string());
    lines.push("| Second | 10:00 | 11:00 | 01:00 |".to_string());
    lines.push("02/01/2024 11:00".to_string());

    assert_eq!(last_entry_description(&lines), Some("Second".to_string()));
}

/// Amending rewrites the last entry and the trailing timestamp.
#[test]
fn amend_updates_last_entry() {
    let mut lines = january_second();
    lines.push("| Doing Stuff | 12:00 | 13:02 | 01:10 |".to_string());
    lines.push("02/01/2024 13:02".to_string());

    amend_last_entry(
        &mut lines,
        "Revised".to_string(),
        dt(2024, 1, 2, 14, 5),
        &RECORD_FORMAT,
    )
    .unwrap();

    assert_eq!(lines[4], "| Revised | 12:00 | 14:05 | 02:10 |");
    assert_eq!(lines[5], "02/01/2024 14:05");
}

/// Amending leaves every earlier entry untouched.
#[test]
fn amend_preserves_earlier_entries() {
    let mut lines = january_second();
    lines.push("| First | 09:00 | 10:00 | 01:00 |".to_string());
    lines.push("| Second | 10:00 | 11:00 | 01:00 |".to_string());
    lines.push("02/01/2024 11:00".to_string());

    amend_last_entry(
        &mut lines,
        "Second Revised".to_string(),
        dt(2024, 1, 2, 11, 30),
        &RECORD_FORMAT,
    )
    .unwrap();

    assert_eq!(lines[4], "| First | 09:00 | 10:00 | 01:00 |");
    assert_eq!(lines[5], "| Second Revised | 10:00 | 11:30 | 01:30 |");
    assert_eq!(lines[6], "02/01/2024 11:30");
}

/// A start time later than "now" is understood as before midnight.
#[test]
fn amend_handles_midnight_rollover() {
    let mut lines = january_first();
    lines.push("| Doing Stuff | 23:30 | 23:45 | 00:15 |".to_string());
    lines.push("01/01/2024 23:45".to_string());

    amend_last_entry(
        &mut lines,
        "Doing Stuff".to_string(),
        dt(2024, 1, 2, 0, 20),
        &RECORD_FORMAT,
    )
    .unwrap();

    assert_eq!(lines[4], "| Doing Stuff | 23:30 | 00:20 | 00:50 |");
    assert_eq!(lines[5], "02/01/2024 00:20");
}

/// A document with no entry row cannot be amended.
#[test]
fn amend_errors_without_entry() {
    let mut lines = january_second();
    lines.push("02/01/2024 12:00".to_string());
    let before = lines.clone();

    assert!(amend_last_entry(
        &mut lines,
        "Revised".to_string(),
        dt(2024, 1, 2, 12, 0),
        &RECORD_FORMAT,
    )
    .is_err());
    assert_eq!(lines, before);
}

/// An unparsable start time makes the amend fail without changing anything.
#[test]
fn amend_errors_on_unparsable_start_time() {
    let mut lines = january_second();
    lines.push("| Doing Stuff | nope | 13:00 | 01:00 |".to_string());
    lines.push("02/01/2024 13:00".to_string());
    let before = lines.clone();

    assert!(amend_last_entry(
        &mut lines,
        "Revised".to_string(),
        dt(2024, 1, 2, 14, 0),
        &RECORD_FORMAT,
    )
    .is_err());
    assert_eq!(lines, before);
}

/// An unparsable trailing timestamp makes the amend fail without changes.
#[test]
fn amend_errors_on_invalid_timestamp() {
    let mut lines = january_second();
    lines.push("| Doing Stuff | 12:00 | 13:00 | 01:00 |".to_string());
    lines.push("garbage".to_string());
    let before = lines.clone();

    assert!(amend_last_entry(
        &mut lines,
        "Revised".to_string(),
        dt(2024, 1, 2, 14, 0),
        &RECORD_FORMAT,
    )
    .is_err());
    assert_eq!(lines, before);
}

/// A format without the "Start Time" column cannot be amended.
#[test]
fn amend_errors_without_start_time_column() {
    let format = RecordsFormat {
        start_time: false,
        end_time: true,
        duration: true,
        duration_rounding: 10,
    };
    let mut lines = january_second();
    lines.push("| Doing Stuff | 13:00 | 01:00 |".to_string());
    lines.push("02/01/2024 13:00".to_string());
    let before = lines.clone();

    assert!(amend_last_entry(
        &mut lines,
        "Revised".to_string(),
        dt(2024, 1, 2, 14, 0),
        &format,
    )
    .is_err());
    assert_eq!(lines, before);
}
