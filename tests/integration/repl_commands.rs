//! End-to-end tests for SQL and meta-commands entered through the REPL.

use crate::common::TempDbFile;
use sqlite::btree::tree::Table;
use sqlite::repl::{MetaCommandResult, do_meta_command};
use sqlite::sql::executor::{ExecuteError, execute_insert, execute_select};
use sqlite::sql::parser::{PrepareError, prepare_statement};
use sqlite::sql::statement::Statement;

/// Mirrors what the REPL's private `handle_statement` does: parse one line
/// and, for an Insert, execute it against the table.
fn run_line(input: &str, table: &mut Table) -> Result<Option<ExecuteError>, PrepareError> {
    match prepare_statement(input)? {
        Statement::Insert(row) => Ok(execute_insert(row, table).err()),
        Statement::Select => {
            execute_select(table);
            Ok(None)
        }
    }
}

#[test]
fn dot_exit_requests_exit() {
    let file = TempDbFile::new("exit");
    let mut table = Table::open(file.path()).unwrap();
    assert_eq!(do_meta_command(".exit", &mut table), MetaCommandResult::Exit);
}

#[test]
fn unknown_dot_command_is_unrecognized() {
    let file = TempDbFile::new("unknown");
    let mut table = Table::open(file.path()).unwrap();
    assert_eq!(
        do_meta_command(".frobnicate", &mut table),
        MetaCommandResult::UnrecognizedCommand(".frobnicate".to_string())
    );
}

#[test]
fn insert_then_select_round_trips_through_the_repl_pipeline() {
    let file = TempDbFile::new("insert_select");
    let mut table = Table::open(file.path()).unwrap();

    assert_eq!(
        run_line("insert 1 alice alice@example.com", &mut table),
        Ok(None)
    );
    assert_eq!(run_line("select", &mut table), Ok(None));
}

#[test]
fn duplicate_insert_through_the_repl_pipeline_is_reported() {
    let file = TempDbFile::new("dup_insert");
    let mut table = Table::open(file.path()).unwrap();

    run_line("insert 1 alice alice@example.com", &mut table).unwrap();
    assert_eq!(
        run_line("insert 1 alice alice@example.com", &mut table),
        Ok(Some(ExecuteError::DuplicateKey))
    );
}

#[test]
fn malformed_statement_through_the_repl_pipeline_is_a_syntax_error() {
    let file = TempDbFile::new("malformed");
    let mut table = Table::open(file.path()).unwrap();
    assert_eq!(
        run_line("insert not_an_id alice alice@example.com", &mut table),
        Err(PrepareError::SyntaxError)
    );
}

#[test]
fn dot_constants_and_dot_btree_succeed_on_a_populated_table() {
    let file = TempDbFile::new("constants_btree");
    let mut table = Table::open(file.path()).unwrap();
    run_line("insert 1 alice alice@example.com", &mut table).unwrap();

    assert_eq!(
        do_meta_command(".constants", &mut table),
        MetaCommandResult::Success
    );
    assert_eq!(
        do_meta_command(".btree", &mut table),
        MetaCommandResult::Success
    );
}
