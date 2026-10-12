//! GTK4/libadwaita front-end for `md_timesheet`.
//!
//! Deliberately thin: it wires the widgets to the library in [`lib.rs`](../lib.rs)
//! and turns I/O results into process exit codes. The `Start`/`Worked for`
//! actions always terminate the program, which is intentional.
//!
//! The "Worked for" button shows the rounded duration that would be recorded
//! if it were pressed now (`--:--` when unknown or more than 24 hours). The
//! timestamp is read once when the window is shown and the label is refreshed
//! every [`REFRESH_SECONDS`] seconds.
//!
//! Keyboard shortcuts:
//!
//! - Enter is equivalent to the "Worked for" button.
//! - Arrow Up loads the last entry for amendment; while in that mode Enter
//!   replaces the last entry and Escape returns to normal entry. In amend mode
//!   the button shows the amended entry's new duration.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::process;
use std::rc::Rc;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::{
    ActionRow, Application, EntryRow, HeaderBar, MessageDialog, PreferencesGroup, PreferencesPage,
};
use clap::Parser;
use gtk::gdk::Key;
use gtk::gio::prelude::FileExt;
use gtk::glib::{self, ControlFlow, Propagation};
use gtk::{
    ApplicationWindow, Box, Button, Entry, EventControllerKey, FileChooserAction,
    FileChooserNative, Label, PropagationPhase, ResponseType,
};
use gtk4 as gtk;

use chrono::prelude::*;

use md_timesheet::{
    amend_last_entry, apply_start, apply_worked, button_label, collapse_tilde, cwd_config_path,
    default_config_contents, last_entry_description, last_entry_start, last_timestamp,
    load_config_from, locate_config, peek_document, pending_duration, read_document,
    serialize_config, write_config, write_document, write_pointer, xdg_config_dir, Config,
    Destination, RecordsFormat,
};

/// The application ID used for D-Bus registration and desktop integration.
const APP_ID: &str = "io.github.TheHolm.md_timesheet";

/// The main window title.
const WINDOW_TITLE: &str = "MD Timesheet";

/// How often, in seconds, the "Worked for" label is recomputed.
const REFRESH_SECONDS: u32 = 30;

/// Command line options.
#[derive(Parser)]
#[command(name = "md_timesheet", about = "Track time in a Markdown timesheet.")]
struct Cli {
    /// Path to the configuration file.
    #[arg(short, long)]
    config: Option<PathBuf>,
}

/// The path to the resolved config file, set once during startup.
static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

