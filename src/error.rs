//! Errors shared by more than one database layer.
//!
//! Parser-only errors stay in `sql/parser.rs`, and execution-only errors stay
//! in `sql/executor.rs`. This file contains failures that can happen in the
//! pager, storage, or B-tree code.

use std::fmt;

#[derive(Debug)]
pub enum DatabaseError {
    /// The operating system could not read or write the database file.
    Io(std::io::Error),

    /// A page number is outside the configured page cache.
    PageOutOfBounds { page_number: u32, maximum: usize },

    /// The database file or page bytes are not valid.
    CorruptDatabase(String),

    /// A caller supplied an invalid database value.
    InvalidInput(String),
}

impl DatabaseError {
    pub fn corrupt(message: impl Into<String>) -> Self {
        Self::CorruptDatabase(message.into())
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "database file error: {error}"),
            Self::PageOutOfBounds {
                page_number,
                maximum,
            } => write!(
                formatter,
                "page {page_number} is outside the maximum of {maximum} pages"
            ),
            Self::CorruptDatabase(message) => write!(formatter, "corrupt database: {message}"),
            Self::InvalidInput(message) => write!(formatter, "invalid database input: {message}"),
        }
    }
}

impl std::error::Error for DatabaseError {}

impl From<std::io::Error> for DatabaseError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_has_readable_message() {
        let error = DatabaseError::invalid_input("ID cannot be negative");
        assert_eq!(
            error.to_string(),
            "invalid database input: ID cannot be negative"
        );
    }
}
