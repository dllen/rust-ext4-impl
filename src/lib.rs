//! A Rust implementation of the ext4 filesystem.

mod block_group;
mod directory;
mod error;
mod file;
mod inode;
mod journal;
mod superblock;

use std::fs::File as StdFile;
use std::io::{Read, Seek, SeekFrom, Write};

pub use block_group::BlockGroup;
use byteorder::WriteBytesExt;
pub use directory::Directory;
pub use error::Ext4Error;
pub use file::File;
pub use inode::Inode;
pub use journal::Journal;
pub use superblock::Superblock;

/// The main struct representing an ext4 filesystem.
pub struct Ext4Filesystem {
    /// The superblock of the filesystem.
    superblock: Superblock,
    /// The block groups of the filesystem.
    block_groups: Vec<BlockGroup>,
    /// The journal of the filesystem.
    journal: Option<Journal>,
    /// The file handle for the filesystem.
    file: StdFile,
}

impl Ext4Filesystem {
    /// Create a new ext4 filesystem from a file.
    pub fn new(path: &str) -> Result<Self, Ext4Error> {
        // Open the file with read-write permissions
        let file = StdFile::options().read(true).write(true).open(path)?;

        // Read the superblock
        let mut file_clone = file.try_clone()?;
        let superblock = Superblock::read(&mut file_clone)?;

        // Read the block groups
        let mut block_groups = Vec::new();
        let block_groups_count = superblock.block_groups_count();
        let block_size = superblock.block_size();

        for i in 0..block_groups_count {
            let mut file_clone = file.try_clone()?;
            let block_group =
                BlockGroup::read(&mut file_clone, i, superblock.first_data_block, block_size)?;
            block_groups.push(block_group);
        }

        // Read the journal if it exists
        let journal = if superblock.rev_level >= 1 {
            // TODO: Implement reading the journal
            None
        } else {
            None
        };

        Ok(Ext4Filesystem {
            superblock,
            block_groups,
            journal,
            file,
        })
    }

    /// Mount an existing ext4 filesystem.
    pub fn mount(path: &str) -> Result<Self, Ext4Error> {
        Self::new(path)
    }

    /// Get the superblock of the filesystem.
    pub fn superblock(&self) -> &Superblock {
        &self.superblock
    }

    /// Get the block groups of the filesystem.
    pub fn block_groups(&self) -> &[BlockGroup] {
        &self.block_groups
    }

    /// Get the journal of the filesystem.
    pub fn journal(&self) -> Option<&Journal> {
        self.journal.as_ref()
    }

    /// Read an inode from the filesystem.
    pub fn read_inode(&mut self, inode_num: u32) -> Result<Inode, Ext4Error> {
        if inode_num == 0 || inode_num > self.superblock.inodes_count {
            return Err(Ext4Error::InvalidInode(format!(
                "Invalid inode number: {}",
                inode_num
            )));
        }

        let group_idx = (inode_num - 1) / self.superblock.inodes_per_group;
        if group_idx as usize >= self.block_groups.len() {
            return Err(Ext4Error::InvalidInode(format!(
                "Invalid block group index: {}",
                group_idx
            )));
        }

        let block_group = &self.block_groups[group_idx as usize];
        let mut file_clone = self.file.try_clone()?;

        Inode::read(
            &mut file_clone,
            256, // Assuming inode size is 256 bytes
            inode_num,
            self.superblock.inodes_per_group,
            block_group.inode_table,
            self.superblock.block_size(),
        )
    }

    /// Read a directory from the filesystem.
    pub fn read_directory(&mut self, inode_num: u32) -> Result<Directory, Ext4Error> {
        let inode = self.read_inode(inode_num)?;
        if !inode.is_directory() {
            return Err(Ext4Error::InvalidDirectory(format!(
                "Inode {} is not a directory",
                inode_num
            )));
        }

        let mut file_clone = self.file.try_clone()?;
        Directory::read(&mut file_clone, inode, self.superblock.block_size())
    }

    /// Open a file from the filesystem.
    pub fn open_file(&mut self, inode_num: u32) -> Result<File, Ext4Error> {
        let inode = self.read_inode(inode_num)?;
        if !inode.is_file() {
            return Err(Ext4Error::InvalidFile(format!(
                "Inode {} is not a regular file",
                inode_num
            )));
        }

        Ok(File::new(inode))
    }

    /// Read data from a file.
    pub fn read_file(
        &mut self,
        inode_num: u32,
        buffer: &mut [u8],
        position: u64,
    ) -> Result<usize, Ext4Error> {
        let mut file = self.open_file(inode_num)?;
        file.seek(position)?;

        let mut file_clone = self.file.try_clone()?;
        file.read(&mut file_clone, buffer, self.superblock.block_size())
    }

    /// Get the root directory of the filesystem.
    pub fn root_directory(&mut self) -> Result<Directory, Ext4Error> {
        // The root directory is always inode 2 in ext4
        self.read_directory(2)
    }

