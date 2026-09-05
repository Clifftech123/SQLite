//! Disk file and in-memory page-cache management.

use crate::config::{PAGE_SIZE, TABLE_MAX_PAGES};
use crate::storage::page::{Page, file_offset, new_page};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};

pub struct Pager {
    file: File,
    file_length: u64,
    pub num_pages: u32,
    pages: Vec<Option<Box<Page>>>,
}

impl Pager {
    /// Opens or creates a page-aligned database file.
    pub fn open(filename: &str) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(filename)?;
        let file_length = file.seek(SeekFrom::End(0))?;
        if file_length % PAGE_SIZE as u64 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "database file is not a whole number of pages",
            ));
        }
        let num_pages = (file_length / PAGE_SIZE as u64) as u32;
        let pages = (0..TABLE_MAX_PAGES).map(|_| None).collect();
        Ok(Self {
            file,
            file_length,
            num_pages,
            pages,
        })
    }

    pub fn get_page(&mut self, page_num: u32) -> &mut Page {
        let index = self.checked_page_index(page_num);
        if self.pages[index].is_none() {
            let page = self.load_page(page_num);
            self.cache_page(index, page, page_num);
        }
        self.pages[index].as_deref_mut().expect("page was loaded")
    }

    pub fn flush(&mut self, page_num: u32) -> io::Result<()> {
        let index = self.page_index_for_flush(page_num)?;
        if let Some(page) = &self.pages[index] {
            let page_copy = page.clone();
            self.write_page(page_num, &page_copy)?;
        }
        Ok(())
    }

    /// Validates a page number for operations that cannot return an error.
    fn checked_page_index(&self, page_num: u32) -> usize {
        let index = page_num as usize;
        assert!(
            index < TABLE_MAX_PAGES,
            "page number {page_num} exceeds cache limit"
        );
        index
    }

    /// Validates a page number for the fallible flush operation.
    fn page_index_for_flush(&self, page_num: u32) -> io::Result<usize> {
        let index = page_num as usize;
        if index >= self.pages.len() {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "page outside cache",
            ))
        } else {
            Ok(index)
        }
    }

    /// Loads a page from disk, or returns a new zero-filled page.
    fn load_page(&mut self, page_num: u32) -> Box<Page> {
        let mut page = new_page();
        let offset = file_offset(page_num);
        if offset < self.file_length {
            self.file
                .seek(SeekFrom::Start(offset))
                .expect("failed to seek to database page");
            self.file
                .read_exact(&mut page[..])
                .expect("failed to read database page");
        }
        page
    }

    /// Stores a loaded page and updates the page count.
    fn cache_page(&mut self, index: usize, page: Box<Page>, page_num: u32) {
        self.pages[index] = Some(page);
        self.num_pages = self.num_pages.max(page_num + 1);
    }

    /// Writes one page at its calculated file offset.
    fn write_page(&mut self, page_num: u32, page: &Page) -> io::Result<()> {
        let offset = file_offset(page_num);
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(page)?;
        self.file.flush()?;
        self.file_length = self.file_length.max(offset + PAGE_SIZE as u64);
        Ok(())
    }
}

impl Drop for Pager {
    fn drop(&mut self) {
        for page_num in 0..self.num_pages {
            if let Err(error) = self.flush(page_num) {
                eprintln!("failed to flush page {page_num}: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// A unique, auto-deleted database file path for one test.
    struct TempDbFile(std::path::PathBuf);

    impl TempDbFile {
        fn new(name: &str) -> Self {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "sqlite_pager_test_{name}_{}_{n}.db",
                std::process::id()
            ));
            let _ = std::fs::remove_file(&path);
            Self(path)
        }

        fn path(&self) -> &str {
            self.0.to_str().expect("temp path must be valid UTF-8")
        }
    }

    impl Drop for TempDbFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn open_creates_an_empty_new_file() {
        let file = TempDbFile::new("open_empty");
        let pager = Pager::open(file.path()).expect("open should succeed");
        assert_eq!(pager.num_pages, 0);
    }

    #[test]
    fn open_rejects_a_file_with_a_partial_page() {
        let file = TempDbFile::new("open_partial");
        std::fs::write(file.path(), vec![0u8; PAGE_SIZE + 10]).unwrap();
        let result = Pager::open(file.path());
        assert!(result.is_err());
    }

    #[test]
    fn get_page_returns_a_zeroed_page_for_new_pages() {
        let file = TempDbFile::new("get_page_zeroed");
        let mut pager = Pager::open(file.path()).unwrap();
        let page = pager.get_page(0);
        assert!(page.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn get_page_grows_num_pages() {
        let file = TempDbFile::new("get_page_grows");
        let mut pager = Pager::open(file.path()).unwrap();
        assert_eq!(pager.num_pages, 0);
        pager.get_page(2);
        assert_eq!(pager.num_pages, 3);
    }

    #[test]
    fn get_page_caches_writes_until_flushed() {
        let file = TempDbFile::new("get_page_caches");
        let mut pager = Pager::open(file.path()).unwrap();
        pager.get_page(0)[0] = 42;
        assert_eq!(pager.get_page(0)[0], 42);
    }

    #[test]
    fn flush_persists_a_page_to_disk() {
        let file = TempDbFile::new("flush_persists");
        {
            let mut pager = Pager::open(file.path()).unwrap();
            pager.get_page(0)[0] = 99;
            pager.flush(0).expect("flush should succeed");
        }
        let mut reopened = Pager::open(file.path()).unwrap();
        assert_eq!(reopened.get_page(0)[0], 99);
    }

    #[test]
    fn drop_flushes_all_pages_automatically() {
        let file = TempDbFile::new("drop_flushes");
        {
            let mut pager = Pager::open(file.path()).unwrap();
            pager.get_page(0)[0] = 7;
            pager.get_page(1)[0] = 8;
        } // Pager dropped here; Drop::drop should flush both pages.
        let mut reopened = Pager::open(file.path()).unwrap();
        assert_eq!(reopened.num_pages, 2);
        assert_eq!(reopened.get_page(0)[0], 7);
        assert_eq!(reopened.get_page(1)[0], 8);
    }

    #[test]
    #[should_panic(expected = "exceeds cache limit")]
    fn get_page_beyond_cache_limit_panics() {
        let file = TempDbFile::new("get_page_oob");
        let mut pager = Pager::open(file.path()).unwrap();
        pager.get_page(TABLE_MAX_PAGES as u32);
    }

    #[test]
    fn flush_beyond_cache_limit_returns_error() {
        let file = TempDbFile::new("flush_oob");
        let mut pager = Pager::open(file.path()).unwrap();
        let result = pager.flush(TABLE_MAX_PAGES as u32);
        assert!(result.is_err());
    }
}
