//! Directory entry for ext4 filesystem.

use std::io::{Read, Seek};
use crate::error::Ext4Error;
use crate::inode::Inode;

/// The directory entry of an ext4 filesystem.
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    /// Inode number.
    pub inode: u32,
    /// Entry length.
    pub rec_len: u16,
    /// Name length.
    pub name_len: u8,
    /// File type.
    pub file_type: u8,
    /// File name.
    pub name: String,
}

/// The directory of an ext4 filesystem.
#[derive(Debug, Clone)]
pub struct Directory {
    /// The inode of the directory.
    pub inode: Inode,
    /// The entries in the directory.
    pub entries: Vec<DirectoryEntry>,
}

impl Directory {

    /// Create a new empty directory
    pub fn new() -> Self {
        Directory {
            inode: Inode::default(),
            entries: Vec::new(),
        }
    }

    /// Read a directory from a reader.
    pub fn read<R: Read + Seek>(reader: &mut R, inode: Inode, block_size: u32) -> Result<Self, Ext4Error> {
        if !inode.is_directory() {
            return Err(Ext4Error::InvalidDirectory("Not a directory".to_string()));
        }

        let entries = Vec::new();
        
        // TODO: Implement reading directory entries from the inode's data blocks
        // This is a complex operation that involves reading the inode's data blocks
        // and parsing the directory entries from them.
        
        Ok(Directory { inode, entries })
    }

    /// Find an entry by name.
    pub fn find_entry(&self, name: &str) -> Option<&DirectoryEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }
}