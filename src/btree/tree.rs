//! Coordinates pages, nodes, and cursors as one database table.

use crate::btree::cursor::Cursor;
use crate::btree::internal::*;
use crate::btree::leaf::*;
use crate::btree::node::*;
use crate::storage::pager::Pager;
use std::io;

/// An open database table backed by a pager and B-tree root.
pub struct Table {
    pub pager: Pager,
    pub root_page_num: u32,
}

impl Table {
    /// Opens a database and initializes an empty root for a new file.
    pub fn open(filename: &str) -> io::Result<Self> {
        let mut pager = Pager::open(filename)?;
        let root_page_num = 0;

        if pager.num_pages == 0 {
            // New database file. Initialize root page 0 as an empty leaf.
            let root_node = pager.get_page(0);
            initialize_leaf_node(root_node);
            set_node_root(root_node, true);
        }

        Ok(Self {
            pager,
            root_page_num,
        })
    }

    /// Finds a key or its sorted insertion position.
    pub fn find(&mut self, key: u32) -> Cursor {
        let root_node = self.pager.get_page(self.root_page_num);
        let root_type = get_node_type(root_node);

        let (page_num, cell_num) = match root_type {
            NodeType::Leaf => leaf_node_find(&mut self.pager, self.root_page_num, key),
            NodeType::Internal => internal_node_find(&mut self.pager, self.root_page_num, key),
        };

        Cursor {
            page_num,
            cell_num,
            end_of_table: false,
        }
    }

    /// Returns a cursor positioned at the first row.
    pub fn start(&mut self) -> Cursor {
        let mut cursor = self.find(0);
        let page = self.pager.get_page(cursor.page_num);
        let num_cells = leaf_node_num_cells(page);
        cursor.end_of_table = num_cells == 0;
        cursor
    }

    /// Returns the page number that should be allocated next.
    pub fn get_unused_page_num(&self) -> u32 {
        self.pager.num_pages
    }

    /// Initializes child pages when an internal root is being split.
    fn initialize_split_children(&mut self, left: u32, right: u32, is_internal: bool) {
        if is_internal {
            initialize_internal_node(self.pager.get_page(right));
            initialize_internal_node(self.pager.get_page(left));
        }
    }

    /// Updates copied grandchildren to point at their new left parent.
    fn reparent_left_children(&mut self, left: u32, is_internal: bool) {
        if !is_internal {
            return;
        }
        let key_count = internal_node_num_keys(self.pager.get_page(left));
        for index in 0..key_count {
            let child = internal_node_child(self.pager.get_page(left), index);
            set_node_parent(self.pager.get_page(child), left);
        }
        let right_child = internal_node_right_child(self.pager.get_page(left));
        set_node_parent(self.pager.get_page(right_child), left);
    }

    /// Promotes a split root into an internal root with two children.
    pub fn create_new_root(&mut self, right_child_page_num: u32) {
        let left_child_page_num = self.get_unused_page_num();

        // 1. Copy old root page into left child page
        let old_root_data = *self.pager.get_page(self.root_page_num);
        let root_is_internal = get_node_type(&old_root_data) == NodeType::Internal;

        self.initialize_split_children(left_child_page_num, right_child_page_num, root_is_internal);

        *self.pager.get_page(left_child_page_num) = old_root_data;
        set_node_root(self.pager.get_page(left_child_page_num), false);

        self.reparent_left_children(left_child_page_num, root_is_internal);

        let left_child_max_key = get_node_max_key(&mut self.pager, left_child_page_num);
        self.write_new_root(
            left_child_page_num,
            right_child_page_num,
            left_child_max_key,
        );
    }

