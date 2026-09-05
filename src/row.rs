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
    pub fn new(id: u32, username: &str, email: &str) -> Self {
        Self {
            id,
            username: username.to_string(),
            email: email.to_string(),
        }
    }

    //  Serializes the Row into exact SQLite disk format (fixed 293-byte slice).
    pub fn serialize_into(&self, dest: &mut [u8]) {
        assert!(
            dest.len() >= ROW_SIZE,
            "Destination slice too small for Row"
        );

        // 1. ID (u32, little-endian)
        dest[ID_OFFSET..ID_OFFSET + ID_SIZE].copy_from_slice(&self.id.to_le_bytes());

        // 2. Username (fixed 33 bytes null-terminated)
        let username_bytes = self.username.as_bytes();
        let copy_len = username_bytes.len().min(COLUMN_USERNAME_SIZE);
        dest[USERNAME_OFFSET..USERNAME_OFFSET + copy_len]
            .copy_from_slice(&username_bytes[..copy_len]);
        // Zero-fill remaining buffer
        for byte in &mut dest[USERNAME_OFFSET + copy_len..USERNAME_OFFSET + USERNAME_SIZE] {
            *byte = 0;
        }

        // 3. Email (fixed 256 bytes null-terminated)
        let email_bytes = self.email.as_bytes();
        let copy_len_email = email_bytes.len().min(COLUMN_EMAIL_SIZE);
        dest[EMAIL_OFFSET..EMAIL_OFFSET + copy_len_email]
            .copy_from_slice(&email_bytes[..copy_len_email]);
        for byte in &mut dest[EMAIL_OFFSET + copy_len_email..EMAIL_OFFSET + EMAIL_SIZE] {
            *byte = 0;
        }
    }

    // Deserializes a Row from a 293-byte disk slice safely.

    pub fn deserialize_from(src: &[u8]) -> Self {
        assert!(src.len() >= ROW_SIZE, "Source slice too small for Row");

        let id = u32::from_le_bytes(src[ID_OFFSET..ID_OFFSET + ID_SIZE].try_into().unwrap());

        let username_slice = &src[USERNAME_OFFSET..USERNAME_OFFSET + USERNAME_SIZE];
        let username_len = username_slice
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(COLUMN_USERNAME_SIZE);
        let username = String::from_utf8_lossy(&username_slice[..username_len]).to_string();

        let email_slice = &src[EMAIL_OFFSET..EMAIL_OFFSET + EMAIL_SIZE];
        let email_len = email_slice
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(COLUMN_EMAIL_SIZE);
        let email = String::from_utf8_lossy(&email_slice[..email_len]).to_string();

        Row {
            id,
            username,
            email,
        }
    }
}

impl fmt::Display for Row {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.id, self.username, self.email)
    }
}
