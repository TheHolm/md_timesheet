//! Integration tests for configuration parsing, serialisation and discovery.

use std::path::Path;

use md_timesheet::{
    canonical_config_path, cwd_config_path, load_config_from, locate_config, parse_config,
    pointer_target, serialize_config, write_config, write_pointer, xdg_config_dir, Config,
    Destination, RecordsFormat, RECORD_FORMAT,
};

/// The built-in default configuration.
fn default_config() -> Config {
    Config::default()
}

/// A config with non-default values, used to exercise every field.
fn custom_config() -> Config {
    Config {
        destination: Destination::TextFile("./other/timesheet.markdown".to_string()),
        format: RecordsFormat {
            start_time: false,
            end_time: true,
            duration: false,
            duration_rounding: 25,
        },
    }
}

/// A complete config file parses into the expected configuration.
#[test]
fn parse_full_config() {
    let contents = "\
# a comment
file_path = /tmp/timesheet.markdown

start_time = false
end_time = true
duration = true
duration_rounding = 15
";
    let config = parse_config(contents).unwrap();
    assert_eq!(
        config.destination,
        Destination::TextFile("/tmp/timesheet.markdown".to_string())
    );
    assert_eq!(
        config.format,
        RecordsFormat {
            start_time: false,
            end_time: true,
            duration: true,
            duration_rounding: 15,
        }
    );
}

/// An unknown key is rejected and named in the error.
#[test]
fn parse_unknown_key_errors() {
    let error = parse_config("file_path = x\nnonsense = 1\n").unwrap_err();
    assert!(error.contains("nonsense"));
}

/// A non-boolean value for a column flag is rejected and named in the error.
#[test]
fn parse_malformed_bool_errors() {
    let error = parse_config("start_time = yes\n").unwrap_err();
    assert!(error.contains("start_time"));
}

/// A non-integer rounding value is rejected and named in the error.
#[test]
fn parse_malformed_rounding_errors() {
    let error = parse_config("duration_rounding = ten\n").unwrap_err();
    assert!(error.contains("duration_rounding"));
}

/// A zero or negative rounding value is rejected.
#[test]
fn parse_non_positive_rounding_errors() {
    assert!(parse_config("duration_rounding = 0\n").is_err());
    assert!(parse_config("duration_rounding = -5\n").is_err());
}

/// A missing required key is reported.
#[test]
fn parse_missing_key_errors() {
    let error = parse_config("file_path = x\n").unwrap_err();
    assert!(error.contains("Missing"));
}

/// A line without an `=` sign is reported.
#[test]
fn parse_line_without_equals_errors() {
    assert!(parse_config("just some text\n").is_err());
}

/// Comments and blank lines are ignored, so the default text parses cleanly.
#[test]
fn parse_default_contents() {
    let config = parse_config(&md_timesheet::default_config_contents()).unwrap();
    assert_eq!(config, default_config());
}

/// Serialising then parsing yields the original configuration.
#[test]
fn serialize_then_parse_roundtrip() {
    let config = custom_config();
    let text = serialize_config(&config).unwrap();
    assert_eq!(parse_config(&text).unwrap(), config);
}

/// A Joplin destination cannot be represented as a config file.
#[test]
fn serialize_joplin_errors() {
    let config = Config {
        destination: Destination::JoplinNote(md_timesheet::JoplinNote {
            id: "id".to_string(),
            url: "http://localhost:41184".to_string(),
            api_key: "key".to_string(),
        }),
        format: RECORD_FORMAT,
    };
    assert!(serialize_config(&config).is_err());
}

/// `$XDG_CONFIG_HOME` wins when set.
#[test]
fn xdg_dir_prefers_xdg_config_home() {
    let dir = xdg_config_dir(Some("/xdg"), Some("/home/me")).unwrap();
    assert_eq!(dir, Path::new("/xdg/md_timesheet"));
}

/// `$HOME/.config` is used when `$XDG_CONFIG_HOME` is absent or empty.
#[test]
fn xdg_dir_falls_back_to_home() {
    assert_eq!(
        xdg_config_dir(None, Some("/home/me")).unwrap(),
        Path::new("/home/me/.config/md_timesheet")
    );
    assert_eq!(
        xdg_config_dir(Some(""), Some("/home/me")).unwrap(),
        Path::new("/home/me/.config/md_timesheet")
    );
}

/// With neither environment variable set, resolution fails.
#[test]
fn xdg_dir_without_env_errors() {
    assert!(xdg_config_dir(None, None).is_err());
}

/// The canonical config file sits inside the XDG config directory.
#[test]
fn canonical_path_ends_with_config() {
    let path = canonical_config_path(Some("/xdg"), None).unwrap();
    assert_eq!(path, Path::new("/xdg/md_timesheet/config"));
}

/// The current-folder config is named `md_timesheet.config`.
#[test]
fn cwd_path_joins_name() {
    assert_eq!(
        cwd_config_path(Path::new("/work")),
        Path::new("/work/md_timesheet.config")
    );
}

