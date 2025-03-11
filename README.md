# 更新 README 文档

我将更新 README 文档，添加更多详细信息并改进格式。

```markdown:/Users/shichaopeng/Work/code/my-code/rust-ext4-impl/README.md
# Rust Ext4 文件系统实现

这是一个用 Rust 语言实现的 ext4 文件系统读取工具。该项目旨在提供一个安全、高效的方式来读取和探索 ext4 文件系统镜像。

## 功能特性

- 读取 ext4 文件系统的超级块和块组描述符
- 读取 inode 信息
- 读取目录条目
- 读取文件内容
- 提供简单的命令行工具来浏览 ext4 镜像
- 支持基本的文件写入和删除操作（实验性功能）

## 安装

确保您已安装 Rust 和 Cargo。然后克隆此仓库并构建项目：

```bash
git clone https://github.com/yourusername/rust-ext4-impl.git
cd rust-ext4-impl
cargo build --release
```

编译完成后，可执行文件将位于 `target/release/` 目录中。

## 使用方法

### 在 macOS 上准备环境

在 macOS 上使用此工具前，需要安装一些依赖：

```bash
# 安装 e2fsprogs (提供 ext4 文件系统工具)
brew install e2fsprogs

# 安装 macFUSE (用于挂载文件系统)
brew install --cask macfuse

# 安装 ext4fuse (可选，用于挂载 ext4 文件系统)
brew install ext4fuse
```

### 创建 ext4 镜像（用于测试）

```bash
# 创建一个 100MB 的空文件
dd if=/dev/zero of=ext4.img bs=1M count=100

# 格式化为 ext4 文件系统
mkfs.ext4 ext4.img

# 创建挂载点
mkdir -p mnt

# 挂载镜像 (需要 root 权限)
sudo mount -o loop ext4.img mnt

# 添加一些测试文件
sudo mkdir -p mnt/test
sudo cp -r /some/files mnt/test/

# 完成后卸载
sudo umount mnt
```

### 使用工具探索 ext4 镜像

```bash
# 显示文件系统信息
cargo run -- ext4.img info

# 列出根目录内容
cargo run -- ext4.img ls /

# 列出指定目录内容
cargo run -- ext4.img ls /test

# 显示文件内容
cargo run -- ext4.img cat /test/somefile.txt

# 写入文件 (实验性功能)
cargo run -- ext4.img write /test/newfile.txt /path/to/local/file.txt

# 删除文件 (实验性功能)
cargo run -- ext4.img rm /test/somefile.txt
```

## 命令详解

- `info`: 显示文件系统信息，包括 inode 数量、块数量、块大小等
- `ls [path]`: 列出指定路径的目录内容（默认为根目录）
- `cat <path>`: 显示指定文件的内容
- `write <target_path> <local_file>`: 将本地文件写入到 ext4 镜像中的指定路径
- `rm <path>`: 从 ext4 镜像中删除指定文件

## 项目结构

```
rust-ext4-impl/
├── src/
│   ├── lib.rs           # 主要的库代码
│   ├── superblock.rs    # ext4 超级块实现
│   ├── inode.rs         # inode 结构实现
│   ├── block_group.rs   # 块组描述符实现
│   ├── journal.rs       # 日志实现
│   ├── directory.rs     # 目录条目实现
│   ├── file.rs          # 文件操作实现
│   ├── error.rs         # 错误类型定义
│   └── main.rs          # 命令行工具实现
├── Cargo.toml           # 项目配置文件
└── README.md            # 项目说明文档
```

## 技术细节

### ext4 文件系统结构

ext4 文件系统是 Linux 系统中广泛使用的文件系统，它的基本结构包括：

1. **超级块 (Superblock)**: 包含文件系统的基本信息，如块大小、inode 数量等
2. **块组描述符 (Block Group Descriptors)**: 描述每个块组的信息
3. **块位图 (Block Bitmap)**: 记录哪些块已被使用
4. **inode 位图 (Inode Bitmap)**: 记录哪些 inode 已被使用
5. **inode 表 (Inode Table)**: 存储所有 inode 的信息
6. **数据块 (Data Blocks)**: 存储文件和目录的实际数据

### 实现细节

本项目实现了 ext4 文件系统的基本读取功能，包括：

- 解析超级块和块组描述符
- 读取 inode 信息
- 解析目录结构
- 读取文件内容

文件写入和删除功能目前处于实验性阶段，可能不完全可靠。

## 限制

当前实现有以下限制：

- 写入和删除操作尚未完全实现，仅作为实验性功能
- 不支持扩展属性
- 不支持日志回放
- 不支持所有 ext4 特性（如 64 位支持、extents 等）
- 不支持文件系统修复
- 不支持权限管理

## 未来计划

- 完善文件写入和删除功能
- 添加对扩展属性的支持
- 实现日志回放功能
- 支持更多 ext4 特性
- 添加文件系统完整性检查

## 贡献

欢迎提交 Pull Request 或创建 Issue 来改进此项目。贡献前请先查看现有 Issue，确保不会重复工作。

## 许可证

[MIT](LICENSE)

## 参考资料

- [Ext4 Disk Layout](https://ext4.wiki.kernel.org/index.php/Ext4_Disk_Layout)
- [The Second Extended File System](https://www.nongnu.org/ext2-doc/ext2.html)
- [Linux Kernel Documentation: ext4](https://www.kernel.org/doc/html/latest/filesystems/ext4/index.html)
```

我对 README 文档进行了以下更新：

1. 添加了对文件写入和删除操作的说明（标记为实验性功能）
2. 增加了 macOS 环境准备的详细步骤
3. 扩展了命令详解部分，包括新增的写入和删除命令
4. 添加了更清晰的项目结构展示
5. 新增了技术细节部分，解释 ext4 文件系统的基本结构
6. 添加了未来计划部分
7. 增加了参考资料部分
8. 改进了整体格式和组织结构