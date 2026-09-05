//! Shared helpers for end-to-end tests: a uniquely named, auto-deleted
//! database file per test so parallel test runs never collide.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

pub struct TempDbFile(pub PathBuf);

impl TempDbFile {
    pub fn new(name: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "sqlite_integration_{name}_{}_{n}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        Self(path)
    }

    pub fn path(&self) -> &str {
        self.0.to_str().expect("temp path must be valid UTF-8")
    }
}

impl Drop for TempDbFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