    /// Find a file or directory by path.
    pub fn find_by_path(&mut self, path: &str) -> Result<u32, Ext4Error> {
        if path.is_empty() || path == "/" {
            return Ok(2); // Root directory inode
        }

        let components: Vec<&str> = path.trim_start_matches('/').split('/').collect();
        let mut current_inode = 2; // Start from the root directory

        for component in components {
            if component.is_empty() {
                continue;
            }

            let directory = self.read_directory(current_inode)?;
            match directory.find_entry(component) {
                Some(entry) => {
                    current_inode = entry.inode;
                }
                None => {
                    return Err(Ext4Error::InvalidFile(format!(
                        "Path component not found: {}",
                        component
                    )));
                }
            }
        }

        Ok(current_inode)
    }

    /// Write a file to the filesystem.
    pub fn write_file(
        &mut self,
        parent_path: &str,
        filename: &str,
        data: &[u8],
    ) -> Result<(), Ext4Error> {
        // Find the parent directory inode
        let parent_inode_num = self.find_by_path(parent_path)?;
        let parent_inode = self.read_inode(parent_inode_num)?;

        if !parent_inode.is_directory() {
            return Err(Ext4Error::InvalidDirectory(format!(
                "'{}' is not a directory",
                parent_path
            )));
        }

        // Check if file already exists
        let directory = self.read_directory(parent_inode_num)?;
        let existing_entry = directory.find_entry(filename);

        let inode_num = match existing_entry {
            Some(entry) => {
                // File exists, read its inode
                let inode_num = entry.inode;
                let inode = self.read_inode(inode_num)?;

                if !inode.is_file() {
                    return Err(Ext4Error::InvalidFile(format!(
                        "'{}' exists but is not a regular file",
                        filename
                    )));
                }

                // Free the existing blocks
                for i in 0..15 {
                    if inode.block[i] != 0 {
                        self.free_block(inode.block[i])?;
                    }
                }

                inode_num
            }
            None => {
                // File doesn't exist, allocate a new inode
                self.allocate_inode()?
            }
        };

        // Create or update the inode
        let mut inode = Inode::default();
        inode.mode = 0x81A4; // Regular file with 0644 permissions
        inode.links_count = 1;
        inode.size = data.len() as u32;

        // Get current time
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;

        inode.atime = now;
        inode.ctime = now;
        inode.mtime = now;

        // Calculate how many blocks we need
        let block_size = self.superblock.block_size() as usize;
        let blocks_needed = (data.len() + block_size - 1) / block_size;

        if blocks_needed > 12 {
            return Err(Ext4Error::InvalidOperation(
                "Files larger than 12 direct blocks are not supported yet".to_string(),
            ));
        }

        // Allocate blocks and write data
        let mut blocks_allocated = 0;
        for i in 0..blocks_needed {
            let block_num = self.allocate_block()?;
            inode.block[i] = block_num;
            blocks_allocated += 1;

            // Write data to this block
            let start = i * block_size;
            let end = std::cmp::min((i + 1) * block_size, data.len());
            let block_data = &data[start..end];

            let mut file_clone = self.file.try_clone()?;
            file_clone.seek(SeekFrom::Start(
                (block_num * self.superblock.block_size()) as u64,
            ))?;
            file_clone.write_all(block_data)?;

            // If this is the last block and it's not full, zero the rest
            if end < (i + 1) * block_size {
                let zeros = vec![0u8; (i + 1) * block_size - end];
                file_clone.write_all(&zeros)?;
            }
        }

        // Update inode blocks count (in 512-byte units)
        inode.blocks = blocks_allocated * (self.superblock.block_size() / 512);

        // Write the inode to disk
        self.write_inode(inode_num, &inode)?;

        // If this is a new file, add an entry to the parent directory
        if existing_entry.is_none() {
            self.add_directory_entry(parent_inode_num, filename, inode_num, 1)?;
            // 1 = regular file
        }

        // Update superblock
        self.superblock.free_blocks_count -= blocks_allocated;
        if existing_entry.is_none() {
            self.superblock.free_inodes_count -= 1;
        }
        self.write_superblock()?;

        Ok(())
    }

    /// Remove a file from the filesystem.
    pub fn remove_file(&mut self, path: &str) -> Result<(), Ext4Error> {
        // Find the file inode
        let inode_num = self.find_by_path(path)?;
        let inode = self.read_inode(inode_num)?;

        if !inode.is_file() {
            return Err(Ext4Error::InvalidFile(format!(
                "'{}' is not a regular file",
                path
            )));
        }

        // Get the parent directory path and filename
        let (parent_path, filename) = match path.rfind('/') {
            Some(pos) => {
                let parent = if pos == 0 { "/" } else { &path[..pos] };
                let name = &path[pos + 1..];
                (parent, name)
            }
            None => ("/", path),
        };

        // Find the parent directory inode
        let parent_inode_num = self.find_by_path(parent_path)?;

        // Remove the directory entry from the parent directory
        self.remove_directory_entry(parent_inode_num, filename)?;

        // Free all blocks used by the file
        let mut blocks_freed = 0;
        for i in 0..15 {
            if inode.block[i] != 0 {
                self.free_block(inode.block[i])?;
                blocks_freed += 1;
            }
        }

        // Mark the inode as free
        self.free_inode(inode_num)?;

        // Update superblock and block group descriptors
        self.superblock.free_blocks_count += blocks_freed;
        self.superblock.free_inodes_count += 1;
        self.write_superblock()?;

        Ok(())
    }

