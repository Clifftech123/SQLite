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

fn assert_has_row_space(bytes: &[u8], message: &str) {
    assert!(bytes.len() >= ROW_SIZE, "{message}");
}

fn write_u32(dest: &mut [u8], offset: usize, value: u32) {
    let end = offset + ID_SIZE;
    dest[offset..end].copy_from_slice(&value.to_le_bytes());
}

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

fn read_u32(src: &[u8], offset: usize) -> u32 {
    let end = offset + ID_SIZE;
    u32::from_le_bytes(src[offset..end].try_into().expect("invalid u32 field"))
}

fn read_fixed_text(src: &[u8], offset: usize, field_size: usize) -> String {
    let field = &src[offset..offset + field_size];
    let text_end = field
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(field_size - 1);
    String::from_utf8_lossy(&field[..text_end]).into_owned()
}

impl fmt::Display for Row {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.id, self.username, self.email)
    }
}