/// Writes a default config to `config_path` and remembers it.
///
/// When `pointer_path` is given, a pointer referencing `config_path` is written
/// there as well, so the config can be found again on the next launch.
fn create_config_at(config_path: &Path, pointer_path: Option<&Path>) {
    if let Err(e) = write_config(config_path, &default_config_contents()) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
    if let Some(pointer) = pointer_path {
        if let Err(e) = write_pointer(pointer, config_path) {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
    CONFIG_PATH.set(config_path.to_path_buf()).ok();
}

/// Shows the first-run dialog asking where to create the config file.
///
/// `window` is presented once a config file has been created; cancelling the
/// dialog exits the program.
fn show_onboarding(
    app: &Application,
    window: &ApplicationWindow,
    cwd: PathBuf,
    xdg: Option<String>,
    home: Option<String>,
) {
    let xdg_dir = xdg_config_dir(xdg.as_deref(), home.as_deref()).ok();

    let dialog = MessageDialog::builder()
        .heading("No configuration found")
        .body("Create a configuration file. Where should it be stored?")
        .build();
    dialog.set_transient_for(Some(window));
    dialog.set_modal(true);
    dialog.add_response("xdg", "XDG config directory");
    dialog.add_response("cwd", "Current folder");
    dialog.add_response("path", "Specify path…");
    dialog.add_response("cancel", "Cancel");
    dialog.set_default_response(Some(if xdg_dir.is_some() { "xdg" } else { "cwd" }));
    dialog.set_close_response("cancel");
    if xdg_dir.is_none() {
        dialog.set_response_enabled("xdg", false);
    }

    dialog.connect_response(None, {
        let app = app.clone();
        let window = window.clone();
        move |_dialog, response| match response {
            "xdg" => {
                if let Some(dir) = &xdg_dir {
                    create_config_at(&dir.join("config"), None);
                    window.present();
                }
            }
            "cwd" => {
                let pointer = xdg_dir.as_ref().map(|dir| dir.join("config"));
                create_config_at(&cwd_config_path(&cwd), pointer.as_deref());
                window.present();
            }
            "path" => choose_config_path(&app, &window, cwd.clone(), xdg.clone(), home.clone()),
            _ => process::exit(0),
        }
    });

    dialog.present();
}

/// Opens a save dialog for choosing where to create the config file.
///
/// Cancelling returns to the onboarding dialog.
fn choose_config_path(
    app: &Application,
    window: &ApplicationWindow,
    cwd: PathBuf,
    xdg: Option<String>,
    home: Option<String>,
) {
    let chooser = FileChooserNative::builder()
        .title("Create configuration file")
        .action(FileChooserAction::Save)
        .accept_label("Create")
        .cancel_label("Cancel")
        .transient_for(window)
        .modal(true)
        .build();
    chooser.set_current_name("md_timesheet.config");

    chooser.connect_response({
        let app = app.clone();
        let window = window.clone();
        move |chooser, response| {
            if response == ResponseType::Accept {
                if let Some(path) = chooser.file().and_then(|file| file.path()) {
                    let pointer = xdg_config_dir(xdg.as_deref(), home.as_deref())
                        .ok()
                        .map(|dir| dir.join("config"));
                    create_config_at(&path, pointer.as_deref());
                    window.present();
                }
            } else {
                show_onboarding(&app, &window, cwd.clone(), xdg.clone(), home.clone());
            }
        }
    });

    chooser.show();
}

/// Loads the configuration from the resolved config file, exiting on failure.
fn config() -> Config {
    let path = CONFIG_PATH
        .get()
        .expect("the configuration path must be resolved before use");
    let home = std::env::var("HOME").ok();
    match load_config_from(path, home.as_deref()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

/// Reads the document, exiting with an error message on failure.
fn read_lines() -> Vec<String> {
    let config = config();
    match read_document(&config.destination) {
        Ok(lines) => lines,
        Err(e) => {
            eprintln!("Error opening file: {}", e);
            process::exit(1);
        }
    }
}

/// Writes the document and exits successfully, or exits with an error.
fn write_lines(lines: Vec<String>) {
    let config = config();
    match write_document(&config.destination, lines) {
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
        Ok(_) => process::exit(0),
    }
}

/// Handles the "Start" button: records the current time and exits.
///
/// An unparsable trailing timestamp is reported but the (unchanged) document
/// is still written and the program exits with status 0, matching the previous
/// behaviour.
fn click_start() {
    let config = config();
    let mut lines = read_lines();
    let current_datetime = Local::now().naive_local();
    if let Err(e) = apply_start(&mut lines, current_datetime, &config.format) {
        println!("Failed to parse date: {}", e);
    }
    write_lines(lines);
}

/// Records `description` as a completed task and exits.
fn record_worked(description: String) {
    let config = config();
    let mut lines = read_lines();
    let current_datetime = Local::now().naive_local();
    if let Err(e) = apply_worked(&mut lines, description, current_datetime, &config.format) {
        eprintln!("Failed to parse date: {}", e);
        process::exit(1);
    }
    write_lines(lines);
}

/// Amends the last entry with `description` and exits.
fn record_amend(description: String) {
    let config = config();
    let mut lines = read_lines();
    let current_datetime = Local::now().naive_local();
    if let Err(e) = amend_last_entry(&mut lines, description, current_datetime, &config.format) {
        eprintln!("Could not amend entry: {}", e);
        process::exit(1);
    }
    write_lines(lines);
}

/// Loads the configuration without exiting on failure.
///
/// Used by the label preview, which must never terminate the program; returns
/// `None` when the config path is unresolved or the file cannot be loaded.
fn try_config() -> Option<Config> {
    let path = CONFIG_PATH.get()?;
    let home = std::env::var("HOME").ok();
    load_config_from(path, home.as_deref()).ok()
}

/// State behind the duration shown on the "Worked for" button.
///
/// Cheap to clone: every field is a reference-counted handle shared by the
/// window, the key handler and the refresh timer.
#[derive(Clone)]
struct Preview {
    /// The "Worked for" button whose label is kept up to date.
    button: Button,
    /// The timestamp on the document's last line, read when the window is shown.
    last: Rc<Cell<Option<NaiveDateTime>>>,
    /// When the last entry started, set on entering amend mode.
    amend_start: Rc<Cell<Option<NaiveDateTime>>>,
    /// Whether the entry field is in amend mode.
    amending: Rc<Cell<bool>>,
}

impl Preview {
    /// Creates the preview state for `button`, outside amend mode and with no
    /// known timestamp yet.
    fn new(button: Button) -> Self {
        Preview {
            button,
            last: Rc::new(Cell::new(None)),
            amend_start: Rc::new(Cell::new(None)),
            amending: Rc::new(Cell::new(false)),
        }
    }

    /// Re-reads the last timestamp from the document and refreshes the label.
    ///
    /// The document is only peeked at: a missing timesheet is not created, and
    /// any read or config error simply results in `--:--`.
    fn reload(&self) {
        let lines = try_config()
            .and_then(|config| peek_document(&config.destination).ok())
            .unwrap_or_default();
        self.last.set(last_timestamp(&lines));
        self.refresh();
    }

    /// Recomputes the label from the cached start point and the current time.
    ///
    /// In amend mode the duration runs from the last entry's start, otherwise
    /// from the document's last timestamp.
    fn refresh(&self) {
        let since = if self.amending.get() {
            self.amend_start.get()
        } else {
            self.last.get()
        };
        let pending = try_config()
            .and_then(|config| pending_duration(since, Local::now().naive_local(), &config.format));
        self.button.set_label(&button_label(pending.as_deref()));
    }
}

/// Handles the "Worked for" button or Enter.
///
/// Does nothing when the entry field is empty.
fn submit(entry: &gtk::Entry, amending: bool) {
    let text = entry.buffer().text().to_string();
    if text.is_empty() {
        return;
    }
    if amending {
        record_amend(text);
    } else {
        record_worked(text);
    }
}

/// Fills the entry with the last entry's description and enters amend mode.
///
/// Also records when the last entry started so the button shows the amended
/// duration (`--:--` when that start cannot be determined, e.g. without a
/// Start Time column). Returns whether amend mode was entered. Nothing happens
/// when the document has no entry or cannot be read.
fn start_amend(entry: &gtk::Entry, status: &Label, preview: &Preview) -> bool {
    let config = config();
    match read_document(&config.destination) {
        Ok(lines) => match last_entry_description(&lines) {
            Some(description) => {
                entry.set_text(&description);
                entry.set_position(-1);
                let now = Local::now().naive_local();
                preview
                    .amend_start
                    .set(last_entry_start(&lines, now, &config.format).ok());
                preview.amending.set(true);
                status.set_visible(true);
                preview.refresh();
                true
            }
            None => false,
        },
        Err(e) => {
            eprintln!("Error opening file: {}", e);
            false
        }
    }
}

/// Leaves amend mode, clears the entry field and restores the normal label.
fn cancel_amend(entry: &gtk::Entry, status: &Label, preview: &Preview) {
    entry.set_text("");
    preview.amending.set(false);
    preview.amend_start.set(None);
    status.set_visible(false);
    preview.refresh();
}

/// Shows the settings window for editing the configuration.
///
/// The current config is loaded into the rows; "Save" writes the values back to
/// the resolved config file, refreshes the "Worked for" label (the rounding may
/// have changed) and closes the window. The next action reloads the config, so
/// the new settings take effect immediately.
fn show_settings(app: &Application, parent: &ApplicationWindow, preview: &Preview) {
    let config = config();

    let (path, start_time, end_time, duration, duration_rounding) = match &config.destination {
        Destination::TextFile(path) => (
            path.clone(),
            config.format.start_time,
            config.format.end_time,
            config.format.duration,
            config.format.duration_rounding,
        ),
        Destination::JoplinNote(_) => {
            eprintln!("Error: Joplin destinations cannot be edited yet.");
            return;
        }
    };

    let window = gtk::Window::builder()
        .application(app)
        .transient_for(parent)
        .modal(true)
        .title("Settings")
        .default_width(400)
        .default_height(500)
        .build();

    let header = HeaderBar::new();
    let save_button = Button::with_label("Save");
    header.pack_end(&save_button);
    window.set_titlebar(Some(&header));

    let page = PreferencesPage::new();

    let storage = PreferencesGroup::builder().title("Storage").build();
    let path_row = EntryRow::builder()
        .title("Timesheet file")
        .text(path)
        .build();
    storage.add(&path_row);

    let columns = PreferencesGroup::builder().title("Columns").build();
    let (start_row, start_switch) = setting_switch("Start Time", start_time);
    let (end_row, end_switch) = setting_switch("End Time", end_time);
    let (duration_row, duration_switch) = setting_switch("Duration", duration);
    columns.add(&start_row);
    columns.add(&end_row);
    columns.add(&duration_row);

    let durations = PreferencesGroup::builder().title("Durations").build();
    let rounding_spin = gtk::SpinButton::with_range(1.0, 1440.0, 1.0);
    rounding_spin.set_value(duration_rounding as f64);
    let rounding_row = ActionRow::builder().title("Round up to (minutes)").build();
    rounding_row.add_suffix(&rounding_spin);
    durations.add(&rounding_row);

    page.add(&storage);
    page.add(&columns);
    page.add(&durations);

    window.set_child(Some(&page));

    save_button.connect_clicked({
        let window = window.clone();
        let preview = preview.clone();
        move |_| {
            let home = std::env::var("HOME").ok();
            let new_config = Config {
                destination: Destination::TextFile(collapse_tilde(
                    &path_row.text(),
                    home.as_deref(),
                )),
                format: RecordsFormat {
                    start_time: start_switch.is_active(),
                    end_time: end_switch.is_active(),
                    duration: duration_switch.is_active(),
                    duration_rounding: rounding_spin.value() as i32,
                },
            };

            let contents = match serialize_config(&new_config) {
                Ok(contents) => contents,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return;
                }
            };

            let config_path = CONFIG_PATH
                .get()
                .expect("the configuration path must be resolved before use");
            match write_config(config_path, &contents) {
                Ok(()) => {
                    preview.refresh();
                    window.close();
                }
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    });

    window.present();
}

/// Builds a titled settings row with a switch on the trailing side.
fn setting_switch(title: &str, value: bool) -> (ActionRow, gtk::Switch) {
    let switch = gtk::Switch::builder()
        .active(value)
        .valign(gtk::Align::Center)
        .build();
    let row = ActionRow::builder().title(title).build();
    row.add_suffix(&switch);
    row.set_activatable_widget(Some(&switch));
    (row, switch)
}

/// Builds the application window and connects the buttons and shortcuts.
///
/// The window is not presented; the caller presents it once a config file has
/// been resolved. Each time it is shown, the last timestamp is re-read for the
/// "Worked for" label, which is then refreshed every [`REFRESH_SECONDS`].
fn build_main_window(app: &Application) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(320)
        .default_height(120)
        .title(WINDOW_TITLE)
        .build();

    let entry_buffer = gtk::EntryBuffer::builder().build();
    let entry = Entry::with_buffer(&entry_buffer);

    let status = Label::new(Some("Amending last entry (Esc to cancel)"));
    status.set_visible(false);

    let button_s = Button::with_label("Start");
    let button_w = Button::with_label(&button_label(None));
    let preview = Preview::new(button_w.clone());

    let settings_button = Button::builder()
        .icon_name("emblem-system-symbolic")
        .tooltip_text("Settings")
        .has_frame(false)
        .valign(gtk::Align::Center)
        .build();
    settings_button.add_css_class("flat");
    settings_button.connect_clicked({
        let app = app.clone();
        let window = window.clone();
        let preview = preview.clone();
        move |_| show_settings(&app, &window, &preview)
    });

    let vbox = Box::new(gtk::Orientation::Vertical, 5);
    let button_box = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(5)
        .homogeneous(true)
        .hexpand(true)
        .build();

    button_box.append(&button_s);
    button_box.append(&button_w);

    let hbox = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(5)
        .margin_start(5)
        .margin_end(5)
        .build();
    hbox.append(&settings_button);
    hbox.append(&button_box);

    vbox.append(&status);
    vbox.append(&entry);
    vbox.append(&hbox);

    window.set_child(Some(&vbox));

    window.connect_map({
        let preview = preview.clone();
        move |_| preview.reload()
    });
    glib::timeout_add_seconds_local(REFRESH_SECONDS, {
        let preview = preview.clone();
        move || {
            preview.refresh();
            ControlFlow::Continue
        }
    });

    button_s.connect_clicked(move |_| click_start());
    button_w.connect_clicked({
        let entry = entry.clone();
        let preview = preview.clone();
        move |_| submit(&entry, preview.amending.get())
    });

    entry.connect_activate({
        let entry = entry.clone();
        let preview = preview.clone();
        move |_| submit(&entry, preview.amending.get())
    });

    let key_controller = EventControllerKey::new();
    key_controller.set_propagation_phase(PropagationPhase::Capture);
    key_controller.connect_key_pressed({
        let entry = entry.clone();
        let status = status.clone();
        let preview = preview.clone();
        move |_, key, _, _| match key {
            Key::Up => {
                if preview.amending.get() || !start_amend(&entry, &status, &preview) {
                    Propagation::Proceed
                } else {
                    Propagation::Stop
                }
            }
            Key::Escape => {
                if preview.amending.get() {
                    cancel_amend(&entry, &status, &preview);
                    Propagation::Stop
                } else {
                    Propagation::Proceed
                }
            }
            _ => Propagation::Proceed,
        }
    });
    entry.add_controller(key_controller);

    window
}

/// Parses the command line, resolves the config file and starts the GUI.
///
/// When no config file is found the onboarding dialog is shown before the main
/// window.
fn main() {
    let cli = Cli::parse();

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_activate(move |app| {
        let window = build_main_window(app);

        let cwd = std::env::current_dir().unwrap_or_else(|e| {
            eprintln!("Error determining the working directory: {}", e);
            process::exit(1);
        });
        let xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let home = std::env::var("HOME").ok();

        match locate_config(cli.config.as_deref(), &cwd, xdg.as_deref(), home.as_deref()) {
            Ok(Some(path)) => {
                CONFIG_PATH.set(path).ok();
                window.present();
            }
            Ok(None) => show_onboarding(app, &window, cwd, xdg, home),
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    });

    app.run();
}