    /// Remove a directory from the filesystem.
    pub fn remove_directory(&mut self, path: &str, force: bool) -> Result<(), Ext4Error> {
        // Find the directory inode
        let inode_num = self.find_by_path(path)?;
        let inode = self.read_inode(inode_num)?;

        if !inode.is_directory() {
            return Err(Ext4Error::InvalidDirectory(format!(
                "'{}' is not a directory",
                path
            )));
        }

        // Check if directory is empty (unless force flag is used)
        if !force {
            let directory = self.read_directory(inode_num)?;
            // Skip "." and ".." entries when checking if directory is empty
            let real_entries = directory
                .entries
                .iter()
                .filter(|entry| entry.name != "." && entry.name != "..")
                .count();

            if real_entries > 0 {
                return Err(Ext4Error::InvalidOperation(format!(
                    "Directory '{}' is not empty. Use force flag to remove anyway.",
                    path
                )));
            }
        }

        // Get the parent directory
        let parent_path = match path.rfind('/') {
            Some(pos) => {
                if pos == 0 {
                    "/"
                } else {
                    &path[..pos]
                }
            }
            None => "/",
        };

        let dirname = match path.rfind('/') {
            Some(pos) => &path[pos + 1..],
            None => path,
        };

        let parent_inode_num = self.find_by_path(parent_path)?;

        // For now, we'll just return an error since we haven't implemented the deallocation methods
        // return Err(Ext4Error::InvalidOperation("Removing directories is not fully implemented yet".to_string()));

        // The following would be the implementation once deallocation methods are implemented:
        // 1. Remove the directory entry from the parent directory
        self.remove_directory_entry(parent_inode_num, dirname)?;

        // 2. Update the parent inode's link count (for the removed ".." entry)
        let mut parent_inode = self.read_inode(parent_inode_num)?;
        parent_inode.links_count -= 1;
        self.write_inode(parent_inode_num, &parent_inode)?;

        // 3. Free the blocks used by the directory
        let mut blocks_freed = 0;
        for i in 0..12 {
            if inode.block[i] != 0 {
                self.free_block(inode.block[i])?;
                blocks_freed += 1;
            }
        }

        // 4. Mark the inode as free
        self.free_inode(inode_num)?;

        // 5. Update superblock and block group descriptors
        self.superblock.free_blocks_count += blocks_freed;
        self.superblock.free_inodes_count += 1;
        self.write_superblock()?;

        Ok(())
    }

    /// Create a new directory in the filesystem.
    pub fn create_directory(&mut self, parent_path: &str, dirname: &str) -> Result<(), Ext4Error> {
        // Find the parent directory inode
        let parent_inode_num = self.find_by_path(parent_path)?;
        let parent_inode = self.read_inode(parent_inode_num)?;

        if !parent_inode.is_directory() {
            return Err(Ext4Error::InvalidDirectory(format!(
                "'{}' is not a directory",
                parent_path
            )));
        }

        // Check if directory already exists
        let directory = self.read_directory(parent_inode_num)?;
        if directory.find_entry(dirname).is_some() {
            return Err(Ext4Error::InvalidOperation(format!(
                "Directory '{}' already exists",
                dirname
            )));
        }

        // For now, we'll just return an error since we haven't implemented the allocation methods
        // return Err(Ext4Error::InvalidOperation("Creating directories is not fully implemented yet".to_string()));

        // The following would be the implementation once allocation methods are implemented:
        // 1. Allocate a new inode for the directory
        let new_inode_num = self.allocate_inode()?;

        // 2. Create a new directory inode
        let mut new_inode = Inode::default();
        new_inode.mode = 0x4180; // Directory with 0755 permissions
        new_inode.links_count = 2; // "." and parent link

        // 3. Allocate a block for the directory data
        let block_num = self.allocate_block()?;
        new_inode.block[0] = block_num;
        new_inode.blocks = self.superblock.block_size() / 512;
        new_inode.size = self.superblock.block_size();

        // 4. Write the directory entries for "." and ".."
        self.write_directory_entries(block_num, new_inode_num, parent_inode_num)?;

        // 5. Write the new inode to disk
        self.write_inode(new_inode_num, &new_inode)?;

        // 6. Add an entry to the parent directory
        self.add_directory_entry(parent_inode_num, dirname, new_inode_num, 2)?; // 2 = directory

        // 7. Update the parent inode's link count (for the new ".." entry)
        let updated_parent = parent_inode.clone();
        // updated_parent.links_count += 1;
        self.write_inode(parent_inode_num, &updated_parent)?;

        // 8. Update superblock and block group descriptors
        self.superblock.free_blocks_count -= 1;
        self.superblock.free_inodes_count -= 1;
        self.write_superblock()?;

        Ok(())
    }

