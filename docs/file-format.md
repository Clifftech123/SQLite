# On-disk file format

This document describes the exact byte layout the pager, row, and B-tree
code use to store data on disk. It's a reference for the constants defined
in `src/config.rs`, `src/row.rs`, `src/btree/node.rs`, `src/btree/leaf.rs`,
and `src/btree/internal.rs` — read those files for the authoritative values.

## Pages

The database file is a flat sequence of fixed-size pages (`PAGE_SIZE` =
4096 bytes, see `src/config.rs`). Page `N` lives at byte offset
`N * PAGE_SIZE` (`storage::page::file_offset`). Every page is one of two
kinds, identified by the first byte of its header: a **leaf node** or an
**internal node**.

## Row format (`src/row.rs`)

Each row is serialized to a fixed-width 293-byte record:

| Field      | Offset | Size (bytes) |
|------------|-------:|-------------:|
| `id`       | 0      | 4             |
| `username` | 4      | 33            |
| `email`    | 37     | 256           |

`username`/`email` are stored as UTF-8 text followed by zero-padding out to
the field width; the padding also marks where the text ends when reading it
back (`row::read_fixed_text`).

## Common node header (`src/btree/node.rs`)

Every page (leaf or internal) starts with a 6-byte common header:

| Field            | Offset | Size (bytes) |
|------------------|-------:|-------------:|
| node type        | 0      | 1             |
| is-root flag     | 1      | 1             |
| parent page num  | 2      | 4             |

## Leaf node layout (`src/btree/leaf.rs`)

```
[ common header (6) | num_cells (4) | next_leaf (4) | cell 0 | cell 1 | ... ]
```

- `num_cells`: how many key/row cells are currently stored on this page.
- `next_leaf`: page number of the next leaf in key order (0 = no next leaf),
  used for full-table scans without walking back up the tree.
- Each **cell** is `key (4 bytes) + serialized Row (293 bytes)` = 297 bytes.

With a 4096-byte page and a 14-byte leaf header, a leaf holds at most 13
rows (`LEAF_NODE_MAX_CELLS`).

## Internal node layout (`src/btree/internal.rs`)

```
[ common header (6) | num_keys (4) | right_child (4) | cell 0 | cell 1 | ... ]
```

- `num_keys`: number of separator keys stored on this page.
- `right_child`: page number of the subtree holding every key greater than
  all stored separator keys.
- Each **cell** is `child page num (4 bytes) + separator key (4 bytes)`.
  Cell `i`'s key is the maximum key stored in child `i`'s subtree; a search
  for `key` walks cells left-to-right and descends into the first child
  whose key is `>= key`, falling through to `right_child` otherwise
  (`internal_node_find_child`).

`INTERNAL_NODE_MAX_KEYS` is fixed at 3, intentionally small so a tree with
only a few dozen rows already has more than one level while testing.

## Known gap: node splitting

`Table::create_new_root` and the `LEAF_NODE_LEFT/RIGHT_SPLIT_COUNT`
constants exist to support splitting a full leaf into two nodes and
promoting a new internal root, but nothing on the insert path calls them
yet. In the current code, a table is limited to one leaf page — at most
`LEAF_NODE_MAX_CELLS` (13) rows — before an insert panics with
`"leaf page is full"`. See `tests/integration/btree_growth.rs` for tests
that document this boundary.
