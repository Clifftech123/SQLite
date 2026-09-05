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
