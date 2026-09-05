// These files are kept in a folder so the test suite stays organized.
// The path attributes make Cargo compile them as one integration-test crate.
#[path = "integration/btree_growth.rs"]
mod btree_growth;

#[path = "integration/persistence.rs"]
mod persistence;

#[path = "integration/repl_commands.rs"]
mod repl_commands;
