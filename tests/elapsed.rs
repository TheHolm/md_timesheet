//! Integration tests for the "Worked for (HH:MM)" duration preview: rounding
//! and formatting helpers, reading the last timestamp, the amend start point
//! and the button label.

use chrono::{NaiveDate, NaiveDateTime};
use md_timesheet::{
    amend_last_entry, apply_worked, button_label, format_hhmm, last_entry_start, last_timestamp,
    pending_duration, round_up_minutes, RecordsFormat, MAX_PENDING_MINUTES, RECORD_FORMAT,
};

/// Builds a `NaiveDateTime` for the given calendar fields.
fn dt(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(year, month, day)
        .unwrap()
        .and_hms_opt(hour, minute, 0)
        .unwrap()
}

/// A day with one entry (09:00-09:30) and a trailing timestamp of 09:30.
fn one_entry_day() -> Vec<String> {
    vec![
        "#   2 January".to_string(),
        "".to_string(),
        "| Description | Start Time | End Time | Duration |".to_string(),
        "| --- | --- | --- | --- |".to_string(),
        "| Task | 09:00 | 09:30 | 00:30 |".to_string(),
        "02/01/2025 09:30".to_string(),
    ]
}

/// The default format with a different rounding step.
fn rounding(minutes: i32) -> RecordsFormat {
    RecordsFormat {
        duration_rounding: minutes,
        ..RECORD_FORMAT
    }
}

/// Zero stays zero rather than being rounded up to one step.
#[test]
fn round_up_zero_stays_zero() {
    assert_eq!(round_up_minutes(0, 10), 0);
}

/// Exact multiples of the step are left unchanged.
#[test]
fn round_up_exact_multiple_unchanged() {
    assert_eq!(round_up_minutes(30, 10), 30);
    assert_eq!(round_up_minutes(60, 15), 60);
}

/// One minute past a multiple rounds up to the next step.
#[test]
fn round_up_partial_step_rounds_up() {
    assert_eq!(round_up_minutes(1, 10), 10);
    assert_eq!(round_up_minutes(31, 10), 40);
    assert_eq!(round_up_minutes(72, 15), 75);
}

/// A rounding step of 1 leaves every value unchanged.
#[test]
fn round_up_step_of_one_is_identity() {
    assert_eq!(round_up_minutes(73, 1), 73);
}

/// Minutes are printed as zero-padded hours and minutes.
#[test]
fn format_hhmm_pads_fields() {
    assert_eq!(format_hhmm(0), "00:00");
    assert_eq!(format_hhmm(5), "00:05");
    assert_eq!(format_hhmm(80), "01:20");
}

/// Hours are not wrapped at 24.
#[test]
fn format_hhmm_does_not_wrap_hours() {
    assert_eq!(format_hhmm(MAX_PENDING_MINUTES), "24:00");
    assert_eq!(format_hhmm(1500), "25:00");
}

/// The trailing timestamp is parsed from the final line.
#[test]
fn last_timestamp_parses_final_line() {
    assert_eq!(
        last_timestamp(&one_entry_day()),
        Some(dt(2025, 1, 2, 9, 30))
    );
}

/// An empty document has no timestamp.
#[test]
fn last_timestamp_empty_document() {
    assert_eq!(last_timestamp(&[]), None);
}

/// A final line that is not a timestamp yields `None`.
#[test]
fn last_timestamp_unparsable_final_line() {
    let lines = vec!["not a timestamp".to_string()];
    assert_eq!(last_timestamp(&lines), None);
}

/// Only the final line counts: a trailing blank line hides the timestamp,
/// matching how `apply_start`/`apply_worked` treat the document.
#[test]
fn last_timestamp_ignores_earlier_lines() {
    let mut lines = one_entry_day();
    lines.push("".to_string());
    assert_eq!(last_timestamp(&lines), None);
}

/// The pending duration is rounded up like the Duration column.
#[test]
fn pending_duration_same_day_is_rounded() {
    let since = Some(dt(2025, 1, 2, 8, 6));
    let now = dt(2025, 1, 2, 9, 18);
    assert_eq!(
        pending_duration(since, now, &RECORD_FORMAT).as_deref(),
        Some("01:20")
    );
}

/// The configured rounding step is honoured.
#[test]
fn pending_duration_uses_configured_rounding() {
    let since = Some(dt(2025, 1, 2, 8, 6));
    let now = dt(2025, 1, 2, 9, 18);
    assert_eq!(
        pending_duration(since, now, &rounding(15)).as_deref(),
        Some("01:15")
    );
    assert_eq!(
        pending_duration(since, now, &rounding(1)).as_deref(),
        Some("01:12")
    );
}

/// No time elapsed shows `00:00`.
#[test]
fn pending_duration_zero_elapsed() {
    let now = dt(2025, 1, 2, 9, 0);
    assert_eq!(
        pending_duration(Some(now), now, &RECORD_FORMAT).as_deref(),
        Some("00:00")
    );
}

/// A gap across midnight is measured in full.
#[test]
fn pending_duration_across_midnight() {
    let since = Some(dt(2025, 1, 1, 23, 30));
    let now = dt(2025, 1, 2, 0, 45);
    assert_eq!(
        pending_duration(since, now, &RECORD_FORMAT).as_deref(),
        Some("01:20")
    );
}

