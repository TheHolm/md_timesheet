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
use std::process;
use std::rc::Rc;

use adw::Application;
use gtk::gdk::Key;
use gtk::glib::Propagation;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Box, Button, Entry, EventControllerKey, Label, PropagationPhase};
use gtk4 as gtk;
use once_cell::sync::Lazy;

use chrono::prelude::*;

use md_timesheet::{
    amend_last_entry, apply_start, apply_worked, last_entry_description, read_document,
    write_document, Destination, RECORD_FORMAT,
};

/// The storage destination used by the GUI (hardcoded for now).
static STORE_IN: Lazy<Destination> =
    Lazy::new(|| Destination::TextFile("timesheet.markdown".to_string()));

/// Reads the document, exiting with an error message on failure.
fn read_lines() -> Vec<String> {
    match read_document(&STORE_IN) {
        Ok(lines) => lines,
        Err(e) => {
            eprintln!("Error opening file: {}", e);
            process::exit(1);
        }
    }
}

/// Writes the document and exits successfully, or exits with an error.
fn write_lines(lines: Vec<String>) {
    match write_document(&STORE_IN, lines) {
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
    let mut lines = read_lines();
    let current_datetime = Local::now().naive_local();
    if let Err(e) = apply_start(&mut lines, current_datetime, &RECORD_FORMAT) {
        println!("Failed to parse date: {}", e);
    }
    write_lines(lines);
}

/// Records `description` as a completed task and exits.
fn record_worked(description: String) {
    let mut lines = read_lines();
    let current_datetime = Local::now().naive_local();
    if let Err(e) = apply_worked(&mut lines, description, current_datetime, &RECORD_FORMAT) {
        eprintln!("Failed to parse date: {}", e);
        process::exit(1);
    }
    write_lines(lines);
}

/// Amends the last entry with `description` and exits.
fn record_amend(description: String) {
    let mut lines = read_lines();
    let current_datetime = Local::now().naive_local();
    if let Err(e) = amend_last_entry(&mut lines, description, current_datetime, &RECORD_FORMAT) {
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
    match read_document(&STORE_IN) {
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
fn main() {
    let app = Application::builder()
        .application_id("com.example.textpopup")
        .build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
            .application(app)
            .default_width(320)
            .default_height(120)
            .title("Enter Text")
            .build();

        let window: Rc<ApplicationWindow> = Rc::new(window);

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
        window.present();

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
    });

    app.run();
}
