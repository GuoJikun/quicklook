# QuickLook — 开发者指南

Windows 文件快速预览工具（Tauri v2：Rust 后端 + Vue 3 前端）。

## 编码前必须执行的检查

- **Rust 检查**依赖 vcpkg：`cargo install cargo-vcpkg`（仅一次），然后：
  ```
  cd src-tauri
  cargo vcpkg build    # 构建 libheif（HEIC/HEIF 支持）
  cargo check --locked --all-targets
  ```
- **类型检查**：`pnpm type-check`（vue-tsc --build --force）
- **代码规范**：`pnpm lint`（eslint --fix）
- **单元测试**：`pnpm test:unit`（vitest）
- **完整构建**：`pnpm tauri build`

推荐验证顺序：TS 侧 `lint → type-check → test:unit`，Rust 侧 `cargo check`。

## 核心架构

详见 `KNOWLEDGE_GRAPH.md`（完整架构图、模块依赖、IPC 命令清单、预览流程）。

## 开发服务器

Vite 默认端口 6688（`vite.config.ts`）。Tauri 开发模式连接 `http://localhost:6688`。

## 工具链版本锁定（Volta）

- Node 24.15.0
- pnpm 11.0.9

## Rust 格式化

`rustfmt.toml`：LF 换行、4 空格缩进、最大宽度 100。提交前执行 `cargo fmt`。

## ESLint

忽略目录：`dist/`、`coverage/`、`target/`、`src-tauri/`。自定义规则：`vue/multi-word-component-names` 关闭。

## Code Review 约定

代码评审使用简体中文（来自 `.github/copilot-instructions.md`）。
