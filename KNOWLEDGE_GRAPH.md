# QuickLook 项目知识图谱

## 项目概览

```
QuickLook (Windows 文件快速预览工具, macOS Quick Look 风格)
├── Rust 后端 (Tauri v2)  ─── IPC ───  Vue 3 前端
│   ├── commands/ (Tauri #[command] handler)
│   ├── helper/   (业务逻辑实现)
│   ├── preview/  (空格键激活预览)
│   └── utils/    (文件分类)
└── 工作空间:
    ├── src-tauri/       → app (主应用)
    ├── crates/archive/  → quicklook-archive (压缩文件解析)
    ├── crates/book/    → quicklook-book (epub/mobi 电子书解析)
    ├── crates/docs/    → quicklook-docs (文档解析)
    ├── crates/error/    → quicklook-error (统一错误类型)
    ├── crates/audio/    → quicklook-audio (音频元数据 + LRC)
    ├── crates/image/    → quicklook-image (图片格式转换)
    └── crates/video/    → quicklook-video (FFmpeg 视频转换)
```

## 目录结构

```
E:/private/Rust/quicklook/
├── Cargo.toml                     # 工作空间定义 (resolver = "2")
├── package.json                   # 前端依赖
├── vite.config.ts                 # Vite 构建配置
├── src/                           # 前端源码
│   ├── main.ts                    # 入口: Pinia + Router + Element Plus + Sentry
│   ├── App.vue                    # 根组件 (<RouterView>)
│   ├── router/index.ts            # 15 个路由
│   ├── stores/index.ts            # Pinia store (音视频配置列表)
│   ├── hooks/                     # theme.ts, use-window.ts
│   ├── utils/                     # index.ts, typescript.ts, theme.ts, sentry.ts
│   │   └── markdown/              # markdown-it 工厂 + 插件
│   ├── components/                # layout-preview, header, footer, setting-item, md-viewer, excel
│   └── views/
│       ├── home/index.vue         # 占位首页
│       ├── settings.vue           # 设置页
│       ├── upgrade.vue            # 更新页
│       └── preview/               # 11 个预览视图
│           ├── index.vue          # 预览外壳 (Escape 关闭)
│           ├── code.vue           # Shiki 高亮
│           ├── md.vue             # Markdown 渲染
│           ├── image.vue          # 图片查看 (PSD/HEIC 自动转码)
│           ├── video.vue          # 视频播放 (xgplayer + HLS)
│           ├── audio.vue          # 音频播放 + LRC 歌词
│           ├── font.vue           # 字体预览
│           ├── book.vue           # epub/mobi 电子书阅读
│           ├── archive.vue        # 压缩包目录树
│           ├── model.vue          # 3D 模型预览 (Three.js)
│           ├── document.vue       # Excel/DOCX 渲染
│           ├── not-support.vue    # 不支持提示
│           └── components/preview-image.vue
├── src-tauri/                     # Rust 后端
│   ├── Cargo.toml                 # 依赖 + vcpkg 配置
│   ├── tauri.conf.json            # Tauri 应用配置
│   ├── capabilities/              # 权限配置
│   ├── icons/                     # 应用图标
│   └── src/
│       ├── main.rs                # 入口: app_lib::run()
│       ├── lib.rs                 # Tauri Builder + setup hook + command 注册
│       ├── error.rs               # QuickLookError 统一错误类型
│       ├── tray.rs                # 系统托盘
│       ├── commands/              # IPC 命令入口
│       │   ├── mod.rs             # 重导出
│       │   ├── archive.rs         # archive()
│       │   ├── document.rs        # document()
│       │   ├── image.rs           # convert_to_png(), clear_image_cache()
│       │   ├── audio.rs           # read_audio_info(), parse_lrc()
│       │   ├── video.rs           # check_ffmpeg(), convert_video_to_hls(), cancel_video_conversion()
│       │   └── system.rs          # set_log_level(), show_open_with_dialog(), get_monitor_info(),
│       │                          #   get_default_program_name(), clear_cache()
│       ├── helper/                # 内部业务逻辑
│       │   ├── mod.rs             # get_webview_window(), get_scaled_size()
│       │   ├── config.rs          # 读取 resources/config.json
│       │   ├── selected_file.rs   # COM 获取前台窗口选中文件路径
│       │   ├── win.rs             # Win32 API 封装
│       │   ├── audio.rs           # 音乐元数据 (lofty) + LRC 歌词
│       │   ├── image.rs           # PSD/HEIC → PNG 转换
│       │   ├── ffmp.rs            # ffmpeg 检测 + 视频 → HLS
│       │   └── monitor.rs         # 屏幕信息 (GDI)
│       ├── preview/               # 预览激活
│       │   ├── mod.rs             # 全局状态 + init_preview_file()
│       │   ├── hook.rs            # 键盘钩子 (WH_KEYBOARD_LL)
│       │   ├── route.rs           # 文件类型 → 前端路由
│       │   └── window.rs          # 创建/导航预览窗口
│       └── utils/
│           └── mod.rs             # File 结构体 + FILE_TYPE_MAPPING (120+扩展名)
├── crates/
│   ├── error/                     # quicklook-error
│   │   └── src/lib.rs             # QuickLookError 统一错误类型
│   ├── audio/                     # quicklook-audio
│   │   └── src/lib.rs             # MusicInfo, read_music_info(), Lrc, parse_lrc()
│   ├── image/                     # quicklook-image
│   │   └── src/lib.rs             # psd_to_png(), heic_to_png(), jxl_to_png(), image_to_png()
│   ├── video/                     # quicklook-video
│   │   └── src/lib.rs             # check_ffmpeg(), convert_video_to_hls(), cancel_video_conversion()
│   ├── archive/                   # quicklook-archive
│   │   ├── src/lib.rs             # Extract, build_tree(), list_archive_tree()
│   │   └── src/extractors/        # zip, tar, gz, bz2, xz, zst, 7z, rar, cpio, ar
│   └── docs/                      # quicklook-docs
│       └── src/lib.rs             # Docs enum (Excel, CSV, DOCX)
│   └── book/                      # quicklook-book
│       ├── src/epub.rs            # epub 解析 (章节/目录/HTML)
│       └── src/mobi.rs            # mobi 解析 (元数据/HTML)
├── .github/workflows/
│   ├── build.yml                  # 发布构建 (NSIS 安装包)
│   ├── check-rust.yml             # cargo check CI
│   └── check-ts-type.yml          # TypeScript 类型检查 CI
└── KNOWLEDGE_GRAPH.md             # 本文件
```

