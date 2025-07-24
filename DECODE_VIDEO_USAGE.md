# decode_video 在前端的使用指南

## 概述

`decode_video` 是一个 Tauri 命令，用于在 Rust 后端进行视频流解码处理，并将数据流式传输到前端。这个命令特别适用于处理大型视频文件或需要实时处理的视频内容。

## 命令签名

```rust
#[command]
pub async fn decode_video(path: String, on_chunk: Channel) -> Result<String, String>
```

### 参数说明

- `path: String` - 视频文件的完整路径
- `on_chunk: Channel` - 用于接收视频数据块的通道
- 返回值：`Result<String, String>` - 成功时返回任务ID，失败时返回错误信息

## 前端使用方法

### 1. 基本使用示例

```typescript
import { invoke } from '@tauri-apps/api/core'

// 启动视频解码
async function startVideoDecoding(videoPath: string) {
    try {
        const taskId = await invoke('decode_video', {
            path: videoPath,
            onChunk: (chunk: Uint8Array) => {
                // 处理接收到的视频数据块
                console.log('Received chunk:', chunk.length, 'bytes')
                handleVideoChunk(chunk)
            }
        })

        console.log('Video decoding started with task ID:', taskId)
        return taskId
    } catch (error) {
        console.error('Failed to start video decoding:', error)
        throw error
    }
}

function handleVideoChunk(chunk: Uint8Array) {
    // 在这里处理视频数据块
    // 例如：保存到数组、创建Blob、实时播放等
}
```

### 2. Vue 组件中的完整实现

```vue
<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'

const videoData = ref<Uint8Array[]>([])
let taskId: string | null = null

onMounted(async () => {
    const videoPath = '/path/to/your/video.mp4'

    try {
        taskId = await invoke('decode_video', {
            path: videoPath,
            onChunk: (chunk: Uint8Array) => {
                videoData.value.push(chunk)

                // 当收集到足够数据时，创建播放源
                if (videoData.value.length > 10) {
                    createVideoBlob()
                }
            }
        })
    } catch (error) {
        console.error('Video decoding failed:', error)
    }
})

function createVideoBlob() {
    // 合并所有数据块
    const totalLength = videoData.value.reduce((sum, chunk) => sum + chunk.length, 0)
    const combined = new Uint8Array(totalLength)

    let offset = 0
    for (const chunk of videoData.value) {
        combined.set(chunk, offset)
        offset += chunk.length
    }

    // 创建 Blob URL
    const blob = new Blob([combined], { type: 'video/mp4' })
    const url = URL.createObjectURL(blob)

    // 使用 URL 播放视频
    const videoElement = document.querySelector('video')
    if (videoElement) {
        videoElement.src = url
    }
}

onBeforeUnmount(async () => {
    // 取消任务
    if (taskId) {
        try {
            await invoke('cancel_task', { taskId })
        } catch (error) {
            console.error('Failed to cancel task:', error)
        }
    }
})
</script>
```

### 3. 配合视频播放器使用

```typescript
import Player from 'xgplayer'

class VideoStreamPlayer {
    private player: Player | null = null
    private chunks: Uint8Array[] = []
    private taskId: string | null = null

    async startStreaming(videoPath: string, containerId: string) {
        try {
            this.taskId = await invoke('decode_video', {
                path: videoPath,
                onChunk: (chunk: Uint8Array) => {
                    this.chunks.push(chunk)
                    this.processChunk()
                }
            })
        } catch (error) {
            console.error('Failed to start video streaming:', error)
        }
    }

    private processChunk() {
        // 当收集到足够的数据时，创建播放器
        if (!this.player && this.chunks.length > 20) {
            this.createPlayer()
        }
    }

    private createPlayer() {
        // 合并数据块
        const blob = this.createBlob()
        const url = URL.createObjectURL(blob)

        this.player = new Player({
            id: 'video-container',
            url: url,
            width: '100%',
            height: '100%'
        })
    }

    private createBlob(): Blob {
        const totalLength = this.chunks.reduce((sum, chunk) => sum + chunk.length, 0)
        const combined = new Uint8Array(totalLength)

        let offset = 0
        for (const chunk of this.chunks) {
            combined.set(chunk, offset)
            offset += chunk.length
        }

        return new Blob([combined], { type: 'video/mp4' })
    }

    async stop() {
        if (this.taskId) {
            await invoke('cancel_task', { taskId: this.taskId })
        }

        if (this.player) {
            this.player.destroy()
        }
    }
}
```

## 相关命令

### 取消视频解码任务

```typescript
// 取消特定的视频解码任务
async function cancelVideoTask(taskId: string) {
    try {
        await invoke('cancel_task', { taskId })
        console.log('Task cancelled successfully')
    } catch (error) {
        console.error('Failed to cancel task:', error)
    }
}
```

### 获取活跃任务数量

```typescript
// 获取当前正在运行的视频任务数量
async function getActiveTasksCount() {
    try {
        const count = await invoke('get_active_tasks_count')
        console.log('Active tasks:', count)
        return count
    } catch (error) {
        console.error('Failed to get tasks count:', error)
        return 0
    }
}
```

## 注意事项

1. **内存管理**：流式处理会持续接收数据，注意及时清理不需要的数据块以避免内存泄漏。

2. **错误处理**：务必包含适当的错误处理逻辑，因为视频处理可能会失败。

3. **任务清理**：在组件卸载或页面离开时，记得取消正在进行的任务。

4. **数据格式**：接收到的数据块是原始的视频文件数据，可能需要根据具体需求进行处理。

5. **性能考虑**：对于大型视频文件，考虑分批处理数据块，避免一次性加载过多数据。

## 故障排除

### 常见问题

1. **Channel 参数错误**：确保 onChunk 参数是一个函数，而不是 Channel 对象。

2. **路径问题**：确保传递的视频文件路径是有效的完整路径。

3. **内存不足**：对于大型视频，考虑实现流式播放而不是一次性加载所有数据。

### 调试技巧

```typescript
// 启用详细日志
const taskId = await invoke('decode_video', {
    path: videoPath,
    onChunk: (chunk: Uint8Array) => {
        console.log(`Chunk received: ${chunk.length} bytes at ${new Date().toISOString()}`)
        // 处理数据块
    }
})
```
