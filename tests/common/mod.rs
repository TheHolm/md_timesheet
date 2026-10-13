//! Shared test helpers for comparing Markdown table rows while tolerating
//! the column padding `pad_table_rows`/`pad_last_table` add in `lib.rs`
//! (those two helpers are private, so tests re-implement the same trimmed
//! cell-splitting here rather than comparing padded text verbatim).
//!
//! This module is compiled separately into every integration test binary
//! that declares `mod common;`; `#[allow(dead_code)]` on the functions a
//! given binary doesn't call avoids spurious "never used" warnings.

/// Splits a Markdown table row into trimmed cells, or `None` if `line` is
/// not a `| ... |` row.
///
/// `"| 12:00      |"` and `"| 12:00 |"` both yield `["12:00"]`, so this is
/// tolerant of any column padding `apply_worked`/`amend_last_entry` may have
/// added.
#[allow(dead_code)]
pub fn cells(line: &str) -> Option<Vec<&str>> {
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

/// Asserts that two document lines are equal.
///
/// When both lines are `| ... |` rows they are compared cell-by-cell (after
/// trimming), so column padding doesn't matter; anything else (day headings,
/// blank lines, the trailing timestamp) is compared as plain text.
#[allow(dead_code)]
pub fn assert_row_eq(actual: &str, expected: &str) {
    match (cells(actual), cells(expected)) {
        (Some(a), Some(e)) => assert_eq!(
            a, e,
            "rows differ\n  actual: {actual:?}\nexpected: {expected:?}"
        ),
        _ => assert_eq!(actual, expected),
    }
}

/// Asserts that two documents are equal, comparing each line with
/// [`assert_row_eq`].
#[allow(dead_code)]
pub fn assert_lines_eq(actual: &[String], expected: &[String]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "line count differs\n  actual: {actual:#?}\nexpected: {expected:#?}"
    );
    for (a, e) in actual.iter().zip(expected) {
        assert_row_eq(a, e);
    }
}
