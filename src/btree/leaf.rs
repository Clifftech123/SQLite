//! Leaf-node layout, cell access, lookup, and insertion.

use crate::btree::node::COMMON_NODE_HEADER_SIZE;
use crate::btree::node::{NodeType, set_node_parent, set_node_root, set_node_type};
use crate::config::INVALID_PAGE_NUM;
use crate::config::PAGE_SIZE;
use crate::row::ROW_SIZE;
use crate::storage::pager::Pager;
use crate::{btree::cursor::Cursor, btree::tree::Table, row::Row};
// Leaf Node Header Layout
pub const LEAF_NODE_NUM_CELLS_SIZE: usize = 4;
pub const LEAF_NODE_NUM_CELLS_OFFSET: usize = COMMON_NODE_HEADER_SIZE;
pub const LEAF_NODE_NEXT_LEAF_SIZE: usize = 4;
pub const LEAF_NODE_NEXT_LEAF_OFFSET: usize = LEAF_NODE_NUM_CELLS_OFFSET + LEAF_NODE_NUM_CELLS_SIZE;
pub const LEAF_NODE_HEADER_SIZE: usize =
    COMMON_NODE_HEADER_SIZE + LEAF_NODE_NUM_CELLS_SIZE + LEAF_NODE_NEXT_LEAF_SIZE;

// Leaf Node Body Layout
pub const LEAF_NODE_KEY_SIZE: usize = 4;
pub const LEAF_NODE_KEY_OFFSET: usize = 0;
pub const LEAF_NODE_VALUE_SIZE: usize = ROW_SIZE;
pub const LEAF_NODE_VALUE_OFFSET: usize = LEAF_NODE_KEY_OFFSET + LEAF_NODE_KEY_SIZE;
pub const LEAF_NODE_CELL_SIZE: usize = LEAF_NODE_KEY_SIZE + LEAF_NODE_VALUE_SIZE;
pub const LEAF_NODE_SPACE_FOR_CELLS: usize = PAGE_SIZE - LEAF_NODE_HEADER_SIZE;
pub const LEAF_NODE_MAX_CELLS: usize = LEAF_NODE_SPACE_FOR_CELLS / LEAF_NODE_CELL_SIZE;
pub const LEAF_NODE_RIGHT_SPLIT_COUNT: usize = LEAF_NODE_MAX_CELLS.div_ceil(2);
pub const LEAF_NODE_LEFT_SPLIT_COUNT: usize =
    (LEAF_NODE_MAX_CELLS + 1) - LEAF_NODE_RIGHT_SPLIT_COUNT;

/// Returns the number of occupied cells in a leaf.
pub fn leaf_node_num_cells(page: &[u8; PAGE_SIZE]) -> u32 {
    u32::from_le_bytes(
        page[LEAF_NODE_NUM_CELLS_OFFSET..LEAF_NODE_NUM_CELLS_OFFSET + LEAF_NODE_NUM_CELLS_SIZE]
            .try_into()
            .unwrap(),
    )
}

/// Stores the number of occupied cells in a leaf header.
pub fn set_leaf_node_num_cells(page: &mut [u8; PAGE_SIZE], num_cells: u32) {
    page[LEAF_NODE_NUM_CELLS_OFFSET..LEAF_NODE_NUM_CELLS_OFFSET + LEAF_NODE_NUM_CELLS_SIZE]
        .copy_from_slice(&num_cells.to_le_bytes());
}

/// Returns the linked-list successor page number.
pub fn leaf_node_next_leaf(page: &[u8; PAGE_SIZE]) -> u32 {
    u32::from_le_bytes(
        page[LEAF_NODE_NEXT_LEAF_OFFSET..LEAF_NODE_NEXT_LEAF_OFFSET + LEAF_NODE_NEXT_LEAF_SIZE]
            .try_into()
            .unwrap(),
    )
}

/// Sets the linked-list successor page number.
pub fn set_leaf_node_next_leaf(page: &mut [u8; PAGE_SIZE], next_leaf: u32) {
    page[LEAF_NODE_NEXT_LEAF_OFFSET..LEAF_NODE_NEXT_LEAF_OFFSET + LEAF_NODE_NEXT_LEAF_SIZE]
        .copy_from_slice(&next_leaf.to_le_bytes());
}

/// Calculates the byte offset of a cell within a leaf page.
pub fn leaf_node_cell_offset(cell_num: u32) -> usize {
    LEAF_NODE_HEADER_SIZE + (cell_num as usize) * LEAF_NODE_CELL_SIZE
}

