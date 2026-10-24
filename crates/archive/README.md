# quicklook-archive

QuickLook 项目的压缩文件解析 crate（工作空间内部库）。

## 功能特性

- **多格式支持**: ZIP、TAR、GZIP、BZIP2、XZ、ZSTD、7Z、RAR、CPIO、AR 等
- **树状结构**: 自动构建目录树状结构
- **密码检测**: 可检测归档是否需要密码（zip/7z/rar）
- **高性能**: 基于 Rust 实现，按格式 feature 门控按需编译

## 支持的格式

| 格式 | 扩展名 | 依赖 |
|------|--------|------|
| ZIP | zip, jar, war, ... | zip |
| TAR | tar | tar |
| GZIP | gz, tgz | flate2 |
| BZIP2 | bz2, tbz2 | bzip2 |
| XZ | xz, txz | lzma-rust2 |
| ZSTD | zst, tzst | ruzstd |
| 7Z | 7z | sevenz-rust |
| RAR | rar | unrar-ng |
| CPIO | cpio | hadris-cpio |
| AR | ar, deb, a | ar |

## 构建

作为工作空间成员由主应用静态链接，也可单独构建：

```bash
cargo build --release
```

## 使用方式

### Rust 中使用

```rust
use quicklook_archive::Extract;

// 列举压缩文件内容（password 传 None 表示无密码）
match Extract::list_archive_tree("example.zip", None) {
    Ok(entries) => {
        for entry in entries {
            println!("Name: {}, Size: {}, Dir: {}",
                entry.name, entry.size, entry.dir);
        }
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## API 参考

### `Extract` 结构体

```rust
pub struct Extract {
    pub name: String,           // 文件/目录名（树节点为归一化完整路径）
    pub size: u64,             // 文件大小
    pub last_modified: String, // 最后修改时间
    pub dir: bool,             // 是否为目录
    pub children: Option<Vec<Extract>>, // 子项（目录树）
}
```

### 主要方法

- `Extract::list_archive_tree(path, password)` - 列举压缩文件内容并构建目录树
- `Extract::is_password_protected(path)` - 检测归档是否需要密码
- `Extract::build_tree(entries)` - 将扁平列表构建为目录树

## 依赖

详见 `Cargo.toml`。核心解析依赖：zip / tar / flate2 / bzip2 / lzma-rust2 / ruzstd / sevenz-rust / unrar-ng / hadris-cpio / ar / encoding_rs。

## 许可证

本项目采用与主 QuickLook 项目相同的许可证。
