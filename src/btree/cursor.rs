//! A position used for point lookups and ordered scans.

use crate::btree::leaf::{leaf_node_next_leaf, leaf_node_num_cells, leaf_node_value_slice};
use crate::btree::tree::Table;
use crate::row::Row;

/// Location of one cell in a leaf page.
#[derive(Debug, Clone)]
pub struct Cursor {
    pub page_num: u32,
    pub cell_num: u32,
    pub end_of_table: bool,
}

impl Cursor {
    /// Decodes and returns the row at the current cursor position.
    pub fn value(&self, table: &mut Table) -> Row {
        let page = table.pager.get_page(self.page_num);
        let slice = leaf_node_value_slice(page, self.cell_num);
        Row::deserialize_from(slice)
    }

    /// Moves to the next cell, following the linked-leaf pointer when needed.
    pub fn advance(&mut self, table: &mut Table) {
        let page = table.pager.get_page(self.page_num);
        let num_cells = leaf_node_num_cells(page);

        self.cell_num += 1;
        if self.cell_num >= num_cells {
            let next_page_num = leaf_node_next_leaf(page);
            if next_page_num == 0 {
                self.end_of_table = true;
            } else {
                self.page_num = next_page_num;
                self.cell_num = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::btree::leaf::{initialize_leaf_node, set_leaf_node_key, set_leaf_node_next_leaf};
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_table(name: &str) -> (Table, std::path::PathBuf) {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sqlite_cursor_test_{name}_{}_{n}.db",
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
    fn value_decodes_the_row_at_the_cursor() {
        let (mut table, path) = temp_table("value");
        let _guard = TempGuard(path);
        let cursor0 = Cursor {
            page_num: 0,
            cell_num: 0,
            end_of_table: false,
        };
        let row = Row::new(1, "dave", "dave@example.com");
        table.leaf_node_insert(&cursor0, 1, &row);

        let cursor = Cursor {
            page_num: 0,
            cell_num: 0,
            end_of_table: false,
        };
        assert_eq!(cursor.value(&mut table), row);
    }

    #[test]
    fn advance_walks_cells_within_one_leaf() {
        let (mut table, path) = temp_table("advance_single_leaf");
        let _guard = TempGuard(path);
        for i in 0..3u32 {
            let cursor = Cursor {
                page_num: 0,
                cell_num: i,
                end_of_table: false,
            };
            table.leaf_node_insert(&cursor, i, &Row::new(i, "u", "e@x.com"));
        }

        let mut cursor = Cursor {
            page_num: 0,
            cell_num: 0,
            end_of_table: false,
        };
        let mut seen = vec![];
        while !cursor.end_of_table {
            seen.push(cursor.value(&mut table).id);
            cursor.advance(&mut table);
        }
        assert_eq!(seen, vec![0, 1, 2]);
    }

    #[test]
    fn advance_follows_the_next_leaf_pointer() {
        let (mut table, path) = temp_table("advance_next_leaf");
        let _guard = TempGuard(path);

        let page0 = table.pager.get_page(0);
        initialize_leaf_node(page0);
        crate::btree::leaf::set_leaf_node_num_cells(page0, 1);
        set_leaf_node_key(page0, 0, 1);
        set_leaf_node_next_leaf(page0, 1);

        let page1 = table.pager.get_page(1);
        initialize_leaf_node(page1);
        crate::btree::leaf::set_leaf_node_num_cells(page1, 1);
        set_leaf_node_key(page1, 0, 2);
        set_leaf_node_next_leaf(page1, 0);

        let mut cursor = Cursor {
            page_num: 0,
            cell_num: 0,
            end_of_table: false,
        };
        cursor.advance(&mut table);
        assert_eq!(cursor.page_num, 1);
        assert_eq!(cursor.cell_num, 0);
        assert!(!cursor.end_of_table);

        cursor.advance(&mut table);
        assert!(cursor.end_of_table);
    }
}
