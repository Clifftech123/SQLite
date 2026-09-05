//! End-to-end tests for closing and reopening a database file.

use crate::common::TempDbFile;
use sqlite::btree::tree::Table;
use sqlite::row::Row;
use sqlite::sql::executor::{execute_insert, execute_select};

#[test]
fn rows_survive_a_close_and_reopen() {
    let file = TempDbFile::new("survive_reopen");

    {
        let mut table = Table::open(file.path()).unwrap();
        execute_insert(Row::new(1, "alice", "alice@example.com"), &mut table).unwrap();
        execute_insert(Row::new(2, "bob", "bob@example.com"), &mut table).unwrap();
        // Table (and its Pager) drops here, which flushes every cached page.
    }

    let mut reopened = Table::open(file.path()).unwrap();
    let mut cursor = reopened.start();
    let mut rows = vec![];
    while !cursor.end_of_table {
        rows.push(cursor.value(&mut reopened));
        cursor.advance(&mut reopened);
    }

    assert_eq!(
        rows,
        vec![
            Row::new(1, "alice", "alice@example.com"),
            Row::new(2, "bob", "bob@example.com"),
        ]
    );
}

#[test]
fn opening_an_existing_file_does_not_lose_its_row_count() {
    let file = TempDbFile::new("preserve_count");

    {
        let mut table = Table::open(file.path()).unwrap();
        for id in 1..=5u32 {
            execute_insert(Row::new(id, "u", "e@x.com"), &mut table).unwrap();
        }
    }

    // Reopening twice in a row should be stable and not duplicate or drop rows.
    {
        let mut table = Table::open(file.path()).unwrap();
        execute_select(&mut table);
    }
    let mut table = Table::open(file.path()).unwrap();
    let mut cursor = table.start();
    let mut count = 0;
    while !cursor.end_of_table {
        count += 1;
        cursor.advance(&mut table);
    }
    assert_eq!(count, 5);
}

#[test]
fn opening_a_missing_file_creates_it() {
    let file = TempDbFile::new("creates_new");
    assert!(!file.0.exists());
    let table = Table::open(file.path()).unwrap();
    drop(table);
    assert!(file.0.exists());
}
