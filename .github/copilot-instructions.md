# QuickLook - AI Coding Instructions

QuickLook 是一个 Windows 平台的文件快速预览工具，使用 Tauri (Rust + Vue 3) 架构开发。

## 项目架构

### 混合技术栈 (Tauri)
- **后端**: Rust (src-tauri/) - 处理 Windows 系统交互、文件解析、COM 调用
- **前端**: Vue 3 + TypeScript + Vite (src/) - 用户界面和文件预览渲染
- **通信**: Tauri commands - 前端通过 `invoke()` 调用后端命令

### Workspace 结构
项目使用 Cargo workspace 管理多个 crate:
- `src-tauri/`: 主应用程序 (Windows API、Tauri 集成)
- `crates/archive/`: 压缩文件处理库
  - **功能**: 无需解压即可列举和预览压缩包内容，构建树状结构
  - **支持格式**: ZIP, TAR, TAR.GZ/TGZ, TAR.BZ2/TBZ2, TAR.XZ/TXZ, 7Z
  - **核心结构**: `Extract` 结构体（name, size, last_modified, dir, children）表示文件/目录条目
  - **树构建**: `Extract::build_tree()` 将扁平列表转换为嵌套树结构（HashMap 映射父子关系）
  - **Extractor 模块**: `zip.rs`/`tar.rs`/`sevenz.rs` 使用对应的 crate（zip, tar+flate2/bzip2/xz2, sevenz-rust）解析
  - **C ABI 导出**: 提供 `archive_list_entries` 和 `archive_free_string` FFI 接口供其他语言调用
  - **错误处理**: 统一的 `ArchiveError` 枚举封装 IO/ZIP/7Z 错误
- `crates/docs/`: 文档解析库
  - **功能**: 解析表格文档为统一的行列数据结构（DSheet）
  - **支持格式**: Excel (.xls/.xlsx/.xlsm/.xlsb/.ods) 和 CSV
  - **核心结构**: `Docs` 枚举（Excel/Docx 变体）, `DSheet` 结构体（name + rows: Vec<Vec<String>>）
  - **Excel 解析**: 使用 `calamine` crate 的 `open_workbook_auto()` 自动检测格式，遍历所有 sheet 和单元格
  - **CSV 解析**: 使用 `csv` crate 的 `ReaderBuilder`，固定创建名为 "sheet1" 的单个 sheet
  - **DOCX 处理**: 暂不解析，返回文件路径由前端 `docx-preview` 库处理
  - **数据流**: Rust 解析 → JSON 序列化 → Tauri command 返回 → Vue 组件用 Handsontable 渲染
- `crates/office-converter/`: Office 文档转 HTML 转换器
  - **功能**: 通过 COM 自动化调用本地 Office 软件将文档转为 HTML
  - **架构**: `detector` 检测已安装软件 → `converter` 调度转换 → `ms_office`/`wps_office` 执行具体转换
  - **COM 调用**: 使用 `com_utils` 封装 `IDispatch` 属性读写和方法调用
  - **支持软件**: Microsoft Office (Word/Excel/PowerPoint) 和 WPS Office (文字/表格/演示)
  - **文件格式**: Word (.doc/.docx → HTML wdFormatHTML=8), Excel (.xls/.xlsx → HTML xlHtml=44), PowerPoint (.ppt/.pptx → HTML ppSaveAsHTMLv3=12)
  - **运行示例**: `cargo run --example detect_office` 检测软件, `cargo run --example convert_document` 转换文档

## 核心工作流程

### 文件预览流程
1. **选择检测**: `src-tauri/src/helper/selected_file.rs` 使用 Windows Shell API 检测 Explorer/Desktop/Dialog 中选中的文件
2. **类型分发**: `src-tauri/src/preview.rs` 中的 `WebRoute::get_route()` 根据文件类型路由到对应的 Vue 视图
3. **后端处理**: Tauri commands (`src-tauri/src/command.rs`) 执行文件解析
4. **前端渲染**: Vue 组件 (`src/views/preview/*.vue`) 调用 `invoke()` 获取数据并渲染

### 支持的预览类型
- **Markdown** → `/preview/md` → Shiki 高亮 + markdown-it 渲染
- **Code** → `/preview/code` → Shiki 语法高亮
- **Image/PSD** → `/preview/image` → psd crate 转 PNG
- **Archive** → `/preview/archive` → quicklook-archive 树形展示
- **Document** → `/preview/document` → quicklook-docs 解析 + Handsontable
- **Audio** → `/preview/audio` → lofty 读取元数据 + xgplayer
- **Video** → `/preview/video` → xgplayer
- **Font** → `/preview/font` → @font-face 预览
- **PDF** → `/preview/book` → pdfjs-dist

## 关键技术要点

