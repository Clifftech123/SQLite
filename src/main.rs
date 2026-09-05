use sqlite::btree::tree::Table;
use sqlite::repl;
use std::{env, process};

/// Opens the requested database and hands control to the REPL.
fn main() {
    let filename = match env::args().nth(1) {
        Some(filename) => filename,
        None => {
            eprintln!("Usage: sqlite <database_file>");
            process::exit(1);
        }
    };

    let mut table = Table::open(&filename).unwrap_or_else(|error| {
        eprintln!("Failed to open database file '{filename}': {error}");
        process::exit(1);
    });

    repl::run(&mut table);
}
