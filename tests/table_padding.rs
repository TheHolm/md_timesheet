//! Integration tests for the column padding `apply_worked` and
//! `amend_last_entry` apply to the day's table a row was just added to or
//! amended in. Unlike the other integration tests, these compare the exact
//! padded text, since padding itself is what's under test here.

use chrono::NaiveDate;
use md_timesheet::{amend_last_entry, apply_worked, RECORD_FORMAT};

/// Builds a `NaiveDateTime` for the given calendar fields.
fn dt(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(year, month, day)
        .unwrap()
        .and_hms_opt(hour, minute, 0)
        .unwrap()
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

/// When every column's data is no wider than its header, the header defines
/// the width and stays exactly as `new_day` built it; the data row (its
/// Start/End/Duration cells are naturally narrower than the fixed `HH:MM`
/// header labels) still gets padded to match.
#[test]
fn short_description_is_padded_up_to_the_header_width() {
    let mut lines = january_second();
    lines.push("02/01/2024 12:00".to_string());

    apply_worked(
        &mut lines,
        "Short".to_string(),
        dt(2024, 1, 2, 13, 0),
        &RECORD_FORMAT,
    )
    .unwrap();

    assert_eq!(
        lines[2],
        "| Description | Start Time | End Time | Duration |"
    );
    assert_eq!(
        lines[3],
        "| ---         | ---        | ---      | ---      |"
    );
    assert_eq!(
        lines[4],
        "| Short       | 12:00      | 13:00    | 01:00    |"
    );
}

/// A description longer than every header widens that column across the
/// header, the separator and the data row alike.
#[test]
fn long_description_widens_the_description_column() {
    let mut lines = january_second();
    lines.push("02/01/2024 12:00".to_string());

    apply_worked(
        &mut lines,
        "A much longer task description".to_string(),
        dt(2024, 1, 2, 13, 0),
        &RECORD_FORMAT,
    )
    .unwrap();

    assert_eq!(
        lines[2],
        "| Description                    | Start Time | End Time | Duration |"
    );
    assert_eq!(
        lines[3],
        "| ---                            | ---        | ---      | ---      |"
    );
    assert_eq!(
        lines[4],
        "| A much longer task description | 12:00      | 13:00    | 01:00    |"
    );
}

/// Adding a second, longer entry on the same day widens the column for
/// every row written so far, including the header and the first entry.
#[test]
fn a_later_longer_entry_widens_earlier_rows_too() {
    let mut lines = january_second();
    lines.push("02/01/2024 09:00".to_string());

    apply_worked(
        &mut lines,
        "Short".to_string(),
        dt(2024, 1, 2, 10, 0),
        &RECORD_FORMAT,
    )
    .unwrap();
    assert_eq!(
        lines[4],
        "| Short       | 09:00      | 10:00    | 01:00    |"
    );

    apply_worked(
        &mut lines,
        "A much longer second task".to_string(),
        dt(2024, 1, 2, 11, 0),
        &RECORD_FORMAT,
    )
    .unwrap();

    assert_eq!(
        lines[2],
        "| Description               | Start Time | End Time | Duration |"
    );
    assert_eq!(
        lines[4],
        "| Short                     | 09:00      | 10:00    | 01:00    |"
    );
    assert_eq!(
        lines[5],
        "| A much longer second task | 10:00      | 11:00    | 01:00    |"
    );
}

/// Adding an entry to a later day never touches an earlier, already-closed
/// day's table, even when the new entry is wider than that older table's
/// columns.
#[test]
fn an_earlier_days_table_is_never_touched() {
    let mut lines = vec![
        "#   1 January".to_string(),
        "".to_string(),
        "| Description | Start Time | End Time | Duration |".to_string(),
        "| --- | --- | --- | --- |".to_string(),
        "| First day task | 09:00 | 10:00 | 01:00 |".to_string(),
        "".to_string(),
    ];
    lines.extend(january_second());
    lines.push("02/01/2024 09:00".to_string());

    let first_day_before = lines[2..5].to_vec();

    apply_worked(
        &mut lines,
        "A much longer second-day task".to_string(),
        dt(2024, 1, 2, 10, 0),
        &RECORD_FORMAT,
    )
    .unwrap();

    assert_eq!(lines[2..5], first_day_before[..]);
}

/// Amending the last entry re-pads the whole table it belongs to, including
/// the header, when the new description changes the required width.
#[test]
fn amend_repads_the_whole_table() {
    let mut lines = january_second();
    lines.push("| Short | 09:00 | 10:00 | 01:00 |".to_string());
    lines.push("02/01/2024 10:00".to_string());

    amend_last_entry(
        &mut lines,
        "A much longer revised task".to_string(),
        dt(2024, 1, 2, 10, 30),
        &RECORD_FORMAT,
    )
    .unwrap();

    assert_eq!(
        lines[2],
        "| Description                | Start Time | End Time | Duration |"
    );
    assert_eq!(
        lines[4],
        "| A much longer revised task | 09:00      | 10:30    | 01:30    |"
    );
}