## 后端模块依赖关系

```plain
main.rs
  └── app_lib::run()

lib.rs (Tauri Builder)
  ├── tauri 插件 x 9  (opener, store, single-instance, shell, dialog, updater, fs, autostart, log)
  ├── setup hook
  │   ├── helper::config::read_config() → app.manage(config)
  │   ├── app.listen("config_update")   → 热更新配置
  │   ├── preview::init_preview_file()  → 安装键盘钩子
  │   └── tray::create_tray()           → 系统托盘
  └── invoke_handler (注册 13 个 command)

commands/ (IPC 入口)     helper/ (业务逻辑)     crates/ (工作空间)
┌──────────┐            ┌────────────┐        ┌──────────────────┐
│ archive  │─────────── │            │        │ quicklook-archive│
│ book     │─────────── │            │        │ quicklook-book   │
│ document │─────────── │            │        │ quicklook-docs   │
│ image    │─────────── │ image.rs   │        └──────────────────┘
│ audio    │─────────── │ audio.rs   │
│ video    │─────────── │ ffmp.rs    │
│ system   │──┬──────── │ win.rs     │
│          │  ├──────── │ monitor.rs │
│          │  ├──────── │ config.rs  │
│          │  └──────── │ ffmp.rs    │
└──────────┘            └────────────┘

preview/ (预览激活流程)
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌─────────-─┐
│ hook.rs  │───→│ window.rs│───→│ route.rs │───→│ 前端路由   │
│ (空格键)  │    │ (创建/   │    │ (类型→    │    │ /preview  │
│          │    │  导航)   │    │  路径)    │    │ /{type}   │
└──────────┘    └──────────┘    └──────────┘    └─────────-─┘
                      │
                      ↓
                 helper/selected_file.rs (COM 获取选中文件)
                      │
                      ↓
                 utils/mod.rs (get_file_info → FILE_TYPE_MAPPING)

error.rs (统一错误类型: QuickLookError)
  ↑ 被所有 commands/ 和 helper/ 模块引用

```

## IPC 命令清单 (13 个)

