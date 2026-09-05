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
