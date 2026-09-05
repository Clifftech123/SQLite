//! Interactive command loop and diagnostic output.

use crate::btree::internal::*;
use crate::btree::leaf::*;
use crate::btree::node::*;
use crate::btree::tree::Table;
use crate::row::ROW_SIZE;
use crate::sql::executor::{ExecuteError, execute_insert, execute_select};
use crate::sql::parser::{PrepareError, prepare_statement};
use crate::sql::statement::Statement;
use crate::storage::pager::Pager;
use std::io::{self, BufRead, Write};

/// Result of processing a dot-prefixed meta-command.
#[derive(Debug, PartialEq, Eq)]
pub enum MetaCommandResult {
    Success,
    Exit,
    UnrecognizedCommand(String),
}

/// Runs the interactive read-evaluate-print loop for an open table.
pub fn run(table: &mut Table) {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut stdout = io::stdout();
    loop {
        if !read_command(&mut reader, &mut stdout, table) {
            break;
        }
    }
}

/// Reads one command and returns whether the REPL should continue.
fn read_command<R: BufRead, W: Write>(reader: &mut R, writer: &mut W, table: &mut Table) -> bool {
    print_prompt(writer);
    let mut input = String::new();
    if reader.read_line(&mut input).unwrap_or(0) == 0 {
        return false;
    }
    let input = input.trim();
    if input.is_empty() {
        return true;
    }
    if input.starts_with('.') {
        return handle_meta_command(input, table);
    }
    handle_statement(input, table);
    true
}

/// Displays and flushes the interactive prompt.
fn print_prompt(writer: &mut impl Write) {
    print!("db > ");
    let _ = writer.flush();
}

/// Handles a meta-command and reports whether execution should continue.
fn handle_meta_command(input: &str, table: &mut Table) -> bool {
    match do_meta_command(input, table) {
        MetaCommandResult::Exit => false,
        MetaCommandResult::Success => true,
        MetaCommandResult::UnrecognizedCommand(command) => {
            println!("Unrecognized command '{command}'");
            true
        }
    }
}

/// Parses, executes, and reports one SQL statement.
fn handle_statement(input: &str, table: &mut Table) {
    match prepare_statement(input) {
        Ok(Statement::Insert(row)) => match execute_insert(row, table) {
            Ok(()) => println!("Executed."),
            Err(ExecuteError::DuplicateKey) => println!("Error: Duplicate key."),
            Err(ExecuteError::TableFull) => println!("Error: Table full."),
        },
        Ok(Statement::Select) => {
            execute_select(table);
            println!("Executed.");
        }
        Err(error) => print_prepare_error(error),
    }
}

/// Converts a parser error into a friendly message.
fn print_prepare_error(error: PrepareError) {
    match error {
        PrepareError::NegativeId => println!("ID must be positive."),
        PrepareError::StringTooLong => println!("String is too long."),
        PrepareError::SyntaxError => println!("Syntax error. Could not parse statement."),
        PrepareError::UnrecognizedStatement(keyword) => {
            println!("Unrecognized keyword at start of '{keyword}'.")
        }
    }
}

/// Prints the important on-disk layout sizes.
pub fn print_constants() {
    println!("ROW_SIZE: {}", ROW_SIZE);
    println!("COMMON_NODE_HEADER_SIZE: {}", COMMON_NODE_HEADER_SIZE);
    println!("LEAF_NODE_HEADER_SIZE: {}", LEAF_NODE_HEADER_SIZE);
    println!("LEAF_NODE_CELL_SIZE: {}", LEAF_NODE_CELL_SIZE);
    println!("LEAF_NODE_SPACE_FOR_CELLS: {}", LEAF_NODE_SPACE_FOR_CELLS);
    println!("LEAF_NODE_MAX_CELLS: {}", LEAF_NODE_MAX_CELLS);
}

/// Prints two spaces per tree depth level.
pub fn indent(level: u32) {
    for _ in 0..level {
        print!("  ");
    }
}

/// Recursively prints the B-tree rooted at the requested page.
pub fn print_tree(pager: &mut Pager, page_num: u32, indentation_level: u32) {
    match get_node_type(pager.get_page(page_num)) {
        NodeType::Leaf => print_leaf(pager, page_num, indentation_level),
        NodeType::Internal => print_internal(pager, page_num, indentation_level),
    }
}

/// Prints one leaf and all keys stored in it.
fn print_leaf(pager: &mut Pager, page_num: u32, level: u32) {
    let page = pager.get_page(page_num);
    let key_count = leaf_node_num_cells(page);
    indent(level);
    println!("- leaf (size {key_count})");
    for index in 0..key_count {
        indent(level + 1);
        println!("- {}", leaf_node_key(page, index));
    }
}

/// Prints an internal node and recursively prints each child subtree.
fn print_internal(pager: &mut Pager, page_num: u32, level: u32) {
    let key_count = internal_node_num_keys(pager.get_page(page_num));
    indent(level);
    println!("- internal (size {key_count})");
    for index in 0..key_count {
        print_internal_child(pager, page_num, index, level);
    }
    if key_count > 0 {
        let right_child = internal_node_right_child(pager.get_page(page_num));
        print_tree(pager, right_child, level + 1);
    }
}

/// Prints one child subtree followed by its separator key.
fn print_internal_child(pager: &mut Pager, page_num: u32, index: u32, level: u32) {
    let child = internal_node_child(pager.get_page(page_num), index);
    print_tree(pager, child, level + 1);
    indent(level + 1);
    println!(
        "- key {}",
        internal_node_key(pager.get_page(page_num), index)
    );
}

/// Executes `.exit`, `.btree`, or `.constants`.
pub fn do_meta_command(input: &str, table: &mut Table) -> MetaCommandResult {
    match input.trim() {
        ".exit" => MetaCommandResult::Exit,
        ".btree" => {
            println!("Tree:");
            print_tree(&mut table.pager, 0, 0);
            MetaCommandResult::Success
        }
        ".constants" => {
            println!("Constants:");
            print_constants();
            MetaCommandResult::Success
        }
        cmd => MetaCommandResult::UnrecognizedCommand(cmd.to_string()),
    }
}
