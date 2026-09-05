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

    #[test]
    fn corrupt_database_has_readable_message() {
        let error = DatabaseError::corrupt("bad header");
        assert_eq!(error.to_string(), "corrupt database: bad header");
    }

    #[test]
    fn page_out_of_bounds_has_readable_message() {
        let error = DatabaseError::PageOutOfBounds {
            page_number: 5,
            maximum: 4,
        };
        assert_eq!(
            error.to_string(),
            "page 5 is outside the maximum of 4 pages"
        );
    }

    #[test]
    fn io_error_is_wrapped_and_displayed() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let error: DatabaseError = io_error.into();
        assert!(error.to_string().starts_with("database file error:"));
    }
}
