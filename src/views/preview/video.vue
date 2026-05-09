<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { useRoute } from 'vue-router'

import { info } from '@tauri-apps/plugin-log'

import Player, { I18N } from 'xgplayer'
import 'xgplayer/dist/index.min.css'
import ZH from 'xgplayer/es/lang/zh-cn'

import type { FileInfo } from '@/utils/typescript'
import LayoutPreview from '@/components/layout-preview.vue'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { load } from '@tauri-apps/plugin-store'
import { getCurrentWindow } from '@tauri-apps/api/window'

const route = useRoute()
let unlistenWindowClose: (() => void) | null = null
let disposed = false

const cleanupPlayer = () => {
    if (player !== null) {
        player.destroy()
        player = null
    }
}

const listenWindowClose = async () => {
    const currentWindow = getCurrentWindow()
    unlistenWindowClose = await currentWindow.onCloseRequested(() => {
        disposed = true
        // 清理逻辑
        info('触发 onCloseRequested 生命周期，正在清理资源...')
        // 如果窗口关闭或切换文件时转码仍在进行，通知后端终止 ffmpeg 进程

        invoke('cancel_video_conversion').catch(err => console.error('停止 ffmpeg 转换失败:', err))
        cleanupPlayer()
        // 不阻止则继续关闭
    })
}

// 启用中文
I18N.use(ZH)

defineOptions({
    name: 'VideoSupport',
})

const fileInfo = ref<FileInfo>()
let player: Player | null = null
const converting = ref(false)
const convertError = ref<string | null>(null)

const initPlayer = (url: string, isHls = false) => {
    if (player !== null) {
        player.destroy()
        ;(document.querySelector('#videos') as HTMLElement).innerHTML = ''
    }
    if (isHls) {
        // 动态加载 xgplayer-hls 插件以支持 m3u8 播放
        import('xgplayer-hls').then(mod => {
            console.log('xgplayer-hls 插件加载成功', mod)
            const HlsPlugin = mod.default
            player = new Player({
                id: 'videos',
                isLive: false,
                url,
                height: '100%',
                width: '100%',
                plugins: [HlsPlugin],
            })
        })
    } else {
        player = new Player({
            id: 'videos',
            isLive: false,
            url,
            height: '100%',
            width: '100%',
        })
    }
}

onMounted(async () => {
    await listenWindowClose()

    fileInfo.value = route.query as unknown as FileInfo
    const filePath = fileInfo.value.path

    // 检查是否启用了本机 ffmpeg 解析
    const store = await load('config.data', { autoSave: false, defaults: {} })
    const useLocalFfmpeg = (await store.get<boolean>('useLocalFfmpeg')) ?? false

    if (useLocalFfmpeg) {
        console.log('启用本机 ffmpeg 解析，正在转换视频...')
        converting.value = true
        convertError.value = null
        try {
            const m3u8Path = await invoke<string>('convert_video_to_hls', { path: filePath })
            if (disposed) {
                return
            }
            console.log('视频转换完成，m3u8 文件路径:', m3u8Path)
            const m3u8Url = convertFileSrc(m3u8Path)
            console.log('视频转换完成，开始播放...', m3u8Url)
            initPlayer(m3u8Url, true)
        } catch (e: unknown) {
            if (disposed) {
                return
            }
            convertError.value = e instanceof Error ? e.message : String(e)
            // 回退到直接播放
            initPlayer(convertFileSrc(filePath), false)
        } finally {
            converting.value = false
        }
    } else {
        console.log('直接播放视频，不使用 ffmpeg 转换')
        initPlayer(convertFileSrc(filePath), false)
    }
})

onUnmounted(() => {
    disposed = true
    invoke('cancel_video_conversion').catch(err => console.error('停止 ffmpeg 转换失败:', err))
    cleanupPlayer()
    if (unlistenWindowClose !== null) {
        unlistenWindowClose()
        unlistenWindowClose = null
    }
})
</script>

<template>
    <LayoutPreview :file="fileInfo">
        <div class="video-support">
            <el-alert
                v-if="convertError"
                :title="'ffmpeg 转换失败，已回退到直接播放：' + convertError"
                type="warning"
                :closable="false"
                style="margin-bottom: 8px"
            />
            <div
                class="video-support-inner"
                v-loading="converting"
                element-loading-text="正在使用 ffmpeg 转换视频，请稍候…"
            >
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
    flex-direction: column;
    justify-content: center;
    align-items: center;
    align-content: center;
    &-inner {
        width: 100%;
        height: 100%;
    }
}
.video-converting {
    padding: 8px 0;
    text-align: center;
}
</style>
