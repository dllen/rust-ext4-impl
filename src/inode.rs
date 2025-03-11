//! Inode structure for ext4 filesystem.

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Read, Seek, SeekFrom};
use crate::error::Ext4Error;

/// The inode structure of an ext4 filesystem.
#[derive(Debug, Clone)]
pub struct Inode {
    /// File mode.
    pub mode: u16,
    /// Owner's user ID.
    pub uid: u16,
    /// Size in bytes (lower 32 bits).
    pub size: u32,
    /// Last access time.
    pub atime: u32,
    /// Creation time.
    pub ctime: u32,
    /// Last modification time.
    pub mtime: u32,
    /// Deletion time.
    pub dtime: u32,
    /// Group ID.
    pub gid: u16,
    /// Hard link count.
    pub links_count: u16,
    /// Blocks count (in 512-byte units).
    pub blocks: u32,
    /// File flags.
    pub flags: u32,
    /// OS-specific value.
    pub osd1: u32,
    /// Direct block pointers.
    pub block: [u32; 15],
    /// File version (for NFS).
    pub generation: u32,
    /// Extended attribute block.
    pub file_acl: u32,
    /// Size in bytes (upper 32 bits) or directory ACL.
    pub dir_acl: u32,
    /// Fragment address.
    pub faddr: u32,
    /// OS-specific value.
    pub osd2: [u8; 12],
}

impl Inode {
    /// Read an inode from a reader.
    pub fn read<R: Read + Seek>(reader: &mut R, inode_size: u32, inode_num: u32, inodes_per_group: u32, inode_table_block: u32, block_size: u32) -> Result<Self, Ext4Error> {
        let group = (inode_num - 1) / inodes_per_group;
        let index = (inode_num - 1) % inodes_per_group;
        let offset = inode_table_block * block_size + index * inode_size;
        
        reader.seek(SeekFrom::Start(offset as u64))?;

        let mode = reader.read_u16::<LittleEndian>()?;
        let uid = reader.read_u16::<LittleEndian>()?;
        let size = reader.read_u32::<LittleEndian>()?;
        let atime = reader.read_u32::<LittleEndian>()?;
        let ctime = reader.read_u32::<LittleEndian>()?;
        let mtime = reader.read_u32::<LittleEndian>()?;
        let dtime = reader.read_u32::<LittleEndian>()?;
        let gid = reader.read_u16::<LittleEndian>()?;
        let links_count = reader.read_u16::<LittleEndian>()?;
        let blocks = reader.read_u32::<LittleEndian>()?;
        let flags = reader.read_u32::<LittleEndian>()?;
        let osd1 = reader.read_u32::<LittleEndian>()?;
        
        let mut block = [0u32; 15];
        for i in 0..15 {
            block[i] = reader.read_u32::<LittleEndian>()?;
        }
        
        let generation = reader.read_u32::<LittleEndian>()?;
        let file_acl = reader.read_u32::<LittleEndian>()?;
        let dir_acl = reader.read_u32::<LittleEndian>()?;
        let faddr = reader.read_u32::<LittleEndian>()?;
        
        let mut osd2 = [0u8; 12];
        reader.read_exact(&mut osd2)?;

        Ok(Inode {
            mode,
            uid,
            size,
            atime,
            ctime,
            mtime,
            dtime,
            gid,
            links_count,
            blocks,
            flags,
            osd1,
            block,
            generation,
            file_acl,
            dir_acl,
            faddr,
            osd2,
        })
    }

    /// Check if this inode represents a regular file.
    pub fn is_file(&self) -> bool {
        (self.mode & 0xF000) == 0x8000
    }

    /// Check if this inode represents a directory.
    pub fn is_directory(&self) -> bool {
        (self.mode & 0xF000) == 0x4000
    }

    /// Check if this inode represents a symbolic link.
    pub fn is_symlink(&self) -> bool {
        (self.mode & 0xF000) == 0xA000
    }

    /// Get the full size of the file in bytes.
    pub fn get_size(&self) -> u64 {
        if self.is_directory() {
            self.size as u64
        } else {
            ((self.dir_acl as u64) << 32) | (self.size as u64)
        }
    }
}

impl Default for Inode {
    fn default() -> Self {
        Inode {
            mode: 0,
            uid: 0,
            size: 0,
            atime: 0,
            ctime: 0,
            mtime: 0,
            dtime: 0,
            gid: 0,
            links_count: 0,
            blocks: 0,
            flags: 0,
            osd1: 0,
            block: [0; 15],
            generation: 0,
            file_acl: 0,
            dir_acl: 0,
            faddr: 0,
            osd2: [0; 12],
        }
    }
}