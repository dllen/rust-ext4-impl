use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use rust_ext4_impl::Ext4Filesystem;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <ext4_image> [command] [args...]", args[0]);
        eprintln!("Commands:");
        eprintln!("  ls [path]                - List directory contents");
        eprintln!("  cat <path>               - Display file contents");
        eprintln!("  write <path> <local_file> - Write file to image");
        eprintln!("  mkdir <path>             - Create a new directory");
        eprintln!("  rm <path>                - Remove file or directory");
        eprintln!("  info                     - Display filesystem information");
        return Ok(());
    }
    
    let image_path = &args[1];
    let mut fs = Ext4Filesystem::mount(image_path)?;
    
    if args.len() < 3 {
        // Default to 'info' command
        print_filesystem_info(&fs);
        return Ok(());
    }
    
    let command = &args[2];
    
    match command.as_str() {
        "ls" => {
            let path = if args.len() > 3 { &args[3] } else { "/" };
            list_directory(&mut fs, path)?;
        },
        "cat" => {
            if args.len() < 4 {
                eprintln!("Error: 'cat' command requires a file path");
                return Ok(());
            }
            let path = &args[3];
            cat_file(&mut fs, path)?;
        },
        "write" => {
            if args.len() < 5 {
                eprintln!("Error: 'write' command requires target path and local file path");
                return Ok(());
            }
            let target_path = &args[3];
            let local_file_path = &args[4];
            write_file(&mut fs, target_path, local_file_path)?;
        },
        "mkdir" => {
            if args.len() < 4 {
                eprintln!("Error: 'mkdir' command requires a directory path");
                return Ok(());
            }
            let path = &args[3];
            create_directory(&mut fs, path)?;
        },
        "rm" => {
            if args.len() < 4 {
                eprintln!("Error: 'rm' command requires a path");
                return Ok(());
            }
            let path = &args[3];
            let force = args.len() > 4 && args[4] == "-f";
            remove_path(&mut fs, path, force)?;
        },
        "info" => {
            print_filesystem_info(&fs);
        },
        _ => {
            eprintln!("Unknown command: {}", command);
        }
    }
    
    Ok(())
}

fn print_filesystem_info(fs: &Ext4Filesystem) {
    let sb = fs.superblock();
    
    println!("Ext4 Filesystem Information:");
    println!("---------------------------");
    println!("Inodes count:      {}", sb.inodes_count);
    println!("Blocks count:      {}", sb.blocks_count);
    println!("Free blocks count: {}", sb.free_blocks_count);
    println!("Free inodes count: {}", sb.free_inodes_count);
    println!("Block size:        {} bytes", sb.block_size());
    println!("Inode size:        256 bytes");
    println!("Blocks per group:  {}", sb.blocks_per_group);
    println!("Inodes per group:  {}", sb.inodes_per_group);
    println!("Block groups:      {}", sb.block_groups_count());
    println!("---------------------------");
}

fn list_directory(fs: &mut Ext4Filesystem, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let inode_num = fs.find_by_path(path)?;
    let directory = fs.read_directory(inode_num)?;
    
    println!("Contents of directory '{}':", path);
    println!("---------------------------");
    println!("Inode    Type    Size    Name");
    
    for entry in &directory.entries {
        let inode = fs.read_inode(entry.inode)?;
        let type_str = if inode.is_directory() {
            "dir "
        } else if inode.is_file() {
            "file"
        } else if inode.is_symlink() {
            "link"
        } else {
            "other"
        };
        
        println!("{:<8} {:<6} {:<8} {}", entry.inode, type_str, inode.get_size(), entry.name);
    }
    
    Ok(())
}

fn cat_file(fs: &mut Ext4Filesystem, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let inode_num = fs.find_by_path(path)?;
    let inode = fs.read_inode(inode_num)?;
    
    if !inode.is_file() {
        eprintln!("Error: '{}' is not a regular file", path);
        return Ok(());
    }
    
    let file_size = inode.get_size() as usize;
    let mut buffer = vec![0u8; file_size];
    
    let bytes_read = fs.read_file(inode_num, &mut buffer, 0)?;
    
    if bytes_read < file_size {
        eprintln!("Warning: Only read {} bytes out of {} bytes", bytes_read, file_size);
    }
    
    io::stdout().write_all(&buffer[..bytes_read])?;
    
    Ok(())
}

/// Write a local file to the ext4 image
fn write_file(fs: &mut Ext4Filesystem, target_path: &str, local_file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Open the local file
    let mut local_file = File::open(local_file_path)?;
    
    // Read the local file content
    let mut buffer = Vec::new();
    local_file.read_to_end(&mut buffer)?;
    
    // Parse the target path to get parent directory and filename
    let parent_path = match target_path.rfind('/') {
        Some(pos) => {
            if pos == 0 {
                "/"
            } else {
                &target_path[..pos]
            }
        },
        None => "/"
    };
    
    let filename = match target_path.rfind('/') {
        Some(pos) => &target_path[pos + 1..],
        None => target_path
    };
    
    if filename.is_empty() {
        return Err("Invalid filename".into());
    }
    
    println!("Writing file '{}' to '{}'", local_file_path, target_path);
    println!("Parent directory: {}, Filename: {}", parent_path, filename);
    
    // This assumes fs.write_file is implemented
    // If not implemented yet, you'll need to add this functionality to Ext4Filesystem
    fs.write_file(parent_path, filename, &buffer)?;
    
    println!("File written successfully, size: {} bytes", buffer.len());
    
    Ok(())
}

/// Remove a file or directory from the ext4 image
fn remove_path(fs: &mut Ext4Filesystem, path: &str, force: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Find the inode for the path
    let inode_num = fs.find_by_path(path)?;
    let inode = fs.read_inode(inode_num)?;
    
    if inode.is_file() {
        println!("Removing file: '{}'", path);
        fs.remove_file(path)?;
        println!("File removed successfully");
    } else if inode.is_directory() {
        println!("Removing directory: '{}'", path);
        
        // Let the filesystem implementation handle the empty directory check
        fs.remove_directory(path, force)?;
        println!("Directory removed successfully");
    } else {
        return Err(format!("'{}' is neither a file nor a directory", path).into());
    }
    
    Ok(())
}

// Keep the original remove_file function for backward compatibility
fn remove_file(fs: &mut Ext4Filesystem, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    remove_path(fs, path, false)
}

/// Create a new directory in the ext4 image
fn create_directory(fs: &mut Ext4Filesystem, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse the path to get parent directory and new directory name
    let parent_path = match path.rfind('/') {
        Some(pos) => {
            if pos == 0 {
                "/"
            } else {
                &path[..pos]
            }
        },
        None => "/"
    };
    
    let dirname = match path.rfind('/') {
        Some(pos) => &path[pos + 1..],
        None => path
    };
    
    if dirname.is_empty() {
        return Err("Invalid directory name".into());
    }
    
    println!("Creating directory '{}' in '{}'", dirname, parent_path);
    
    // This assumes fs.create_directory is implemented
    // If not implemented yet, you'll need to add this functionality to Ext4Filesystem
    fs.create_directory(parent_path, dirname)?;
    
    println!("Directory created successfully");
    
    Ok(())
}
