use gtk4 as gtk;
use gtk::{ApplicationWindow, Button, Entry, Box};
use adw::{Application};
use gtk::prelude::*;

// use std::fs::File;
// use std::io::Write;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};

use std::process;
// use glib::{timeout_add_seconds_local};
use std::rc::Rc;
use once_cell::sync::Lazy;

use chrono::prelude::*;

struct JoplinNote {
    id: String,
    url: String,
    api_key: String
}

struct RecordsFormat {
    start_time: bool,
    end_time: bool,
    duration: bool,
    duration_rounding: i32
}

enum Destination {
    TextFile(String), // contains path to file
    JoplinNote(JoplinNote)
}


fn read_document(dest: &Destination) -> Result<Vec<String>,String> {
    match dest {
        Destination::JoplinNote(_) => Err("Joplin support has not been implemented.".to_string()),
        Destination::TextFile(file_path) => {
            let file: Result<std::fs::File,std::io::Error> = fs::OpenOptions::new().write(true).read(true).create(true).open(&file_path);
            match file {   // .map_err(|e| format!("Error writing to file: {}", e))
                Err(e) => Err(format!("Error opening file for reading or creating a new file: {}", e.to_string())),
                Ok(file) => {
                    let reader = BufReader::new(file);
                    let lines: Result<Vec<String>, std::io::Error> = reader.lines().collect::<Result<_,     _>>();
                    match lines {
                        Err(e) => Err(format!("Error while splitting file content into lines: {}", e.to_string())),
                        Ok(lines) => Ok(lines)
                    }
                }
            }
        }
    }
} // end of fn read_document(dest: Destination) -> Result<Vec<String>,String>{}

fn write_document(dest: &Destination, lines: Vec<String> ) -> Result<(),String> {
    match dest {
        Destination::JoplinNote(_) => Err("Joplin support has not been implemented.".to_string()),
        Destination::TextFile(file_path) => {
            let file: Result<std::fs::File,std::io::Error> =  OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&file_path);
            match file {
                Err(_) => Err("Error opening file for write".to_string()),
                Ok(mut file) => {
                    for line in lines {
                        writeln!(file, "{}", line).map_err(|e| e.to_string())?
                    }
                    Ok(())
                }
            }

        }
    }
} // end of fn write_document(dest: Destination, lines: Vec<String> ) -> Result<()>

fn new_day(date: &NaiveDateTime, format: &RecordsFormat) -> Vec<String> {
  let mut day_header: Vec<String> = Vec::new();
  day_header.push(format!("{}{}", "#  ", date.format("%e %B").to_string()));
  day_header.push("".to_string());

  let mut columns: Vec<String> = Vec::new();
  columns.push("Description".to_string());
  if format.start_time { columns.push("Start Time".to_string());}
  if format.end_time { columns.push("End Time".to_string());}
  if format.duration { columns.push("Duration".to_string());}

  day_header.push(format!("| {} |",columns.join(" | ")));
  day_header.push(format!("| {} |", columns.iter().map(|_| "---").collect::<Vec<&str>>().join(" | ")));

  day_header
}

fn new_entry(description: String, start_date: &NaiveDateTime, end_date: &NaiveDateTime, format: &RecordsFormat) -> Vec<String> {
  let time_format = "%H:%M";
  let mut day_header: Vec<String> = Vec::new();

  let duration: i64 = end_date.signed_duration_since(*start_date).num_minutes();
  let duration: i64 = (duration/format.duration_rounding as i64)*format.duration_rounding as i64 + if duration%(format.duration_rounding as i64) > 0 { format.duration_rounding as i64 } else { 0 };

  let mut columns: Vec<String> = Vec::new();
  columns.push(description);
  if format.start_time { columns.push(start_date.format(&time_format).to_string());}
  if format.end_time { columns.push(end_date.format(&time_format).to_string());}
  if format.duration { columns.push(format!("{:02}:{:02}",duration/60,duration%60));}

  day_header.push(format!("| {} |",columns.join(" | ")));

  day_header
}


// defaults need to be moved out this function
// static mut store_in: Destination = Destination::TextFile("timesheet.txt".to_string());
static store_in: Lazy<Destination> = Lazy::new(|| {
    Destination::TextFile("timesheet.markdown".to_string())
});
static record_format:RecordsFormat = RecordsFormat {
    start_time: true,
    end_time: true,
    duration: true,
    duration_rounding: 10
};


fn click_start() {
    let lines = read_document(&store_in);

    match lines {
        Err(e) => {
              eprintln!("Error opening file: {}", e);
              process::exit(1);
        },
        Ok(mut lines) => {
            let date_and_time: String = Local::now().naive_local().format("%d/%m/%Y %H:%M").to_string(); // we will need in any case

            if  lines.len() == 0 { // empty file
                lines.append(&mut new_day(&Local::now().naive_local(),&record_format));
                lines.push(date_and_time);
            } else {
                if let Some(last_line) = lines.last_mut() {
                    let format = "%d/%m/%Y %H:%M";
                    match NaiveDateTime::parse_from_str(last_line, format) {
                        Ok(_naive_datetime) => { // The last line contains only the date and time.
                            *last_line = date_and_time;
                        }
                        Err(e) => {
                            println!("Failed to parse date: {}", e);
                        }
                    }
                };
            };

            match write_document(&store_in,lines) {
               Err(e) =>  {
                     eprintln!("Error: {}", e);
                     process::exit(1);
                },
                Ok(_) => {
                    process::exit(0);
                }

            }
        }
    }
}


fn click_worked(entry: gtk::Entry) {

    let text = entry.buffer().text().to_string();

    if !text.is_empty() {
        let lines = read_document(&store_in);

        match lines {
            Err(e) => {
                  eprintln!("Error opening file: {}", e);
                  process::exit(1);
            },
            Ok(mut lines) => {
                let current_datetime = Local::now().naive_local(); // we will need in any case

                if  lines.len() == 0 { // empty file so it will add 0 minute long task after header
                    lines.append(&mut new_day(&current_datetime,&record_format));
                    lines.append(&mut new_entry(text, &current_datetime, &current_datetime, &record_format));
                    lines.push(current_datetime.format("%d/%m/%Y %H:%M").to_string());

                } else {
                    if let Some(last_line) = lines.last_mut() {
                        let format = "%d/%m/%Y %H:%M";
                        match NaiveDateTime::parse_from_str(last_line, format) {
                            Ok(previous_datetime) => { // The last line contains only the date and time.
                                if previous_datetime.date() == current_datetime.date() {
                                    lines.pop();
                                    lines.append(&mut new_entry(text, &previous_datetime, &current_datetime, &record_format));
                                    lines.push(current_datetime.format("%d/%m/%Y %H:%M").to_string());
                                } else { // new day started.
                                    lines.pop();
                                    lines.append(&mut new_entry(text, &previous_datetime, &current_datetime, &record_format));
                                    lines.append(&mut new_day(&current_datetime,&record_format));
                                    lines.push(current_datetime.format("%d/%m/%Y %H:%M").to_string());
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse date: {}", e);
                                process::exit(1)
                            }
                        }
                    };
                };

                match write_document(&store_in,lines) {
                   Err(e) =>  {
                         eprintln!("Error: {}", e);
                         process::exit(1);
                    },
                    Ok(_) => {
                        process::exit(0);
                    }
                }
            }
        }
        // let mut file = File::create("output.txt").expect("Could not create file");
        // file.write_all(text.as_bytes()).expect("Could not write to file");
   }
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

        let entry_buffer = gtk::EntryBuffer::builder().build();
        let entry =  Entry::with_buffer(&entry_buffer);

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
} // end of fn main()