/// An explicit `-c`/`--config` path takes precedence over everything else.
#[test]
fn locate_prefers_cli_config() {
    let dir = tempfile::tempdir().unwrap();
    let cli = dir.path().join("explicit.config");
    std::fs::write(&cli, serialize_config(&default_config()).unwrap()).unwrap();
    let cwd_config = cwd_config_path(dir.path());
    std::fs::write(&cwd_config, "").unwrap();

    let found = locate_config(Some(&cli), dir.path(), None, None).unwrap();
    assert_eq!(found, Some(cli));
}

/// An explicit `-c`/`--config` path that does not exist is an error.
#[test]
fn locate_missing_cli_config_errors() {
    let dir = tempfile::tempdir().unwrap();
    let cli = dir.path().join("missing.config");
    assert!(locate_config(Some(&cli), dir.path(), None, None).is_err());
}

/// The current folder is searched before the XDG config directory.
#[test]
fn locate_prefers_cwd_over_xdg() {
    let dir = tempfile::tempdir().unwrap();
    let cwd_config = cwd_config_path(dir.path());
    std::fs::write(&cwd_config, "").unwrap();

    let xdg_home = dir.path().join("xdg");
    let xdg_path = canonical_config_path(Some(xdg_home.to_str().unwrap()), None).unwrap();
    std::fs::create_dir_all(xdg_path.parent().unwrap()).unwrap();
    std::fs::write(&xdg_path, "").unwrap();

    let found = locate_config(None, dir.path(), Some(xdg_home.to_str().unwrap()), None).unwrap();
    assert_eq!(found, Some(cwd_config));
}

/// The XDG config directory is used when the current folder has no config.
#[test]
fn locate_uses_xdg_when_no_cwd_config() {
    let dir = tempfile::tempdir().unwrap();
    let xdg_home = dir.path().join("xdg");
    let xdg_path = canonical_config_path(Some(xdg_home.to_str().unwrap()), None).unwrap();
    std::fs::create_dir_all(xdg_path.parent().unwrap()).unwrap();
    std::fs::write(&xdg_path, "").unwrap();

    let found = locate_config(None, dir.path(), Some(xdg_home.to_str().unwrap()), None).unwrap();
    assert_eq!(found, Some(xdg_path));
}

/// When nothing is found, `locate_config` reports `None`.
#[test]
fn locate_returns_none_when_nothing_found() {
    let dir = tempfile::tempdir().unwrap();
    let found = locate_config(None, dir.path(), None, None).unwrap();
    assert_eq!(found, None);
}

/// Loading a path that does not exist reports an error.
#[test]
fn load_missing_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    assert!(load_config_from(&dir.path().join("config")).is_err());
}

/// A written config is read back as the same configuration.
#[test]
fn load_reads_and_parses() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config");
    std::fs::write(&path, serialize_config(&custom_config()).unwrap()).unwrap();

    assert_eq!(load_config_from(&path).unwrap(), custom_config());
}

/// A pointer file's `config_path` is extracted, ignoring comments and blanks.
#[test]
fn pointer_target_reads_config_path() {
    let contents = "# a pointer\n\nconfig_path = /etc/md_timesheet/config\n";
    assert_eq!(
        pointer_target(contents),
        Some(Path::new("/etc/md_timesheet/config").to_path_buf())
    );
}

/// A regular configuration is not mistaken for a pointer.
#[test]
fn pointer_target_none_for_regular_config() {
    assert_eq!(
        pointer_target(&md_timesheet::default_config_contents()),
        None
    );
}

/// Writing a pointer is readable back by `pointer_target`.
#[test]
fn write_pointer_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let pointer_path = dir.path().join("config");
    let target = dir.path().join("real.config");

    write_pointer(&pointer_path, &target).unwrap();

    let contents = std::fs::read_to_string(&pointer_path).unwrap();
    assert_eq!(pointer_target(&contents), Some(target));
}

/// `write_config` creates missing parent directories.
#[test]
fn write_config_creates_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("deeper").join("config");

    write_config(&path, "hello").unwrap();

    assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
}

/// The XDG file may point at a config stored elsewhere.
#[test]
fn locate_follows_xdg_pointer() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("real.config");
    std::fs::write(&target, serialize_config(&default_config()).unwrap()).unwrap();

    let xdg_home = dir.path().join("xdg");
    let pointer_path = canonical_config_path(Some(xdg_home.to_str().unwrap()), None).unwrap();
    write_pointer(&pointer_path, &target).unwrap();

    let found = locate_config(None, dir.path(), Some(xdg_home.to_str().unwrap()), None).unwrap();
    assert_eq!(found, Some(target));
}

/// A pointer referencing a missing file is an error.
#[test]
fn locate_dangling_pointer_errors() {
    let dir = tempfile::tempdir().unwrap();
    let xdg_home = dir.path().join("xdg");
    let pointer_path = canonical_config_path(Some(xdg_home.to_str().unwrap()), None).unwrap();
    write_pointer(&pointer_path, &dir.path().join("missing.config")).unwrap();

    assert!(locate_config(None, dir.path(), Some(xdg_home.to_str().unwrap()), None).is_err());
}
