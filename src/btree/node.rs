//! Shared B-tree node header fields and helpers.

use crate::btree::internal::internal_node_right_child;
use crate::btree::leaf::{leaf_node_key, leaf_node_num_cells};
use crate::config::PAGE_SIZE;
use crate::storage::pager::Pager;

pub const NODE_TYPE_SIZE: usize = 1;
pub const NODE_TYPE_OFFSET: usize = 0;
pub const IS_ROOT_SIZE: usize = 1;
pub const IS_ROOT_OFFSET: usize = NODE_TYPE_SIZE;
pub const PARENT_POINTER_SIZE: usize = 4;
pub const PARENT_POINTER_OFFSET: usize = IS_ROOT_OFFSET + IS_ROOT_SIZE;
pub const COMMON_NODE_HEADER_SIZE: usize = NODE_TYPE_SIZE + IS_ROOT_SIZE + PARENT_POINTER_SIZE;

// Enums & Structs
/// Identifies whether a page is a leaf or internal node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Internal = 0,
    Leaf = 1,
}

impl From<u8> for NodeType {
    fn from(val: u8) -> Self {
        match val {
            0 => NodeType::Internal,
            1 => NodeType::Leaf,
            _ => panic!("Unknown node type is byte: {}", val),
        }
    }
}

/// Reads the node kind byte from a page.
pub fn get_node_type(page: &[u8; PAGE_SIZE]) -> NodeType {
    NodeType::from(page[NODE_TYPE_OFFSET])
}

/// Writes the node kind byte to a page.
pub fn set_node_type(page: &mut [u8; PAGE_SIZE], node_type: NodeType) {
    page[NODE_TYPE_OFFSET] = node_type as u8;
}

/// Returns whether this node is the tree root.
pub fn is_node_root(page: &[u8; PAGE_SIZE]) -> bool {
    page[IS_ROOT_OFFSET] != 0
}

/// Sets or clears the root flag.
pub fn set_node_root(page: &mut [u8; PAGE_SIZE], is_root: bool) {
    page[IS_ROOT_OFFSET] = if is_root { 1 } else { 0 };
}

/// Reads the parent page number.
pub fn get_node_parent(page: &[u8; PAGE_SIZE]) -> u32 {
    u32::from_le_bytes(
        page[PARENT_POINTER_OFFSET..PARENT_POINTER_OFFSET + PARENT_POINTER_SIZE]
            .try_into()
            .unwrap(),
    )
}

/// Writes the parent page number.
pub fn set_node_parent(page: &mut [u8; PAGE_SIZE], parent: u32) {
    page[PARENT_POINTER_OFFSET..PARENT_POINTER_OFFSET + PARENT_POINTER_SIZE]
        .copy_from_slice(&parent.to_le_bytes());
}

/// Finds the greatest key stored below a node.
pub fn get_node_max_key(pager: &mut Pager, page_num: u32) -> u32 {
    match get_node_type(pager.get_page(page_num)) {
        NodeType::Leaf => {
            let page = pager.get_page(page_num);
            let count = leaf_node_num_cells(page);
            assert!(count > 0, "cannot get a key from an empty leaf");
            leaf_node_key(page, count - 1)
        }
        NodeType::Internal => {
            let right_child = internal_node_right_child(pager.get_page(page_num));
            get_node_max_key(pager, right_child)
        }
    }
}
