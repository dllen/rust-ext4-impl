//! File operations for ext4 filesystem.

use std::io::{self, Read, Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};
use crate::error::Ext4Error;
use crate::inode::Inode;

/// The file of an ext4 filesystem.
#[derive(Debug, Clone)]
pub struct File {
    /// The inode of the file.
    pub inode: Inode,
    /// The current position in the file.
    pub position: u64,
}

impl File {
    /// Create a new file from an inode.
    pub fn new(inode: Inode) -> Self {
        File { inode, position: 0 }
    }

    /// Read data from the file.
    pub fn read<R: Read + Seek>(&mut self, reader: &mut R, buffer: &mut [u8], block_size: u32) -> Result<usize, Ext4Error> {
        if !self.inode.is_file() {
            return Err(Ext4Error::InvalidFile("Not a regular file".to_string()));
        }

        let file_size = self.inode.get_size();
        if self.position >= file_size {
            return Ok(0);
        }

        // Calculate how many bytes we can read
        let bytes_to_read = std::cmp::min(buffer.len() as u64, file_size - self.position) as usize;
        if bytes_to_read == 0 {
            return Ok(0);
        }

        // Calculate which block to start reading from
        let start_block = (self.position / block_size as u64) as usize;
        let offset_in_block = (self.position % block_size as u64) as usize;
        
        // Read data from direct blocks first
        let mut bytes_read = 0;
        let mut remaining = bytes_to_read;
        
        // Handle direct blocks (0-11)
        for i in start_block..12 {
            if i >= 12 || bytes_read >= bytes_to_read {
                break;
            }
            
            if self.inode.block[i] == 0 {
                // Sparse file, fill with zeros
                let zeros_to_write = std::cmp::min(remaining, block_size as usize - offset_in_block);
                for j in 0..zeros_to_write {
                    buffer[bytes_read + j] = 0;
                }
                bytes_read += zeros_to_write;
                remaining -= zeros_to_write;
                self.position += zeros_to_write as u64;
                continue;
            }
            
            // Seek to the block
            let block_pos = self.inode.block[i] as u64 * block_size as u64;
            reader.seek(SeekFrom::Start(block_pos + offset_in_block as u64))?;
            
            // Read data from the block
            let to_read = std::cmp::min(remaining, block_size as usize - offset_in_block);
            let n = reader.read(&mut buffer[bytes_read..bytes_read + to_read])?;
            
            bytes_read += n;
            remaining -= n;
            self.position += n as u64;
            
            if n < to_read {
                // End of file or error
                break;
            }
        }
        
        // TODO: Handle indirect blocks (12), double indirect blocks (13), and triple indirect blocks (14)
        
        Ok(bytes_read)
    }
    
    /// Read data from indirect blocks.
    fn read_indirect<R: Read + Seek>(
        &mut self,
        reader: &mut R,
        buffer: &mut [u8],
        bytes_read: &mut usize,
        remaining: &mut usize,
        block_size: u32,
        indirect_block: u32,
        level: u32,
    ) -> Result<(), Ext4Error> {
        if indirect_block == 0 || *remaining == 0 {
            return Ok(());
        }
        
        // Number of block pointers per block
        let pointers_per_block = block_size as usize / 4;
        
        // Read the indirect block
        let mut block_pointers = vec![0u32; pointers_per_block];
        reader.seek(SeekFrom::Start(indirect_block as u64 * block_size as u64))?;
        
        for i in 0..pointers_per_block {
            block_pointers[i] = reader.read_u32::<LittleEndian>()?;
        }
        
        // Process the block pointers
        for &ptr in &block_pointers {
            if ptr == 0 || *remaining == 0 {
                continue;
            }
            
            if level > 1 {
                // Recursively process the next level of indirection
                self.read_indirect(reader, buffer, bytes_read, remaining, block_size, ptr, level - 1)?;
            } else {
                // Read data from the data block
                reader.seek(SeekFrom::Start(ptr as u64 * block_size as u64))?;
                
                let to_read = std::cmp::min(*remaining, block_size as usize);
                let n = reader.read(&mut buffer[*bytes_read..*bytes_read + to_read])?;
                
                *bytes_read += n;
                *remaining -= n;
                self.position += n as u64;
                
                if n < to_read {
                    // End of file or error
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Seek to a position in the file.
    pub fn seek(&mut self, position: u64) -> Result<u64, Ext4Error> {
        let file_size = self.inode.get_size();
        if position > file_size {
            return Err(Ext4Error::InvalidFile("Seek position beyond end of file".to_string()));
        }
    
        self.position = position;
        Ok(self.position)
    }
}