### Windows API 集成
- 使用 `windows` crate (0.61.0) 调用 Win32 API
- **COM 初始化**: 必须在子线程中使用 `CoInitializeEx(None, COINIT_APARTMENTTHREADED)`
- **Shell 交互**: `IShellWindows`, `IShellView`, `IUIAutomation` 用于文件选择检测
- **钩子机制**: `SetWindowsHookExW(WH_KEYBOARD_LL)` 实现全局快捷键 (Space 键预览)

### Tauri Commands 模式
```rust
// 后端定义 (src-tauri/src/command.rs)
#[command]
pub fn archive(path: &str, mode: &str) -> Result<Vec<Extract>, String> { ... }

// 前端调用 (src/views/preview/archive.vue)
const txt: Array<ExtractedFile> = await invoke('archive', { path: val, mode })
```

### 配置管理
- **开发环境**: 直接使用默认配置，日志级别为 Info
- **生产环境**: 从 `tauri-plugin-store` 读取用户配置 (`config.data`)
- **配置文件**: `src-tauri/resources/config.json` (打包时复制到应用目录)

### 全局状态钩子
- `PreviewFile` 结构体 (`src-tauri/src/preview.rs`) 使用 `LazyLock<Arc<Mutex<>>>` 管理全局键盘钩子
- 钩子注册/注销必须在同一线程，避免内存泄漏

## 开发工作流

### 启动项目
```bash
pnpm i              # 安装依赖 (使用 Volta 锁定版本)
pnpm tauri dev      # 前后端同时启动
```

### 构建发布
```bash
pnpm tauri build    # 生成 NSIS 安装包
```
- 自动生成更新文件 (`latest.json`) 用于 `tauri-plugin-updater`
- 使用 Sentry Vite plugin 上传 sourcemap (需要 `VITE_SENTRY_TOKEN`)

### 测试
```bash
pnpm test:unit      # Vitest + jsdom
pnpm lint           # ESLint (自动修复)
pnpm format         # Prettier
```

### 添加新的预览类型
1. 在 `src-tauri/src/utils/mod.rs` 的 `File::get_file_type()` 添加扩展名映射
2. 在 `src-tauri/src/command.rs` 添加 Tauri command 处理函数
3. 在 `src-tauri/src/preview.rs` 的 `WebRoute::get_route()` 添加路由
4. 创建 Vue 组件 `src/views/preview/<type>.vue`
5. 在 `src/router/index.ts` 注册路由

### Rust 模块约定
- 使用 `#[path = "./xxx.rs"]` 显式声明模块路径
- Helper 模块 (`helper/mod.rs`) 包含辅助功能：监控信息、音频解析、Windows 工具
- COM 调用必须在子线程中使用 `mpsc::channel` 返回结果

## 项目特定注意事项

### 路径处理
- Tauri 使用 `convertFileSrc()` 将文件系统路径转为 `https://asset.localhost/` 协议
- 后端路径必须使用 Windows 反斜杠，前端使用 URL 编码

### 错误处理
- Rust commands 返回 `Result<T, String>` (String 为错误信息)
- 使用 `log` crate 记录日志 (通过 `tauri-plugin-log`)
- 开发环境日志级别为 Info，生产环境默认为 Off

### 依赖版本固定
- Node.js 22.21.0 + pnpm 10.24.0 (通过 Volta)
- Rust 1.77.2+ (Tauri 2.x 要求)

### 文件大小优化
- Vite 配置手动分块 (`manualChunks`) 将 vue/vue-router/pinia 提取为 vender
- pdfjs-dist 的 cmaps 使用 `vite-plugin-static-copy` 单独复制

### Windows 特定功能
- **托盘图标**: `src-tauri/src/tray.rs` 使用 `tauri::tray::TrayIconBuilder`
- **自启动**: `tauri-plugin-autostart` (仅在生产环境启用)
- **单实例**: `tauri-plugin-single-instance` 防止多开

## 常见问题

### COM 调用阻塞
COM 操作必须在子线程中执行，避免阻塞主线程 GUI 消息循环。示例见 `selected_file.rs` 的 `thread::spawn` 模式。

### 键盘钩子泄漏
确保在 `Drop` trait 中调用 `UnhookWindowsHookEx`，释放全局钩子资源。

### Office 文档转换失败
`office-converter` crate 需要本地安装 Microsoft Office 或 WPS Office，通过 COM 接口调用转换：
- **检测逻辑**: 读取注册表路径 (`HKLM\SOFTWARE\Microsoft\Office` 或 `HKLM\SOFTWARE\Kingsoft\Office`) 确认安装
- **COM ProgID**: MS Office 使用固定 CLSID (Word=`000209FF-...`, Excel=`00024500-...`), WPS 使用 ProgID (`KWps.Application`, `KET.application`, `KWPP.application`)
- **转换流程**: `CoInitializeEx` → 创建应用实例 → 打开文档 → `SaveAs` 保存为 HTML → 退出应用 → `CoUninitialize`
- **调试方法**: 运行 `cargo run --example detect_office` 检查软件检测，查看 `com_utils.rs` 确认 COM 方法签名
