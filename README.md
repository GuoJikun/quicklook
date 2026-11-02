# quicklook

QuickLook 是一个 windows 平台的快速预览工具。

## 主要功能

- 在文件资源管理器（Explorer）预览
- 在桌面（Desktop）预览
- 在文件选择弹窗（FileOpenDialog）预览

## 支持预览的格式

- Markdown：markdown、md
- Doc：docx、xls、xlsx、xlsm、xlsb、xla、xlam、ods、csv、pdf
- Code：txt、cpp、js、mjs、cjs、ts、mts、tsx、rs、py、java、html、css、scss、sass、less、styl、c、cs、go、vue、svelte、astro、jsx、json、yml、yaml、toml、bat、ps1、ini、swift、kt、php、h、xml、sql、pug、lua、r、d、vb、pas、scala、dart、rb、m、log、bash、zig
- Image：jpg、jpeg、png、gif、webp、bmp、ico、svg、apng、psd、tiff、tif、tga、pbm、pgm、ppm、qoi、exr、heic、heif、jxl
- Video：mp4、webm、mkv、avi、mov、wmv、mpg、mpeg、m4v、3gp、3g2
- Audio：mp3、ogg、m4a
- Book：epub
- Font：ttf、otf、woff2、woff
- Archive：zip、7z、rar、tar、gz、tgz、bz2、tbz2、xz、txz、zst、tzst、cpio、ar、a、deb、jar、war、ear、apk、aar、whl、vsix、nupkg、crx、xpi、egg、kra、xps、oxps
- 3D Model：gltf、glb、stl、obj、ply、fbx、3mf、dae、3ds、amf、wrl、lwo、lws

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
pnpm i
# 安装项目依赖

pnpm tauri dev
# 运行项目
```

### 构建 ffmpeg 二进制（首次打包前必须执行）

视频预览依赖随包分发的精简版 ffmpeg（LGPL 构建，不含 GPL 组件）：

```bash
./scripts/build-ffmpeg.sh                  # x64
TARGET_ARCH=arm64 ./scripts/build-ffmpeg.sh # arm64
```

脚本产出 `src-tauri/binaries/{ffmpeg,ffprobe}-<triple>.exe`（已加入 `.gitignore`），
Tauri 打包时按 target triple 自动选取对应架构。

两个二进制由脚本分两阶段构建，组件白名单不同：

| 产物 | 用途 | 启用组件 | 体积 |
|---|---|---|---|
| `ffmpeg.exe` | HLS 转码 | `h264_mf` / `aac` 编码器、常用解码器、`hls` / `mpegts` 封装、转码链路必需的滤镜 | 11 MB |
| `ffprobe.exe` | 读取容器元数据 | 仅解封装、解析器与 `aac` 解码器（ts 内 AAC 的采样率/声道数只能由解码器补齐），无滤镜、无编码器 | 4.3 MB |

两者均以 `--disable-everything` 为基准、按白名单显式启用，因此新增功能时若用到
新的解码器或滤镜，必须同步写进 `scripts/build-ffmpeg.sh`，否则运行期会报
`no decoder found for: xxx` 或 `Error configuring filter graph`。

只调整 `ffmpeg` 的构建参数时，可用 `SKIP_FFPROBE=1 ./scripts/build-ffmpeg.sh`
跳过 ffprobe 阶段，省去一次无谓的全量重编。

> 注意：`bundle.externalBin` 在编译期校验文件是否存在，**未构建 ffmpeg 时 `cargo check` /
> `pnpm tauri dev` 会直接报 `resource path ... doesn't exist`**。因此 CI 与本地首次构建
> 都必须先执行上面的脚本，或从 Release 下载对应架构的二进制放入该目录。

### 打包

```bash
pnpm tauri build
```

## 使用到的开源软件

详见 [THIRD_PARTY_LIB.md](./THIRD_PARTY_LIB.md)

## 软件截图

### 预览 Code (utf-8)

![preview-code.png](./screenshots/preview-code.png)

### 预览 Docx

![preview-docx.png](./screenshots/preview-docx.png)

### 预览 Excel

![preview-excel.png](./screenshots/preview-excel.png)

### 预览 Image

![preview-image.png](./screenshots/preview-image.png)

### 预览 Md

![preview-md.png](./screenshots/preview-md.png)

### 预览 Pdf

![preview-pdf.png](./screenshots/preview-pdf.png)

### 预览 Zip

![preview-zip.png](./screenshots/preview-zip.png)

### 预览 Video

![preview-video.png](./screenshots/preview-video.png)

### 预览 Audio

![preview-audio.png](./screenshots/preview-audio.png)

## License/许可证

项目使用了 MIT 和 Apache 2.0。
