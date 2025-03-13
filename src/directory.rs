//! Directory entry for ext4 filesystem.

use std::io::{Read, Seek, SeekFrom, Write};
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

     /// 将目录项写入到文件中
     pub fn write<W: Write + Seek>(&self, writer: &mut W, block_size: u32) -> Result<(), Ext4Error> {
        // 遍历目录使用的所有数据块
        for i in 0..12 {
            let block_num = self.inode.block[i];
            if block_num == 0 {
                break;
            }

            // 定位到数据块位置
            writer.seek(SeekFrom::Start((block_num * block_size) as u64))?;

            let mut block_data = vec![0u8; block_size as usize];
            let mut offset = 0;

            // 写入目录项到数据块
            for entry in &self.entries {
                if offset + 8 + entry.name.len() > block_size as usize {
                    // 当前块空间不足，写入当前块并继续下一个块
                    writer.write_all(&block_data[..offset])?;
                    break;
                }

                // 写入目录项头部
                use byteorder::{LittleEndian, WriteBytesExt};
                let mut cursor = std::io::Cursor::new(&mut block_data[offset..]);
                cursor.write_u32::<LittleEndian>(entry.inode)?;
                cursor.write_u16::<LittleEndian>(entry.rec_len)?;
                cursor.write_u8(entry.name_len)?;
                cursor.write_u8(entry.file_type)?;

                // 写入文件名
                let name_bytes = entry.name.as_bytes();
                block_data[offset + 8..offset + 8 + name_bytes.len()].copy_from_slice(name_bytes);

                offset += entry.rec_len as usize;
            }

            // 写入最后一个块的数据
            if offset > 0 {
                writer.write_all(&block_data[..offset])?;
            }
        }

        Ok(())
    }

    /// Read a directory from a reader.
    pub fn read<R: Read + Seek>(reader: &mut R, inode: Inode, block_size: u32) -> Result<Self, Ext4Error> {
        if !inode.is_directory() {
            return Err(Ext4Error::InvalidDirectory("Not a directory".to_string()));
        }

        let mut entries = Vec::new();
        
        // 遍历目录的所有直接数据块
        for i in 0..12 {
            let block_num = inode.block[i];
            if block_num == 0 {
                break;
            }

            // 定位到数据块的位置
            reader.seek(SeekFrom::Start((block_num * block_size) as u64))?;
            
            // 读取数据块，处理可能的 EOF 情况
            let mut block_data = vec![0u8; block_size as usize];
            match reader.read(&mut block_data) {
                Ok(n) if n == 0 => break,  // EOF
                Ok(n) if n < block_size as usize => {
                    block_data.truncate(n);  // 只保留实际读取的数据
                }
                Ok(_) => {},  // 读取了完整的块
                Err(e) => return Err(Ext4Error::Io(e)),
            }

            // 解析数据块中的目录项
            let mut offset = 0;
            while offset < block_data.len() {
                if offset + 8 > block_data.len() {
                    break;
                }

                // 读取目录项头部
                use byteorder::{LittleEndian, ReadBytesExt};
                let mut cursor = std::io::Cursor::new(&block_data[offset..]);
                
                let entry_inode = cursor.read_u32::<LittleEndian>()?;
                if entry_inode == 0 {
                    // 跳过已删除的目录项
                    offset += 8;
                    continue;
                }

                let rec_len = cursor.read_u16::<LittleEndian>()?;
                let name_len = cursor.read_u8()?;
                let file_type = cursor.read_u8()?;

                // 确保名称长度有效
                if name_len == 0 || offset + 8 + name_len as usize > block_size as usize {
                    break;
                }

                // 读取文件名
                let name_bytes = &block_data[offset + 8..offset + 8 + name_len as usize];
                let name = String::from_utf8_lossy(name_bytes).to_string();

                // 创建目录项
                entries.push(DirectoryEntry {
                    inode: entry_inode,
                    rec_len,
                    name_len,
                    file_type,
                    name,
                });

                // 移动到下一个目录项
                offset += rec_len as usize;
                if offset >= block_size as usize || rec_len == 0 {
                    break;
                }
            }
        }

        Ok(Directory { inode, entries })
    }

    /// Find an entry by name.
    pub fn find_entry(&self, name: &str) -> Option<&DirectoryEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }
}