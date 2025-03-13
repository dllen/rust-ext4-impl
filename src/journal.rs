//! Journal for ext4 filesystem.

use std::io::{Read, Seek};
use crate::error::Ext4Error;

/// The magic number of an ext4 journal.
const JBD2_MAGIC_NUMBER: u32 = 0xC03B3998;

/// The journal superblock of an ext4 filesystem.
#[derive(Debug, Clone)]
pub struct JournalSuperblock {
    /// Journal block magic number.
    pub magic: u32,
    /// Journal block type.
    pub block_type: u32,
    /// Journal block sequence number.
    pub sequence: u32,
    /// Journal block size.
    pub blocksize: u32,
    /// Total number of blocks in journal.
    pub maxlen: u32,
    /// First block of log data.
    pub first: u32,
    /// First commit ID expected in log.
    pub sequence_id: u32,
    /// Start of log. */
    pub start: u32,
}

/// The journal of an ext4 filesystem.
#[derive(Debug, Clone)]
pub struct Journal {
    /// The journal superblock.
    pub superblock: JournalSuperblock,
}

impl Journal {
    /// Read a journal from a reader.
    pub fn read<R: Read + Seek>(_reader: &mut R, _journal_inode: u32, block_size: u32) -> Result<Self, Ext4Error> {
        // TODO: Implement reading the journal from the journal inode
        // For now, we'll just create a dummy journal
        let superblock = JournalSuperblock {
            magic: JBD2_MAGIC_NUMBER,
            block_type: 0,
            sequence: 0,
            blocksize: block_size,
            maxlen: 0,
            first: 0,
            sequence_id: 0,
            start: 0,
        };

        Ok(Journal { superblock })
    }
}