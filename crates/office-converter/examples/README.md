# Office Converter 示例代码

本目录包含了 `office-converter` 库的示例代码。

## 📋 示例列表

### 1. 检测办公软件 (`detect_office.rs`)

检测系统中已安装的办公软件（Microsoft Office 和 WPS Office）。

**运行方式:**

```bash
cargo run --example detect_office
```

**功能:**

- 自动检测已安装的 Microsoft Office
- 自动检测已安装的 WPS Office
- 显示版本信息和安装路径
- 显示推荐使用的办公软件

**示例输出:**

```
==========================================
  办公软件检测示例
==========================================

🔍 正在检测已安装的办公软件...

✅ 检测到 1 个办公软件:

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📦 办公软件 #1
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   类型: 🏢 Microsoft Office
   版本: 16.0.17328.20184
   路径: C:\Program Files\Microsoft Office\root\Office16
   状态: ✅ 安装有效
```

---

### 2. 转换文档为 HTML (`convert_document.rs`)

将 Office 文档（Word、Excel、PowerPoint）转换为 HTML 格式。

**运行方式:**

使用默认测试文件:

```bash
cargo run --example convert_document
```

指定文件路径:

```bash
cargo run --example convert_document -- "C:\path\to\document.docx"
```

或者:

```bash
cargo run --example convert_document -- "path/to/file.xlsx"
```

**功能:**

- 支持多种格式: .doc, .docx, .xls, .xlsx, .ppt, .pptx
- 两种转换方式:
  - 方法 1: 转换为 HTML 字符串
  - 方法 2: 转换并保存到文件
- 显示转换统计信息
- 显示 HTML 预览

**示例输出:**

```
==========================================
  Office 文档转 HTML 示例
==========================================

✅ 检测到办公软件:
   - Microsoft Office 16.0.17328.20184

📄 输入文件: test.docx
📋 文件类型: DOCX
🔧 使用软件: MsOffice

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
开始转换...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

方法 1: 转换为 HTML 字符串
✅ 转换成功!
   HTML 长度: 5432 字符
   HTML 大小: 5.30 KB

📝 HTML 预览 (前 300 字符):
   ------------------------------------------------------------
   <html>
   <head>
   <meta http-equiv="Content-Type" content="text/html">
   ...
```

---

## 🚀 快速开始

### 准备测试文件

为了运行转换示例，你需要准备一些测试文件：

1. 创建一个简单的 Word 文档并命名为 `test.docx`
2. 将其放在项目根目录
3. 运行示例程序

### 运行所有示例

```bash
# 检测办公软件
cargo run --example detect_office

# 转换文档（使用默认文件）
cargo run --example convert_document

# 转换指定文档
cargo run --example convert_document -- "your_document.docx"
```

---

## 📝 注意事项

1. **需要安装办公软件**: 运行这些示例需要在 Windows 上安装 Microsoft Office 或 WPS Office

2. **文件路径**:
   - 支持相对路径和绝对路径
   - Windows 路径需要使用引号包裹
   - 例如: `"C:\Users\Documents\file.docx"`

3. **支持的格式**:
   - Word: `.doc`, `.docx`
   - Excel: `.xls`, `.xlsx`
   - PowerPoint: `.ppt`, `.pptx`

4. **转换时间**:
   - 转换时间取决于文档大小和复杂度
   - 大型文档可能需要几秒到几十秒

5. **Office 占用**:
   - 转换过程会启动 Office 应用程序
   - 转换完成后会自动关闭
   - 确保 Office 没有被其他程序占用

---

## 🔧 故障排查

### 未检测到办公软件

```
❌ 未检测到任何办公软件!
```

**解决方案:**

- 确保已正确安装 Microsoft Office 或 WPS Office
- 尝试重新安装或修复 Office
- 检查注册表信息是否正确

### 文件不存在

```
❌ 错误: 文件不存在!
```

**解决方案:**

- 检查文件路径是否正确
- 使用绝对路径
- 确保文件确实存在

### 转换失败

```
❌ 转换失败: Windows COM 错误
```

**解决方案:**

- 关闭所有 Office 应用程序后重试
- 以管理员权限运行
- 检查文件是否损坏
- 尝试用 Office 手动打开文件确认文件有效

---

## 📚 更多信息

查看主项目的 [README.md](../README.md) 了解更多关于 API 使用的信息。
