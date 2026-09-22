//! Integration tests for the pure state transitions behind the buttons.

use chrono::NaiveDate;
use md_timesheet::{apply_start, apply_worked, new_day, RECORD_FORMAT};

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

/// "Start" on an empty document creates a day and writes the timestamp.
#[test]
fn start_empty_document() {
    let mut lines = Vec::new();
    apply_start(&mut lines, dt(2024, 1, 2, 12, 0), &RECORD_FORMAT).unwrap();

    let mut expected = january_second();
    expected.push("02/01/2024 12:00".to_string());
    assert_eq!(lines, expected);
}

/// "Start" on a previous day inserts a blank line and a fresh day header.
#[test]
fn start_rolls_over_day() {
    let mut lines = january_first();
    lines.push("01/01/2024 10:00".to_string());

    apply_start(&mut lines, dt(2024, 1, 2, 9, 0), &RECORD_FORMAT).unwrap();

    let mut expected = january_first();
    expected.push("".to_string());
    expected.extend(new_day(&dt(2024, 1, 2, 9, 0), &RECORD_FORMAT));
    expected.push("02/01/2024 09:00".to_string());
    assert_eq!(lines, expected);
}

/// "Start" on the same day only updates the trailing timestamp.
#[test]
fn start_same_day_updates_timestamp() {
    let mut lines = january_second();
    lines.push("02/01/2024 10:00".to_string());

    apply_start(&mut lines, dt(2024, 1, 2, 12, 0), &RECORD_FORMAT).unwrap();

    let mut expected = january_second();
    expected.push("02/01/2024 12:00".to_string());
    assert_eq!(lines, expected);
}

/// "Start" with an unparsable trailing line errors and changes nothing.
#[test]
fn start_unparsable_last_line() {
    let mut lines = january_second();
    lines.push("not a timestamp".to_string());
    let before = lines.clone();

    assert!(apply_start(&mut lines, dt(2024, 1, 2, 12, 0), &RECORD_FORMAT).is_err());
    assert_eq!(lines, before);
}

/// "Worked on" on an empty document creates a day and a zero-length entry.
#[test]
fn worked_empty_document() {
    let mut lines = Vec::new();
    apply_worked(
        &mut lines,
        "Doing Stuff".to_string(),
        dt(2024, 1, 2, 12, 0),
        &RECORD_FORMAT,
    )
    .unwrap();

    let mut expected = january_second();
    expected.push("| Doing Stuff | 12:00 | 12:00 | 00:00 |".to_string());
    expected.push("02/01/2024 12:00".to_string());
    assert_eq!(lines, expected);
}

/// "Worked on" on the same day appends an entry from the previous timestamp.
#[test]
fn worked_same_day() {
    let mut lines = january_second();
    lines.push("02/01/2024 10:00".to_string());

    apply_worked(
        &mut lines,
        "Doing Stuff".to_string(),
        dt(2024, 1, 2, 11, 30),
        &RECORD_FORMAT,
    )
    .unwrap();

    let mut expected = january_second();
    expected.push("| Doing Stuff | 10:00 | 11:30 | 01:30 |".to_string());
    expected.push("02/01/2024 11:30".to_string());
    assert_eq!(lines, expected);
}

/// "Worked on" across midnight closes the previous day and starts a new one.
#[test]
fn worked_rolls_over_midnight() {
    let mut lines = january_first();
    lines.push("01/01/2024 23:30".to_string());

    apply_worked(
        &mut lines,
        "Doing Stuff".to_string(),
        dt(2024, 1, 2, 0, 20),
        &RECORD_FORMAT,
    )
    .unwrap();

    let mut expected = january_first();
    expected.push("| Doing Stuff | 23:30 | 00:20 | 00:50 |".to_string());
    expected.push("".to_string());
    expected.extend(new_day(&dt(2024, 1, 2, 0, 20), &RECORD_FORMAT));
    expected.push("02/01/2024 00:20".to_string());
    assert_eq!(lines, expected);
}

/// "Worked on" with an unparsable trailing line errors and changes nothing.
#[test]
fn worked_unparsable_last_line() {
    let mut lines = january_second();
    lines.push("garbage".to_string());
    let before = lines.clone();

    assert!(apply_worked(
        &mut lines,
        "Doing Stuff".to_string(),
        dt(2024, 1, 2, 12, 0),
        &RECORD_FORMAT,
    )
    .is_err());
    assert_eq!(lines, before);
}
