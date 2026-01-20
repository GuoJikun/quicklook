# quicklook

QuickLook 是一个 windows 平台的快速预览工具。

## 主要功能

- 在文件资源管理器（Explorer）预览
- 在桌面（Desktop）预览
- 在文件选择弹窗（FileOpenDialog）预览

## 支持预览的格式

- markdown：markdown、md
- Docs：docx、xls、xlsx、xlsm、xlsb、xla、xlam、ods、csv
- Code：cpp、js、mjs、cjs、ts、mts、tsx、rs、py、java、html、css、scss、sass、less、styl、c、cs、go、vue、svelte、astro、jsx、json、yml、yaml、toml、bat、ps1、ini、swift、kt、php、h、xml、sql、pug、lua、r、d、vb、pas、scala、m、log
- Image：jpg、jpeg、png、webp、svg、apng
- Video：mp4、mkv、webm
- Auido：mp3
- Book：pdf
- Font: ttf、otf、woff2、woff
- Archive：zip、tar、gz、tgz、bz2、tbz2、xz、txz、7z

## 如何运行项目

### 前置依赖

- Rust [官方网站](https://www.rust-lang.org/tools/install)
- Tauri [官方网站](https://tauri.app/start/prerequisites/)
- NodeJS [官方网站](https://nodejs.org/)

### 拉取项目代码

```bash
git clone https://github.com/GuoJikun/quicklook.git 
```

### 运行项目

> 推荐使用 pnpm；
> 使用 Volta 来锁定 NodeJS 和 pnpm 版本

```bash
pnpm i #安装项目依赖
pnpm tauri dev 运行项目
```

### 打包

```bash
pnpm tauri build
```

## 使用到的开源软件

- csv: 解析 csv 文件
- calamine: 解析 Excel 文件
- vitepress 的样式
- zip: 解析 zip 格式的文件
- docx-preview
- image
- windows: windows api
- tauri: 一个跨平台应用开发框架
- markdown-it: 解析 md 文件
- handsontable: 显示解析后的 excel 文件和 csv 文件的内容
- pdfjs-dist: 解析 pdf 并显示
- shiki: 解析代码文件和样式显示
- vue: 前端使用的框架

## 软件截图

### 预览 Code (utf-8)

![code.png](./screenshots/preview-code.png)

### 预览 Docx

![code.png](./screenshots/preview-docx.png)

### 预览 Excel

![code.png](./screenshots/preview-excel.png)

### 预览 Image

![code.png](./screenshots/preview-image.png)

### 预览 Md

![code.png](./screenshots/preview-md.png)

### 预览 Pdf

![code.png](./screenshots/preview-pdf.png)

### 预览 Zip

![code.png](./screenshots/preview-zip.png)

## License/许可证

项目使用了 MIT 和 Apache 2.0。
