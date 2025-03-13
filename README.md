# Rust Ext4 Implementation

A Rust implementation of the ext4 filesystem for educational purposes.

## Features

- Read ext4 filesystem metadata (superblock, block groups)
- List directory contents
- Read files
- Write files
- Create directories
- Remove files and directories

## Usage

```bash
dd if=/dev/zero of=ext4.img bs=1M count=100

mkfs.ext4 ext4.img

cargo run -- <ext4_image> [command] [args...]
```

### Commands

- `ls [path]` - List directory contents
- `cat <path>` - Display file contents
- `write <path> <local_file>` - Write file to image
- `mkdir <path>` - Create a new directory
- `rm <path>` - Remove file or directory (use `-f` flag to force remove non-empty directories)
- `info` - Display filesystem information

### Examples

Display filesystem information:
```bash
cargo run -- ext4.img info
```

List root directory contents:
```bash
cargo run -- ext4.img ls /
```

Read a file:
```bash
cargo run -- ext4.img cat /etc/passwd
```

Write a file:
```bash
cargo run -- ext4.img write /test.txt local_file.txt
```

Create a directory:
```bash
cargo run -- ext4.img mkdir /new_directory
```

Remove a file:
```bash
cargo run -- ext4.img rm /test.txt
```

Remove a directory (must be empty unless -f flag is used):
```bash
cargo run -- ext4.img rm /new_directory
```

Force remove a directory (even if not empty):
```bash
cargo run -- ext4.img rm /new_directory -f
```

## Building

```bash
cargo build --release
```

## License

MIT
