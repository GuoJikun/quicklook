# 视频转码实现说明

## 当前实现

我已经实现了两个版本的视频转码功能：

### 1. 静态链接版本（推荐）- `ffmpeg_simple.rs`

**特点**：

- ✅ **无需系统依赖**：FFmpeg 库直接编译进项目
- ✅ **智能转码**：自动判断是否需要转码
- ✅ **降级处理**：如果转码失败，自动回退到原文件传输
- ✅ **内存优化**：限制分辨率和比特率

**工作流程**：

1. 检查输入文件格式和参数
2. 如果是合适的 MP4 格式且分辨率适中，直接分块传输
3. 否则尝试使用内置转码功能
4. 如果转码失败，回退到原文件传输

### 2. 命令行版本 - `ffmpeg.rs`

**特点**：

- ⚠️ **需要系统 FFmpeg**：依赖系统安装的 FFmpeg 命令行工具
- ✅ **完整功能**：支持所有 FFmpeg 参数和功能
- ✅ **高质量转码**：使用经过优化的 FFmpeg 参数

## 主要特性

1. **格式转换**: 支持将各种视频格式（AVI, MKV, MOV, WMV 等）转换为 MP4
2. **优化参数**：
   - 使用 H.264 编码器
   - 限制最大比特率为 1Mbps，降低内存占用
   - 使用 ultrafast 预设，加快转码速度
   - 限制最大分辨率为 1280x720，减少文件大小
   - 使用 faststart 标志，优化网络流式传输

3. **流式传输**：
   - 将转码后的文件分成 64KB 的小块
   - 逐步发送到前端，避免内存溢出
   - 支持取消操作

### 技术实现

#### 后端 (Rust)

```rust
// 主要转码逻辑
async fn transcode_video(
    input_path: &str,
    sink: Channel,
    mut cancel_receiver: oneshot::Receiver<()>,
) -> Result<(), String>
```

**转码参数:**

- `-c:v libx264`: 使用 H.264 编码器
- `-preset ultrafast`: 快速编码
- `-crf 28`: 适中的质量设置
- `-maxrate 1M`: 限制最大比特率
- `-bufsize 2M`: 设置缓冲区大小
- `-vf scale=...`: 限制分辨率，保持宽高比
- `-movflags +faststart`: 优化流式传输

#### 前端 (Vue/TypeScript)

```typescript
// 使用转码后的视频流
const taskId = await invoke('decode_video', {
    path: videoPath,
    onChunk: (chunk: Uint8Array) => {
        // 处理接收到的MP4数据块
        videoData.value.push(chunk)
        
        // 当收集到足够数据时创建播放源
        if (videoData.value.length > 10) {
            createVideoBlob()
        }
    }
})
```

### 系统要求

#### 静态链接版本（当前使用）

- ✅ **无需外部依赖**：所有库都静态链接到项目中
- ✅ **跨平台**：支持 Windows、Linux、macOS
- ✅ **开箱即用**：无需安装额外软件

#### 命令行版本（备用）

1. **FFmpeg**: 系统需要安装 FFmpeg 命令行工具
2. **依赖项**：
   - `ffmpeg-next`: FFmpeg Rust 绑定
   - `tempfile`: 临时文件管理
   - `tokio`: 异步运行时

### 安装 FFmpeg

#### Windows

```bash
# 使用 Scoop
scoop install ffmpeg

# 使用 Chocolatey
choco install ffmpeg

# 或直接下载二进制文件
# 从 https://ffmpeg.org/download.html 下载
```

#### Linux

```bash
# Ubuntu/Debian
sudo apt install ffmpeg

# CentOS/RHEL
sudo yum install ffmpeg
```

#### macOS

```bash
# 使用 Homebrew
brew install ffmpeg
```

### 内存优化

1. **流式处理**: 不会一次性加载整个视频文件到内存
2. **分块传输**: 64KB 小块，适合网络传输
3. **临时文件**: 使用临时文件避免内存累积
4. **参数优化**: 限制比特率和分辨率

### 支持的格式

输入格式（常见的）:

- MP4, AVI, MKV, MOV, WMV, FLV
- MPEG, 3GP, WEBM, OGV
- 以及 FFmpeg 支持的其他格式

输出格式:

- MP4 (H.264 + AAC)

### 错误处理

1. **文件验证**: 转码前验证输入文件
2. **格式检查**: 确保包含视频流
3. **命令执行**: 检查 FFmpeg 命令执行状态
4. **取消机制**: 支持中途取消转码任务

### 使用示例

```typescript
// 前端调用示例
try {
    const taskId = await invoke('decode_video', {
        path: '/path/to/video.avi',
        onChunk: (chunk: Uint8Array) => {
            console.log(`Received ${chunk.length} bytes`)
            // 处理数据chunks
        }
    })
    
    console.log('Task started:', taskId)
} catch (error) {
    console.error('Transcode failed:', error)
}
```

### 性能优化建议

1. **预设选择**: 根据需求调整 FFmpeg 预设
   - `ultrafast`: 最快速度，较大文件
   - `fast`: 平衡速度和质量
   - `medium`: 默认设置

2. **质量设置**: 调整 CRF 值
   - `18-23`: 高质量
   - `24-28`: 标准质量（推荐）
   - `29-35`: 较低质量，小文件

3. **分辨率限制**: 根据目标设备调整最大分辨率

### 故障排除

1. **FFmpeg 未找到**: 确保 FFmpeg 在系统 PATH 中
2. **格式不支持**: 检查输入文件格式
3. **内存不足**: 降低最大比特率或分辨率
4. **转码失败**: 查看日志中的详细错误信息