    /// Allocate a new inode.
    fn allocate_inode(&mut self) -> Result<u32, Ext4Error> {
        // Iterate through each block group to find a free inode
        for (group_idx, block_group) in self.block_groups.iter().enumerate() {
            let inode_bitmap_block = block_group.inode_bitmap;
            let block_size = self.superblock.block_size();

            // Read the inode bitmap
            let mut file_clone = self.file.try_clone()?;
            file_clone.seek(SeekFrom::Start((inode_bitmap_block * block_size) as u64))?;

            let mut bitmap = vec![0u8; block_size as usize];
            file_clone.read_exact(&mut bitmap)?;

            // Search for a free inode (bit set to 0)
            for byte_idx in 0..block_size as usize {
                if bitmap[byte_idx] != 0xFF {
                    // If not all bits are set
                    for bit_idx in 0..8 {
                        if (bitmap[byte_idx] & (1 << bit_idx)) == 0 {
                            // Found a free inode
                            let inode_idx = byte_idx * 8 + bit_idx;

                            // Make sure it's within the valid range
                            if inode_idx < self.superblock.inodes_per_group as usize {
                                // Mark the inode as used (set bit to 1)
                                bitmap[byte_idx] |= 1 << bit_idx;

                                // Write the updated bitmap back to disk
                                file_clone.seek(SeekFrom::Start(
                                    (inode_bitmap_block * block_size) as u64,
                                ))?;
                                file_clone.write_all(&bitmap)?;

                                // Calculate the global inode number
                                let inode_num = group_idx as u32 * self.superblock.inodes_per_group
                                    + inode_idx as u32
                                    + 1;

                                // Update the block group descriptor
                                let mut bg = self.block_groups[group_idx].clone();
                                bg.free_inodes_count -= 1;
                                // We would update the block group descriptor on disk here
                                self.block_groups[group_idx] = bg;

                                return Ok(inode_num);
                            }
                        }
                    }
                }
            }
        }

        // No free inodes found
        Err(Ext4Error::NoSpace("No free inodes available".to_string()))
    }

    /// Allocate a new block.
    fn allocate_block(&mut self) -> Result<u32, Ext4Error> {
        // Iterate through each block group to find a free block
        for (group_idx, block_group) in self.block_groups.iter().enumerate() {
            let block_bitmap_block = block_group.block_bitmap;
            let block_size = self.superblock.block_size();

            // Read the block bitmap
            let mut file_clone = self.file.try_clone()?;
            file_clone.seek(SeekFrom::Start((block_bitmap_block * block_size) as u64))?;

            let mut bitmap = vec![0u8; block_size as usize];
            file_clone.read_exact(&mut bitmap)?;

            // Search for a free block (bit set to 0)
            for byte_idx in 0..block_size as usize {
                if bitmap[byte_idx] != 0xFF {
                    // If not all bits are set
                    for bit_idx in 0..8 {
                        if (bitmap[byte_idx] & (1 << bit_idx)) == 0 {
                            // Found a free block
                            let block_idx = byte_idx * 8 + bit_idx;

                            // Make sure it's within the valid range
                            if block_idx < self.superblock.blocks_per_group as usize {
                                // Mark the block as used (set bit to 1)
                                bitmap[byte_idx] |= 1 << bit_idx;

                                // Write the updated bitmap back to disk
                                file_clone.seek(SeekFrom::Start(
                                    (block_bitmap_block * block_size) as u64,
                                ))?;
                                file_clone.write_all(&bitmap)?;

                                // Calculate the global block number
                                let block_num = group_idx as u32 * self.superblock.blocks_per_group
                                    + block_idx as u32
                                    + (if group_idx == 0 {
                                        self.superblock.first_data_block
                                    } else {
                                        0
                                    });

                                // Update the block group descriptor
                                let mut bg = self.block_groups[group_idx].clone();
                                bg.free_blocks_count -= 1;
                                // We would update the block group descriptor on disk here
                                self.block_groups[group_idx] = bg;

                                return Ok(block_num);
                            }
                        }
                    }
                }
            }
        }

        // No free blocks found
        Err(Ext4Error::NoSpace("No free blocks available".to_string()))
    }

    /// Free an inode.
    fn free_inode(&mut self, inode_num: u32) -> Result<(), Ext4Error> {
        if inode_num == 0 || inode_num > self.superblock.inodes_count {
            return Err(Ext4Error::InvalidInode(format!(
                "Invalid inode number: {}",
                inode_num
            )));
        }

        // Calculate which block group this inode belongs to
        let group_idx = (inode_num - 1) / self.superblock.inodes_per_group;
        if group_idx as usize >= self.block_groups.len() {
            return Err(Ext4Error::InvalidInode(format!(
                "Invalid block group index: {}",
                group_idx
            )));
        }

        // Get the block group and inode bitmap block
        let block_group = &self.block_groups[group_idx as usize];
        let inode_bitmap_block = block_group.inode_bitmap;
        let block_size = self.superblock.block_size();

        // Calculate the index within the block group
        let index_in_group = (inode_num - 1) % self.superblock.inodes_per_group;
        let byte_idx = (index_in_group / 8) as usize;
        let bit_idx = (index_in_group % 8) as u8;

        // Read the inode bitmap
        let mut file_clone = self.file.try_clone()?;
        file_clone.seek(SeekFrom::Start((inode_bitmap_block * block_size) as u64))?;

        let mut bitmap = vec![0u8; block_size as usize];
        file_clone.read_exact(&mut bitmap)?;

        // Check if the inode is already free
        if (bitmap[byte_idx] & (1 << bit_idx)) == 0 {
            return Err(Ext4Error::InvalidOperation(format!(
                "Inode {} is already free",
                inode_num
            )));
        }

        // Mark the inode as free (clear the bit)
        bitmap[byte_idx] &= !(1 << bit_idx);

        // Write the updated bitmap back to disk
        file_clone.seek(SeekFrom::Start((inode_bitmap_block * block_size) as u64))?;
        file_clone.write_all(&bitmap)?;

        // Update the block group descriptor
        let mut bg = self.block_groups[group_idx as usize].clone();
        bg.free_inodes_count += 1;
        self.block_groups[group_idx as usize] = bg;

        // Update the block group descriptor on disk
        // This would require writing the updated block group descriptor to disk
        // For simplicity, we'll skip this step for now

        Ok(())
    }

