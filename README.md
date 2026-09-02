# -SQLite

A disk-backed SQLite-style B-Tree storage engine written in Rust.

## Features

- Fixed-size page file format (4 KiB pages)
- Persistent metadata page with root pointer
- On-disk leaf and internal B-Tree node serialization
- Ordered `u64 -> u64` insert and lookup operations
- Automatic leaf/internal node splitting during growth

## Quick start

```rust
use sqlite_btree_engine::BTreeEngine;

# fn main() -> std::io::Result<()> {
let mut db = BTreeEngine::open("example.db")?;
db.insert(42, 9001)?;
assert_eq!(db.get(42)?, Some(9001));
db.flush()?;
# Ok(()) }
```

## Validate

```bash
cargo test
```