| 命令 | 方向 | 前端模块 | 后端入口 | 后端实现 |
|------|------|----------|----------|----------|
| `archive` | FE → BE | archive.vue | `commands/archive.rs` | quicklook-archive (8种格式) |
| `document` | FE → BE | document.vue | `commands/document.rs` | quicklook-docs (Excel/CSV/DOCX) |
| `get_epub_info` | FE → BE | book.vue | `commands/book.rs` | quicklook-book::epub |
| `get_epub_chapter` | FE → BE | book.vue | `commands/book.rs` | quicklook-book::epub |
| `get_mobi_info` | FE → BE | book.vue | `commands/book.rs` | quicklook-book::mobi |
| `get_mobi_content` | FE → BE | book.vue | `commands/book.rs` | quicklook-book::mobi |
| `convert_to_png` | FE → BE | image.vue | `commands/image.rs` | helper/image (psd/heic) |
| `clear_image_cache` | FE → BE | settings.vue | `commands/image.rs` | 删除 %TEMP%/quicklook_images/ |
| `read_audio_info` | FE → BE | audio.vue | `commands/audio.rs` | helper/audio (lofty crate) |
| `parse_lrc` | FE → BE | audio.vue | `commands/audio.rs` | helper/audio (LRC 解析) |
| `check_ffmpeg` | FE → BE | video.vue | `commands/video.rs` | helper/ffmp (which ffmpeg) |
| `convert_video_to_hls` | FE → BE | video.vue | `commands/video.rs` | helper/ffmp (ffmpeg 转码) |
| `cancel_video_conversion` | FE → BE | video.vue | `commands/video.rs` | helper/ffmp (taskkill) |
| `clear_cache` | FE → BE | settings.vue | `commands/system.rs` | helper/ffmp + helper/image |
| `get_monitor_info` | FE → BE | window.rs | `commands/system.rs` | helper/monitor (GDI) |
| `show_open_with_dialog` | FE → BE | header.vue | `commands/system.rs` | helper/win (SHOpenWithDialog) |
| `get_default_program_name` | FE → BE | header.vue | `commands/system.rs` | helper/win (AssocQueryStringW) |
| `set_log_level` | FE → BE | lib.rs / settings.vue | `commands/system.rs` | log::set_max_level |

## 空格键预览完整流程

```
[用户在资源管理器/桌面/对话框 按 Space]
        │
        ▼
hook.rs: WH_KEYBOARD_LL 低级键盘钩子
        │  ncode >= 0 且 WM_KEYDOWN 且 vkCode == VK_SPACE
        │  Selected::get_focused_type() 识别焦点窗口类型
        │    └── CabinetWClass  → 资源管理器
        │    └── Progman/WorkerW → 桌面
        │    └── #32770        → 文件对话框
        ▼
window.rs: PreviewFile::preview_file()
        │  1. Selected::new()
        │     └── COM: IShellWindows → IShellView.GetItemObject(SVGIO_SELECTION)
        │     └── 或 IUIAutomation (对话框)
        │
        │  2. get_file_info(path, customExts)
        │     └── FILE_TYPE_MAPPING (120+ 扩展名)
        │     └── detect_language_no_ext() (无扩展名文件)
        │     └── 返回 File { file_type, path, extension, size, last_modified, name }
        │
        │  3. PreviewState.inner.input_path = file_path
        │
        │  4. WebRoute::get_route(file_type, file_info)
        │     └── Markdown → /preview/md?file_type=Markdown&path=...&...
        │     └── Image    → /preview/image?...
        │     └── Audio    → /preview/audio?...
        │     └── Video    → /preview/video?...
        │     └── Model3D  → /preview/model?... (Three.js 直接加载)
        │     └── ...
        │
        │  5. 窗口管理
        │     ┌── 已有 preview 窗口 → window.eval("location.href = '...'")
        │     └── 无 preview 窗口 → WebviewWindowBuilder
        │           ├── 无边框 decorations(false)
        │           ├── 80% 屏幕大小 (Audio 除外 560x200)
        │           ├── center() + skip_taskbar(false)
        │           └── on_page_load 时导航到目标路由
        ▼
[Vue 前端路由匹配]
        │
        ▼
预览视图读取 URL query params
        │  { file_type, path, extension, size, last_modified, name }
        │
        ├── Code 视图:  readTextFile(path) → Shiki highlight → v-html
        ├── Md 视图:    readTextFile(path) → markdown-it render → MdViewer
        ├── Image 视图: invoke('convert_to_png') + convertFileSrc(path) → <img> 拖拽缩放
        ├── Video 视图: convertFileSrc(path) → xgplayer
        │              (ffmpeg 开启时: invoke('convert_video_to_hls') → HLS.m3u8)
        ├── Audio 视图: <audio> + invoke('read_audio_info') + invoke('parse_lrc')
        ├── Font 视图:  FontFace.load(convertFileSrc(path)) → 示例文本
        ├── Book 视图:   epub/mobi → IPC 获取章节 HTML → iframe/内联渲染
        ├── Archive:    invoke('archive') → el-tree
        └── Document:
              ├── Excel: invoke('document') → Handsontable
              └── DOCX:  readFile(path) → docx-preview.renderAsync()
```

