# vcpkg 包管理和缓存指南

本文档介绍如何在 QuickLook 项目中使用 vcpkg 管理 C++ 依赖包以及如何配置缓存以提升构建速度。

## 什么是 vcpkg

vcpkg 是 Microsoft 开发的跨平台 C/C++ 包管理器，用于获取和管理 C/C++ 库。

## 项目配置

### vcpkg.json - 依赖清单

项目根目录的 `vcpkg.json` 文件定义了项目所需的 C++ 依赖：

```json
{
  "name": "quicklook",
  "version": "0.11.0",
  "dependencies": [
    {
      "name": "ffmpeg",
      "features": ["avcodec", "avformat", "avutil", "swscale", "swresample"],
      "platform": "windows"
    }
  ]
}
```

### vcpkg-configuration.json - 配置文件

`vcpkg-configuration.json` 文件用于配置 vcpkg 的行为：

```json
{
  "default-registry": {
    "kind": "git", 
    "repository": "https://github.com/Microsoft/vcpkg",
    "baseline": "2024.11.16"
  }
}
```

## 环境变量配置

为了让 vcpkg 正常工作，需要配置以下环境变量：

### Windows

**临时设置 (当前会话)**:
```cmd
# Command Prompt
set VCPKG_ROOT=C:\path\to\vcpkg

# PowerShell  
$env:VCPKG_ROOT = "C:\path\to\vcpkg"
```

**永久设置**:
```powershell
# PowerShell (管理员权限)
[Environment]::SetEnvironmentVariable("VCPKG_ROOT", "C:\path\to\vcpkg", [EnvironmentVariableTarget]::User)
```

或通过系统属性 > 环境变量手动添加。

### Linux/macOS

**临时设置 (当前会话)**:
```bash
export VCPKG_ROOT=/path/to/vcpkg
```

**永久设置**:
```bash
# 添加到 ~/.bashrc 或 ~/.zshrc
echo 'export VCPKG_ROOT=/path/to/vcpkg' >> ~/.bashrc
source ~/.bashrc
```

### 验证配置

```bash
# Windows
echo %VCPKG_ROOT%

# Linux/macOS  
echo $VCPKG_ROOT
```

## 本地开发设置

### 1. 安装 vcpkg

```bash
# 克隆 vcpkg 仓库到本地
git clone https://github.com/Microsoft/vcpkg.git C:\vcpkg
cd C:\vcpkg

# Windows
.\bootstrap-vcpkg.bat

# Linux/macOS  
./bootstrap-vcpkg.sh
```

### 2. 设置环境变量

```bash
# Windows (PowerShell)
$env:VCPKG_ROOT = "C:\vcpkg"

# Windows (Command Prompt)
set VCPKG_ROOT=C:\vcpkg

# Linux/macOS (Bash)
export VCPKG_ROOT=/path/to/vcpkg
```

### 3. 安装项目依赖

```bash
# 在项目根目录执行，vcpkg 会自动读取 vcpkg.json
vcpkg install --triplet x64-windows-static-md
```

支持的 triplet:
- `x64-windows-static-md` - x64 静态链接 (推荐)
- `aarch64-windows-static-md` - ARM64 静态链接
- `x64-windows` - x64 动态链接

## 缓存配置

### 本地缓存

vcpkg 会自动在以下位置缓存构建的包:

- **Windows**: `%LOCALAPPDATA%\vcpkg`
- **Linux/macOS**: `~/.cache/vcpkg`

### CI/CD 缓存 (GitHub Actions)

我们的 GitHub Actions 工作流已配置自动缓存以提升构建速度:

```yaml
- name: Cache vcpkg packages and installed dependencies
  uses: actions/cache@v4
  with:
    path: |
      .\vcpkg\installed
      .\vcpkg\packages  
      .\vcpkg\buildtrees
    key: vcpkg-${{ runner.os }}-${{ matrix.settings.target }}-${{ hashFiles('vcpkg.json', 'vcpkg-configuration.json') }}
```

缓存键基于:
- 操作系统
- 目标架构 
- 依赖清单文件的哈希值

### 清理缓存

如果需要清理本地缓存:

```bash
# 清理特定包
vcpkg remove ffmpeg:x64-windows-static-md

# 清理所有已安装的包
vcpkg remove --recurse

# 清理构建缓存
rmdir /s vcpkg\buildtrees
rmdir /s vcpkg\packages
```

### 二进制缓存 (Binary Caching)

vcpkg 支持二进制缓存，可以显著减少重复构建的时间。

#### 本地二进制缓存

```bash
# 启用文件系统二进制缓存
vcpkg install --binarysource=files,C:\vcpkg-cache,readwrite

# 或者在环境变量中设置
set VCPKG_BINARY_SOURCES=files,C:\vcpkg-cache,readwrite
```

#### Azure 或其他云端缓存

对于团队开发，可以配置云端二进制缓存：

```bash
# Azure Blob Storage 示例
vcpkg install --binarysource=x-azurl,https://[account].blob.core.windows.net/[container],readwrite
```

详见 [vcpkg 二进制缓存文档](https://learn.microsoft.com/en-us/vcpkg/users/binarycaching)。

## 集成与 Rust

### Cargo.toml 配置

项目的 `src-tauri/Cargo.toml` 中启用了 vcpkg 集成:

```toml
[build-dependencies]
vcpkg = "0.2"
```

### build.rs 脚本

`src-tauri/build.rs` 文件配置了 vcpkg 库查找:

```rust
fn main() {
    #[cfg(target_os = "windows")]
    {
        if let Ok(_) = std::env::var("VCPKG_ROOT") {
            vcpkg::Config::new()
                .cargo_metadata(true)
                .probe("ffmpeg")
                .ok();
        }
    }
    
    tauri_build::build()
}
```

这确保了 Rust 编译器能够找到通过 vcpkg 安装的 FFmpeg 库。

## 故障排除

### 常见问题

1. **找不到 VCPKG_ROOT**: 确保环境变量正确设置
2. **包安装失败**: 检查网络连接，某些包需要从源码构建
3. **链接错误**: 确保使用了正确的 triplet (建议使用 static-md 变体)

### 调试命令

```bash
# 查看已安装的包
vcpkg list

# 查看可用的包
vcpkg search ffmpeg

# 显示包的详细信息
vcpkg info ffmpeg

# 检查集成状态
vcpkg integrate install
```

## 最佳实践

1. **使用清单模式**: 通过 `vcpkg.json` 管理依赖而不是手动安装
2. **固定基线**: 在 `vcpkg-configuration.json` 中指定基线以确保可重现构建
3. **选择合适的 triplet**: 推荐使用 static-md 变体以减少部署复杂性
4. **定期更新**: 定期更新 vcpkg 基线以获取安全修复和性能改进

## 参考链接

- [vcpkg 官方文档](https://github.com/Microsoft/vcpkg)
- [vcpkg 清单模式](https://github.com/Microsoft/vcpkg/blob/master/docs/users/manifests.md)
- [FFmpeg vcpkg 包](https://github.com/Microsoft/vcpkg/tree/master/ports/ffmpeg)