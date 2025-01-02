# md_timesheet

**!!! WORK IN PROGRESS !!!** ( but usable )

This is a simple GTK4 program for creating timesheets formatted in Markdown. At present, only file storage is supported, and the file path is hardcoded as (./timesheet.markdown). Support for Joplin is in the pipeline.

# How to use

* Configure the desktop environment to start the program on a keystroke.
* Launch it for the first time and press the "Start" button. The program will create a *timesheet.markdown* file in the current directory, add a day header to it, and then terminate. (The program always terminates after any button is clicked; this is not a bug but a feature! :-) )
* When you **finish** working on a task, start it again by entering the task description into the entry field and pressing "worked at". This will add an entry to the existing table and terminate. If you worked past midnight, the entry will be added to the previous day, and a new table for the current day will be created.
* When you begin your day or return from a break, click "start" to begin.

Important: **Never remove the timestamp on the last line**

All errors are sent to STDERR at the moment; better error handling is in the plan.

# Note format

```text
#   2 January

| Description | Start Time | End Time | Duration |
| --- | --- | --- | --- |
| Doing Stuff | 12:00 | 13:02 | 01:10 |
24/12/2024 15:41
```

last line contains timestamp of last operation formated as  %d/%m/%Y %H:%M (example 24/12/2024 15:41)


# GUI Buttons  
"Start" - Begin time tracking; the text field is ignored.  
"Worked at" - The task you completed. The text field is added to the description column.  

# button actions

* start - if last timestamp was yesterday, start new day and update last timestamp, else just update timestamp
* worked at - get last timestamp calculate duration add line to table add current timestamp to the end of document
if last timestamp was yesterday, add "worked at" to last day and start new day.

# Compiling

Needs Rust 1.73. The *docker* folder contains a *Dockerfile* that can be used to compile the project.

```
git clone git@github.com:TheHolm/md_timesheet.git
cd docker
sudo docker build .
sudo docker run -ti --name rust-joplin -v "../":/code rust:local
cd /code
cargo build --release
```
Compiled executable will be in target/release/md_timesheet

# TODO

* Proper error handling; a popup window needs to be displayed instead of writing to STDERR.
* Reading and writing changes need to be asynchronous and not performed from the main loop.
* Configuration should be done via the GUI (currently hardcoded).
* Check the last non-empty line for the timestamp instead of just the last line.
* Tests?
* Joplin note support
* Packaging for Debian/Ubuntu and possibly something else.
* Improved formatting for the tables with all columns of the same width so that they are easily readable as text.

# Notes:
* API DOC https://joplinapp.org/help/api/references/rest_api/
* curl 'http://localhost:41184/notes/{note-id}?token={API-KEY}&fields=body'
    returning: {"body":"23 Dec\n\nDoing things 23/12/2024 10:00\\n\n\n","type_":1}