    /// Free a block.
    fn free_block(&mut self, block_num: u32) -> Result<(), Ext4Error> {
        if block_num < self.superblock.first_data_block || block_num >= self.superblock.blocks_count
        {
            return Err(Ext4Error::InvalidBlock(format!(
                "Invalid block number: {}",
                block_num
            )));
        }

        // Calculate which block group this block belongs to
        let group_idx =
            (block_num - self.superblock.first_data_block) / self.superblock.blocks_per_group;
        if group_idx as usize >= self.block_groups.len() {
            return Err(Ext4Error::InvalidBlock(format!(
                "Invalid block group index: {}",
                group_idx
            )));
        }

        // Get the block group and block bitmap block
        let block_group = &self.block_groups[group_idx as usize];
        let block_bitmap_block = block_group.block_bitmap;
        let block_size = self.superblock.block_size();

        // Calculate the index within the block group
        let index_in_group =
            (block_num - self.superblock.first_data_block) % self.superblock.blocks_per_group;
        let byte_idx = (index_in_group / 8) as usize;
        let bit_idx = (index_in_group % 8) as u8;

        // Read the block bitmap
        let mut file_clone = self.file.try_clone()?;
        file_clone.seek(SeekFrom::Start((block_bitmap_block * block_size) as u64))?;

        let mut bitmap = vec![0u8; block_size as usize];
        file_clone.read_exact(&mut bitmap)?;

        // Check if the block is already free
        if (bitmap[byte_idx] & (1 << bit_idx)) == 0 {
            return Err(Ext4Error::InvalidOperation(format!(
                "Block {} is already free",
                block_num
            )));
        }

        // Mark the block as free (clear the bit)
        bitmap[byte_idx] &= !(1 << bit_idx);

        // Write the updated bitmap back to disk
        file_clone.seek(SeekFrom::Start((block_bitmap_block * block_size) as u64))?;
        file_clone.write_all(&bitmap)?;

        // Update the block group descriptor
        let mut bg = self.block_groups[group_idx as usize].clone();
        bg.free_blocks_count += 1;
        self.block_groups[group_idx as usize] = bg;

        // Update the block group descriptor on disk
        // This would require writing the updated block group descriptor to disk
        // For simplicity, we'll skip this step for now

        Ok(())
    }

