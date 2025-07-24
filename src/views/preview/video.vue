<script setup lang="ts">
import { onMounted, ref, onBeforeUnmount } from 'vue'
import { useRoute } from 'vue-router'

import Player, { I18N } from 'xgplayer'
import 'xgplayer/dist/index.min.css'
import ZH from 'xgplayer/es/lang/zh-cn'

import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { convertFileSrc, invoke, Channel } from '@tauri-apps/api/core'

const route = useRoute()

// 启用中文
I18N.use(ZH)

defineOptions({
    name: 'VideoSupport',
})

const fileInfo = ref<FileInfo>()
let player: Player | null = null
let videoStreamTaskId: string | null = null

const videoData = ref<Uint8Array[]>([])

onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo

    try {
        console.log('🚀 Starting video streaming for:', fileInfo.value.path)

        // 方法 1: 尝试使用 Channel
        const onChunk = new Channel()
        console.log('📡 Channel created:', onChunk)

        // 监听 Channel 传来的数据
        onChunk.onmessage = (data: unknown) => {
            console.log('✅ Channel onmessage triggered!')
            console.log('Data received:', data)

            // 尝试不同的数据类型处理
            let chunk: Uint8Array
            if (data instanceof Uint8Array) {
                chunk = data
            } else if (data instanceof ArrayBuffer) {
                chunk = new Uint8Array(data)
            } else if (Array.isArray(data)) {
                chunk = new Uint8Array(data)
            } else {
                console.warn('❌ Unexpected data type:', typeof data, data)
                return
            }

            console.log('✅ Received video chunk:', chunk.length, 'bytes')
            videoData.value.push(chunk)
            console.log('📊 Total chunks received:', videoData.value.length)

            // 处理接收到的视频数据块
            processVideoChunk(chunk)
        }

        // 启动视频解码流
        console.log('🎬 Invoking decode_video command...')
        videoStreamTaskId = await invoke('decode_video', {
            path: fileInfo.value.path,
            onChunk: onChunk,
        })

        console.log('✅ Video streaming task started with ID:', videoStreamTaskId)

        // 设置超时检测，如果一段时间内没有收到数据，回退到直接播放
        setTimeout(() => {
            if (videoData.value.length === 0 && fileInfo.value) {
                console.warn('⚠️ No chunks received after 5 seconds, falling back to direct file access')
                const path = convertFileSrc(fileInfo.value.path)
                initializePlayer(path)
            }
        }, 5000)
    } catch (error) {
        console.error('Failed to start video streaming:', error)

        // 如果流式处理失败，回退到直接使用文件路径
        const path = convertFileSrc(fileInfo.value.path)
        initializePlayer(path)
    }
})

// 处理视频数据块
function processVideoChunk(chunk: Uint8Array) {
    // 记录接收到的数据块信息（用于调试）
    console.debug('Processing chunk of size:', chunk.length)

    // 这里可以实现自定义的视频数据处理逻辑
    // 例如：
    // 1. 将数据块合并成完整的视频文件
    // 2. 创建 Blob URL 用于播放
    // 3. 实时流式播放

    // 示例：当收集到足够的数据后创建播放器
    if (!player && videoData.value.length > 1) {
        // 将所有数据块合并成一个 Blob
        const combinedData = new Uint8Array(videoData.value.reduce((acc, curr) => acc + curr.length, 0))
        let offset = 0
        for (const data of videoData.value) {
            combinedData.set(data, offset)
            offset += data.length
        }

        const blob = new Blob([combinedData])
        console.log('Creating Blob with type:', blob)
        const url = URL.createObjectURL(blob)
        console.log('Creating player with Blob URL:', url, blob.type)
        initializePlayer(url)
    }
}

// 初始化播放器
function initializePlayer(url: string) {
    if (player !== null) {
        player.destroy()
        ;(document.querySelector('#videos') as HTMLElement).innerHTML = ''
    }

    player = new Player({
        id: 'videos',
        isLive: false,
        url: url,
        height: '100%',
        width: '100%',
    })
}

// 组件卸载时清理资源
onBeforeUnmount(async () => {
    if (videoStreamTaskId) {
        try {
            await invoke('cancel_task', { taskId: videoStreamTaskId })
        } catch (error) {
            console.error('Failed to cancel video stream task:', error)
        }
    }

    if (player) {
        player.destroy()
    }
})
</script>

<template>
    <LayoutPreview :file="fileInfo">
        <div class="video-support">
            <div class="video-support-inner">
                <div id="videos"></div>
            </div>
        </div>
    </LayoutPreview>
</template>

<style scoped lang="scss">
.video-support {
    width: 100%;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    align-content: center;
    &-inner {
        width: 100%;
        height: 100%;
    }
}
</style>
