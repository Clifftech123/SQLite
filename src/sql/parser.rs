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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_valid_insert() {
        let statement = prepare_statement("insert 1 alice alice@example.com").unwrap();
        match statement {
            Statement::Insert(row) => {
                assert_eq!(row.id, 1);
                assert_eq!(row.username, "alice");
                assert_eq!(row.email, "alice@example.com");
            }
            Statement::Select => panic!("expected an Insert statement"),
        }
    }

    #[test]
    fn parses_a_select() {
        assert!(matches!(prepare_statement("select"), Ok(Statement::Select)));
    }

    #[test]
    fn empty_input_is_a_syntax_error() {
        assert_eq!(prepare_statement(""), Err(PrepareError::SyntaxError));
        assert_eq!(prepare_statement("   "), Err(PrepareError::SyntaxError));
    }

    #[test]
    fn unrecognized_keyword_is_reported() {
        assert_eq!(
            prepare_statement("delete 1 a b"),
            Err(PrepareError::UnrecognizedStatement("delete".to_string()))
        );
    }

    #[test]
    fn insert_with_wrong_argument_count_is_a_syntax_error() {
        assert_eq!(
            prepare_statement("insert 1 alice"),
            Err(PrepareError::SyntaxError)
        );
        assert_eq!(
            prepare_statement("insert 1 alice a@x.com extra"),
            Err(PrepareError::SyntaxError)
        );
    }

    #[test]
    fn insert_with_non_numeric_id_is_a_syntax_error() {
        assert_eq!(
            prepare_statement("insert abc alice a@x.com"),
            Err(PrepareError::SyntaxError)
        );
    }

    #[test]
    fn insert_with_negative_id_is_reported() {
        assert_eq!(
            prepare_statement("insert -1 alice a@x.com"),
            Err(PrepareError::NegativeId)
        );
    }

    #[test]
    fn insert_with_id_beyond_u32_is_a_syntax_error() {
        let too_big = (u32::MAX as i64) + 1;
        let input = format!("insert {too_big} alice a@x.com");
        assert_eq!(prepare_statement(&input), Err(PrepareError::SyntaxError));
    }

    #[test]
    fn insert_with_username_too_long_is_reported() {
        let long_username = "a".repeat(COLUMN_USERNAME_SIZE + 1);
        let input = format!("insert 1 {long_username} a@x.com");
        assert_eq!(prepare_statement(&input), Err(PrepareError::StringTooLong));
    }

    #[test]
    fn insert_with_email_too_long_is_reported() {
        let long_email = "a".repeat(COLUMN_EMAIL_SIZE + 1);
        let input = format!("insert 1 alice {long_email}");
        assert_eq!(prepare_statement(&input), Err(PrepareError::StringTooLong));
    }

    #[test]
    fn insert_accepts_text_at_the_exact_size_limit() {
        let username = "a".repeat(COLUMN_USERNAME_SIZE);
        let email = "b".repeat(COLUMN_EMAIL_SIZE);
        let input = format!("insert 1 {username} {email}");
        assert!(prepare_statement(&input).is_ok());
    }
}
