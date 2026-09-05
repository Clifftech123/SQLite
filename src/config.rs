//! Database-wide format sizes and limits.

/// Maximum username bytes stored in a row.
pub const COLUMN_USERNAME_SIZE: usize = 32;
/// Maximum email bytes stored in a row.
pub const COLUMN_EMAIL_SIZE: usize = 255;

/// Number of bytes in every database page.
pub const PAGE_SIZE: usize = 4096;
/// Maximum number of cached pages.
pub const TABLE_MAX_PAGES: usize = 400;
/// Sentinel used when a node has no parent or child page.
pub const INVALID_PAGE_NUM: u32 = u32::MAX;
