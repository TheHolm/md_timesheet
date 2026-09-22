//! GTK4/libadwaita front-end for `md_timesheet`.
//!
//! Deliberately thin: it wires the widgets to the library in [`lib.rs`](../lib.rs)
//! and turns I/O results into process exit codes. The `Start`/`Worked on`
//! actions always terminate the program, which is intentional.

use std::process;
use std::rc::Rc;

use adw::Application;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Box, Button, Entry};
use gtk4 as gtk;
use once_cell::sync::Lazy;

use chrono::prelude::*;

use md_timesheet::{
    apply_start, apply_worked, read_document, write_document, Destination, RECORD_FORMAT,
};

/// The storage destination used by the GUI (hardcoded for now).
static STORE_IN: Lazy<Destination> =
    Lazy::new(|| Destination::TextFile("timesheet.markdown".to_string()));

/// Handles the "Start" button: records the current time and exits.
///
/// A read or write failure exits with status 1; an unparsable trailing
/// timestamp is reported but the (unchanged) document is still written and the
/// program exits with status 0, matching the previous behaviour.
fn click_start() {
    let mut lines = match read_document(&STORE_IN) {
        Ok(lines) => lines,
        Err(e) => {
            eprintln!("Error opening file: {}", e);
            process::exit(1);
        }
    };

    let current_datetime = Local::now().naive_local();
    if let Err(e) = apply_start(&mut lines, current_datetime, &RECORD_FORMAT) {
        println!("Failed to parse date: {}", e);
    }

    match write_document(&STORE_IN, lines) {
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
        Ok(_) => process::exit(0),
    }
}

/// Handles the "Worked on" button: records the completed task and exits.
///
/// Does nothing when the entry field is empty. A read/write failure or an
/// unparsable trailing timestamp exits with status 1.
fn click_worked(entry: gtk::Entry) {
    let text = entry.buffer().text().to_string();

    if !text.is_empty() {
        let mut lines = match read_document(&STORE_IN) {
            Ok(lines) => lines,
            Err(e) => {
                eprintln!("Error opening file: {}", e);
                process::exit(1);
            }
        };

        let current_datetime = Local::now().naive_local();
        if let Err(e) = apply_worked(&mut lines, text, current_datetime, &RECORD_FORMAT) {
            eprintln!("Failed to parse date: {}", e);
            process::exit(1);
        }

        match write_document(&STORE_IN, lines) {
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
            Ok(_) => process::exit(0),
        }
    }
}

/// Builds the application window and connects the buttons.
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

        vbox.append(&entry);
        vbox.append(&hbox);

        window.set_child(Some(&vbox));
        window.present();

        button_s.connect_clicked(move |_| click_start());
        button_w.connect_clicked(move |_| click_worked(entry.clone()));
    });

    app.run();
}
