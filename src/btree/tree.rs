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
