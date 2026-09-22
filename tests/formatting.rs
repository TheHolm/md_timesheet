//! Integration tests for Markdown day/entry generation.

use chrono::NaiveDate;
use md_timesheet::{new_day, new_entry, RecordsFormat, RECORD_FORMAT};

/// Builds a `NaiveDateTime` for the given calendar fields.
fn dt(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(year, month, day)
        .unwrap()
        .and_hms_opt(hour, minute, 0)
        .unwrap()
}

/// A fully-featured day header lists every enabled column.
#[test]
fn new_day_all_columns() {
    let day = new_day(&dt(2024, 1, 2, 12, 0), &RECORD_FORMAT);
    assert_eq!(
        day,
        vec![
            "#   2 January",
            "",
            "| Description | Start Time | End Time | Duration |",
            "| --- | --- | --- | --- |",
        ]
    );
}

/// Disabled columns are omitted from the day header and its separator.
#[test]
fn new_day_subset_of_columns() {
    let format = RecordsFormat {
        start_time: false,
        end_time: false,
        duration: true,
        duration_rounding: 10,
    };
    let day = new_day(&dt(2024, 1, 2, 12, 0), &format);
    assert_eq!(
        day,
        vec![
            "#   2 January",
            "",
            "| Description | Duration |",
            "| --- | --- |",
        ]
    );
}

/// With every optional column disabled only the description column remains.
#[test]
fn new_day_description_only() {
    let format = RecordsFormat {
        start_time: false,
        end_time: false,
        duration: false,
        duration_rounding: 10,
    };
    let day = new_day(&dt(2024, 12, 31, 0, 0), &format);
    assert_eq!(day[0], "#  31 December");
    assert_eq!(day[2], "| Description |");
    assert_eq!(day[3], "| --- |");
}

/// A partial minute is rounded up to the next whole rounding step.
#[test]
fn new_entry_rounds_up() {
    let entry = new_entry(
        "Doing Stuff".to_string(),
        &dt(2024, 1, 2, 12, 0),
        &dt(2024, 1, 2, 13, 2),
        &RECORD_FORMAT,
    );
    assert_eq!(entry, vec!["| Doing Stuff | 12:00 | 13:02 | 01:10 |"]);
}

/// A duration already on a rounding boundary is left unchanged.
#[test]
fn new_entry_exact_multiple() {
    let entry = new_entry(
        "Doing Stuff".to_string(),
        &dt(2024, 1, 2, 12, 0),
        &dt(2024, 1, 2, 13, 0),
        &RECORD_FORMAT,
    );
    assert_eq!(entry, vec!["| Doing Stuff | 12:00 | 13:00 | 01:00 |"]);
}

/// A zero-length entry rounds to `00:00`.
#[test]
fn new_entry_zero_duration() {
    let entry = new_entry(
        "Doing Stuff".to_string(),
        &dt(2024, 1, 2, 12, 0),
        &dt(2024, 1, 2, 12, 0),
        &RECORD_FORMAT,
    );
    assert_eq!(entry, vec!["| Doing Stuff | 12:00 | 12:00 | 00:00 |"]);
}

/// Disabled columns are omitted from an entry row.
#[test]
fn new_entry_description_only() {
    let format = RecordsFormat {
        start_time: false,
        end_time: false,
        duration: false,
        duration_rounding: 10,
    };
    let entry = new_entry(
        "Doing Stuff".to_string(),
        &dt(2024, 1, 2, 12, 0),
        &dt(2024, 1, 2, 13, 0),
        &format,
    );
    assert_eq!(entry, vec!["| Doing Stuff |"]);
}

/// A custom rounding step is honoured.
#[test]
fn new_entry_custom_rounding() {
    let format = RecordsFormat {
        start_time: false,
        end_time: true,
        duration: true,
        duration_rounding: 15,
    };
    let entry = new_entry(
        "Doing Stuff".to_string(),
        &dt(2024, 1, 2, 12, 0),
        &dt(2024, 1, 2, 12, 16),
        &format,
    );
    assert_eq!(entry, vec!["| Doing Stuff | 12:16 | 00:30 |"]);
}
