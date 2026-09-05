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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_table(name: &str) -> (Table, std::path::PathBuf) {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sqlite_executor_test_{name}_{}_{n}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let table = Table::open(path.to_str().unwrap()).unwrap();
        (table, path)
    }

    struct TempGuard(std::path::PathBuf);
    impl Drop for TempGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn insert_then_read_back_via_cursor() {
        let (mut table, path) = temp_table("insert_read");
        let _guard = TempGuard(path);

        let row = Row::new(1, "alice", "alice@example.com");
        execute_insert(row.clone(), &mut table).unwrap();

        let cursor = table.start();
        assert!(!cursor.end_of_table);
        assert_eq!(cursor.value(&mut table), row);
    }

    #[test]
    fn duplicate_key_is_rejected() {
        let (mut table, path) = temp_table("duplicate");
        let _guard = TempGuard(path);

        execute_insert(Row::new(1, "a", "a@x.com"), &mut table).unwrap();
        let result = execute_insert(Row::new(1, "b", "b@x.com"), &mut table);
        assert_eq!(result, Err(ExecuteError::DuplicateKey));
    }

    #[test]
    fn inserts_out_of_order_are_read_back_sorted() {
        let (mut table, path) = temp_table("sorted");
        let _guard = TempGuard(path);

        for id in [5u32, 1, 3, 2, 4] {
            execute_insert(
                Row::new(id, "u", "e@x.com"),
                &mut table,
            )
            .unwrap();
        }

        let mut cursor = table.start();
        let mut ids = vec![];
        while !cursor.end_of_table {
            ids.push(cursor.value(&mut table).id);
            cursor.advance(&mut table);
        }
        assert_eq!(ids, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn execute_select_does_not_panic_on_an_empty_table() {
        let (mut table, path) = temp_table("empty_select");
        let _guard = TempGuard(path);
        execute_select(&mut table);
    }
}
