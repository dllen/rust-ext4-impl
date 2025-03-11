//! Error types for the ext4 filesystem.

use thiserror::Error;
use std::io;

/// Errors that can occur when working with an ext4 filesystem.
#[derive(Error, Debug)]
pub enum Ext4Error {
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// The filesystem is not a valid ext4 filesystem.
    #[error("Invalid ext4 filesystem: {0}")]
    InvalidFilesystem(String),

    /// The superblock is invalid.
    #[error("Invalid superblock: {0}")]
    InvalidSuperblock(String),

    /// The inode is invalid.
    #[error("Invalid inode: {0}")]
    InvalidInode(String),

    /// The block group is invalid.
    #[error("Invalid block group: {0}")]
    InvalidBlockGroup(String),

    /// The journal is invalid.
    #[error("Invalid journal: {0}")]
    InvalidJournal(String),

    /// The directory is invalid.
    #[error("Invalid directory: {0}")]
    InvalidDirectory(String),

    /// The file is invalid.
    #[error("Invalid file: {0}")]
    InvalidFile(String),

    /// The operation is not implemented.
    #[error("Operation not implemented: {0}")]
    InvalidOperation(String),
    
    /// No space left on the filesystem.
    #[error("No space left on filesystem: {0}")]
    NoSpace(String),
    
    /// The block is invalid.
    #[error("Invalid block: {0}")]
    InvalidBlock(String),
}