## 文件分类映射 (utils/mod.rs)

```
FILE_TYPE_MAPPING (约 120 个扩展名)

Markdown  : md, markdown
Doc       : docx, xlsx, xls, xlsm, xlsb, xla, xlam, ods, csv
Image     : jpg, jpeg, png, gif, webp, bmp, ico, svg, apng
            psd, tiff, tif, tga, pbm, pgm, ppm, qoi, exr, heic, heif
Video     : mp4, webm, mkv, avi, mov, wmv, mpg, mpeg, m4v, 3gp, 3g2
Audio     : mp3, ogg, m4a, flac, wav, aac, wma, opus, ape, aiff, aifc, aif
Book      : (预留，即将支持 epub, mobi)
Font      : ttf, otf, woff2, woff, eot
Archive   : zip, 7z, rar, tar, gz, tgz, bz2, tbz2, xz, txz, zst, tzst
            cpio, ar, deb, a, jar, war, ear, apk, aar, whl, vsix
            nupkg, crx, xpi, egg, kra, xps, oxps
Code (50+): cpp, js, mjs, cjs, ts, mts, tsx, rs, py, java, html, css
            scss, sass, less, styl, c, cs, go, vue, svelte, astro, jsx
            json, yml, yaml, toml, bat, ps1, ini, swift, kt, php, h
            xml, sql, pug, lua, r, d, vb, pas, scala, m, log, sh, bash, zsh, zig
Model3D   : gltf, glb, stl, obj, ply, fbx, 3mf, dae, 3ds, amf, wrl, lwo, lws
无扩展名检测: README → markdown, Makefile → makefile, Dockerfile → docker,
           .bashrc → bash, .gitignore → gitignore, ...

用户自定义: customCodeExtensions (plugin-store)
           customVideoExtensions (plugin-store)
```

## 前端路由表

| 路径 | 组件 | 说明 |
|------|------|------|
| `/` | HomeView | 占位首页 |
| `/preview` | PreviewIndex | 预览外壳 (router-view 容器) |
| `/preview/not-support` | NotSupport | 不支持的文件类型 |
| `/preview/video` | VideoSupport | xgplayer 视频播放 |
| `/preview/image` | ImageSupport | 图片查看 (拖拽/缩放) |
| `/preview/audio` | AudioSupport | 音频 + 歌词同步 |
| `/preview/code` | CodeSupport | Shiki 语法高亮 |
| `/preview/font` | FontSupport | 字体预览 |
| `/preview/md` | MdSupport | Markdown 渲染 |
| `/preview/book` | BookSupport | epub/mobi 电子书阅读 |
| `/preview/archive` | ArchiveSupport | 压缩包目录树 |
| `/preview/model` | ModelSupport | 3D 模型预览 (Three.js) |
| `/preview/document` | DocumentSupport | Excel/CSV/DOCX |
| `/settings` | Settings | 设置页面 |
| `/upgrade` | Upgrade | 更新管理 |

## 前端组件树

```
App.vue
└── <RouterView>
    ├── / → HomeView
    │
    ├── /preview → PreviewIndex (Escape 关闭, 禁用右键菜单)
    │   └── LayoutPreview
    │       ├── Header (文件名称, 主题切换, 固定窗口, 打开, 打开方式, 最大化, 关闭)
    │       ├── Body
    │       │   ├── Image:  <img> + PreviewImage (滚轮缩放 + 鼠标拖拽)
    │       │   ├── Video:  xgplayer (HLS 流)
    │       │   ├── Audio:  <audio> + 封面 + LRC 歌词
    │       │   ├── Code:   <div v-html="shikiHtml">
    │       │   ├── Md:     MdViewer <div v-html="markdownHtml">
    │       │   ├── Font:   <div style="font-family: MyFont"> 示例
    │       │   ├── Book:   epub/mobi 阅读 (章节 HTML 渲染) + 侧栏目录
    │       │   ├── Archive: <el-tree> (Element Plus)
        │       │   ├── Model:   Three.js WebGL (直接加载，无后端依赖)
    │       │   └── Document:
    │       │       ├── Excel: ExcelView (Handsontable + 多 sheet 标签页)
    │       │       └── DOCX:  <div ref="docxContainer">
    │       └── Footer (文件类型, 扩展名, 格式化大小)
    │
    ├── /settings → Settings
    │   ├── 格式列表 (音频/视频白名单 + 扩展名配置)
    │   ├── 自定义扩展名 (Code / Video)
    │   ├── ffmpeg 开关 + 检测
    │   ├── 缓存管理 (一键清理)
    │   ├── 日志级别
    │   └── 关于 (版本号)
    │
    └── /upgrade → Upgrade
        ├── 版本检测 (tauri-plugin-updater)
        ├── 下载进度条
        └── 发布说明 (MdViewer)
```

