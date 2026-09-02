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
