use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

const PAGE_SIZE: usize = 4096;
const META_MAGIC: &[u8; 8] = b"SQLBTREE";
const META_PAGE_ID: u32 = 0;
const INITIAL_ROOT_PAGE_ID: u32 = 1;

const NODE_LEAF: u8 = 1;
const NODE_INTERNAL: u8 = 2;

const NODE_HEADER_SIZE: usize = 8;
const LEAF_CELL_SIZE: usize = 16;
const INTERNAL_CELL_SIZE: usize = 12;

const LEAF_CAPACITY: usize = (PAGE_SIZE - NODE_HEADER_SIZE) / LEAF_CELL_SIZE;
const INTERNAL_CAPACITY: usize = (PAGE_SIZE - NODE_HEADER_SIZE) / INTERNAL_CELL_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LeafEntry {
    key: u64,
    value: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LeafNode {
    entries: Vec<LeafEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InternalNode {
    children: Vec<u32>,
    max_keys: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    Leaf(LeafNode),
    Internal(InternalNode),
}

struct Pager {
    file: File,
    page_count: u32,
}

impl Pager {
    fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        let file_len = file.metadata()?.len();
        if file_len % PAGE_SIZE as u64 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "database file is not page-aligned",
            ));
        }
        Ok(Self {
            file,
            page_count: (file_len / PAGE_SIZE as u64) as u32,
        })
    }

    fn read_page(&mut self, page_id: u32) -> io::Result<[u8; PAGE_SIZE]> {
        let mut buf = [0_u8; PAGE_SIZE];
        if page_id >= self.page_count {
            return Ok(buf);
        }
        self.file
            .seek(SeekFrom::Start(page_id as u64 * PAGE_SIZE as u64))?;
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn write_page(&mut self, page_id: u32, page: &[u8; PAGE_SIZE]) -> io::Result<()> {
        self.file
            .seek(SeekFrom::Start(page_id as u64 * PAGE_SIZE as u64))?;
        self.file.write_all(page)?;
        if page_id >= self.page_count {
            self.page_count = page_id + 1;
        }
        Ok(())
    }

    fn allocate_page(&mut self) -> io::Result<u32> {
        let page_id = self.page_count;
        self.write_page(page_id, &[0_u8; PAGE_SIZE])?;
        Ok(page_id)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.sync_all()
    }
}

/// Disk-backed SQLite-style B-Tree storage engine for `u64` keys and values.
pub struct BTreeEngine {
    pager: Pager,
    root_page: u32,
}

impl BTreeEngine {
    /// Open or create a database file.
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut pager = Pager::open(path.as_ref())?;

        if pager.page_count == 0 {
            pager.allocate_page()?; // metadata page
            pager.allocate_page()?; // initial root page

            let root = Node::Leaf(LeafNode {
                entries: Vec::new(),
            });
            write_node(&mut pager, INITIAL_ROOT_PAGE_ID, &root)?;
            write_metadata(&mut pager, INITIAL_ROOT_PAGE_ID)?;
            pager.flush()?;
        }

        let root_page = read_metadata(&mut pager)?;