## 状态管理

```
Pinia Store (stores/index.ts)
└── useMainStore
    ├── audio: string[]           (所有音频格式列表)
    ├── audioChecked: string[]    (用户启用的音频格式)
    ├── video: string[]           (所有视频格式列表)
    └── videoChecked: string[]    (用户启用的视频格式)

Tauri Plugin Store (config.data) -- 持久化到磁盘
├── customCodeExtensions: string[]   (用户自定义代码扩展名)
├── customVideoExtensions: string[]  (用户自定义视频扩展名)
├── logLevel: number                 (日志级别: 0-5)
├── autostart: boolean               (开机自启)
└── ... (其他设置项)

resources/config.json (编译时内嵌)
├── preview.markdown: string[]     (Markdown 格式白名单)
├── preview.markdown.checked: string[]
├── preview.audio: string[]
├── preview.audio.checked: string[]
├── preview.video: string[]
├── preview.video.checked: string[]
├── preview.model: string[]        (3D 模型格式白名单)
└── preview.model.checked: string[]
```

## Rust 工作空间依赖

```
Cargo workspace (resolver = "2")
│
├── app (src-tauri/)
│   ├── tauri 2.x (protocol-asset, tray-icon)
│   ├── tauri-plugins x 9
│   ├── windows 0.61 (Win32/COM/UI Automation)
│   ├── serde + serde_json
│   ├── urlencoding
│   └── workspace crates (以下全部)
│
├── quicklook-error (crates/error/)
│   ├── thiserror
│   └── serde
│
├── quicklook-audio (crates/audio/)
│   ├── lofty (音频元数据)
│   └── quicklook-error
│
├── quicklook-image (crates/image/)
│   ├── image crate (多种图像格式)
│   ├── psd (Photoshop 解析)
│   ├── libheif-rs (HEIC/HEIF 解码，需 vcpkg)
│   ├── jxl-oxide (JPEG XL 解码)
│   └── quicklook-error
│
├── quicklook-video (crates/video/)
│   ├── log (ffmpeg 进程管理)
│   └── quicklook-error
│
├── quicklook-archive (crates/archive/)
│   ├── zip, tar, flate2, bzip2, xz2
│   ├── sevenz-rust, ruzstd, hadris-cpio, ar
│   ├── unrar-ng (RAR 支持)
│   └── 条件 feature gate (每个格式独立)
│
└── quicklook-docs (crates/docs/)
    ├── calamine (Excel: .xls/.xlsx/.xlsb/.ods)
    └── csv (CSV 解析)

└── quicklook-book (crates/book/)
    ├── epub (EPUB 解析: 章节/目录/HTML)
    └── mobi (MOBI 解析: 元数据/HTML)
```

## 前端 NPM 依赖

```
生产依赖:
├── Vue 3.5 + vue-router 4.6 + pinia 3.x
├── Element Plus 2.13 (UI 组件库, 中文语言包)
├── Naive UI 2.44 (仅 NIcon)
├── @vueuse/core (dark mode, 元素尺寸, 节流, 事件监听)
├── @vicons/fluent + @vicons/ionicons5 (图标)
├── markdown-it 14.x + 11 个插件 (markdown 渲染)
├── shiki 4.x + @shikijs/markdown-it (代码高亮)
├── handsontable (Excel 表格)
├── docx-preview (DOCX 渲染)
├── pdfjs-dist (PDF 解析, document.vue 中 PdfViewer 使用)
├── leafer-ui 2.x + 6 个插件 (PDF 画布渲染, document.vue 中 PdfViewer 使用)
├── xgplayer 3.x + xgplayer-hls (视频播放)
├── @sentry/vue (错误监控)
└── @tauri-apps/api + 9 个插件绑定

开发依赖:
├── Vite 6 + TypeScript 6
├── vue-tsc, vitest, eslint, prettier, sass
└── @sentry/vite-plugin
```
