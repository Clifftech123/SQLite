//! A database row and its fixed-width on-disk representation.

use std::fmt;

use crate::config::COLUMN_EMAIL_SIZE;
use crate::config::COLUMN_USERNAME_SIZE;



pub const ID_SIZE: usize = 4;
pub const USERNAME_SIZE: usize = COLUMN_USERNAME_SIZE + 1; // 33 bytes
pub const EMAIL_SIZE: usize = COLUMN_EMAIL_SIZE + 1; // 256 bytes

pub const ID_OFFSET: usize = 0;
pub const USERNAME_OFFSET: usize = ID_OFFSET + ID_SIZE;
pub const EMAIL_OFFSET: usize = USERNAME_OFFSET + USERNAME_SIZE;
pub const ROW_SIZE: usize = ID_SIZE + USERNAME_SIZE + EMAIL_SIZE; // 293 byte

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: u32,
    pub username: String,
    pub email: String,
}

impl Row {
    /// Creates an owned row from an ID, username, and email.
    pub fn new(id: u32, username: &str, email: &str) -> Self {
        Self {
            id,
            username: username.to_string(),
            email: email.to_string(),
        }
    }

    /// Serializes this row into the fixed-width disk format.
    pub fn serialize_into(&self, dest: &mut [u8]) {
        assert_has_row_space(dest, "Destination slice too small for Row");
        write_u32(dest, ID_OFFSET, self.id);
        write_fixed_text(
            dest,
            USERNAME_OFFSET,
            USERNAME_SIZE,
            COLUMN_USERNAME_SIZE,
            &self.username,
        );
        write_fixed_text(
            dest,
            EMAIL_OFFSET,
            EMAIL_SIZE,
            COLUMN_EMAIL_SIZE,
            &self.email,
        );
    }

    /// Reads one row from its fixed-width disk representation.
    pub fn deserialize_from(src: &[u8]) -> Self {
        assert_has_row_space(src, "Source slice too small for Row");
        Row {
            id: read_u32(src, ID_OFFSET),
            username: read_fixed_text(src, USERNAME_OFFSET, USERNAME_SIZE),
            email: read_fixed_text(src, EMAIL_OFFSET, EMAIL_SIZE),
        }
    }
}

/// Panics if `bytes` is too small to hold one serialized row.
fn assert_has_row_space(bytes: &[u8], message: &str) {
    assert!(bytes.len() >= ROW_SIZE, "{message}");
}

/// Writes a little-endian `u32` at `offset`.
fn write_u32(dest: &mut [u8], offset: usize, value: u32) {
    let end = offset + ID_SIZE;
    dest[offset..end].copy_from_slice(&value.to_le_bytes());
}

/// Writes `value` into a fixed-width field, truncating to `max_text_size`
/// bytes and zero-padding the remainder so `read_fixed_text` can find the
/// end of the string again.
fn write_fixed_text(
    dest: &mut [u8],
    offset: usize,
    field_size: usize,
    max_text_size: usize,
    value: &str,
) {
    let bytes = value.as_bytes();
    let copy_len = bytes.len().min(max_text_size);
    let text_end = offset + copy_len;
    dest[offset..text_end].copy_from_slice(&bytes[..copy_len]);
    dest[text_end..offset + field_size].fill(0);
}

/// Reads a little-endian `u32` at `offset`.
fn read_u32(src: &[u8], offset: usize) -> u32 {
    let end = offset + ID_SIZE;
    u32::from_le_bytes(src[offset..end].try_into().expect("invalid u32 field"))
}

/// Reads a fixed-width field back to a `String`, stopping at the first zero
/// byte written by `write_fixed_text` (or at the end of the field if none
/// is found).
fn read_fixed_text(src: &[u8], offset: usize, field_size: usize) -> String {
    let field = &src[offset..offset + field_size];
    let text_end = field
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(field_size - 1);
    String::from_utf8_lossy(&field[..text_end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_then_deserialize_round_trips() {
        let row = Row::new(7, "alice", "alice@example.com");
        let mut buf = [0u8; ROW_SIZE];
        row.serialize_into(&mut buf);
        assert_eq!(Row::deserialize_from(&buf), row);
    }

    #[test]
    fn serialize_zero_pads_unused_field_bytes() {
        let row = Row::new(1, "a", "b");
        let mut buf = [0xFFu8; ROW_SIZE];
        row.serialize_into(&mut buf);
        // Everything past the written text must be zeroed, not left as 0xFF.
        assert_eq!(buf[USERNAME_OFFSET + 1], 0);
        assert_eq!(buf[EMAIL_OFFSET + 1], 0);
    }

    #[test]
    fn deserialize_reads_full_width_field_with_no_terminator() {
        // A field with no zero byte anywhere should read as the whole field
        // minus the last byte (the position().unwrap_or fallback).
        let mut buf = [0u8; ROW_SIZE];
        buf[USERNAME_OFFSET..USERNAME_OFFSET + USERNAME_SIZE].fill(b'x');
        buf[EMAIL_OFFSET..EMAIL_OFFSET + EMAIL_SIZE].fill(b'y');
        let row = Row::deserialize_from(&buf);
        assert_eq!(row.username.len(), USERNAME_SIZE - 1);
        assert_eq!(row.email.len(), EMAIL_SIZE - 1);
    }

    #[test]
    fn empty_strings_round_trip() {
        let row = Row::new(0, "", "");
        let mut buf = [0u8; ROW_SIZE];
        row.serialize_into(&mut buf);
        assert_eq!(Row::deserialize_from(&buf), row);
    }

    #[test]
    #[should_panic(expected = "Destination slice too small for Row")]
    fn serialize_into_undersized_buffer_panics() {
        let row = Row::new(1, "a", "b");
        let mut buf = [0u8; 4];
        row.serialize_into(&mut buf);
    }

    #[test]
    #[should_panic(expected = "Source slice too small for Row")]
    fn deserialize_from_undersized_buffer_panics() {
        let buf = [0u8; 4];
        Row::deserialize_from(&buf);
    }

    #[test]
    fn display_formats_as_tuple() {
        let row = Row::new(3, "bob", "bob@example.com");
        assert_eq!(row.to_string(), "(3, bob, bob@example.com)");
    }
}

impl fmt::Display for Row {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.id, self.username, self.email)
    }
}
