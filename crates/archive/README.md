# quicklook-archive

这是 QuickLook 项目的独立压缩文件处理动态库。

## 功能特性

- **多格式支持**: 支持 ZIP、TAR、TAR.GZ、TAR.BZ2、TAR.XZ、7Z 等多种压缩格式
- **动态库**: 可以作为独立的动态库被其他项目使用
- **C ABI**: 提供 C 兼容的 ABI，可以被其他语言调用
- **树状结构**: 自动构建目录树状结构
- **高性能**: 基于 Rust 实现，性能优异

## 支持的格式

- **ZIP**: 标准 ZIP 压缩文件
- **TAR**: 标准 TAR 归档文件
- **TAR.GZ/TGZ**: GZIP 压缩的 TAR 文件
- **TAR.BZ2/TBZ2**: BZIP2 压缩的 TAR 文件  
- **TAR.XZ/TXZ**: XZ 压缩的 TAR 文件
- **7Z**: 7-Zip 压缩文件

## 构建

### 作为 Rust 库

```bash
cargo build --release
```

### 作为动态库

```bash
cargo build --release --lib
```

生成的动态库文件位于 `target/release/` 目录下:

- Windows: `quicklook_archive.dll`
- Linux: `libquicklook_archive.so`
- macOS: `libquicklook_archive.dylib`

## 使用方式

### Rust 中使用

```rust
use quicklook_archive::{Extract, ArchiveError};

// 列举压缩文件内容
match Extract::list_archive_tree("example.zip") {
    Ok(entries) => {
        for entry in entries {
            println!("Name: {}, Size: {}, Dir: {}", 
                entry.name, entry.size, entry.dir);
        }
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### C/C++ 中使用

```c
#include <stdio.h>

// 函数声明
extern int archive_list_entries(const char* path, char** result);
extern void archive_free_string(char* s);

int main() {
    char* result = NULL;
    int ret = archive_list_entries("example.zip", &result);
    
    if (ret == 0 && result) {
        printf("Archive contents: %s\n", result);
        archive_free_string(result);
    } else {
        printf("Error processing archive\n");
    }
    
    return 0;
}
```

### Python 中使用

```python
import ctypes
import json

# 加载动态库
lib = ctypes.CDLL('./libquicklook_archive.so')

# 设置函数签名
lib.archive_list_entries.argtypes = [ctypes.c_char_p, ctypes.POINTER(ctypes.c_char_p)]
lib.archive_list_entries.restype = ctypes.c_int
lib.archive_free_string.argtypes = [ctypes.c_char_p]

def list_archive_entries(path):
    result = ctypes.c_char_p()
    ret = lib.archive_list_entries(path.encode('utf-8'), ctypes.byref(result))
    
    if ret == 0 and result.value:
        json_str = result.value.decode('utf-8')
        lib.archive_free_string(result)
        return json.loads(json_str)
    else:
        return None

# 使用示例
entries = list_archive_entries("example.zip")
if entries:
    for entry in entries:
        print(f"Name: {entry['name']}, Size: {entry['size']}")
```

## API 参考

### Rust API

#### `Extract` 结构体

```rust
pub struct Extract {
    pub name: String,           // 文件/目录名
    pub size: u64,             // 文件大小
    pub last_modified: String, // 最后修改时间
    pub dir: bool,             // 是否为目录
    pub children: Option<Vec<Extract>>, // 子项（目录树）
}
```

#### 主要方法

- `Extract::list_archive_tree(path)` - 列举压缩文件内容并构建目录树
- `Extract::build_tree(entries)` - 将扁平列表构建为目录树

### C API

- `int archive_list_entries(const char* path, char** result)` - 列举压缩文件内容
- `void archive_free_string(char* s)` - 释放字符串内存

## 依赖

- `serde` - 序列化支持
- `zip` - ZIP 格式支持  
- `tar` - TAR 格式支持
- `flate2` - GZIP 压缩支持
- `bzip2` - BZIP2 压缩支持
- `xz2` - XZ 压缩支持
- `sevenz-rust` - 7Z 格式支持
- `chrono` - 时间处理

## 许可证

本项目采用与主 QuickLook 项目相同的许可证。
