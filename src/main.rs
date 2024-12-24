use gtk4 as gtk;
use gtk::{Application, ApplicationWindow, Button, Entry, Box};
use gtk::prelude::*;
use std::fs::File;
use std::io::Write;

fn main() {
    let app = Application::builder().application_id("com.example.textpopup").build();

    app.connect_activate(|app| {
        let window = ApplicationWindow::builder()
        .application(app)
        .default_width(320)
        .default_height(120)
        .title("Enter Text")
        .build();

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
            window.close();
        });
    });

    app.run();
}
