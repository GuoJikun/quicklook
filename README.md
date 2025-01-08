# quicklook

QuickLook 是一个 windows 平台的快速预览工具。

## 主要功能

- 在文件资源管理器（Explorer）预览
- 在桌面（Desktop）预览
- 在文件选择弹窗（FileOpenDialog）预览

## 支持预览的格式

- markdown：markdown、md
- Docs：docx、xls、xlsx、xlsm、xlsb、xla、xlam、ods、csv
- Code：cpp、js、mjs、cjs、ts、mts、tsx、rs、py、java、html、css、scss、sass、less、c、go、vue、svelte、astro、jsx、json、yml、yaml、toml、bat、ps1、ini、swift、kt、php、h、xml、sql
- Image：jpg、jpeg、png、webp、svg、apng
- Video：mp4、mkv、、、
- Auido：mp3
- Book：pdf
- Font: ttf、otf、woff2、woff
- Archive：zip

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
    - [x] html
    - [x] css
    - [x] js
    - [x] ts
    - [x] tsx
    - [x] c
    - [x] cpp
    - [x] rs
    - [x] py
    - [x] json
    - [x] yml
  - 字体:
    - [x] ttf
    - [x] otf
    - [x] woff
    - [x] woff2
  - 书籍:
    - [x] pdf
  - 压缩文件:
    - [x] zip
    - [ ] rar
    - [ ] 7z
- 设置
  - [x] 支持格式的显示
  - [ ] 版本显示以及更新
  - [ ] 自启动

## 使用到的开源软件

- csv: 解析 csv 文件
- calamine: 解析 Excel 文件
- vitepress 的样式
- zip: 解析 zip 格式的文件
- docx-rs
- docx-preview
- image
- windows: windows api
- tauri: 一个跨平台应用开发框架