    /// Add an entry to a directory.
    fn add_directory_entry(
        &mut self,
        dir_inode_num: u32,
        name: &str,
        inode_num: u32,
        file_type: u8,
    ) -> Result<(), Ext4Error> {
        // Validate inputs
        if name.is_empty() || name.len() > 255 {
            return Err(Ext4Error::InvalidOperation(format!(
                "Invalid filename length: {}",
                name.len()
            )));
        }

        // Read the directory inode
        let mut dir_inode = self.read_inode(dir_inode_num)?;
        if !dir_inode.is_directory() {
            return Err(Ext4Error::InvalidDirectory(format!(
                "Inode {} is not a directory",
                dir_inode_num
            )));
        }

        // Calculate entry size (header + name + padding to 4-byte alignment)
        let name_len = name.len();
        let entry_size = 8 + name_len; // 8 bytes for header, name_len for name
        let padding = (4 - (entry_size % 4)) % 4; // Padding to align to 4 bytes
        let total_size = entry_size + padding;

        // Read the directory data
        let block_size = self.superblock.block_size() as usize;

        // Iterate through directory blocks to find space for the new entry
        for i in 0..12 {
            // Only handling direct blocks for now
            if dir_inode.block[i] == 0 {
                // Allocate a new block for the directory
                let block_num = self.allocate_block()?;
                dir_inode.block[i] = block_num;

                // Initialize the block with zeros
                let mut file_clone = self.file.try_clone()?;
                file_clone.seek(SeekFrom::Start(
                    (block_num * self.superblock.block_size()) as u64,
                ))?;
                let zeros = vec![0u8; block_size];
                file_clone.write_all(&zeros)?;

                // Update directory inode size and blocks
                if i == 0 {
                    dir_inode.size = block_size as u32;
                } else {
                    dir_inode.size += block_size as u32;
                }
                dir_inode.blocks += self.superblock.block_size() / 512;

                // Write the entry at the beginning of the new block
                let mut file_clone = self.file.try_clone()?;
                file_clone.seek(SeekFrom::Start(
                    (block_num * self.superblock.block_size()) as u64,
                ))?;

                use byteorder::{LittleEndian, WriteBytesExt};

                // Write entry header
                file_clone.write_u32::<LittleEndian>(inode_num)?; // inode number
                file_clone.write_u16::<LittleEndian>(block_size as u16)?; // rec_len (use entire block)
                file_clone.write_u8(name_len as u8)?; // name_len
                file_clone.write_u8(file_type)?; // file_type

                // Write filename
                file_clone.write_all(name.as_bytes())?;

                // Write padding
                if padding > 0 {
                    file_clone.write_all(&vec![0u8; padding])?;
                }

                // Update the directory inode on disk
                self.write_inode(dir_inode_num, &dir_inode)?;

                return Ok(());
            }

            // Read existing block data
            let block_num = dir_inode.block[i];
            let mut file_clone = self.file.try_clone()?;
            file_clone.seek(SeekFrom::Start(
                (block_num * self.superblock.block_size()) as u64,
            ))?;

            let mut block_data = vec![0u8; block_size];
            file_clone.read_exact(&mut block_data)?;

            // Parse directory entries to find available space
            let mut offset = 0;
            while offset < block_size {
                // Read entry header
                if offset + 8 > block_size {
                    break;
                }

                use byteorder::{LittleEndian, ReadBytesExt};
                let mut cursor = std::io::Cursor::new(&block_data[offset..]);

                let entry_inode = cursor.read_u32::<LittleEndian>()?;
                let rec_len = cursor.read_u16::<LittleEndian>()? as usize;
                let name_len = cursor.read_u8()? as usize;
                let _file_type = cursor.read_u8()?;

                if entry_inode == 0 || rec_len == 0 {
                    // Found an empty or deleted entry, use it
                    let mut file_clone = self.file.try_clone()?;
                    file_clone.seek(SeekFrom::Start(
                        (block_num * self.superblock.block_size() + offset as u32) as u64,
                    ))?;

                    use byteorder::{LittleEndian, WriteBytesExt};

                    // Write entry header
                    file_clone.write_u32::<LittleEndian>(inode_num)?;
                    file_clone.write_u16::<LittleEndian>(rec_len as u16)?;
                    file_clone.write_u8(name_len as u8)?;
                    file_clone.write_u8(file_type)?;

                    // Write filename
                    file_clone.write_all(name.as_bytes())?;

                    return Ok(());
                }

                // Calculate actual entry size (header + name + padding)
                let actual_size = 8 + name_len;
                let entry_padding = (4 - (actual_size % 4)) % 4;
                let min_size = actual_size + entry_padding;

                // Check if this entry has enough extra space to split
                if rec_len >= min_size + total_size {
                    // Split the entry
                    let new_rec_len = min_size;
                    let remaining_space = rec_len - new_rec_len;

                    // Update the current entry's rec_len
                    let mut file_clone = self.file.try_clone()?;
                    file_clone.seek(SeekFrom::Start(
                        (block_num * self.superblock.block_size() + offset as u32 + 4) as u64,
                    ))?;
                    file_clone.write_u16::<LittleEndian>(new_rec_len as u16)?;

                    // Write the new entry after the current one
                    let new_offset = offset + new_rec_len;
                    file_clone.seek(SeekFrom::Start(
                        (block_num * self.superblock.block_size() + new_offset as u32) as u64,
                    ))?;

                    // Write entry header
                    file_clone.write_u32::<LittleEndian>(inode_num)?;
                    file_clone.write_u16::<LittleEndian>(remaining_space as u16)?;
                    file_clone.write_u8(name.len() as u8)?;
                    file_clone.write_u8(file_type)?;

                    // Write filename
                    file_clone.write_all(name.as_bytes())?;

                    return Ok(());
                }

                // Move to the next entry
                offset += rec_len;
            }
        }

        // No space found in existing blocks
        return Err(Ext4Error::NoSpace(
            "No space left in directory blocks".to_string(),
        ));
    }

