//! Converts user-entered SQL text into validated statements.

use crate::config::{COLUMN_EMAIL_SIZE, COLUMN_USERNAME_SIZE};
use crate::row::Row;
use crate::sql::statement::Statement;

/// Reasons that SQL text cannot be prepared for execution.
#[derive(Debug, PartialEq, Eq)]
pub enum PrepareError {
    NegativeId,
    StringTooLong,
    SyntaxError,
    UnrecognizedStatement(String),
}

/// Parses one line of SQL into a validated statement.
pub fn prepare_statement(input: &str) -> Result<Statement, PrepareError> {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    if tokens.is_empty() {
        return Err(PrepareError::SyntaxError);
    }

    match tokens[0] {
        "insert" => parse_insert(&tokens),
        "select" => Ok(Statement::Select),
        _ => Err(PrepareError::UnrecognizedStatement(tokens[0].to_string())),
    }
}

/// Parses and validates the four tokens in an INSERT command.
fn parse_insert(tokens: &[&str]) -> Result<Statement, PrepareError> {
    if tokens.len() != 4 {
        return Err(PrepareError::SyntaxError);
    }

    let id = parse_id(tokens[1])?;
    validate_text_lengths(tokens[2], tokens[3])?;
    Ok(Statement::Insert(Row::new(id, tokens[2], tokens[3])))
}

/// Parses a non-negative row ID that fits in `u32`.
fn parse_id(text: &str) -> Result<u32, PrepareError> {
    let id: i64 = text.parse().map_err(|_| PrepareError::SyntaxError)?;
    if id < 0 {
        return Err(PrepareError::NegativeId);
    }
    u32::try_from(id).map_err(|_| PrepareError::SyntaxError)
}

/// Ensures text fields fit their fixed-width disk slots.
fn validate_text_lengths(username: &str, email: &str) -> Result<(), PrepareError> {
    if username.len() > COLUMN_USERNAME_SIZE || email.len() > COLUMN_EMAIL_SIZE {
        Err(PrepareError::StringTooLong)
    } else {
        Ok(())
    }
}
