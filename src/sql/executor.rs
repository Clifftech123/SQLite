//! Executes validated statements against the B-tree table.

#[derive(Debug, PartialEq, Eq)]
pub enum ExecuteError {
    DuplicateKey,
    TableFull,
}

/// Inserts one row unless its ID already exists.
pub fn execute_insert(row_to_insert: Row, table: &mut Table) -> Result<(), ExecuteError> {
    let key_to_insert = row_to_insert.id;
    let cursor = table.find(key_to_insert);

    let page = table.pager.get_page(cursor.page_num);
    let num_cells = leaf_node_num_cells(page);

    if cursor.cell_num < num_cells {
        let key_at_index = leaf_node_key(page, cursor.cell_num);
        if key_at_index == key_to_insert {
            return Err(ExecuteError::DuplicateKey);
        }
    }

    table.leaf_node_insert(&cursor, key_to_insert, &row_to_insert);
    Ok(())
}

/// Prints every row in ascending key order.
pub fn execute_select(table: &mut Table) {
    let mut cursor = table.start();
    while !cursor.end_of_table {
        let row = cursor.value(table);
        println!("{}", row);
        cursor.advance(table);
    }
}
use crate::btree::leaf::{leaf_node_key, leaf_node_num_cells};
use crate::btree::tree::Table;
use crate::row::Row;
