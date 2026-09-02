# Building a SQLite-Like Database in Rust

This project is a lightweight, disk-backed SQL database engine built from
scratch in Rust. It provides an interactive command-line interface for running
SQL statements and uses a B-tree storage engine to organize and retrieve rows
efficiently.

The database stores its data in fixed-size pages, manages those pages through a
disk pager, and serializes rows into a stable binary format. Its SQL layer
parses commands and executes them against the B-tree-backed storage layer.

## Features

- Interactive database command-line interface
- Basic `INSERT` and `SELECT` SQL statements
- Persistent, file-backed data storage
- Fixed-size database pages and page caching
- Binary row serialization and deserialization
- B-tree leaf and internal nodes
- Efficient key lookup and sequential table scans
- Automatic node splitting as the database grows
- Duplicate-key and input validation

## Architecture

The project is divided into three main layers:

- **SQL layer:** Parses SQL input and executes statements.
- **B-tree layer:** Organizes rows, performs searches, and manages node splits.
- **Storage layer:** Reads and writes fixed-size pages in the database file.

## Project structure

```text
SQLite/
|-- src/
|   |-- main.rs             # Starts the program
|   |-- lib.rs              # Connects the project modules
|   |-- config.rs           # Global sizes and limits
|   |-- row.rs              # Row type and byte serialization
|   |-- error.rs            # Errors shared across layers
|   |-- repl.rs             # Prompt and meta-commands
|   |-- storage/
|   |   |-- mod.rs          # Storage module declarations
|   |   |-- page.rs         # Fixed-size database pages
|   |   `-- pager.rs        # Database file and page cache
|   |-- btree/
|   |   |-- mod.rs          # B-tree module declarations
|   |   |-- node.rs         # NodeType and shared node headers
|   |   |-- leaf.rs         # Leaf nodes and leaf splitting
|   |   |-- internal.rs     # Internal nodes and splitting
|   |   |-- cursor.rs       # Lookup and scan cursor
|   |   `-- tree.rs         # Table and tree coordination
|   `-- sql/
|       |-- mod.rs          # SQL module declarations
|       |-- statement.rs    # Statement enum
|       |-- parser.rs       # SQL parsing and PrepareError
|       `-- executor.rs     # SQL execution and ExecuteError
|-- tests/
|   `-- integration/
|       |-- repl_commands.rs # Command-line behavior
|       |-- persistence.rs   # Saving and reopening data
|       `-- btree_growth.rs  # B-tree split behavior
|-- docs/                    # Design and file-format documents
|-- examples/                # Database usage examples
|-- data/                    # Local database files
|-- Cargo.toml
`-- README.md
```

## Type locations

| Type | File |
|---|---|
| `Row` | `src/row.rs` |
| `Pager` | `src/storage/pager.rs` |
| `NodeType` | `src/btree/node.rs` |
| `Cursor` | `src/btree/cursor.rs` |
| `Table` | `src/btree/tree.rs` |
| `Statement` | `src/sql/statement.rs` |
| `PrepareError` | `src/sql/parser.rs` |
| `ExecuteError` | `src/sql/executor.rs` |
| `MetaCommandResult` | `src/repl.rs` |
