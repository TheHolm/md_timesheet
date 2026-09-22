//! GTK4/libadwaita front-end for `md_timesheet`.
//!
//! Deliberately thin: it wires the widgets to the library in [`lib.rs`](../lib.rs)
//! and turns I/O results into process exit codes. The `Start`/`Worked on`
//! actions always terminate the program, which is intentional.
//!
//! Keyboard shortcuts:
//!
//! - Enter is equivalent to the "Worked on" button.
//! - Arrow Up loads the last entry for amendment; while in that mode Enter
//!   replaces the last entry and Escape returns to normal entry.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::process;
use std::rc::Rc;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::{Application, MessageDialog};
use clap::Parser;
use gtk::gdk::Key;
use gtk::gio::prelude::FileExt;
use gtk::glib::Propagation;
use gtk::{
    ApplicationWindow, Box, Button, Entry, EventControllerKey, FileChooserAction,
    FileChooserNative, Label, PropagationPhase, ResponseType,
};
use gtk4 as gtk;

use chrono::prelude::*;

use md_timesheet::{
    amend_last_entry, apply_start, apply_worked, cwd_config_path, default_config_contents,
    last_entry_description, load_config_from, locate_config, read_document, write_config,
    write_document, write_pointer, xdg_config_dir, Config,
};

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
    match load_config_from(path) {
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

/// Handles the "Worked on" button or Enter in normal mode.
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
/// Returns whether amend mode was entered. Nothing happens when the document
/// has no entry or cannot be read.
fn start_amend(entry: &gtk::Entry, status: &Label, amending: &Cell<bool>) -> bool {
    let config = config();
    match read_document(&config.destination) {
        Ok(lines) => match last_entry_description(&lines) {
            Some(description) => {
                entry.set_text(&description);
                entry.set_position(-1);
                amending.set(true);
                status.set_visible(true);
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

/// Leaves amend mode and clears the entry field.
fn cancel_amend(entry: &gtk::Entry, status: &Label, amending: &Cell<bool>) {
    entry.set_text("");
    amending.set(false);
    status.set_visible(false);
}

/// Builds the application window and connects the buttons and shortcuts.
///
/// The window is not presented; the caller presents it once a config file has
/// been resolved.
fn build_main_window(app: &Application) -> ApplicationWindow {
    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(320)
        .default_height(120)
        .title("Enter Text")
        .build();

    let entry_buffer = gtk::EntryBuffer::builder().build();
    let entry = Entry::with_buffer(&entry_buffer);

    let status = Label::new(Some("Amending last entry (Esc to cancel)"));
    status.set_visible(false);

    let vbox = Box::new(gtk::Orientation::Vertical, 5);
    let hbox = Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(5)
        .margin_start(5)
        .margin_end(5)
        .homogeneous(true)
        .hexpand(true)
        .build();

    let button_s = Button::with_label("Start");
    let button_w = Button::with_label("Worked on");

    hbox.append(&button_s);
    hbox.append(&button_w);

    vbox.append(&status);
    vbox.append(&entry);
    vbox.append(&hbox);

    window.set_child(Some(&vbox));

    let amending = Rc::new(Cell::new(false));

    button_s.connect_clicked(move |_| click_start());
    button_w.connect_clicked({
        let entry = entry.clone();
        move |_| submit(&entry, false)
    });

    entry.connect_activate({
        let entry = entry.clone();
        let amending = amending.clone();
        move |_| submit(&entry, amending.get())
    });

    let key_controller = EventControllerKey::new();
    key_controller.set_propagation_phase(PropagationPhase::Capture);
    key_controller.connect_key_pressed({
        let entry = entry.clone();
        let status = status.clone();
        let amending = amending.clone();
        move |_, key, _, _| match key {
            Key::Up => {
                if amending.get() || !start_amend(&entry, &status, &amending) {
                    Propagation::Proceed
                } else {
                    Propagation::Stop
                }
            }
            Key::Escape => {
                if amending.get() {
                    cancel_amend(&entry, &status, &amending);
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

    let app = Application::builder()
        .application_id("com.example.textpopup")
        .build();

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
