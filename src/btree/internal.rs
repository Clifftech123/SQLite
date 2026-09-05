//! Internal-node layout, child/key access, and lookup.

use crate::btree::leaf::leaf_node_find;
use crate::btree::node::get_node_type;
use crate::btree::node::{
    COMMON_NODE_HEADER_SIZE, NodeType, set_node_parent, set_node_root, set_node_type,
};
use crate::config::{INVALID_PAGE_NUM, PAGE_SIZE};
use crate::storage::pager::Pager;
use std::process;

// Internal Node Header Layout
pub const INTERNAL_NODE_NUM_KEYS_SIZE: usize = 4;
pub const INTERNAL_NODE_NUM_KEYS_OFFSET: usize = COMMON_NODE_HEADER_SIZE;
pub const INTERNAL_NODE_RIGHT_CHILD_SIZE: usize = 4;
pub const INTERNAL_NODE_RIGHT_CHILD_OFFSET: usize =
    INTERNAL_NODE_NUM_KEYS_OFFSET + INTERNAL_NODE_NUM_KEYS_SIZE;
pub const INTERNAL_NODE_HEADER_SIZE: usize =
    COMMON_NODE_HEADER_SIZE + INTERNAL_NODE_NUM_KEYS_SIZE + INTERNAL_NODE_RIGHT_CHILD_SIZE;

// Internal Node Body Layout
pub const INTERNAL_NODE_KEY_SIZE: usize = 4;
pub const INTERNAL_NODE_CHILD_SIZE: usize = 4;
pub const INTERNAL_NODE_CELL_SIZE: usize = INTERNAL_NODE_CHILD_SIZE + INTERNAL_NODE_KEY_SIZE;
pub const INTERNAL_NODE_MAX_KEYS: usize = 3; // Kept small for testing tree branching

/// Returns the number of separator keys in an internal node.
pub fn internal_node_num_keys(page: &[u8; PAGE_SIZE]) -> u32 {
    u32::from_le_bytes(
        page[INTERNAL_NODE_NUM_KEYS_OFFSET
            ..INTERNAL_NODE_NUM_KEYS_OFFSET + INTERNAL_NODE_NUM_KEYS_SIZE]
            .try_into()
            .unwrap(),
    )
}

/// Stores the separator-key count.
pub fn set_internal_node_num_keys(page: &mut [u8; PAGE_SIZE], num_keys: u32) {
    page[INTERNAL_NODE_NUM_KEYS_OFFSET
        ..INTERNAL_NODE_NUM_KEYS_OFFSET + INTERNAL_NODE_NUM_KEYS_SIZE]
        .copy_from_slice(&num_keys.to_le_bytes());
}

/// Returns the rightmost child page number.
pub fn internal_node_right_child(page: &[u8; PAGE_SIZE]) -> u32 {
    u32::from_le_bytes(
        page[INTERNAL_NODE_RIGHT_CHILD_OFFSET
            ..INTERNAL_NODE_RIGHT_CHILD_OFFSET + INTERNAL_NODE_RIGHT_CHILD_SIZE]
            .try_into()
            .unwrap(),
    )
}
/// Sets the rightmost child page number.
pub fn set_internal_node_right_child(page: &mut [u8; PAGE_SIZE], right_child: u32) {
    page[INTERNAL_NODE_RIGHT_CHILD_OFFSET
        ..INTERNAL_NODE_RIGHT_CHILD_OFFSET + INTERNAL_NODE_RIGHT_CHILD_SIZE]
        .copy_from_slice(&right_child.to_le_bytes());
}

/// Calculates the byte offset of an internal-node cell.
pub fn internal_node_cell_offset(cell_num: u32) -> usize {
    INTERNAL_NODE_HEADER_SIZE + (cell_num as usize) * INTERNAL_NODE_CELL_SIZE
}

/// Returns a child page, including the special rightmost child.
pub fn internal_node_child(page: &[u8; PAGE_SIZE], child_num: u32) -> u32 {
    let num_keys = internal_node_num_keys(page);
    validate_child_index(child_num, num_keys);
    if child_num == num_keys {
        valid_right_child(page)
    } else {
        valid_cell_child(page, child_num)
    }
}