/// A gap of exactly 24 hours is still reported.
#[test]
fn pending_duration_exactly_24_hours() {
    let since = Some(dt(2025, 1, 1, 9, 0));
    let now = dt(2025, 1, 2, 9, 0);
    assert_eq!(
        pending_duration(since, now, &RECORD_FORMAT).as_deref(),
        Some("24:00")
    );
}

/// One minute over 24 hours is reported as unknown.
#[test]
fn pending_duration_over_24_hours_is_none() {
    let since = Some(dt(2025, 1, 1, 9, 0));
    let now = dt(2025, 1, 2, 9, 1);
    assert_eq!(pending_duration(since, now, &RECORD_FORMAT), None);
}

/// Many days ago is reported as unknown.
#[test]
fn pending_duration_days_ago_is_none() {
    let since = Some(dt(2024, 12, 24, 15, 41));
    let now = dt(2025, 1, 2, 9, 0);
    assert_eq!(pending_duration(since, now, &RECORD_FORMAT), None);
}

/// A timestamp in the future (clock skew) is reported as unknown.
#[test]
fn pending_duration_future_timestamp_is_none() {
    let since = Some(dt(2025, 1, 2, 10, 0));
    let now = dt(2025, 1, 2, 9, 0);
    assert_eq!(pending_duration(since, now, &RECORD_FORMAT), None);
}

/// Without a starting point there is nothing to report.
#[test]
fn pending_duration_without_start_is_none() {
    assert_eq!(
        pending_duration(None, dt(2025, 1, 2, 9, 0), &RECORD_FORMAT),
        None
    );
}

/// The preview matches the Duration column that "Worked for" then writes.
#[test]
fn pending_duration_matches_recorded_entry() {
    let now = dt(2025, 1, 2, 10, 43);
    let mut lines = one_entry_day();
    let pending = pending_duration(last_timestamp(&lines), now, &RECORD_FORMAT).unwrap();

    apply_worked(&mut lines, "Next".to_string(), now, &RECORD_FORMAT).unwrap();

    assert_eq!(pending, "01:20");
    assert_eq!(
        lines[lines.len() - 2],
        format!("| Next | 09:30 | 10:43 | {} |", pending)
    );
}

/// The amend start is the last entry's start time on the current day.
#[test]
fn last_entry_start_same_day() {
    let now = dt(2025, 1, 2, 10, 0);
    assert_eq!(
        last_entry_start(&one_entry_day(), now, &RECORD_FORMAT),
        Ok(dt(2025, 1, 2, 9, 0))
    );
}

/// A start time later than now belongs to the previous day.
#[test]
fn last_entry_start_across_midnight() {
    let lines = vec![
        "| Late task | 23:10 | 23:50 | 00:40 |".to_string(),
        "01/01/2025 23:50".to_string(),
    ];
    let now = dt(2025, 1, 2, 0, 30);
    assert_eq!(
        last_entry_start(&lines, now, &RECORD_FORMAT),
        Ok(dt(2025, 1, 1, 23, 10))
    );
}

/// Without a Start Time column there is no amend start.
#[test]
fn last_entry_start_without_start_column_errors() {
    let format = RecordsFormat {
        start_time: false,
        ..RECORD_FORMAT
    };
    assert!(last_entry_start(&one_entry_day(), dt(2025, 1, 2, 10, 0), &format).is_err());
}

/// A document without any entry has no amend start.
#[test]
fn last_entry_start_without_entry_errors() {
    let lines = vec![
        "| Description | Start Time | End Time | Duration |".to_string(),
        "| --- | --- | --- | --- |".to_string(),
        "02/01/2025 09:30".to_string(),
    ];
    assert!(last_entry_start(&lines, dt(2025, 1, 2, 10, 0), &RECORD_FORMAT).is_err());
}

/// An unparsable trailing timestamp means amending would fail, so there is no
/// amend start either.
#[test]
fn last_entry_start_with_invalid_timestamp_errors() {
    let mut lines = one_entry_day();
    lines.push("garbage".to_string());
    assert!(last_entry_start(&lines, dt(2025, 1, 2, 10, 0), &RECORD_FORMAT).is_err());
}

/// The amend preview matches the duration that amending then writes.
#[test]
fn amend_preview_matches_amended_entry() {
    let now = dt(2025, 1, 2, 10, 2);
    let mut lines = one_entry_day();
    let start = last_entry_start(&lines, now, &RECORD_FORMAT).unwrap();
    let pending = pending_duration(Some(start), now, &RECORD_FORMAT).unwrap();

    amend_last_entry(&mut lines, "Task".to_string(), now, &RECORD_FORMAT).unwrap();

    assert_eq!(pending, "01:10");
    assert_eq!(lines[4], format!("| Task | 09:00 | 10:02 | {} |", pending));
}

/// A known duration is shown in brackets after "Worked for".
#[test]
fn button_label_with_duration() {
    assert_eq!(button_label(Some("01:20")), "Worked for (01:20)");
}

/// An unknown duration is shown as `--:--`.
#[test]
fn button_label_without_duration() {
    assert_eq!(button_label(None), "Worked for (--:--)");
}
