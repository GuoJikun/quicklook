# 使用到的开源软件

## Rust 依赖

**Tauri（框架 + 插件）**

- [tauri](https://github.com/tauri-apps/tauri) — 跨平台应用开发框架与官方插件（opener / fs / dialog / shell / store / log / autostart / single-instance / updater）

**Windows 平台集成**

- [windows](https://github.com/microsoft/windows-rs) — Windows API 绑定

**序列化与基础库**

- [serde](https://github.com/serde-rs/serde) — 序列化/反序列化
- [serde_json](https://github.com/serde-rs/json) — JSON 支持
- [log](https://github.com/rust-lang/log) — 日志门面
- [regex](https://github.com/rust-lang/regex) — 正则表达式
- [urlencoding](https://github.com/bt/rust_urlencoding) — URL 编码
- [chrono](https://github.com/chronotope/chrono) — 时间日期处理

**压缩与归档（crates/archive）**

- [zip](https://github.com/zip-rs/zip) — ZIP 格式
- [tar](https://github.com/alexcrichton/tar-rs) — TAR 归档
- [flate2](https://github.com/rust-lang/flate2-rs) — gzip / zlib 压缩
- [bzip2](https://github.com/alexcrichton/bzip2-rs) — bzip2 压缩
- [xz2](https://github.com/alexcrichton/xz2-rs) — xz / lzma 压缩
- [sevenz-rust](https://github.com/dyz1990/sevenz-rust) — 7z 格式
- [ruzstd](https://github.com/KillingSpark/zstd-rs) — Zstandard 压缩 (tar.zst)
- [hadris-cpio](https://github.com/hxyulin/hadris) — CPIO 归档
- [ar](https://github.com/mdsteele/rust-ar) — Unix ar 归档（.a / .deb 外层）

**图像处理**

- [image](https://github.com/image-rs/image) — 通用图像编解码
- [psd](https://github.com/soenkehahn/psd) — PSD 解析
- [libheif-rs](https://github.com/oknotokcomputer/libheif-rs) — HEIC/HEIF 解析
- [jxl-oxide](https://github.com/tirr-c/jxl-oxide) — JPEG XL 解析

**文档解析（crates/docs）**

- [calamine](https://github.com/tafia/calamine) — Excel 文件解析
- [csv](https://github.com/BurntSushi/rust-csv) — CSV 文件解析

**音频元数据**

- [lofty](https://github.com/Serial-ATA/lofty-rs) — 音频元数据读取

**3D 模型解析（crates/model）**

- [gltf](https://github.com/gltf-rs/gltf) — glTF/GLB 格式解析
- [stl_io](https://github.com/huonw/stl_io) — STL 格式解析
- [obj](https://github.com/nickel-org/rust-obj) — OBJ 格式解析

## JavaScript / TypeScript 依赖

**框架与 UI**

- [vue](https://github.com/vuejs/core) — 前端框架
- [vue-router](https://github.com/vuejs/router) — 路由
- [pinia](https://github.com/vuejs/pinia) — 状态管理
- [element-plus](https://github.com/element-plus/element-plus) — UI 组件库
- [@element-plus/icons-vue](https://github.com/element-plus/icons) — Element Plus 图标
- [naive-ui](https://github.com/tusen-ai/naive-ui) — 备用 UI 组件库
- [@vueuse/core](https://github.com/vueuse/vueuse) — Vue 组合式工具集
- [@vicons/fluent](https://github.com/07akioni/vicons) — Fluent 风格图标
- [@vicons/ionicons5](https://github.com/07akioni/vicons) — Ionicons 图标

**Markdown 渲染**

- [markdown-it](https://github.com/markdown-it/markdown-it) — Markdown 解析器
- [markdown-it-abbr](https://github.com/markdown-it/markdown-it-abbr) — 缩写支持
- [markdown-it-anchor](https://github.com/valeriangalliat/markdown-it-anchor) — 标题锚点
- [markdown-it-container](https://github.com/markdown-it/markdown-it-container) — 自定义容器
- [markdown-it-deflist](https://github.com/markdown-it/markdown-it-deflist) — 定义列表
- [markdown-it-emoji](https://github.com/markdown-it/markdown-it-emoji) — Emoji
- [markdown-it-ins](https://github.com/markdown-it/markdown-it-ins) — 下划线插入
- [markdown-it-katex](https://github.com/waylonflinn/markdown-it-katex) — 数学公式
- [markdown-it-table-of-contents](https://github.com/Oktavilla/markdown-it-table-of-contents) — TOC
- [markdown-it-task-lists](https://github.com/revin/markdown-it-task-lists) — 任务列表
- [markdown-it-toc-and-anchor](https://github.com/medfreeman/markdown-it-toc-and-anchor) — TOC + 锚点

**代码高亮**

- [shiki](https://github.com/shikijs/shiki) — 基于 VS Code TextMate 的代码高亮（含 markdown-it 集成）

**3D 渲染**

- [three](https://github.com/mrdoob/three.js) — WebGL 3D 渲染引擎（含 GLTFLoader / STLLoader / OBJLoader / OrbitControls）
- [@types/three](https://github.com/DefinitelyTyped/DefinitelyTyped/tree/master/types/three) — Three.js 类型定义

**表格**

- [handsontable](https://github.com/handsontable/handsontable) — Excel 风格表格组件（含 Vue 3 集成）

**视频播放**

- [xgplayer](https://github.com/bytedance/xgplayer) — 字节跳动视频播放器（含 HLS 支持）

**PDF 渲染与画布**

- [pdfjs-dist](https://github.com/mozilla/pdf.js) — Mozilla PDF 渲染
- [leafer-ui](https://github.com/leaferjs/leafer) — 画布渲染引擎（含 animate / color / resize / scroll / view / viewport 插件）

**文档预览**

- [docx-preview](https://github.com/VolodymyrBaydalka/docx-preview) — DOCX 浏览器内渲染

**错误监控**

- [@sentry/vue](https://github.com/getsentry/sentry-javascript) — Sentry 错误上报（含 vite-plugin）

**开发工具**

- [vite](https://github.com/vitejs/vite) — 构建工具（含 vue / vue-jsx / static-copy 插件）
- [typescript](https://github.com/microsoft/TypeScript) — 类型系统
- [vue-tsc](https://github.com/vuejs/language-tools) — Vue 类型检查
- [vitest](https://github.com/vitest-dev/vitest) — 单元测试框架（含 @vue/test-utils）
- [jsdom](https://github.com/jsdom/jsdom) — DOM 模拟
- [sass](https://github.com/sass/dart-sass) — CSS 预处理器
- [eslint](https://github.com/eslint/eslint) — 代码检查（含 eslint-plugin-vue / @vitest/eslint-plugin）
- [prettier](https://github.com/prettier/prettier) — 代码格式化
- [npm-run-all2](https://github.com/bestguy/npm-run-all2) — 并行/串行 npm 脚本
