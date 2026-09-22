# md_timesheet

**!!! WORK IN PROGRESS !!!** ( but usable )

This is a simple GTK4 program for creating timesheets formatted in Markdown. At present, only file storage is supported, and the timesheet file is configured via a config file (see [Configuration](#configuration)). Support for Joplin is in the pipeline.

# Configuration

The program reads its settings from a small `key = value` config file. It looks
for the file in this order and uses the first one it finds:

1. the path passed with `-c` / `--config` (must exist);
2. `./md_timesheet.config` in the current folder;
3. `$XDG_CONFIG_HOME/md_timesheet/config`, falling back to
   `$HOME/.config/md_timesheet/config`.

If no config file is found, the program shows a dialog offering to create one:
in the XDG config directory, in the current folder, or at a path you choose.
When the current folder or a custom path is used, a small pointer to that file
is written to `$XDG_CONFIG_HOME/md_timesheet/config` so it is found again on the
next launch.

The file has the following keys, all of which are required:

```ini
# Path to the timesheet markdown file (~/ expands to your home directory).
file_path = ~/Documents/timesheet.markdown

# Which table columns to include.
start_time = true
end_time = true
duration = true

# Round durations up to the next multiple of this many minutes.
duration_rounding = 10
```

Lines starting with `#` and blank lines are ignored. A copy of the above is
provided as `md_timesheet.config.example`; copy it to one of the locations
above and adjust the values.

The settings can also be edited in the GUI: the gear button next to the
Start/Worked on buttons opens a settings window where the path, columns and
rounding can be changed and saved back to the config file. A timesheet path
inside your home directory is saved back using `~/`.

# How to use

* On the first launch, let the program create a config file (or create one yourself, see above), then configure the desktop environment to start the program on a keystroke.
* Launch it for the first time and press the "Start" button. The program will create the timesheet file configured via *file_path*, add a day header to it, and then terminate. (The program always terminates after any button is clicked; this is not a bug but a feature! :-) )
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
"Worked on" - The task you completed. The text field is added to the description column.  

# button actions

* start - if last timestamp was yesterday, start new day and update last timestamp, else just update timestamp
* worked at - get last timestamp calculate duration add line to table add current timestamp to the end of document
if last timestamp was yesterday, add "worked at" to last day and start new day.

# Target platforms

Generic Linux desktops with GTK4 and libadwaita are the supported target.
Tagged releases also ship Debian/Ubuntu `.deb` packages and a FreeBSD `.pkg`,
but FreeBSD is **best effort**: the binary is cross-compiled and packaged
against FreeBSD 15 packages, but it is not run or validated on a real FreeBSD
system, and that pipeline step is allowed to fail without blocking a release.

# Compiling

Needs Rust 1.78 or newer (the committed `Cargo.lock` uses lockfile format v4). The *docker* folder contains a *Dockerfile* that can be used to compile the project.

```
git clone git@github.com:TheHolm/md_timesheet.git
cd docker
sudo docker build -t md_timesheet_build .
sudo docker run -ti --name rust-joplin -v "../":/code md_timesheet_build
cd /code
cargo build --release
```
Compiled executable will be in target/release/md_timesheet

# Testing

The non-GUI logic lives in a library crate (`src/lib.rs`) so it can be tested
without a display. Run the test suite with:

```
cargo test
```

Line coverage is generated with [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov):

```
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
cargo llvm-cov --tests
```

The GUI in `src/main.rs` is a thin wrapper and is not covered by the tests.

# TODO

* Proper error handling; a popup window needs to be displayed instead of writing to STDERR.
* Reading and writing changes need to be asynchronous and not performed from the main loop.
* Check the last non-empty line for the timestamp instead of just the last line.
* Joplin note support
* Packaging for Debian/Ubuntu and possibly something else.
* Improved formatting for the tables with all columns of the same width so that they are easily readable as text.

# Notes:
* API DOC https://joplinapp.org/help/api/references/rest_api/
* curl 'http://localhost:41184/notes/{note-id}?token={API-KEY}&fields=body'
    returning: {"body":"23 Dec\n\nDoing things 23/12/2024 10:00\\n\n\n","type_":1}