        Ok(Self { pager, root_page })
    }

    /// Insert or overwrite a key/value pair.
    pub fn insert(&mut self, key: u64, value: u64) -> io::Result<()> {
        let maybe_right = self.insert_recursive(self.root_page, key, value)?;
        if let Some(right_page) = maybe_right {
            let new_root = self.pager.allocate_page()?;
            let left_max = self.max_key_of_page(self.root_page)?;
            let root = Node::Internal(InternalNode {
                children: vec![self.root_page, right_page],
                max_keys: vec![left_max],
            });
            write_node(&mut self.pager, new_root, &root)?;
            self.root_page = new_root;
            write_metadata(&mut self.pager, self.root_page)?;
        }
        Ok(())
    }

    /// Retrieve a value by key.
    pub fn get(&mut self, key: u64) -> io::Result<Option<u64>> {
        self.get_recursive(self.root_page, key)
    }

    /// Flush all pending writes to disk.
    pub fn flush(&mut self) -> io::Result<()> {
        self.pager.flush()
    }

    fn get_recursive(&mut self, page_id: u32, key: u64) -> io::Result<Option<u64>> {
        match read_node(&mut self.pager, page_id)? {
            Node::Leaf(leaf) => Ok(leaf
                .entries
                .binary_search_by_key(&key, |entry| entry.key)
                .ok()
                .map(|index| leaf.entries[index].value)),
            Node::Internal(internal) => {
                let idx = internal
                    .max_keys
                    .iter()
                    .position(|max_key| key <= *max_key)
                    .unwrap_or(internal.max_keys.len());
                self.get_recursive(internal.children[idx], key)
            }
        }
    }

    fn insert_recursive(&mut self, page_id: u32, key: u64, value: u64) -> io::Result<Option<u32>> {
        match read_node(&mut self.pager, page_id)? {
            Node::Leaf(mut leaf) => {
                match leaf.entries.binary_search_by_key(&key, |entry| entry.key) {
                    Ok(index) => leaf.entries[index].value = value,
                    Err(index) => leaf.entries.insert(index, LeafEntry { key, value }),
                }

                if leaf.entries.len() <= LEAF_CAPACITY {
                    write_node(&mut self.pager, page_id, &Node::Leaf(leaf))?;
                    return Ok(None);
                }

                let split_index = leaf.entries.len() / 2;
                let right_entries = leaf.entries.split_off(split_index);
                let right_page = self.pager.allocate_page()?;

                write_node(&mut self.pager, page_id, &Node::Leaf(leaf))?;
                write_node(
                    &mut self.pager,
                    right_page,
                    &Node::Leaf(LeafNode {
                        entries: right_entries,
                    }),
                )?;

                Ok(Some(right_page))
            }
            Node::Internal(mut internal) => {
                let target_idx = internal
                    .max_keys
                    .iter()
                    .position(|max_key| key <= *max_key)
                    .unwrap_or(internal.max_keys.len());
                let target_child = internal.children[target_idx];

                if let Some(new_right_child) = self.insert_recursive(target_child, key, value)? {
                    internal.children.insert(target_idx + 1, new_right_child);
                }

                internal.max_keys = self.compute_max_keys(&internal.children)?;

                if internal.max_keys.len() <= INTERNAL_CAPACITY {
                    write_node(&mut self.pager, page_id, &Node::Internal(internal))?;
                    return Ok(None);
                }

                let split_child_index = internal.children.len() / 2;
                let right_children = internal.children.split_off(split_child_index);
                internal.max_keys = self.compute_max_keys(&internal.children)?;

                let right_max_keys = self.compute_max_keys(&right_children)?;
                let right_page = self.pager.allocate_page()?;

                write_node(&mut self.pager, page_id, &Node::Internal(internal))?;
                write_node(
                    &mut self.pager,
                    right_page,
                    &Node::Internal(InternalNode {
                        children: right_children,
                        max_keys: right_max_keys,
                    }),
                )?;

                Ok(Some(right_page))
            }
        }
    }

    fn compute_max_keys(&mut self, children: &[u32]) -> io::Result<Vec<u64>> {
        if children.len() <= 1 {
            return Ok(Vec::new());
        }

        let mut max_keys = Vec::with_capacity(children.len() - 1);
        for child in &children[..children.len() - 1] {
            max_keys.push(self.max_key_of_page(*child)?);
        }
        Ok(max_keys)
    }

    fn max_key_of_page(&mut self, page_id: u32) -> io::Result<u64> {
        match read_node(&mut self.pager, page_id)? {
            Node::Leaf(leaf) => leaf.entries.last().map(|entry| entry.key).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "leaf node unexpectedly empty")
            }),
            Node::Internal(internal) => {
                let last = *internal.children.last().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "internal node has no children")
                })?;
                self.max_key_of_page(last)
            }
        }
    }
}

fn read_metadata(pager: &mut Pager) -> io::Result<u32> {
    let page = pager.read_page(META_PAGE_ID)?;
    if &page[0..8] != META_MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "database header magic mismatch",
        ));
    }
    Ok(u32::from_le_bytes([page[8], page[9], page[10], page[11]]))
}

fn write_metadata(pager: &mut Pager, root_page: u32) -> io::Result<()> {
    let mut page = [0_u8; PAGE_SIZE];
    page[0..8].copy_from_slice(META_MAGIC);
    page[8..12].copy_from_slice(&root_page.to_le_bytes());
    pager.write_page(META_PAGE_ID, &page)
}

