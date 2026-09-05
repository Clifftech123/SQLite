//! Structured commands produced by the SQL parser.

use crate::row::Row;

/// SQL operations currently supported by the database.
#[derive(Debug, PartialEq)]
pub enum Statement {
    Insert(Row),
    Select,
}
