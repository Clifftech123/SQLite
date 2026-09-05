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