    /// Remove an entry from a directory.
    fn remove_directory_entry(&mut self, dir_inode_num: u32, name: &str) -> Result<(), Ext4Error> {
        // Validate inputs
        if name.is_empty() {
            return Err(Ext4Error::InvalidOperation(
                "Empty filename is not allowed".to_string(),
            ));
        }

        // Read the directory inode
        let mut dir_inode = self.read_inode(dir_inode_num)?;
        if !dir_inode.is_directory() {
            return Err(Ext4Error::InvalidDirectory(format!(
                "Inode {} is not a directory",
                dir_inode_num
            )));
        }

        // Read the directory data
        let block_size = self.superblock.block_size() as usize;

        // Iterate through directory blocks to find the entry
        for i in 0..12 {
            // Only handling direct blocks for now
            if dir_inode.block[i] == 0 {
                continue; // Skip empty blocks
            }

            // Read existing block data
            let block_num = dir_inode.block[i];
            let mut file_clone = self.file.try_clone()?;
            file_clone.seek(SeekFrom::Start(
                (block_num * self.superblock.block_size()) as u64,
            ))?;

            let mut block_data = vec![0u8; block_size];
            file_clone.read_exact(&mut block_data)?;

            // Parse directory entries to find the one to remove
            let mut offset = 0;
            let mut prev_offset = 0;
            let mut prev_rec_len = 0;

            while offset < block_size {
                // Read entry header
                if offset + 8 > block_size {
                    break;
                }

                use byteorder::{LittleEndian, ReadBytesExt};
                let mut cursor = std::io::Cursor::new(&block_data[offset..]);

                let entry_inode = cursor.read_u32::<LittleEndian>()?;
                let rec_len = cursor.read_u16::<LittleEndian>()? as usize;
                let name_len = cursor.read_u8()? as usize;
                let _file_type = cursor.read_u8()?;

                // Skip deleted entries
                if entry_inode == 0 || rec_len == 0 {
                    prev_offset = offset;
                    prev_rec_len = rec_len;
                    offset += rec_len;
                    continue;
                }

                // Check if this is the entry we want to remove
                if name_len == name.len() {
                    let entry_name =
                        String::from_utf8_lossy(&block_data[offset + 8..offset + 8 + name_len]);
                    if entry_name == name {
                        // Found the entry to remove

                        // Strategy 1: Mark as deleted by setting inode to 0
                        let mut file_clone = self.file.try_clone()?;
                        file_clone.seek(SeekFrom::Start(
                            (block_num * self.superblock.block_size() + offset as u32) as u64,
                        ))?;

                        use byteorder::{LittleEndian, WriteBytesExt};
                        file_clone.write_u32::<LittleEndian>(0)?; // Set inode to 0 to mark as deleted

                        // Strategy 2: If this is not the last entry in the block, merge with previous entry
                        if offset + rec_len < block_size && prev_rec_len > 0 {
                            // There's another entry after this one, so extend the previous entry
                            let mut file_clone = self.file.try_clone()?;
                            file_clone.seek(SeekFrom::Start(
                                (block_num * self.superblock.block_size() + prev_offset as u32 + 4)
                                    as u64,
                            ))?;

                            file_clone
                                .write_u16::<LittleEndian>((prev_rec_len + rec_len) as u16)?;
                        }

                        // Strategy 3: If this is the last entry in the block, adjust the previous entry's rec_len
                        if offset + rec_len >= block_size && prev_rec_len > 0 {
                            let mut file_clone = self.file.try_clone()?;
                            file_clone.seek(SeekFrom::Start(
                                (block_num * self.superblock.block_size() + prev_offset as u32 + 4)
                                    as u64,
                            ))?;

                            file_clone
                                .write_u16::<LittleEndian>((block_size - prev_offset) as u16)?;
                        }

                        // If this is the only entry in the block, we could potentially free the block
                        // but for simplicity, we'll just leave it marked as deleted

                        return Ok(());
                    }
                }

                // Move to the next entry
                prev_offset = offset;
                prev_rec_len = rec_len;
                offset += rec_len;
            }
        }

        // Entry not found
        Err(Ext4Error::InvalidFile(format!(
            "Directory entry '{}' not found",
            name
        )))
    }

    /// Write an inode back to disk.
    fn write_inode(&mut self, inode_num: u32, inode: &Inode) -> Result<(), Ext4Error> {
        if inode_num == 0 || inode_num > self.superblock.inodes_count {
            return Err(Ext4Error::InvalidInode(format!(
                "Invalid inode number: {}",
                inode_num
            )));
        }

        let group_idx = (inode_num - 1) / self.superblock.inodes_per_group;
        if group_idx as usize >= self.block_groups.len() {
            return Err(Ext4Error::InvalidInode(format!(
                "Invalid block group index: {}",
                group_idx
            )));
        }

        let block_group = &self.block_groups[group_idx as usize];
        let index = (inode_num - 1) % self.superblock.inodes_per_group;
        let offset = block_group.inode_table * self.superblock.block_size() + index * 256; // Assuming inode size is 256 bytes

        let mut file_clone = self.file.try_clone()?;
        file_clone.seek(SeekFrom::Start(offset as u64))?;

        // For now, we'll just return an error since writing to disk is not fully implemented
        // return Err(Ext4Error::InvalidOperation("Writing inodes to disk is not fully implemented yet".to_string()));

        // The following would be the implementation for writing the inode:
        use byteorder::{LittleEndian, WriteBytesExt};

        file_clone.write_u16::<LittleEndian>(inode.mode)?;
        file_clone.write_u16::<LittleEndian>(inode.uid)?;
        file_clone.write_u32::<LittleEndian>(inode.size)?;
        file_clone.write_u32::<LittleEndian>(inode.atime)?;
        file_clone.write_u32::<LittleEndian>(inode.ctime)?;
        file_clone.write_u32::<LittleEndian>(inode.mtime)?;
        file_clone.write_u32::<LittleEndian>(inode.dtime)?;
        file_clone.write_u16::<LittleEndian>(inode.gid)?;
        file_clone.write_u16::<LittleEndian>(inode.links_count)?;
        file_clone.write_u32::<LittleEndian>(inode.blocks)?;
        file_clone.write_u32::<LittleEndian>(inode.flags)?;
        file_clone.write_u32::<LittleEndian>(inode.osd1)?;

        for i in 0..15 {
            file_clone.write_u32::<LittleEndian>(inode.block[i])?;
        }

        file_clone.write_u32::<LittleEndian>(inode.generation)?;
        file_clone.write_u32::<LittleEndian>(inode.file_acl)?;
        file_clone.write_u32::<LittleEndian>(inode.dir_acl)?;
        file_clone.write_u32::<LittleEndian>(inode.faddr)?;
        file_clone.write_all(&inode.osd2)?;

        Ok(())
    }