/// Rejects an index beyond the node's child range.
fn validate_child_index(child_num: u32, num_keys: u32) {
    if child_num > num_keys {
        eprintln!("child index {child_num} exceeds key count {num_keys}");
        process::exit(1);
    }
}

/// Reads and validates the special rightmost child pointer.
fn valid_right_child(page: &[u8; PAGE_SIZE]) -> u32 {
    let child = internal_node_right_child(page);
    validate_child_page(child, "right child");
    child
}

/// Reads and validates a child pointer stored in a normal cell.
fn valid_cell_child(page: &[u8; PAGE_SIZE], child_num: u32) -> u32 {
    let offset = internal_node_cell_offset(child_num);
    let child = u32::from_le_bytes(
        page[offset..offset + INTERNAL_NODE_CHILD_SIZE]
            .try_into()
            .expect("internal child field must contain four bytes"),
    );
    validate_child_page(child, "cell child");
    child
}

/// Rejects the sentinel used for a missing child page.
fn validate_child_page(child: u32, label: &str) {
    if child == INVALID_PAGE_NUM {
        eprintln!("{label} points to an invalid page");
        process::exit(1);
    }
}

/// Stores a child page number in an internal cell.
pub fn set_internal_node_child(page: &mut [u8; PAGE_SIZE], child_num: u32, child_page: u32) {
    let offset = internal_node_cell_offset(child_num);
    page[offset..offset + INTERNAL_NODE_CHILD_SIZE].copy_from_slice(&child_page.to_le_bytes());
}

/// Reads a separator key.
pub fn internal_node_key(page: &[u8; PAGE_SIZE], key_num: u32) -> u32 {
    let offset = internal_node_cell_offset(key_num) + INTERNAL_NODE_CHILD_SIZE;
    u32::from_le_bytes(
        page[offset..offset + INTERNAL_NODE_KEY_SIZE]
            .try_into()
            .unwrap(),
    )
}

/// Writes a separator key.
pub fn set_internal_node_key(page: &mut [u8; PAGE_SIZE], key_num: u32, key: u32) {
    let offset = internal_node_cell_offset(key_num) + INTERNAL_NODE_CHILD_SIZE;
    page[offset..offset + INTERNAL_NODE_KEY_SIZE].copy_from_slice(&key.to_le_bytes());
}

/// Initializes an empty internal node.
pub fn initialize_internal_node(page: &mut [u8; PAGE_SIZE]) {
    set_node_type(page, NodeType::Internal);
    set_node_root(page, false);
    set_internal_node_num_keys(page, 0);
    set_internal_node_right_child(page, INVALID_PAGE_NUM);
    set_node_parent(page, INVALID_PAGE_NUM);
}

/// Finds which child should contain a search key.
pub fn internal_node_find_child(page: &[u8; PAGE_SIZE], key: u32) -> u32 {
    let num_keys = internal_node_num_keys(page);
    let mut min_index = 0;
    let mut max_index = num_keys;

    while min_index != max_index {
        let index = (min_index + max_index) / 2;
        let key_to_right = internal_node_key(page, index);
        if key_to_right >= key {
            max_index = index;
        } else {
            min_index = index + 1;
        }
    }

    min_index
}

