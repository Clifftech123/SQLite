//! End-to-end tests for inserts that grow a table's single leaf page.
//!
//! NOTE: `Table::create_new_root` and the `LEAF_NODE_*_SPLIT_COUNT` constants
//! exist in `btree::tree`/`btree::leaf`, but nothing currently calls them
//! from the insert path — there is no leaf- or internal-node-split routine
//! wired up yet. In practice a table can hold at most `LEAF_NODE_MAX_CELLS`
//! rows before insertion panics. These tests cover growth up to that limit
//! and document the current panic behavior beyond it; they do not exercise
//! multi-page trees, since that code path doesn't exist yet.

use crate::common::TempDbFile;
use sqlite::btree::leaf::LEAF_NODE_MAX_CELLS;
use sqlite::btree::tree::Table;
use sqlite::row::Row;
use sqlite::sql::executor::execute_insert;

#[test]
fn a_single_leaf_fills_up_to_its_maximum_cell_count() {
    let file = TempDbFile::new("fill_leaf");
    let mut table = Table::open(file.path()).unwrap();

    for id in 0..LEAF_NODE_MAX_CELLS as u32 {
        execute_insert(Row::new(id, "u", "e@x.com"), &mut table).unwrap();
    }

    let mut cursor = table.start();
    let mut ids = vec![];
    while !cursor.end_of_table {
        ids.push(cursor.value(&mut table).id);
        cursor.advance(&mut table);
    }
    assert_eq!(ids, (0..LEAF_NODE_MAX_CELLS as u32).collect::<Vec<_>>());
}

#[test]
fn out_of_order_inserts_up_to_capacity_still_read_back_sorted() {
    let file = TempDbFile::new("fill_leaf_unordered");
    let mut table = Table::open(file.path()).unwrap();

    // Insert descending so every row lands at the front of the leaf.
    for id in (0..LEAF_NODE_MAX_CELLS as u32).rev() {
        execute_insert(Row::new(id, "u", "e@x.com"), &mut table).unwrap();
    }

    let mut cursor = table.start();
    let mut ids = vec![];
    while !cursor.end_of_table {
        ids.push(cursor.value(&mut table).id);
        cursor.advance(&mut table);
    }
    assert_eq!(ids, (0..LEAF_NODE_MAX_CELLS as u32).collect::<Vec<_>>());
}

#[test]
#[should_panic(expected = "leaf page is full")]
fn inserting_past_the_single_leaf_capacity_panics() {
    let file = TempDbFile::new("overflow_leaf");
    let mut table = Table::open(file.path()).unwrap();

    for id in 0..=(LEAF_NODE_MAX_CELLS as u32) {
        execute_insert(Row::new(id, "u", "e@x.com"), &mut table).unwrap();
    }
}
