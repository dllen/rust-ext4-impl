//! Directory entry for ext4 filesystem.

use std::io::{Read, Seek, SeekFrom, Write};
use crate::error::Ext4Error;
use crate::inode::Inode;
use byteorder::{LittleEndian, WriteBytesExt};

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

     /// 打印目录的详细信息
     pub fn print_details(&self) {
        println!("\n目录详细信息:");
        println!("========================================");
        
        // 打印 inode 信息
        println!("Inode 信息:");
        println!("  模式:       {:o}", self.inode.mode);
        println!("  大小:       {} 字节", self.inode.size);
        println!("  链接数:     {}", self.inode.links_count);
        println!("  数据块数:   {}", self.inode.blocks);
        println!("  访问时间:   {}", self.inode.atime);
        println!("  修改时间:   {}", self.inode.mtime);
        println!("  创建时间:   {}", self.inode.ctime);
        
        // 打印数据块信息
        println!("\n数据块列表:");
        for (i, block) in self.inode.block.iter().enumerate() {
            if *block != 0 {
                println!("  块 #{}: {}", i, block);
            }
        }
        
        // 打印目录项信息
        println!("\n目录项列表 (共 {} 项):", self.entries.len());
        println!("----------------------------------------");
        for (idx, entry) in self.entries.iter().enumerate() {
            println!("条目 #{}", idx + 1);
            println!("  inode:     {}", entry.inode);
            println!("  rec_len:   {} 字节", entry.rec_len);
            println!("  name_len:  {} 字节", entry.name_len);
            println!("  file_type: {} ({})", entry.file_type, match entry.file_type {
                0 => "未知",
                1 => "普通文件",
                2 => "目录",
                3 => "字符设备",
                4 => "块设备",
                5 => "FIFO",
                6 => "套接字",
                7 => "符号链接",
                _ => "未知类型"
            });
            println!("  name:      {}", entry.name);
            println!("----------------------------------------");
        }
        println!("========================================\n");
    }

     /// 将目录项写入到文件中
     pub fn write<W: Write + Seek>(&self, writer: &mut W, block_size: u32) -> Result<(), Ext4Error> {
        println!("开始写入目录项，总条目数: {}", self.entries.len());
        
        // 确保至少有一个数据块
        if self.inode.block[0] == 0 {
            return Err(Ext4Error::InvalidDirectory("目录没有分配数据块".to_string()));
        }

        // 只使用第一个数据块来存储目录项
        let block_num = self.inode.block[0];
        println!("使用数据块 #{}", block_num);

        // 定位到数据块位置
        writer.seek(SeekFrom::Start((block_num * block_size) as u64))?;

        // 创建一个新的数据块缓冲区
        let mut block_data = vec![0u8; block_size as usize];
        let mut offset = 0;

        // 写入所有目录项
        for (idx, entry) in self.entries.iter().enumerate() {
            println!("写入第 {} 个目录项: {}", idx + 1, entry.name);
            
            if offset + 8 + entry.name.len() > block_size as usize {
                return Err(Ext4Error::NoSpace("数据块空间不足".to_string()));
            }

            // 写入目录项头部
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

        // 一次性写入整个数据块
        writer.seek(SeekFrom::Start((block_num * block_size) as u64))?;
        writer.write_all(&block_data[..offset])?;
        
        // 如果有剩余空间，用0填充
        if offset < block_size as usize {
            let zeros = vec![0u8; block_size as usize - offset];
            writer.write_all(&zeros)?;
        }

        println!("目录项写入完成，总写入字节数: {}", offset);
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