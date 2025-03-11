//! The superblock of an ext4 filesystem.

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};
use crate::error::Ext4Error;

/// The magic number of an ext4 filesystem.
const EXT4_MAGIC: u16 = 0xEF53;

/// The superblock of an ext4 filesystem.
#[derive(Debug, Clone)]
pub struct Superblock {
    /// Total number of inodes in the filesystem.
    pub inodes_count: u32,
    /// Total number of blocks in the filesystem.
    pub blocks_count: u32,
    /// Number of blocks reserved for the superuser.
    pub r_blocks_count: u32,
    /// Number of unallocated blocks.
    pub free_blocks_count: u32,
    /// Number of unallocated inodes.
    pub free_inodes_count: u32,
    /// Block number of the first data block.
    pub first_data_block: u32,
    /// Block size (in bytes) = 2^(10 + log_block_size).
    pub log_block_size: u32,
    /// Fragment size (in bytes) = 2^(10 + log_frag_size).
    pub log_frag_size: i32,  // Changed from u32 to i32 to allow negative values
    /// Number of blocks per block group.
    pub blocks_per_group: u32,
    /// Number of fragments per block group.
    pub frags_per_group: u32,
    /// Number of inodes per block group.
    pub inodes_per_group: u32,
    /// Last mount time (in UNIX time).
    pub mtime: u32,
    /// Last write time (in UNIX time).
    pub wtime: u32,
    /// Number of mounts since the last fsck.
    pub mnt_count: u16,
    /// Maximum number of mounts before fsck is required.
    pub max_mnt_count: u16,
    /// Signature (0xEF53).
    pub magic: u16,
    /// Filesystem state.
    pub state: u16,
    /// Behavior when detecting errors.
    pub errors: u16,
    /// Minor revision level.
    pub minor_rev_level: u16,
    /// Time of last check (in UNIX time).
    pub lastcheck: u32,
    /// Maximum time between checks (in UNIX time).
    pub checkinterval: u32,
    /// Creator OS.
    pub creator_os: u32,
    /// Revision level.
    pub rev_level: u32,
    /// Default uid for reserved blocks.
    pub def_resuid: u16,
    /// Default gid for reserved blocks.
    pub def_resgid: u16,
    // ... more fields would be added here for a complete implementation
}

impl Superblock {
    /// Read a superblock from a reader.
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self, Ext4Error> {
        // The superblock starts at offset 1024 bytes
        reader.seek(SeekFrom::Start(1024))?;

        let inodes_count = reader.read_u32::<LittleEndian>()?;
        let blocks_count = reader.read_u32::<LittleEndian>()?;
        let r_blocks_count = reader.read_u32::<LittleEndian>()?;
        let free_blocks_count = reader.read_u32::<LittleEndian>()?;
        let free_inodes_count = reader.read_u32::<LittleEndian>()?;
        let first_data_block = reader.read_u32::<LittleEndian>()?;
        let log_block_size = reader.read_u32::<LittleEndian>()?;
        let log_frag_size = reader.read_i32::<LittleEndian>()?;  // Changed to read_i32
        let blocks_per_group = reader.read_u32::<LittleEndian>()?;
        let frags_per_group = reader.read_u32::<LittleEndian>()?;
        let inodes_per_group = reader.read_u32::<LittleEndian>()?;
        let mtime = reader.read_u32::<LittleEndian>()?;
        let wtime = reader.read_u32::<LittleEndian>()?;
        let mnt_count = reader.read_u16::<LittleEndian>()?;
        let max_mnt_count = reader.read_u16::<LittleEndian>()?;
        let magic = reader.read_u16::<LittleEndian>()?;
        let state = reader.read_u16::<LittleEndian>()?;
        let errors = reader.read_u16::<LittleEndian>()?;
        let minor_rev_level = reader.read_u16::<LittleEndian>()?;
        let lastcheck = reader.read_u32::<LittleEndian>()?;
        let checkinterval = reader.read_u32::<LittleEndian>()?;
        let creator_os = reader.read_u32::<LittleEndian>()?;
        let rev_level = reader.read_u32::<LittleEndian>()?;
        let def_resuid = reader.read_u16::<LittleEndian>()?;
        let def_resgid = reader.read_u16::<LittleEndian>()?;

        // Check the magic number
        if magic != EXT4_MAGIC {
            return Err(Ext4Error::InvalidSuperblock(format!(
                "Invalid magic number: {:x}, expected: {:x}",
                magic, EXT4_MAGIC
            )));
        }

        Ok(Superblock {
            inodes_count,
            blocks_count,
            r_blocks_count,
            free_blocks_count,
            free_inodes_count,
            first_data_block,
            log_block_size,
            log_frag_size,
            blocks_per_group,
            frags_per_group,
            inodes_per_group,
            mtime,
            wtime,
            mnt_count,
            max_mnt_count,
            magic,
            state,
            errors,
            minor_rev_level,
            lastcheck,
            checkinterval,
            creator_os,
            rev_level,
            def_resuid,
            def_resgid,
        })
    }

    /// Get the block size in bytes.
    pub fn block_size(&self) -> u32 {
        1024 << self.log_block_size
    }

    /// Get the fragment size in bytes.
    pub fn fragment_size(&self) -> u32 {
        if self.log_frag_size >= 0 {
            1024 << (self.log_frag_size as u32)
        } else {
            1024 >> (-self.log_frag_size as u32)
        }
    }

    /// Get the number of block groups.
    pub fn block_groups_count(&self) -> u32 {
        (self.blocks_count + self.blocks_per_group - 1) / self.blocks_per_group
    }
}