fn read_node(pager: &mut Pager, page_id: u32) -> io::Result<Node> {
    let page = pager.read_page(page_id)?;
    let kind = page[0];
    let count = u16::from_le_bytes([page[1], page[2]]) as usize;

    match kind {
        NODE_LEAF => {
            if count > LEAF_CAPACITY {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "leaf node capacity exceeded on disk",
                ));
            }
            let mut entries = Vec::with_capacity(count);
            for i in 0..count {
                let start = NODE_HEADER_SIZE + i * LEAF_CELL_SIZE;
                let key = u64::from_le_bytes(page[start..start + 8].try_into().unwrap());
                let value = u64::from_le_bytes(page[start + 8..start + 16].try_into().unwrap());
                entries.push(LeafEntry { key, value });
            }
            Ok(Node::Leaf(LeafNode { entries }))
        }
        NODE_INTERNAL => {
            if count > INTERNAL_CAPACITY {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "internal node capacity exceeded on disk",
                ));
            }

            let right_child = u32::from_le_bytes([page[4], page[5], page[6], page[7]]);
            let mut children = Vec::with_capacity(count + 1);
            let mut max_keys = Vec::with_capacity(count);

            for i in 0..count {
                let start = NODE_HEADER_SIZE + i * INTERNAL_CELL_SIZE;
                let child = u32::from_le_bytes(page[start..start + 4].try_into().unwrap());
                let max_key = u64::from_le_bytes(page[start + 4..start + 12].try_into().unwrap());
                children.push(child);
                max_keys.push(max_key);
            }
            children.push(right_child);

            Ok(Node::Internal(InternalNode { children, max_keys }))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unknown node type",
        )),
    }
}

fn write_node(pager: &mut Pager, page_id: u32, node: &Node) -> io::Result<()> {
    let mut page = [0_u8; PAGE_SIZE];

    match node {
        Node::Leaf(leaf) => {
            if leaf.entries.len() > LEAF_CAPACITY {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "leaf node capacity exceeded",
                ));
            }
            page[0] = NODE_LEAF;
            page[1..3].copy_from_slice(&(leaf.entries.len() as u16).to_le_bytes());

            for (i, entry) in leaf.entries.iter().enumerate() {
                let start = NODE_HEADER_SIZE + i * LEAF_CELL_SIZE;
                page[start..start + 8].copy_from_slice(&entry.key.to_le_bytes());
                page[start + 8..start + 16].copy_from_slice(&entry.value.to_le_bytes());
            }
        }
        Node::Internal(internal) => {
            if internal.children.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "internal node must have at least one child",
                ));
            }
            if internal.max_keys.len() + 1 != internal.children.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "internal node key/child count mismatch",
                ));
            }
            if internal.max_keys.len() > INTERNAL_CAPACITY {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "internal node capacity exceeded",
                ));
            }

            page[0] = NODE_INTERNAL;
            page[1..3].copy_from_slice(&(internal.max_keys.len() as u16).to_le_bytes());

            if let Some(right_child) = internal.children.last() {
                page[4..8].copy_from_slice(&right_child.to_le_bytes());
            }

            for (i, (&child, &max_key)) in internal
                .children
                .iter()
                .zip(internal.max_keys.iter())
                .enumerate()
            {
                let start = NODE_HEADER_SIZE + i * INTERNAL_CELL_SIZE;
                page[start..start + 4].copy_from_slice(&child.to_le_bytes());
                page[start + 4..start + 12].copy_from_slice(&max_key.to_le_bytes());
            }
        }
    }

    pager.write_page(page_id, &page)
}

#[cfg(test)]
mod tests {
    use super::BTreeEngine;
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn db_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("sqlite-btree-engine-{name}-{nanos}.db"))
    }

    #[test]
    fn round_trip_insert_and_get() -> io::Result<()> {
        let path = db_path("round-trip");
        {
            let mut db = BTreeEngine::open(&path)?;
            db.insert(1, 10)?;
            db.insert(2, 20)?;
            db.insert(2, 99)?;
            db.flush()?;
        }

        {
            let mut db = BTreeEngine::open(&path)?;
            assert_eq!(db.get(1)?, Some(10));
            assert_eq!(db.get(2)?, Some(99));
            assert_eq!(db.get(3)?, None);
        }

        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn handles_leaf_and_internal_splits() -> io::Result<()> {
        let path = db_path("splits");
        {
            let mut db = BTreeEngine::open(&path)?;
            for i in 0..2_000_u64 {
                db.insert(i, i * 7)?;
            }
            db.flush()?;
        }

        {
            let mut db = BTreeEngine::open(&path)?;
            for key in [0_u64, 1, 500, 999, 1_999] {
                assert_eq!(db.get(key)?, Some(key * 7));
            }
            assert_eq!(db.get(2_000)?, None);
        }

        fs::remove_file(path)?;
        Ok(())
    }
}
