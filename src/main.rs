use gtk4 as gtk;
use gtk::{glib, ApplicationWindow, Button, Entry, Box, Spinner};
use adw::{Application};
use gtk::prelude::*;
// use gtk::subclass::prelude::*;
use std::fs::File;
use std::io::Write;

use glib::{timeout_add_seconds_local};
use std::rc::Rc;

use std::cell::Cell;
use glib::{ParamSpec, Properties, Value};

#[derive(Properties, Default)]
#[properties(wrapper_type = super::CustomButton)]
pub struct SaveResults {
    #[property(get, set)]
    succes: Cell<bool>,
    #[property(get, set)]
    error: Cell<String>,
}

async fn some_computation() -> String {
    "represents the result of the computation".to_string()
}



fn main() {
    let app = Application::builder().application_id("com.example.textpopup").build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
        .application(app)
        .default_width(320)
        .default_height(120)
        .title("Enter Text")
        .build();

        let window: Rc<ApplicationWindow> = Rc::new(window);

        let vbox =   Box::new(gtk::Orientation::Vertical, 5);
        // let hbox =   Box::new(gtk::Orientation::Horizontal, 5);
        let hbox =   Box::builder()
                     .orientation(gtk::Orientation::Horizontal)
                     .spacing(5)
                     .margin_start(5)
                     .margin_end(5)
                     .homogeneous(true)
                     .hexpand(true)
                     .build();

        let button_s = Button::with_label("Start");
        let button_w = Button::with_label("worked at");

        let entry_buffer = gtk::EntryBuffer::builder().build();
        let entry =  Entry::with_buffer(&entry_buffer);

        hbox.append(&button_s);
        hbox.append(&button_w);

        vbox.append(&entry);
        vbox.append(&hbox);

        window.set_child(Some(&vbox));
        window.present();

        button_s.connect_clicked(move |_| {
            let text = entry.buffer().text().to_string();
            if !text.is_empty() {
                let mut file = File::create("output.txt").expect("Could not create file");
                file.write_all(text.as_bytes()).expect("Could not write to file");
            }

            tokio::spawn(async move {
                let res = some_computation();
            });

            let window_clone: Rc<ApplicationWindow> = Rc::clone(&window);
            timeout_add_seconds_local(1, move || {
                let spinner = Spinner::new();
                spinner.start();
                window_clone.set_child(Some(&spinner));
                glib::ControlFlow::Break // Stop the timeout
            });
            // window.present();
            // window.close();
        });
    });

    app.run();
}