    /// Write the "." and ".." directory entries to a newly allocated directory block.
    fn write_directory_entries(
        &mut self,
        block_num: u32,
        dir_inode_num: u32,
        parent_inode_num: u32,
    ) -> Result<(), Ext4Error> {
        let block_size = self.superblock.block_size();
        let offset = block_num * block_size;

        let mut file_clone = self.file.try_clone()?;
        file_clone.seek(SeekFrom::Start(offset as u64))?;

        use byteorder::{LittleEndian, WriteBytesExt};

        // Write "." entry (points to this directory)
        // inode (4 bytes)
        file_clone.write_u32::<LittleEndian>(dir_inode_num)?;
        // rec_len (2 bytes) - 12 bytes for this entry (8 bytes header + 1 byte name + 3 bytes padding)
        file_clone.write_u16::<LittleEndian>(12)?;
        // name_len (1 byte)
        file_clone.write_u8(1)?;
        // file_type (1 byte) - 2 is directory
        file_clone.write_u8(2)?;
        // name (1 byte + padding)
        file_clone.write_all(b".")?;
        // padding to 4-byte alignment
        file_clone.write_all(&[0, 0, 0])?;

        // Write ".." entry (points to parent directory)
        // inode (4 bytes)
        file_clone.write_u32::<LittleEndian>(parent_inode_num)?;
        // rec_len (2 bytes) - remaining space in the block
        file_clone.write_u16::<LittleEndian>((block_size - 12) as u16)?;
        // name_len (1 byte)
        file_clone.write_u8(2)?;
        // file_type (1 byte) - 2 is directory
        file_clone.write_u8(2)?;
        // name (2 bytes + padding)
        file_clone.write_all(b"..")?;
        // padding to 4-byte alignment
        file_clone.write_all(&[0, 0])?;

        // Fill the rest of the block with zeros
        let remaining = block_size as usize - 24; // 12 bytes for "." + 12 bytes for ".."
        if remaining > 0 {
            let zeros = vec![0u8; remaining];
            file_clone.write_all(&zeros)?;
        }

        Ok(())
    }

    /// Write the superblock back to disk.
    fn write_superblock(&mut self) -> Result<(), Ext4Error> {
        let mut file_clone = self.file.try_clone()?;

        // The primary superblock is located at offset 1024 bytes
        file_clone.seek(SeekFrom::Start(1024))?;

        // For now, we'll just return an error since writing to disk is not fully implemented
        // return Err(Ext4Error::InvalidOperation("Writing superblock to disk is not fully implemented yet".to_string()));

        // The following would be the implementation for writing the superblock:
        use byteorder::{LittleEndian, WriteBytesExt};

        file_clone.write_u32::<LittleEndian>(self.superblock.inodes_count)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.blocks_count)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.r_blocks_count)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.free_blocks_count)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.free_inodes_count)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.first_data_block)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.log_block_size)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.log_block_size)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.blocks_per_group)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.frags_per_group)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.inodes_per_group)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.mtime)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.wtime)?;
        file_clone.write_u16::<LittleEndian>(self.superblock.mnt_count)?;
        file_clone.write_u16::<LittleEndian>(self.superblock.max_mnt_count)?;
        file_clone.write_u16::<LittleEndian>(self.superblock.magic)?;
        file_clone.write_u16::<LittleEndian>(self.superblock.state)?;
        file_clone.write_u16::<LittleEndian>(self.superblock.errors)?;
        file_clone.write_u16::<LittleEndian>(self.superblock.minor_rev_level)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.lastcheck)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.checkinterval)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.creator_os)?;
        file_clone.write_u32::<LittleEndian>(self.superblock.rev_level)?;
        file_clone.write_u16::<LittleEndian>(self.superblock.def_resuid)?;
        file_clone.write_u16::<LittleEndian>(self.superblock.def_resgid)?;

        // Write the rest of the superblock fields...

        // Also update backup superblocks in other block groups
        if self.superblock.rev_level >= 1 {
            // For sparse superblock feature, backups are in block groups 0, 1, and powers of 3, 5, and 7
            let mut bg_idx = 1;
            while bg_idx < self.block_groups.len() as u32 {
                let offset =
                    bg_idx * self.superblock.blocks_per_group * self.superblock.block_size() + 1024;
                file_clone.seek(SeekFrom::Start(offset as u64))?;
                // Write the same superblock data here
                // ...

                // Next backup location
                if bg_idx == 1 {
                    bg_idx = 3;
                } else if bg_idx % 3 == 0 {
                    bg_idx *= 5 / 3;
                } else if bg_idx % 5 == 0 {
                    bg_idx *= 7 / 5;
                } else {
                    break;
                }
            }
        }

        Ok(())
    }
}