    /// Writes the new root header, separator key, and child pointers.
    fn write_new_root(&mut self, left: u32, right: u32, left_max_key: u32) {
        let root = self.pager.get_page(self.root_page_num);
        initialize_internal_node(root);
        set_node_root(root, true);
        set_internal_node_num_keys(root, 1);
        set_internal_node_child(root, 0, left);
        set_internal_node_key(root, 0, left_max_key);
        set_internal_node_right_child(root, right);
        set_node_parent(self.pager.get_page(left), self.root_page_num);
        set_node_parent(self.pager.get_page(right), self.root_page_num);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::row::Row;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_table(name: &str) -> (Table, std::path::PathBuf) {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sqlite_tree_test_{name}_{}_{n}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let table = Table::open(path.to_str().unwrap()).expect("open should succeed");
        (table, path)
    }

    struct TempGuard(std::path::PathBuf);
    impl Drop for TempGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn open_initializes_a_new_file_with_an_empty_leaf_root() {
        let (mut table, path) = temp_table("open_new");
        let _guard = TempGuard(path);
        let page = table.pager.get_page(0);
        assert_eq!(get_node_type(page), NodeType::Leaf);
        assert!(is_node_root(page));
        assert_eq!(leaf_node_num_cells(page), 0);
    }

    #[test]
    fn open_does_not_reinitialize_an_existing_file() {
        let (mut table, path) = temp_table("open_existing");
        let cursor = table.find(1);
        table.leaf_node_insert(&cursor, 1, &Row::new(1, "a", "a@x.com"));
        table.pager.flush(0).unwrap();
        drop(table);

        let mut reopened = Table::open(path.to_str().unwrap()).unwrap();
        let _guard = TempGuard(path);
        assert_eq!(leaf_node_num_cells(reopened.pager.get_page(0)), 1);
    }

    #[test]
    fn find_on_empty_table_returns_start_of_leaf() {
        let (mut table, path) = temp_table("find_empty");
        let _guard = TempGuard(path);
        let cursor = table.find(5);
        assert_eq!(cursor.page_num, 0);
        assert_eq!(cursor.cell_num, 0);
    }

    #[test]
    fn start_on_empty_table_is_end_of_table() {
        let (mut table, path) = temp_table("start_empty");
        let _guard = TempGuard(path);
        let cursor = table.start();
        assert!(cursor.end_of_table);
    }

    #[test]
    fn start_on_nonempty_table_points_at_first_row() {
        let (mut table, path) = temp_table("start_nonempty");
        let _guard = TempGuard(path);
        let cursor = table.find(1);
        table.leaf_node_insert(&cursor, 1, &Row::new(1, "a", "a@x.com"));

        let cursor = table.start();
        assert!(!cursor.end_of_table);
        assert_eq!(cursor.cell_num, 0);
    }

    #[test]
    fn get_unused_page_num_matches_pager_page_count() {
        let (mut table, path) = temp_table("unused_page");
        let _guard = TempGuard(path);
        assert_eq!(table.get_unused_page_num(), table.pager.num_pages);
        table.pager.get_page(3);
        assert_eq!(table.get_unused_page_num(), table.pager.num_pages);
    }

    #[test]
    fn create_new_root_splits_a_leaf_root_into_an_internal_root() {
        let (mut table, path) = temp_table("create_new_root");
        let _guard = TempGuard(path);

        // Old (leaf) root gets some rows before the split.
        for key in [1u32, 2, 3] {
            let cursor = table.find(key);
            table.leaf_node_insert(&cursor, key, &Row::new(key, "u", "e@x.com"));
        }

        // Manually prepare a second leaf holding the "right" half, as the
        // (currently unimplemented) leaf-split logic would.
        let right_page_num = table.get_unused_page_num();
        {
            let right_page = table.pager.get_page(right_page_num);
            initialize_leaf_node(right_page);
            set_leaf_node_num_cells(right_page, 1);
            set_leaf_node_key(right_page, 0, 10);
        }

        table.create_new_root(right_page_num);

        let root = table.pager.get_page(table.root_page_num);
        assert_eq!(get_node_type(root), NodeType::Internal);
        assert!(is_node_root(root));
        assert_eq!(internal_node_num_keys(root), 1);
        assert_eq!(internal_node_right_child(root), right_page_num);

        let left_child = internal_node_child(root, 0);
        assert_eq!(internal_node_key(root, 0), 3); // old root's max key
        assert_eq!(get_node_type(table.pager.get_page(left_child)), NodeType::Leaf);
        assert!(!is_node_root(table.pager.get_page(left_child)));
        assert_eq!(leaf_node_num_cells(table.pager.get_page(left_child)), 3);
    }
}
