# Office Converter

用于检测和转换 Office 文档的 Rust 库（仅支持 Windows）。

## 功能

1. **办公软件检测**: 检测本地是否已安装 WPS 和 Microsoft Office
2. **文档转换**: 将 Office 文档（Word、Excel、PowerPoint）转换为 HTML 格式

## 快速开始

### 运行示例程序

**方式 1: 使用脚本（推荐）**

```bash
# PowerShell
.\run_examples.ps1

# 或 CMD
run_examples.bat
```

**方式 2: 直接运行**

```bash
# 示例 1: 检测办公软件
cargo run --example detect_office

# 示例 2: 转换文档（默认文件）
cargo run --example convert_document

# 示例 2: 转换指定文档
cargo run --example convert_document -- "path/to/document.docx"
```

### 示例 1: 检测已安装的办公软件

```rust
use office_converter::{detect_office_apps, get_preferred_office};

fn main() {
    // 检测所有已安装的办公软件
    let apps = detect_office_apps();
    
    for app in apps {
        println!("找到: {:?}", app.app);
        println!("版本: {}", app.version);
        println!("路径: {:?}", app.install_path);
    }
    
    // 获取首选办公软件（优先 Microsoft Office）
    if let Ok(office) = get_preferred_office() {
        println!("推荐使用: {:?}", office.app);
    }
}
```

### 示例 2: 转换文档为 HTML

```bash
# 使用默认测试文件
cargo run --example convert_document

# 或指定文件路径
cargo run --example convert_document -- "path/to/document.docx"
```

```rust
use office_converter::{convert_to_html, convert_to_html_with_options, OfficeApp, ConvertOptions};
use std::path::PathBuf;

fn main() {
    // 方法 1: 简单转换
    let html = convert_to_html("document.docx", OfficeApp::MsOffice)
        .expect("转换失败");
    println!("HTML: {}", html);
    
    // 方法 2: 使用自定义选项
    let options = ConvertOptions {
        office_app: Some(OfficeApp::MsOffice),
        output_path: Some(PathBuf::from("output.html")),
        include_styles: true,
        include_images: true,
    };
    
    let html = convert_to_html_with_options("document.docx", options)
        .expect("转换失败");
}
```

## 使用方式

在你的 `Cargo.toml` 中添加依赖:

```toml
[dependencies]
office-converter = { path = "../office-converter" }
```

## 支持的格式

- Microsoft Word (.doc, .docx)
- Microsoft Excel (.xls, .xlsx)
- Microsoft PowerPoint (.ppt, .pptx)
- WPS 相关格式

## 系统要求

- **操作系统**: Windows
- **必需软件**: Microsoft Office 或 WPS Office（至少安装其中一个）

## API 文档

### 检测办公软件

```rust
// 检测所有已安装的办公软件
pub fn detect_office_apps() -> Vec<OfficeInfo>

// 获取首选办公软件（优先 Microsoft Office）
pub fn get_preferred_office() -> Result<OfficeInfo>

// 检查是否安装了任何办公软件
pub fn is_office_installed() -> bool
```

### 转换文档

```rust
// 简单转换
pub fn convert_to_html<P: AsRef<Path>>(
    input_path: P,
    office_app: OfficeApp,
) -> Result<String>

// 使用自定义选项转换
pub fn convert_to_html_with_options<P: AsRef<Path>>(
    input_path: P,
    options: ConvertOptions,
) -> Result<String>
```

### 数据结构

```rust
pub enum OfficeApp {
    MsOffice,  // Microsoft Office
    Wps,       // WPS Office
}

pub struct OfficeInfo {
    pub app: OfficeApp,
    pub version: String,
    pub install_path: PathBuf,
}

pub struct ConvertOptions {
    pub office_app: Option<OfficeApp>,
    pub output_path: Option<PathBuf>,
    pub include_styles: bool,
    pub include_images: bool,
}
```

## 更多示例

查看 [examples](examples/) 目录获取完整的示例代码：

- `detect_office.rs` - 检测办公软件示例
- `convert_document.rs` - 文档转换示例

## 故障排查

### 未检测到办公软件

确保已正确安装 Microsoft Office 或 WPS Office，并尝试修复安装。

### 转换失败

- 关闭所有 Office 应用程序
- 确保文件没有被占用
- 检查文件是否损坏
- 尝试以管理员权限运行

## 依赖要求

- Windows 操作系统
- 已安装 Microsoft Office 或 WPS Office
