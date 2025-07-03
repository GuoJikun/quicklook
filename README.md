# quicklook

QuickLook 是一个 windows 平台的快速预览工具。

## 主要功能

- 在文件资源管理器（Explorer）预览
- 在桌面（Desktop）预览
- 在文件选择弹窗（FileOpenDialog）预览

## 支持预览的格式

- markdown：markdown、md
- Docs：docx、xls、xlsx、xlsm、xlsb、xla、xlam、ods、csv
- Code：cpp、js、mjs、cjs、ts、mts、tsx、rs、py、java、html、css、scss、sass、less、styl、c、cs、go、vue、svelte、astro、jsx、json、yml、yaml、toml、bat、ps1、ini、swift、kt、php、h、xml、sql、pug、lua、r、d、vb、pas、scala、m
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
<!-- - ffmpeg 使用 vcpkg 安装 -->

<!-- ### 安装 Ffmpeg

- vcpkg [官方网站](https://github.com/microsoft/vcpkg)
- llvm 使用 choco、scoop 任何方法安装 -->

### 拉取项目代码

```bash
git clone https://github.com/GuoJikun/quicklook.git 
```

### 运行项目

> 推荐使用 pnpm

```bash
pnpm i #安装项目依赖
pnpm tauri dev 运行项目
```

### 打包

```bash
pnpm tauri build
```

## TODO

> 带 √ 为已经完成

- 支持的预览格式
  - 图片:
    - [x] svg
    - [x] png
    - [x] apng
    - [x] jpg
    - [x] jpeg
    - [x] gif
    - [x] bmp
    - [x] webp
  - Markdown:
    - [x] md
    - [x] markdown
  - 文档:
    - [x] xlsx
    - [x] xls
    - [x] xlsm
    - [x] xlsb
    - [x] xla
    - [x] xlam
    - [x] ods
    - [x] csv
    - [x] docx
  - 文本文件(utf8):
    - [x] cpp
    - [x] js
    - [x] mjs
    - [x] cjs
    - [x] ts
    - [x] mts
    - [x] tsx
    - [x] rs
    - [x] py
    - [x] java
    - [x] html
    - [x] css
    - [x] scss
    - [x] sass
    - [x] less
    - [x] styl
    - [x] c
    - [x] cs
    - [x] go
    - [x] vue
    - [x] svelte
    - [x] astro
    - [x] jsx
    - [x] json
    - [x] yml
    - [x] yaml
    - [x] toml
    - [x] bat
    - [x] ps1
    - [x] ini
    - [x] swift
    - [x] kt
    - [x] php
    - [x] h
    - [x] xml
    - [x] sql
    - [x] pug
    - [x] lua
    - [x] r
    - [x] d
    - [x] vb
    - [x] pas
    - [x] scala
    - [x] m
  - 字体:
    - [x] ttf
    - [x] otf
    - [x] woff
    - [x] woff2
  - 书籍:
    - [x] pdf
  - 视频:
    - [x] mkv
    - [x] mp4
    - [x] webm
  - 压缩文件:
    - [x] zip
    - [x] tar
    - [x] gz
    - [x] tgz
    - [x] bz2
    - [x] tbz2
    - [x] xz
    - [x] txz
    - [x] 7z
- 设置
  - [x] 支持格式的显示
  - [ ] 版本显示以及更新
  - [x] 自启动

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