/// Reads one cell's key.
pub fn leaf_node_key(page: &[u8; PAGE_SIZE], cell_num: u32) -> u32 {
    let offset = leaf_node_cell_offset(cell_num);
    u32::from_le_bytes(
        page[offset..offset + LEAF_NODE_KEY_SIZE]
            .try_into()
            .unwrap(),
    )
}

/// Writes one cell's key.
pub fn set_leaf_node_key(page: &mut [u8; PAGE_SIZE], cell_num: u32, key: u32) {
    let offset = leaf_node_cell_offset(cell_num);
    page[offset..offset + LEAF_NODE_KEY_SIZE].copy_from_slice(&key.to_le_bytes());
}

/// Returns the serialized-row bytes for one cell.
pub fn leaf_node_value_slice(page: &[u8; PAGE_SIZE], cell_num: u32) -> &[u8] {
    let offset = leaf_node_cell_offset(cell_num) + LEAF_NODE_KEY_SIZE;
    &page[offset..offset + LEAF_NODE_VALUE_SIZE]
}

/// Initializes an empty leaf page header.
pub fn initialize_leaf_node(page: &mut [u8; PAGE_SIZE]) {
    set_node_type(page, NodeType::Leaf);
    set_node_root(page, false);
    set_leaf_node_num_cells(page, 0);
    set_leaf_node_next_leaf(page, 0);
    set_node_parent(page, INVALID_PAGE_NUM);
}

/// Finds a key or its sorted insertion position in a leaf.
pub fn leaf_node_find(pager: &mut Pager, page_num: u32, key: u32) -> (u32, u32) {
    let page = pager.get_page(page_num);
    let num_cells = leaf_node_num_cells(page);

    // Binary search
    let mut min_index = 0;
    let mut one_past_max_index = num_cells;
    while one_past_max_index != min_index {
        let index = (min_index + one_past_max_index) / 2;
        let key_at_index = leaf_node_key(page, index);
        if key == key_at_index {
            return (page_num, index);
        }
        if key < key_at_index {
            one_past_max_index = index;
        } else {
            min_index = index + 1;
        }
    }

    (page_num, min_index)
}

impl Table {
    /// Inserts a row into a non-full leaf. Splitting is added by the tree layer.
    pub fn leaf_node_insert(&mut self, cursor: &Cursor, key: u32, value: &Row) {
        let page = self.pager.get_page(cursor.page_num);
        let count = leaf_node_num_cells(page);
        ensure_leaf_has_space(count);
        shift_cells_right(page, cursor.cell_num, count);
        set_leaf_node_num_cells(page, count + 1);
        write_leaf_cell(page, cursor.cell_num, key, value);
    }
}

/// Verifies that one additional cell fits on the page.
fn ensure_leaf_has_space(cell_count: u32) {
    assert!(
        (cell_count as usize) < LEAF_NODE_MAX_CELLS,
        "leaf page is full"
    );
}

/// Opens a gap by moving later cells one position to the right.
fn shift_cells_right(page: &mut [u8; PAGE_SIZE], insertion: u32, count: u32) {
    for index in (insertion..count).rev() {
        let source = leaf_node_cell_offset(index);
        let destination = leaf_node_cell_offset(index + 1);
        page.copy_within(source..source + LEAF_NODE_CELL_SIZE, destination);
    }
}

