//! Integration tests for reading and writing the timesheet document.

use std::path::{Path, PathBuf};

use md_timesheet::{read_document, write_document, Destination, JoplinNote};

/// Builds a `TextFile` destination pointing at `timesheet.markdown` in `dir`.
fn text_file(dir: &Path) -> (Destination, PathBuf) {
    let path = dir.join("timesheet.markdown");
    (
        Destination::TextFile(path.to_str().unwrap().to_string()),
        path,
    )
}

/// Builds a `TextFile` destination whose file already exists, as it always
/// does after the GUI has read it once.
fn existing_file(dir: &Path) -> (Destination, PathBuf) {
    let (dest, path) = text_file(dir);
    read_document(&dest).unwrap();
    (dest, path)
}

/// Reading a missing file creates it and yields no lines.
#[test]
fn read_creates_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    let (dest, path) = text_file(dir.path());

    let lines = read_document(&dest).unwrap();
    assert!(lines.is_empty());
    assert!(path.exists());
}

/// Written lines are read back verbatim.
#[test]
fn write_then_read_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let (dest, _path) = existing_file(dir.path());

    let content = vec![
        "#   2 January".to_string(),
        "".to_string(),
        "| Doing Stuff | 12:00 | 13:00 | 01:00 |".to_string(),
    ];
    write_document(&dest, content.clone()).unwrap();

    assert_eq!(read_document(&dest).unwrap(), content);
}

/// Writing again replaces the previous contents rather than appending.
#[test]
fn write_overwrites_previous_content() {
    let dir = tempfile::tempdir().unwrap();
    let (dest, _path) = existing_file(dir.path());

    write_document(&dest, vec!["first".to_string()]).unwrap();
    write_document(&dest, vec!["second".to_string()]).unwrap();

    assert_eq!(read_document(&dest).unwrap(), vec!["second"]);
}

/// Writing an empty document truncates the file to nothing.
#[test]
fn write_empty_truncates() {
    let dir = tempfile::tempdir().unwrap();
    let (dest, path) = existing_file(dir.path());

    write_document(&dest, vec!["something".to_string()]).unwrap();
    write_document(&dest, Vec::new()).unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
}

/// Reading a path that cannot be opened reports an error.
#[test]
fn read_unopenable_path_errors() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir
        .path()
        .join("no-such-directory")
        .join("timesheet.markdown");
    let dest = Destination::TextFile(missing.to_str().unwrap().to_string());

    assert!(read_document(&dest).is_err());
}

/// Reading a file whose bytes are not valid UTF-8 reports an error.
#[test]
fn read_invalid_utf8_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("timesheet.markdown");
    std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
    let dest = Destination::TextFile(path.to_str().unwrap().to_string());

    assert!(read_document(&dest).is_err());
}

/// Writing to a file that was never created or read reports an error.
#[test]
fn write_missing_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let (dest, _path) = text_file(dir.path());

    assert!(write_document(&dest, vec!["x".to_string()]).is_err());
}

/// A Joplin destination reports that it is not implemented.
#[test]
fn joplin_is_not_implemented() {
    let dest = Destination::JoplinNote(JoplinNote {
        id: "note-id".to_string(),
        url: "http://localhost:41184".to_string(),
        api_key: "api-key".to_string(),
    });

    assert!(read_document(&dest).is_err());
    assert!(write_document(&dest, Vec::new()).is_err());
}
