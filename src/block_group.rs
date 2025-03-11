//! Block group descriptor for ext4 filesystem.

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{Read, Seek, SeekFrom};
use crate::error::Ext4Error;

/// The block group descriptor of an ext4 filesystem.
#[derive(Debug, Clone)]
pub struct BlockGroup {
    /// Block bitmap block.
    pub block_bitmap: u32,
    /// Inode bitmap block.
    pub inode_bitmap: u32,
    /// Inode table block.
    pub inode_table: u32,
    /// Free blocks count.
    pub free_blocks_count: u16,
    /// Free inodes count.
    pub free_inodes_count: u16,
    /// Directories count.
    pub used_dirs_count: u16,
    /// Padding.
    pub pad: u16,
    /// Reserved.
    pub reserved: [u8; 12],
}

impl BlockGroup {
    /// Read a block group descriptor from a reader.
    pub fn read<R: Read + Seek>(reader: &mut R, group_num: u32, first_data_block: u32, block_size: u32) -> Result<Self, Ext4Error> {
        // The block group descriptor table starts at the first block after the superblock
        let offset = (first_data_block + 1) * block_size + group_num * 32;
        reader.seek(SeekFrom::Start(offset as u64))?;

        let block_bitmap = reader.read_u32::<LittleEndian>()?;
        let inode_bitmap = reader.read_u32::<LittleEndian>()?;
        let inode_table = reader.read_u32::<LittleEndian>()?;
        let free_blocks_count = reader.read_u16::<LittleEndian>()?;
        let free_inodes_count = reader.read_u16::<LittleEndian>()?;
        let used_dirs_count = reader.read_u16::<LittleEndian>()?;
        let pad = reader.read_u16::<LittleEndian>()?;
        
        let mut reserved = [0u8; 12];
        reader.read_exact(&mut reserved)?;

        Ok(BlockGroup {
            block_bitmap,
            inode_bitmap,
            inode_table,
            free_blocks_count,
            free_inodes_count,
            used_dirs_count,
            pad,
            reserved,
        })
    }
}