/// Descends internal nodes until the target leaf is reached.
pub fn internal_node_find(pager: &mut Pager, page_num: u32, key: u32) -> (u32, u32) {
    let child_index = internal_node_find_child(pager.get_page(page_num), key);
    let child_num = internal_node_child(pager.get_page(page_num), child_index);
    let child_type = get_node_type(pager.get_page(child_num));

    match child_type {
        NodeType::Leaf => leaf_node_find(pager, child_num, key),
        NodeType::Internal => internal_node_find(pager, child_num, key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::btree::leaf::{initialize_leaf_node, set_leaf_node_key, set_leaf_node_num_cells};
    use crate::btree::node::is_node_root;
    use crate::storage::page::new_page;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// A Pager backed by a fresh, auto-deleted temp file.
    struct TempPager {
        pager: Pager,
        path: std::path::PathBuf,
    }

    impl TempPager {
        fn new(name: &str) -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "sqlite_internal_test_{name}_{}_{n}.db",
                std::process::id()
            ));
            let _ = std::fs::remove_file(&path);
            let pager = Pager::open(path.to_str().unwrap()).unwrap();
            Self { pager, path }
        }
    }

    impl Drop for TempPager {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    #[test]
    fn num_keys_round_trips() {
        let mut page = new_page();
        set_internal_node_num_keys(&mut page, 2);
        assert_eq!(internal_node_num_keys(&page), 2);
    }

    #[test]
    fn right_child_round_trips() {
        let mut page = new_page();
        set_internal_node_right_child(&mut page, 5);
        assert_eq!(internal_node_right_child(&page), 5);
    }

    #[test]
    fn initialize_internal_node_sets_defaults() {
        let mut page = new_page();
        initialize_internal_node(&mut page);
        assert_eq!(get_node_type(&page), NodeType::Internal);
        assert!(!is_node_root(&page));
        assert_eq!(internal_node_num_keys(&page), 0);
        assert_eq!(internal_node_right_child(&page), INVALID_PAGE_NUM);
    }

    #[test]
    fn child_and_key_round_trip() {
        let mut page = new_page();
        initialize_internal_node(&mut page);
        set_internal_node_num_keys(&mut page, 2);
        set_internal_node_child(&mut page, 0, 10);
        set_internal_node_key(&mut page, 0, 100);
        set_internal_node_child(&mut page, 1, 20);
        set_internal_node_key(&mut page, 1, 200);
        set_internal_node_right_child(&mut page, 30);

        assert_eq!(internal_node_child(&page, 0), 10);
        assert_eq!(internal_node_key(&page, 0), 100);
        assert_eq!(internal_node_child(&page, 1), 20);
        assert_eq!(internal_node_key(&page, 1), 200);
        // Index == num_keys reads the special right-child slot.
        assert_eq!(internal_node_child(&page, 2), 30);
    }

    #[test]
    fn find_child_returns_leftmost_key_greater_or_equal() {
        let mut page = new_page();
        initialize_internal_node(&mut page);
        set_internal_node_num_keys(&mut page, 3);
        set_internal_node_key(&mut page, 0, 10);
        set_internal_node_key(&mut page, 1, 20);
        set_internal_node_key(&mut page, 2, 30);

        assert_eq!(internal_node_find_child(&page, 5), 0);
        assert_eq!(internal_node_find_child(&page, 10), 0);
        assert_eq!(internal_node_find_child(&page, 15), 1);
        assert_eq!(internal_node_find_child(&page, 30), 2);
        // Greater than every key: belongs under the implicit right child.
        assert_eq!(internal_node_find_child(&page, 999), 3);
    }

    #[test]
    fn internal_node_find_descends_to_the_correct_leaf() {
        let mut tp = TempPager::new("find_descend");

        // Page 1: leaf with keys [1, 2]
        let leaf_a = tp.pager.get_page(1);
        initialize_leaf_node(leaf_a);
        set_leaf_node_num_cells(leaf_a, 2);
        set_leaf_node_key(leaf_a, 0, 1);
        set_leaf_node_key(leaf_a, 1, 2);

        // Page 2: leaf with keys [5, 9]
        let leaf_b = tp.pager.get_page(2);
        initialize_leaf_node(leaf_b);
        set_leaf_node_num_cells(leaf_b, 2);
        set_leaf_node_key(leaf_b, 0, 5);
        set_leaf_node_key(leaf_b, 1, 9);

        // Page 0: internal root, one key(2) splitting leaf_a | leaf_b
        let root = tp.pager.get_page(0);
        initialize_internal_node(root);
        set_internal_node_num_keys(root, 1);
        set_internal_node_child(root, 0, 1);
        set_internal_node_key(root, 0, 2);
        set_internal_node_right_child(root, 2);

        assert_eq!(internal_node_find(&mut tp.pager, 0, 1), (1, 0));
        assert_eq!(internal_node_find(&mut tp.pager, 0, 9), (2, 1));
        // A key that isn't present yet still returns a sorted insertion point.
        assert_eq!(internal_node_find(&mut tp.pager, 0, 7), (2, 1));
    }

    // Note: internal_node_child's sentinel and bounds checks call
    // process::exit(1) rather than panicking, so they aren't exercised here
    // — doing so would terminate the whole test binary instead of failing
    // just this test.
}