/// Writes a key and serialized row into one leaf cell.
fn write_leaf_cell(page: &mut [u8; PAGE_SIZE], cell_num: u32, key: u32, value: &Row) {
    set_leaf_node_key(page, cell_num, key);
    let value_start = leaf_node_cell_offset(cell_num) + LEAF_NODE_VALUE_OFFSET;
    value.serialize_into(&mut page[value_start..value_start + LEAF_NODE_VALUE_SIZE]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::btree::node::{get_node_parent, get_node_type, is_node_root};
    use crate::storage::page::new_page;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_table(name: &str) -> (Table, std::path::PathBuf) {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sqlite_leaf_test_{name}_{}_{n}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let table = Table::open(path.to_str().unwrap()).expect("open should succeed");
        (table, path)
    }

    impl Drop for TempGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    struct TempGuard(std::path::PathBuf);

    #[test]
    fn num_cells_round_trips() {
        let mut page = new_page();
        assert_eq!(leaf_node_num_cells(&page), 0);
        set_leaf_node_num_cells(&mut page, 7);
        assert_eq!(leaf_node_num_cells(&page), 7);
    }

    #[test]
    fn next_leaf_round_trips() {
        let mut page = new_page();
        set_leaf_node_next_leaf(&mut page, 3);
        assert_eq!(leaf_node_next_leaf(&page), 3);
    }

    #[test]
    fn initialize_leaf_node_sets_defaults() {
        let mut page = new_page();
        initialize_leaf_node(&mut page);
        assert_eq!(get_node_type(&page), NodeType::Leaf);
        assert!(!is_node_root(&page));
        assert_eq!(leaf_node_num_cells(&page), 0);
        assert_eq!(leaf_node_next_leaf(&page), 0);
        assert_eq!(get_node_parent(&page), INVALID_PAGE_NUM);
    }

    #[test]
    fn key_and_value_round_trip() {
        let mut page = new_page();
        initialize_leaf_node(&mut page);
        let row = Row::new(4, "carol", "carol@example.com");
        write_leaf_cell(&mut page, 0, 4, &row);
        assert_eq!(leaf_node_key(&page, 0), 4);
        assert_eq!(Row::deserialize_from(leaf_node_value_slice(&page, 0)), row);
    }

    #[test]
    fn find_on_empty_leaf_returns_position_zero() {
        let mut page = new_page();
        initialize_leaf_node(&mut page);
        let (table, path) = temp_table("find_empty");
        let _guard = TempGuard(path);
        let mut pager = table.pager;
        *pager.get_page(0) = *page;
        assert_eq!(leaf_node_find(&mut pager, 0, 42), (0, 0));
    }

    #[test]
    fn find_returns_exact_match_index() {
        let (table, path) = temp_table("find_exact");
        let _guard = TempGuard(path);
        let mut pager = table.pager;
        let page = pager.get_page(0);
        initialize_leaf_node(page);
        for (i, key) in [10u32, 20, 30, 40].into_iter().enumerate() {
            set_leaf_node_key(page, i as u32, key);
        }
        set_leaf_node_num_cells(page, 4);
        assert_eq!(leaf_node_find(&mut pager, 0, 30), (0, 2));
    }

    #[test]
    fn find_returns_sorted_insertion_point_when_missing() {
        let (table, path) = temp_table("find_missing");
        let _guard = TempGuard(path);
        let mut pager = table.pager;
        let page = pager.get_page(0);
        initialize_leaf_node(page);
        for (i, key) in [10u32, 20, 30, 40].into_iter().enumerate() {
            set_leaf_node_key(page, i as u32, key);
        }
        set_leaf_node_num_cells(page, 4);
        assert_eq!(leaf_node_find(&mut pager, 0, 25), (0, 2));
        assert_eq!(leaf_node_find(&mut pager, 0, 5), (0, 0));
        assert_eq!(leaf_node_find(&mut pager, 0, 100), (0, 4));
    }

    #[test]
    fn leaf_node_insert_appends_in_sorted_position() {
        let (mut table, path) = temp_table("insert_sorted");
        let _guard = TempGuard(path);
        let cursor0 = crate::btree::cursor::Cursor {
            page_num: 0,
            cell_num: 0,
            end_of_table: false,
        };
        table.leaf_node_insert(&cursor0, 10, &Row::new(10, "a", "a@x.com"));
        let cursor1 = crate::btree::cursor::Cursor {
            page_num: 0,
            cell_num: 1,
            end_of_table: false,
        };
        table.leaf_node_insert(&cursor1, 20, &Row::new(20, "b", "b@x.com"));
        let cursor_mid = crate::btree::cursor::Cursor {
            page_num: 0,
            cell_num: 1,
            end_of_table: false,
        };
        table.leaf_node_insert(&cursor_mid, 15, &Row::new(15, "c", "c@x.com"));

        let page = table.pager.get_page(0);
        assert_eq!(leaf_node_num_cells(page), 3);
        assert_eq!(leaf_node_key(page, 0), 10);
        assert_eq!(leaf_node_key(page, 1), 15);
        assert_eq!(leaf_node_key(page, 2), 20);
    }

    #[test]
    #[should_panic(expected = "leaf page is full")]
    fn leaf_node_insert_past_capacity_panics() {
        let (mut table, path) = temp_table("insert_full");
        let _guard = TempGuard(path);
        for i in 0..(LEAF_NODE_MAX_CELLS as u32 + 1) {
            let cursor = crate::btree::cursor::Cursor {
                page_num: 0,
                cell_num: i,
                end_of_table: false,
            };
            table.leaf_node_insert(&cursor, i, &Row::new(i, "u", "e@x.com"));
        }
    }
}
