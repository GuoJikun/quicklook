# QuickLook

Windows 文件快速预览工具 —— 在资源管理器里选中文件，按下 <kbd>空格</kbd> 即可预览，无需打开对应软件。

[![Release](https://img.shields.io/github/v/release/GuoJikun/quicklook?label=release)](https://github.com/GuoJikun/quicklook/releases/latest)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%2F%2011-0078D4)](#下载与安装)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#许可证)
[![Check Rust](https://github.com/GuoJikun/quicklook/actions/workflows/check-rust.yml/badge.svg)](https://github.com/GuoJikun/quicklook/actions/workflows/check-rust.yml)
[![Check TS](https://github.com/GuoJikun/quicklook/actions/workflows/check-ts-type.yml/badge.svg)](https://github.com/GuoJikun/quicklook/actions/workflows/check-ts-type.yml)

## 功能特性

**触发位置**

- 文件资源管理器（Explorer）、桌面（Desktop）、文件打开对话框（FileOpenDialog）内均可预览

**预览能力**

- **代码 / 文本**：Shiki 语法高亮，行号、自动换行、字号调节、一键复制，并识别语言
- **Markdown**：目录抽屉、预览与源码切换、字号调节、复制全文，正文按阅读列宽排版
- **文档**：PDF（pdfium 渲染，支持目录、缩放、旋转、页码跳转）、DOCX、Excel / CSV（多工作表）
- **图片**：缩放、旋转、适应窗口、1:1、全屏，双击在适应窗口与实际大小间切换；PSD / HEIC / JXL 自动转码
- **视频**：xgplayer 播放，内置 ffmpeg 对不兼容编码自动转码为 HLS 后播放
- **音频**：沉浸式单窗口界面，封面 + LRC 歌词同步
- **电子书**：EPUB 阅读，目录、字号档位、阅读底色、章节与全书进度
- **压缩包**：树形列表，就地过滤、按名称 / 大小 / 时间排序；加密包可输入密码解压
- **字体**：字号阶梯、字形网格、自定义样例文本、明暗底切换
- **3D 模型**：Three.js 渲染，线框、视角重置、自动旋转、背景切换

**应用特性**

- 界面支持简体中文与 English，切换后托盘菜单、系统对话框与窗口标题同步更新
- 主题跟随系统 / 浅色 / 深色，可在预览窗口内快速切换
- 标题栏可固定窗口（置顶）、切换明暗、调用「打开方式」、最大化与关闭
- 设置页支持自定义代码与视频扩展名、ffmpeg 解析开关、缓存清理、日志级别
- 内置自动更新（tauri-plugin-updater），托盘菜单提供开机自启与检查更新

## 支持预览的格式

| 类型 | 数量 | 扩展名 |
| --- | ---: | --- |
| Markdown | 2 | `markdown` `md` |
| 文档 | 10 | `docx` `xls` `xlsx` `xlsm` `xlsb` `xla` `xlam` `ods` `csv` `pdf` |
| 代码 | 50 | `txt` `cpp` `js` `mjs` `cjs` `ts` `mts` `tsx` `rs` `py` `java` `html` `css` `scss` `sass` `less` `styl` `c` `cs` `go` `vue` `svelte` `astro` `jsx` `json` `yml` `yaml` `toml` `bat` `ps1` `ini` `swift` `kt` `php` `h` `xml` `sql` `pug` `lua` `r` `d` `vb` `pas` `scala` `dart` `rb` `m` `log` `bash` `zig` |
| 图片 | 21 | `jpg` `jpeg` `png` `gif` `webp` `bmp` `ico` `svg` `apng` `psd` `tiff` `tif` `tga` `pbm` `pgm` `ppm` `qoi` `exr` `heic` `heif` `jxl` |
| 视频 | 11 | `mp4` `webm` `mkv` `avi` `mov` `wmv` `mpg` `mpeg` `m4v` `3gp` `3g2` |
| 音频 | 6 | `mp3` `ogg` `m4a` `wav` `flac` `opus` |
| 电子书 | 1 | `epub` |
| 字体 | 4 | `ttf` `otf` `woff2` `woff` |
| 压缩包 | 30 | `zip` `7z` `rar` `tar` `gz` `tgz` `bz2` `tbz2` `xz` `txz` `zst` `tzst` `cpio` `ar` `deb` `a` `jar` `war` `ear` `apk` `aar` `whl` `vsix` `nupkg` `crx` `xpi` `egg` `kra` `xps` `oxps` |
| 3D 模型 | 13 | `gltf` `glb` `stl` `obj` `ply` `fbx` `3mf` `dae` `3ds` `amf` `wrl` `lwo` `lws` |

此外还支持无扩展名文件的识别（如 `README`、`Makefile`、`Dockerfile`、`.env`），以及用户自定义的代码与视频扩展名。

## 下载与安装

从 [Releases](https://github.com/GuoJikun/quicklook/releases/latest) 下载对应架构的 NSIS 安装包，或使用 WinGet：

```powershell
winget install --id GuoJikun.QuickLook -e
```

**系统要求**：Windows 10 / 11，x64 或 arm64。安装包内置 WebView2 引导程序，缺失时会自动补齐。

## 使用

1. 在资源管理器、桌面或文件打开对话框中选中一个文件；
2. 按下 <kbd>空格</kbd> 打开预览窗口，按 <kbd>Esc</kbd> 关闭；
3. 预览窗口标题栏提供固定窗口（置顶）、切换明暗、使用默认或其他程序打开等操作；
4. 托盘菜单可进入设置页、切换开机自启、检查更新。

## 软件截图

<details>
<summary>展开查看全部预览界面</summary>

| | |
| --- | --- |
| 代码<br>![预览代码](./screenshots/preview-code.png) | Markdown<br>![预览 Markdown](./screenshots/preview-md.png) |
| DOCX<br>![预览 DOCX](./screenshots/preview-docx.png) | Excel<br>![预览 Excel](./screenshots/preview-excel.png) |
| PDF<br>![预览 PDF](./screenshots/preview-pdf.png) | 图片<br>![预览图片](./screenshots/preview-image.png) |
| 视频<br>![预览视频](./screenshots/preview-video.png) | 音频<br>![预览音频](./screenshots/preview-audio.png) |
| 压缩包<br>![预览压缩包](./screenshots/preview-zip.png) | 字体<br>![预览字体](./screenshots/preview-font.png) |

</details>

## 开发

### 环境要求

- **Rust**：<https://www.rust-lang.org/tools/install>
- **Tauri 前置依赖**：<https://tauri.app/start/prerequisites/>
- **Node.js**：<https://nodejs.org/>（项目通过 [Volta](https://volta.sh/) 锁定 Node 24.21.0、pnpm 11.28.5，推荐安装 Volta 后自动生效）

### 拉取与运行

```bash
git clone https://github.com/GuoJikun/quicklook.git
cd quicklook

pnpm i            # 安装前端依赖
pnpm tauri dev    # 启动开发模式（Vite 端口 6688）
```

### 首次打包前：构建 ffmpeg

视频预览依赖随包分发的精简版 ffmpeg / ffprobe（LGPL 构建，不含 GPL 组件）。

```bash
./scripts/build-ffmpeg.sh                    # x64
TARGET_ARCH=arm64 ./scripts/build-ffmpeg.sh  # arm64（交叉编译）
```

x64 在 MSYS2 MINGW64 下原生编译：

```bash
pacman -S --needed base-devel mingw-w64-x86_64-gcc mingw-w64-x86_64-nasm make
```

arm64 需在 x64 主机上用 [llvm-mingw](https://github.com/mstorsjo/llvm-mingw/releases)（选 `*-ucrt-x86_64.zip`）交叉编译，把解压后的 `bin` 加入 `PATH`。MSYS2 只提供 x86_64 的 mingw-w64 工具链，脚本会校验 `aarch64-w64-mingw32-gcc` 是否存在，缺失直接报错，不会静默退回宿主编译器。

两个二进制分两阶段构建，组件白名单不同：

| 产物 | 用途 | 启用组件 | 体积（x64 / arm64） |
| --- | --- | --- | --- |
| `ffmpeg.exe` | HLS 转码 | `h264_mf` / `aac` 编码器、常用解码器、`hls` / `mpegts` 封装、转码链路必需的滤镜 | 11 MB / 5.1 MB |
| `ffprobe.exe` | 读取容器元数据 | 仅解封装、解析器与 `aac` 解码器（ts 内 AAC 的采样率 / 声道数只能由解码器补齐），无滤镜、无编码器 | 4.3 MB / 1.5 MB |

arm64 体积明显更小，是因为 x86 的 SIMD 汇编（尤其 h264 / hevc 解码）代码量大得多，两边启用组件完全相同（可对比 `config_components.h`）。

两者均以 `--disable-everything` 为基准按白名单显式启用，因此新增功能若用到新的解码器或滤镜，必须同步写进 `scripts/build-ffmpeg.sh`，否则运行期会报 `no decoder found for: xxx` 或 `Error configuring filter graph`。只调整 ffmpeg 参数时，可用 `SKIP_FFPROBE=1 ./scripts/build-ffmpeg.sh` 跳过 ffprobe 阶段。

脚本产出 `src-tauri/binaries/{ffmpeg,ffprobe}-<triple>.exe`（已加入 `.gitignore`），Tauri 打包时按 target triple 选取对应架构；拷贝前会读 PE 头的 Machine 字段校验架构（x64 为 `0x8664`、arm64 为 `0xaa64`），工具链配错时在构建阶段即失败。

> **注意**：`bundle.externalBin` 在编译期校验文件是否存在，未构建 ffmpeg 时 `cargo check` 与 `pnpm tauri dev` 会直接报 `resource path ... doesn't exist`。CI 的 `PublishApp` 工作流会在打包前自动构建（MSYS2 + llvm-mingw），产物按架构缓存。

### 质量检查

提交前建议依次执行，与 CI 门禁保持一致：

```bash
pnpm lint && pnpm type-check && pnpm test:unit

cargo fmt --check
cargo check --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

### 打包

```bash
pnpm tauri build
```

## 项目结构

```
quicklook/
├── src/                 # Vue 3 前端（路由、预览视图、设置页、i18n）
├── src-tauri/           # Tauri v2 主应用（IPC 命令、键盘钩子、托盘、窗口管理）
├── crates/              # Rust 工作空间
│   ├── archive/         # 压缩包解析
│   ├── book/            # EPUB 解析
│   ├── docs/            # Excel / DOCX / PDF 解析与渲染
│   ├── image/           # PSD / HEIC / JXL 转码
│   ├── video/           # FFmpeg 视频转码
│   ├── audio/           # 音频元数据与 LRC 解析
│   ├── cache/           # 临时缓存管理
│   └── error/           # 统一错误类型
├── design/              # 设计基准与界面原型
└── scripts/             # ffmpeg 构建脚本
```

架构细节（模块依赖、IPC 命令清单、预览流程、路由表）见 [KNOWLEDGE_GRAPH.md](./KNOWLEDGE_GRAPH.md)。

## 开源组件与文档

- 第三方组件清单：[THIRD_PARTY_LIB.md](./THIRD_PARTY_LIB.md)
- 更新日志：[CHANGELOG.md](./CHANGELOG.md)
- 隐私说明：[PRIVACY.md](./PRIVACY.md)

## 许可证

本项目采用 MIT 或 Apache-2.0 双许可，详见 [LICENSE-MIT](./LICENSE-MIT) 与 [LICENSE-Apache](./LICENSE-Apache)。
