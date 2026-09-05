//! General helpers for one fixed-size database page.

use crate::config::PAGE_SIZE;

/// One fixed-size byte buffer used by storage and B-tree code.
pub type Page = [u8; PAGE_SIZE];
/// Heap-owned page suitable for the pager cache.
pub type BoxedPage = Box<Page>;

/// Creates a zero-filled page.
pub fn new_page() -> BoxedPage {
    Box::new([0; PAGE_SIZE])
}

pub fn file_offset(page_number: u32) -> u64 {
    page_number as u64 * PAGE_SIZE as u64
}

pub fn clear(page: &mut Page) {
    page.fill(0);
}

pub fn copy_into(destination: &mut Page, source: &Page) {
    destination.copy_from_slice(source);
}

pub fn read_range(page: &Page, offset: usize, length: usize) -> &[u8] {
    let end = checked_end(offset, length);
    &page[offset..end]
}

pub fn write_range<'a>(page: &'a mut Page, offset: usize, bytes: &[u8]) -> &'a mut [u8] {
    let end = checked_end(offset, bytes.len());
    page[offset..end].copy_from_slice(bytes);
    &mut page[offset..end]
}

fn checked_end(offset: usize, length: usize) -> usize {
    let end = offset
        .checked_add(length)
        .expect("page range offset overflowed");
    assert!(end <= PAGE_SIZE, "page range exceeds PAGE_SIZE");
    end
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_page_is_zeroed() {
        assert!(new_page().iter().all(|byte| *byte == 0));
    }
    #[test]
    fn range_helpers_work() {
        let mut page = new_page();
        write_range(&mut page, 10, &[1, 2, 3]);
        assert_eq!(read_range(&page, 10, 3), &[1, 2, 3]);
    }
    #[test]
    fn file_offset_uses_page_size() {
        assert_eq!(file_offset(2), (PAGE_SIZE * 2) as u64);
    }
    #[test]
    fn clear_zeroes_a_dirty_page() {
        let mut page = new_page();
        write_range(&mut page, 0, &[1, 2, 3]);
        clear(&mut page);
        assert!(page.iter().all(|byte| *byte == 0));
    }
    #[test]
    fn copy_into_duplicates_contents() {
        let mut source = new_page();
        write_range(&mut source, 0, &[9, 8, 7]);
        let mut destination = new_page();
        copy_into(&mut destination, &source);
        assert_eq!(read_range(&destination, 0, 3), &[9, 8, 7]);
    }
    #[test]
    #[should_panic(expected = "page range exceeds PAGE_SIZE")]
    fn write_range_past_page_end_panics() {
        let mut page = new_page();
        write_range(&mut page, PAGE_SIZE - 1, &[1, 2]);
    }
    #[test]
    #[should_panic]
    fn write_range_offset_overflow_panics() {
        let mut page = new_page();
        write_range(&mut page, usize::MAX, &[1]);
    }
}
