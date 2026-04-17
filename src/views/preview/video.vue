<script setup lang="ts">
import { onMounted, ref, onBeforeUnmount } from 'vue'
import { useRoute } from 'vue-router'

import Player, { I18N } from 'xgplayer'
import 'xgplayer/dist/index.min.css'
import ZH from 'xgplayer/es/lang/zh-cn'
import HlsJsPlugin from 'xgplayer-hls.js'

import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { invoke } from '@tauri-apps/api/core'

const route = useRoute()

// 启用中文
I18N.use(ZH)

defineOptions({
    name: 'VideoSupport',
})

const fileInfo = ref<FileInfo>()
let player: Player | null = null

onMounted(async () => {
    fileInfo.value = route.query as unknown as FileInfo
    const result = await invoke('start_hls_process', { input: fileInfo.value.path })
    console.log('HLS process started:', result)

    // 等待第一个切片生成后再开始播放
    await waitForFirstSegment()
    initializePlayer('http://127.0.0.1:17878/playlist.m3u8')
})

// 等待第一个切片生成
async function waitForFirstSegment() {
    const maxWaitTime = 30000 // 最多等待30秒
    const checkInterval = 500 // 每500ms检查一次
    let elapsedTime = 0

    while (elapsedTime < maxWaitTime) {
        try {
            const response = await fetch('http://127.0.0.1:17878/playlist.m3u8')
            if (response.ok) {
                const content = await response.text()
                // 检查是否包含至少一个 .ts 文件
                if (content.includes('.ts')) {
                    console.log('First segment is ready, starting playback')
                    return
                }
            }
        } catch {
            // 服务器还没准备好，继续等待
        }

        await new Promise(resolve => setTimeout(resolve, checkInterval))
        elapsedTime += checkInterval
    }

    console.warn('Timeout waiting for first segment, starting playback anyway')
}

// 初始化播放器
function initializePlayer(url: string) {
    if (player !== null) {
        player.destroy()
    }

    player = new Player({
        id: 'videos',
        isLive: false,
        url: url,
        height: '100%',
        width: '100%',
        plugins: [HlsJsPlugin],
    })
}

// 组件卸载时清理资源
onBeforeUnmount(async () => {
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
